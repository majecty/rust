//! 진단 출력: E0428 + 에러 체인展开.

/// resolve 진단을 종류별로 출력한다 (렌더링이 불가하면 원인까지 fallback).
pub fn print_resolve_error(file: &str, src: &str, e: &rtoy_resolve::ResolveError) {
    match e.kind {
        rtoy_resolve::ResolveKind::Duplicate => print_duplicate(file, src, e),
        rtoy_resolve::ResolveKind::UndefinedVar => print_undefined(file, src, e),
    }
}

/// rustc E0428 스타일: 같은 이름을 두 번 정의.
fn print_duplicate(file: &str, src: &str, e: &rtoy_resolve::ResolveError) {
    log_span_backtrace("dup", &e.span);
    let Some(first_span) = e.first_span else {
        print_plain(e);
        return;
    };
    log_span_backtrace("first", &first_span);
    let dup_loc = rtoy_span::offset_to_line_col(src, e.span.lo);
    let first_caret = rtoy_span::caret_line(src, first_span);
    let dup_caret = rtoy_span::caret_line(src, e.span);
    match (
        dup_loc,
        first_caret,
        dup_caret,
    ) {
        (
            Some((line, col)),
            Some((first_line, first_text, first_caret)),
            Some((_, dup_text, dup_caret)),
        ) => {
            eprintln!(
                "error[E0428]: the name `{}` is defined multiple times",
                e.name
            );
            eprintln!(" --> {file}:{line}:{col}");
            eprintln!("{:>3} |", "");
            eprintln!("{:>3} | {first_text}", first_line);
            // caret 본문은 접두사 없음 — 줄번호 렌더링 폭({:>3} | )과 같은 폭으로 조립
            eprintln!(
                "{:>3} | {} previous definition of the value `{}` here",
                "", first_caret, e.name
            );
            eprintln!("{:>3} | {dup_text}", line);
            eprintln!("{:>3} | {} `{}` redefined here", "", dup_caret, e.name);
            eprintln!("{:>3} |", "");
            eprintln!(
                "  = note: `{}` must be defined only once in the value namespace of this module",
                e.name
            );
        }
        _ => print_plain(e),
    }
}

/// rustc E0425 스타일: 이 함수 프레임에 없는 지역변수 참조.
fn print_undefined(file: &str, src: &str, e: &rtoy_resolve::ResolveError) {
    log_span_backtrace("undefined", &e.span);
    match (rtoy_span::offset_to_line_col(src, e.span.lo), rtoy_span::caret_line(src, e.span)) {
        (Some((line, col)), Some((_, text, caret))) => {
            eprintln!("error[E0425]: cannot find value `{}` in this function frame", e.name);
            eprintln!(" --> {file}:{line}:{col}");
            eprintln!("{:>3} |", "");
            eprintln!("{:>3} | {text}", line);
            eprintln!("{:>3} | {caret} not found in this scope", "");
            eprintln!("{:>3} |", "");
            eprintln!("  = help: 지역변수는 `let`으로 먼저 선언해야 함 (함수 파라미터는 아직 없음)");
        }
        _ => print_plain(e),
    }
}

/// span 렌더링이 불가능할 때 가장 안쪽 원인까지 노출한다.
fn print_plain(e: &rtoy_resolve::ResolveError) {
    eprintln!("error: {e}");
    if let Some(src_err) = std::error::Error::source(e) {
        eprintln!("caused by: {src_err}");
    }
}

fn log_span_backtrace(tag: &str, sp: &rtoy_span::Span) {
    eprintln!("[backtrace] {tag} span=[{}..{}@c{}] chain={}", sp.lo, sp.hi, sp.ctxt.0, sp.chain());
    let mut cur = *sp;
    let mut depth = 0;
    while let Some(d) = cur.expansion() {
        eprintln!("[backtrace] {tag} depth={depth} call=[{}..{}] parent=[{}..{}]", d.call_site.lo, d.call_site.hi, d.parent_span.lo, d.parent_span.hi);
        cur = d.parent_span;
        depth += 1;
        if depth > 8 { break; }
    }
    if depth == 0 {
        eprintln!("[backtrace] {tag} depth=0 root, no expansion parent");
    }
}

/// 에러 체인을 가장 안쪽까지 stderr에 노출한다.
/// e.g. `error: lowering failed: ... caused by: span [..] ...`
pub fn print_error_chain(top: &str, err: &dyn std::error::Error) {
    eprintln!("error: {top}: {err}");
    let mut src = err.source();
    while let Some(e) = src {
        eprintln!("caused by: {e}");
        src = e.source();
    }
}
