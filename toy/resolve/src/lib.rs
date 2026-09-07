//! rtoy resolve — AST 이름 검사 (rustc `compiler/rustc_resolve`의 최소 부분).
//! 첫 단계: 함수 이름 중복 정의를 찾아 span과 원문 스니펫을 에러로 노출한다.

use rtoy_ast::Crate;
use rtoy_span::{Span, SpanError};

/// 이름 중복 정의. 두 정의의 span·스니펫을 모두 담아 진단에 쓰인다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    pub context: &'static str,
    pub name: String,
    /// 중복(두 번째) 정의 span — 에러를 가리키는 지점.
    pub span: Span,
    /// 첫 번째 정의 span — "여기서 이미 정의됨" 힌트.
    pub first_span: Span,
    /// 두 span의 원문. src가 없으면 `<invalid span>`.
    pub snippet: String,
    pub first_snippet: String,
    pub source: Option<SpanError>,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: duplicate definition of `{}` (first: {:?} at [{}..{}], duplicate: {:?} at [{}..{}])",
            self.context, self.name, self.first_snippet, self.first_span.start, self.first_span.end,
            self.snippet, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ResolveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e as &dyn std::error::Error)
    }
}

fn snippet_of(span: &Span, src: &str) -> String {
    span.try_snippet(src).unwrap_or("<invalid span>").to_string()
}

/// 함수 이름 중복 검사. 첫 정의는 두고, 이후 중복마다 에러를 모은다.
pub fn resolve(krate: &Crate, src: &str) -> Result<(), Vec<ResolveError>> {
    let mut seen: Vec<(&str, Span)> = Vec::new();
    let mut errs = Vec::new();
    for item in &krate.items {
        if let Some((_, first_span)) = seen.iter().find(|(n, _)| *n == item.name.name) {
            errs.push(ResolveError {
                context: "resolve",
                name: item.name.name.clone(),
                span: item.name.span,
                first_span: *first_span,
                snippet: snippet_of(&item.name.span, src),
                first_snippet: snippet_of(first_span, src),
                source: None,
            });
        } else {
            seen.push((&item.name.name, item.name.span));
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtoy_lexer::tokenize;
    use rtoy_tokenstream_lowering::try_lower;

    #[test]
    fn unique_fn_names_pass() {
        let src = "fn main() { 1 } fn foo() { 2 }";
        let krate = try_lower(&tokenize(src), src).unwrap();
        assert!(resolve(&krate, src).is_ok());
    }

    #[test]
    fn duplicate_fn_names_error() {
        let src = "fn main() { 1 } fn main() { 2 }";
        let krate = try_lower(&tokenize(src), src).unwrap();
        let errs = resolve(&krate, src).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].name, "main");
        assert_eq!(errs[0].span.snippet(src), "main");
        assert_eq!(errs[0].first_span.snippet(src), "main");
        assert!(errs[0].span.start > errs[0].first_span.start);
        let msg = errs[0].to_string();
        assert!(msg.contains("duplicate definition of `main`"), "{msg}");
        assert!(msg.contains("[19..23]"), "{msg}");
    }
}
