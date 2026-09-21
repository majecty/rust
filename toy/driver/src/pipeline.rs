//! 실행 파이프라인: 파일 읽기 -> lex -> lowering -> expand -> resolve.

use crate::args;
use crate::diagnostics::{print_error_chain, print_resolve_error};
use crate::trace::trace_crate;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;

pub fn run(args: &[String]) -> i32 {
    let cli = match args::parse(args) {
        Ok(Some(cli)) => cli,
        Ok(None) => {
            print_help();
            return EXIT_SUCCESS;
        }
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("error: {msg}");
            }
            print_help();
            return EXIT_FAILURE;
        }
    };
    let ast_only = cli.ast_only;
    let mut owned_path = String::new();
    let src_path = resolve_src_path(&cli, &mut owned_path);
    let src = match read_src(&src_path) {
        Ok(src) => src,
        Err(code) => return code,
    };
    let tokens = rtoy_lexer::tokenize(&src);
    if cli.lex_only {
        return run_lex_only(&tokens, &src);
    }
    run_pipeline(&src_path, &src, cli.trace, ast_only)
}

fn run_pipeline(src_path: &str, src: &str, trace: bool, ast_only: bool) -> i32 {
    let tokens = rtoy_lexer::tokenize(src);
    if trace {
        println!("== lex ({} tokens) ==", tokens.len());
        for tok in &tokens {
            let text = tok.span.try_snippet(src).unwrap_or("<invalid>");
            println!(
                "{:?} [{}..{}] {:?}",
                tok.kind, tok.span.lo, tok.span.hi, text
            );
        }
    }
    let krate = match rtoy_tokenstream_lowering::try_lower(&tokens, src) {
        Ok(krate) => krate,
        Err(e) => {
            print_error_chain("lowering failed", &e);
            return EXIT_FAILURE;
        }
    };
    if trace {
        trace_crate("lowering (before expand)", &krate, src);
    }
    let krate = match rtoy_expand::expand_crate(krate) {
        Ok(k) => k,
        Err(e) => {
            print_error_chain("expand failed", &e);
            return EXIT_FAILURE;
        }
    };
    if trace {
        trace_crate("expand (after expand)", &krate, src);
    }
    if let Err(errs) = rtoy_resolve::resolve(&krate, src) {
        for e in &errs {
            print_resolve_error(src_path, src, e);
        }
        return EXIT_FAILURE;
    }
    if trace {
        trace_crate("final", &krate, src);
    }
    if ast_only {
        println!("ast: {:#?}", krate);
        return EXIT_SUCCESS;
    }
    match rtoy_eval::eval_crate(&krate) {
        Ok(value) => println!("value: {value}"),
        Err(e) => {
            print_error_chain("eval failed", &e);
            return EXIT_FAILURE;
        }
    }
    EXIT_SUCCESS
}

fn run_lex_only(tokens: &[rtoy_lexer::Token], src: &str) -> i32 {
    for tok in tokens {
        match tok.span.try_snippet(src) {
            Ok(text) => println!(
                "{:?} [{}..{}] {:?}",
                tok.kind, tok.span.lo, tok.span.hi, text
            ),
            Err(span_err) => {
                print_error_chain("lex snippet failed", &span_err);
                return EXIT_FAILURE;
            }
        }
    }
    EXIT_SUCCESS
}

fn read_src(src_path: &str) -> Result<String, i32> {
    match std::fs::read_to_string(src_path) {
        Ok(src) => Ok(src),
        Err(e) => {
            eprintln!(
                "error: cannot read {src_path}: {e} (kind={:?}, os_code={:?})",
                e.kind(),
                e.raw_os_error()
            );
            if let Some(src) = std::error::Error::source(&e) {
                let mut cur: Option<&dyn std::error::Error> = Some(src);
                while let Some(inner) = cur {
                    eprintln!("caused by: {inner}");
                    cur = inner.source();
                }
            }
            Err(EXIT_FAILURE)
        }
    }
}

fn resolve_src_path(cli: &args::CliArgs, owned: &mut String) -> String {
    match (&cli.sample, &cli.path) {
        (Some(name), _) => {
            let file = name.strip_suffix(".rs").unwrap_or(name);
            *owned = samples_dir()
                .join(format!("{file}.rs"))
                .to_string_lossy()
                .into_owned();
            owned.clone()
        }
        (None, Some(p)) => p.clone(),
        (None, None) => unreachable!("args::parse가 입력 없음을 이미 거부함"),
    }
}

fn print_help() {
    println!("rtoy [--lex|--ast] <file.rs> | --sample <name> — minimal rustc_driver toy");
    println!("  --lex: lex 결과물만 출력하고 종료");
    println!("  --ast: AST만 출력하고 종료");
    println!("  (기본) main을 eval 실행해 `value:` 출력");
    println!("  --sample <name>: toy/samples/<name>.rs 실행");
    println!("  --trace: lex→lowering(before)→expand(after) 단계별 출력");
    println!("  passes (stub): lex -> parse -> ast_lower -> hir -> done");
    let names = sample_names();
    if names.is_empty() {
        println!("  samples: (없음 — toy/samples/*.rs)");
    } else {
        println!("  samples: {}", names.join(", "));
    }
}

/// samples/*.rs 이름 목록 (확장자 제외, 정렬). 디렉터리가 없으면 빈 목록.
fn sample_names() -> Vec<String> {
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(samples_dir()) else {
        return names;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().is_some_and(|e| e == "rs") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            names.push(stem.to_string());
        }
    }
    names.sort();
    names
}

/// 컴파일 시점 driver 위치 기준 samples 디렉터리 (`toy/samples`).
fn samples_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../samples"))
}
