//! 진단 출력: E0428 + 에러 체인展开.

/// rustc E0428 스타일 진단. 실패 시 가장 안쪽 원인까지 fallback으로 노출한다.
pub fn print_resolve_error(file: &str, src: &str, e: &rtoy_resolve::ResolveError) {
    log_span_backtrace("dup", &e.span);
    log_span_backtrace("first", &e.first_span);
    let dup_loc = rtoy_span::offset_to_line_col(src, e.span.lo);
    let first_caret = rtoy_span::caret_line(src, e.first_span);
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
        _ => {
            eprintln!("error: {e}");
            if let Some(src_err) = std::error::Error::source(e) {
                eprintln!("caused by: {src_err}");
            }
        }
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
