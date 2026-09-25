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
    let mut owned_path = String::new();
    let src_path = resolve_src_path(&cli, &mut owned_path);
    let src = match read_src(&src_path) {
        Ok(src) => src,
        Err(code) => return code,
    };
    if cli.lex_only {
        let tokens = rtoy_lexer::tokenize(&src);
        return run_lex_only(&tokens, &src);
    }
    run_pipeline(&src_path, &src, &cli)
}

fn run_pipeline(src_path: &str, src: &str, cli: &args::CliArgs) -> i32 {
    let trace = cli.trace;
    let mir_steps = cli.mir_steps;
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
    let mut krate = krate;
    if trace {
        trace_crate("expand (after expand)", &krate, src);
    }
    if let Err(errs) = rtoy_resolve::resolve(&mut krate, src) {
        for e in &errs {
            print_resolve_error(src_path, src, e);
        }
        return EXIT_FAILURE;
    }
    if trace {
        trace_crate("final", &krate, src);
    }
    if cli.ast_only {
        println!("ast: {:#?}", krate);
        return EXIT_SUCCESS;
    }
    if cli.hir_only || cli.thir_only || cli.thir_tree {
        return run_hir_thir(&krate, cli);
    }
    if cli.mir_only || cli.mir_eval || mir_steps.is_some() {
        let mir = match rtoy_ast_lowering::lower_crate(&krate) {
            Ok(mir) => mir,
            Err(e) => {
                print_error_chain("mir lowering failed", &e);
                return EXIT_FAILURE;
            }
        };
        if cli.mir_only {
            print!("{}", mir.dump());
            return EXIT_SUCCESS;
        }
        if let Some(budget) = mir_steps {
            return run_mir_steps(&mir, budget);
        }
        return match rtoy_eval::eval_mir(&mir) {
            Ok(rt) => {
                println!("value: {}", rt.format_value(&rt.value));
                EXIT_SUCCESS
            }
            Err(e) => {
                print_error_chain("mir eval failed", &e);
                EXIT_FAILURE
            }
        };
    }
    match rtoy_eval::eval_crate(&krate) {
        Ok(rt) => println!("value: {}", rt.format_value(&rt.value)),
        Err(e) => {
            print_error_chain("eval failed", &e);
            return EXIT_FAILURE;
        }
    }
    EXIT_SUCCESS
}

/// `--hir`/`--thir`/`--thir-tree` — 해석·desugar된 트리 덤프 (rustc `-Zunpretty=hir`/`thir-flat`/`thir-tree` 흔내).
fn run_hir_thir(krate: &rtoy_ast::Crate, cli: &args::CliArgs) -> i32 {
    let hir = match rtoy_hir::lower(krate) {
        Ok(hir) => hir,
        Err(e) => {
            print_error_chain("hir lowering failed", &e);
            return EXIT_FAILURE;
        }
    };
    if cli.hir_only {
        print!("{}", hir.dump());
        return EXIT_SUCCESS;
    }
    match rtoy_thir::lower_crate(&hir) {
        Ok(thir) => {
            print!("{}", if cli.thir_tree { thir.dump_tree() } else { thir.dump() });
            EXIT_SUCCESS
        }
        Err(errs) => {
            for e in &errs {
                print_error_chain("thir lowering failed", e);
            }
            EXIT_FAILURE
        }
    }
}

/// `--mir-steps=N` 출력 — 실행 경로/현재 위치/값을 `==` 마커로 나눈다 (웹 서버가 파싱한다).
fn run_mir_steps(mir: &rtoy_mir::MirCrate, budget: u64) -> i32 {
    let run = match rtoy_eval::eval_mir_traced(mir, budget) {
        Ok(run) => run,
        Err(e) => {
            print_error_chain("mir eval failed", &e);
            return EXIT_FAILURE;
        }
    };
    println!("== trace ==");
    for ev in &run.trace {
        let slot = ev.stmt.map(|i| format!(":{i}")).unwrap_or_default();
        println!("#{} d{} {} bb{}{} {} | {}", ev.index, ev.depth, ev.fn_name, ev.bb, slot, ev.kind.as_str(), ev.text);
    }
    println!("== cursor ==");
    match &run.cursor {
        Some(c) => {
            println!("fn={} bb={} depth={} steps={} budget={} halted={}", c.fn_name, c.bb, c.depth, run.steps, run.budget, run.halted as u8);
            for (name, value) in &c.locals {
                println!("{name} = {value}");
            }
        }
        None => println!("fn=? bb=0 depth=0 steps={} budget={} halted={}", run.steps, run.budget, run.halted as u8),
    }
    println!("== value ==");
    if run.halted {
        println!("(실행 중: {}스텝)", run.steps);
    } else {
        println!("value: {}", run.runtime.format_value(&run.runtime.value));
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
    println!("rtoy [--lex|--ast|--hir|--thir|--thir-tree|--mir|--mir-eval|--mir-steps=N] <file.rs> | --sample <name> — minimal rustc_driver toy");
    println!("  --lex: lex 결과물만 출력하고 종료");
    println!("  --ast: AST만 출력하고 종료");
    println!("  --hir: HIR(desugar+이름해석)만 출력하고 종료 (rustc -Zunpretty=hir 흥내)");
    println!("  --thir: THIR 평탄 arena 덤프 (rustc -Zunpretty=thir-flat 흥내)");
    println!("  --thir-tree: THIR을 body에서 재귀 전개한 트리로 출력 (rustc -Zunpretty=thir-tree 흥내)");
    println!("  --mir: MIR 덤프만 출력하고 종료 (rustc -Zunpretty=mir 흉내)");
    println!("  --mir-eval: MIR을 실행 (기본은 AST eval)");
    println!("  --mir-steps=N: MIR을 N스텝만 실행하고 실행 경로·현재 위치·지역변수 출력");
    println!("  (기본) main을 eval 실행해 `value:` 출력");
    println!("  --sample <name>: toy/samples/<name>.rs 실행");
    println!("  --trace: lex→lowering(before)→expand(after) 단계별 출력");
    println!("  passes: lex -> parse(ast) -> expand -> resolve -> hir -> thir -> mir");
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
