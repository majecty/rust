//! rtoy driver — minimal counterpart to `compiler/rustc_driver`.
//! 흐름: main -> run(args) -> read file -> lex -> (stub) parse/lower.

use std::process::ExitCode;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;

/// 컴파일 시점 driver 위치 기준 samples 디렉터리 (`toy/samples`).
fn samples_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../samples"))
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

fn print_help() {
    println!("rtoy [--lex|--ast] <file.rs> | --sample <name> — minimal rustc_driver toy");
    println!("  --lex: lex 결과물만 출력하고 종료");
    println!("  --ast: AST까지 출력하고 종료 (기본 동작과 동일, 명시용)");
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

/// rustc E0428 스타일 진단. 실패 시 가장 안쪽 원인까지 fallback으로 노출한다.
fn print_resolve_error(file: &str, src: &str, e: &rtoy_resolve::ResolveError) {
    let dup_loc = rtoy_span::offset_to_line_col(src, e.span.lo);
    let first_caret = rtoy_span::caret_line(src, e.first_span);
    let dup_caret = rtoy_span::caret_line(src, e.span);
    match (dup_loc, first_caret, dup_caret) {
        (Some((line, col)), Some((first_line, first_text, first_caret)), Some((_, dup_text, dup_caret))) => {
            eprintln!("error[E0428]: the name `{}` is defined multiple times", e.name);
            eprintln!(" --> {file}:{line}:{col}");
            eprintln!("{:>3} |", "");
            eprintln!("{:>3} | {first_text}", first_line);
            // caret 본문은 접두사 없음 — 줄번호 렌더링 폭({:>3} | )과 같은 폭으로 조립
            eprintln!("{:>3} | {} previous definition of the value `{}` here", "", first_caret, e.name);
            eprintln!("{:>3} | {dup_text}", line);
            eprintln!("{:>3} | {} `{}` redefined here", "", dup_caret, e.name);
            eprintln!("{:>3} |", "");
            eprintln!("  = note: `{}` must be defined only once in the value namespace of this module", e.name);
        }
        _ => {
            eprintln!("error: {e}");
            if let Some(src_err) = std::error::Error::source(e) {
                eprintln!("caused by: {src_err}");
            }
        }
    }
}
/// 에러 체인을 가장 안쪽까지 stderr에 노출한다.
/// e.g. `error: lowering failed: ... caused by: span [..] ...`
fn print_error_chain(top: &str, err: &dyn std::error::Error) {
    eprintln!("error: {top}: {err}");
    let mut src = err.source();
    while let Some(e) = src {
        eprintln!("caused by: {e}");
        src = e.source();
    }
}

/// trace용 한 줄 요약. 장황한 {:#?} 대신 snippet+span만 노출한다.
fn short_span(s: &rtoy_span::Span) -> String {
    format!("[{}..{}]", s.lo, s.hi)
}

fn expr_sum(e: &rtoy_ast::Expr, src: &str) -> String {
    let snip = e.span.try_snippet(src).unwrap_or("<invalid>");
    let kind = match &e.kind {
        rtoy_ast::ExprKind::Int(n) => format!("Int({n})"),
        rtoy_ast::ExprKind::Var(v) => format!("Var({})", v.name),
        rtoy_ast::ExprKind::Call { callee, args } => format!("Call({}/{})", callee.name, args.len()),
        rtoy_ast::ExprKind::Binary { op, .. } => format!("Binary({op:?})"),
        rtoy_ast::ExprKind::Macro { name, args } => format!("Macro({}!/{})", name.name, args.len()),
    };
    format!("{kind} {snip:?} {}", short_span(&e.span))
}

fn trace_crate(label: &str, krate: &rtoy_ast::Crate, src: &str) {
    println!("== {label} ==");
    for item in &krate.items {
        println!("fn {} {}", item.name.name, short_span(&item.span));
        let rtoy_ast::ItemKind::Fn(f) = &item.kind;
        for s in &f.body.stmts {
            match &s.kind {
                rtoy_ast::StmtKind::Let(l) => println!("  let {} = {} {}", l.name.name, l.init.as_ref().map(|e| expr_sum(e, src)).unwrap_or("-".into()), short_span(&s.span)),
                rtoy_ast::StmtKind::Expr(e) => println!("  expr {} {}", expr_sum(e, src), short_span(&s.span)),
            }
        }
        match &f.body.tail {
            Some(t) => println!("  tail {}", expr_sum(t, src)),
            None => println!("  tail -")
        }
    }
}

fn run(args: &[String]) -> i32 {
    let mut lex_only = false;
    let mut ast_only = false;
    let mut trace = false;
    let mut sample: Option<String> = None;
    let mut path: Option<&String> = None;
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--lex" {
            lex_only = true;
        } else if arg == "--ast" {
            ast_only = true;
        } else if arg == "--trace" {
            trace = true;
        } else if arg == "--sample" {
            i += 1;
            match args.get(i) {
                Some(name) => sample = Some(name.clone()),
                None => {
                    eprintln!("error: --sample 뒤에 이름이 필요함");
                    print_help();
                    return EXIT_FAILURE;
                }
            }
        } else if arg == "--help" || arg == "-h" {
            print_help();
            return EXIT_SUCCESS;
        } else if path.is_none() {
            path = Some(arg);
        } else {
            print_help();
            return EXIT_FAILURE;
        }
        i += 1;
    }
    if lex_only && ast_only {
        eprintln!("error: --lex와 --ast는 함께 쓸 수 없음");
        print_help();
        return EXIT_FAILURE;
    }
    if sample.is_some() && path.is_some() {
        eprintln!("error: --sample과 파일 인자는 함께 쓸 수 없음");
        print_help();
        return EXIT_FAILURE;
    }
    let owned_path: String;
    let src_path: &str = match (&sample, path) {
        (Some(name), _) => {
            let file = name.strip_suffix(".rs").unwrap_or(name);
            owned_path = samples_dir().join(format!("{file}.rs")).to_string_lossy().into_owned();
            &owned_path
        }
        (None, Some(p)) => p.as_str(),
        (None, None) => {
            print_help();
            return EXIT_FAILURE;
        }
    };
    let src = match std::fs::read_to_string(src_path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("error: cannot read {src_path}: {e} (kind={:?}, os_code={:?})", e.kind(), e.raw_os_error());
            if let Some(src) = std::error::Error::source(&e) {
                let mut cur: Option<&dyn std::error::Error> = Some(src);
                while let Some(inner) = cur {
                    eprintln!("caused by: {inner}");
                    cur = inner.source();
                }
            }
            return EXIT_FAILURE;
        }
    };
    let tokens = rtoy_lexer::tokenize(&src);
    if lex_only {
        for tok in &tokens {
            match tok.span.try_snippet(&src) {
                Ok(text) => println!("{:?} [{}..{}] {:?}", tok.kind, tok.span.lo, tok.span.hi, text),
                Err(span_err) => {
                    print_error_chain("lex snippet failed", &span_err);
                    return EXIT_FAILURE;
                }
            }
        }
        return EXIT_SUCCESS;
    }
    if trace {
        println!("== lex ({} tokens) ==", tokens.len());
        for tok in &tokens {
            let text = tok.span.try_snippet(&src).unwrap_or("<invalid>");
            println!("{:?} [{}..{}] {:?}", tok.kind, tok.span.lo, tok.span.hi, text);
        }
    }
    match rtoy_tokenstream_lowering::try_lower(&tokens, &src) {
        Ok(krate) => {
            if trace {
                trace_crate("lowering (before expand)", &krate, &src);
            }
            let krate = match rtoy_expand::expand_crate(krate) {
                Ok(k) => k,
                Err(e) => {
                    print_error_chain("expand failed", &e);
                    return EXIT_FAILURE;
                }
            };
            if trace {
                trace_crate("expand (after expand)", &krate, &src);
            }
            if let Err(errs) = rtoy_resolve::resolve(&krate, &src) {
                for e in &errs {
                    print_resolve_error(src_path, &src, e);
                }
                return EXIT_FAILURE;
            }
            if trace {
                trace_crate("final", &krate, &src);
            } else {
                println!("ast: {:#?}", krate);
            }
            // TODO: parser -> ast_lowering (stub for now)
            EXIT_SUCCESS
        }
        Err(e) => {
            print_error_chain("lowering failed", &e);
            EXIT_FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    ExitCode::from(run(&args) as u8)
}
