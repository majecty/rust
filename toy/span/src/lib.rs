//! rtoy span — 계층 위치 정보 (breaking).
//! Original: compiler/rustc_span (SpanData 간소형). 호환심 없음.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
    pub ctxt: SyntaxContext,
    pub parent: Option<ExpnId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxContext(pub u32);

impl SyntaxContext {
    pub fn root() -> Self {
        Self(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpnId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpnData {
    pub call_site: Span,
    pub parent_span: Span,
}

fn expn_store() -> &'static std::sync::Mutex<Vec<ExpnData>> {
    static STORE: std::sync::OnceLock<std::sync::Mutex<Vec<ExpnData>>> = std::sync::OnceLock::new();
    STORE.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

fn ctxt_counter() -> &'static std::sync::Mutex<u32> {
    static CTR: std::sync::OnceLock<std::sync::Mutex<u32>> = std::sync::OnceLock::new();
    CTR.get_or_init(|| std::sync::Mutex::new(1))
}

fn fresh_ctxt() -> SyntaxContext {
    let mut n = ctxt_counter().lock().unwrap_or_else(|e| panic!("ctxt counter poisoned: {e}"));
    let c = *n;
    *n = n.checked_add(1).expect("ctxt overflow");
    SyntaxContext(c)
}

fn intern_expansion(call_site: Span, parent_span: Span) -> ExpnId {
    let mut v = expn_store().lock().unwrap_or_else(|e| panic!("expn store poisoned: {e}"));
    let id = ExpnId(v.len() as u32);
    v.push(ExpnData { call_site, parent_span });
    id
}

impl Span {
    pub fn new(lo: usize, hi: usize, ctxt: SyntaxContext, parent: Option<ExpnId>) -> Self {
        Self { lo, hi, ctxt, parent }
    }
    pub fn root(lo: usize, hi: usize) -> Self {
        Self::new(lo, hi, SyntaxContext::root(), None)
    }
    pub fn with_ctxt(mut self, ctxt: SyntaxContext) -> Self {
        self.ctxt = ctxt;
        self
    }
    pub fn expanded_from(call_site: Span, arg: Span) -> ExpnId {
        intern_expansion(call_site, arg)
    }
    pub fn fresh_child(&self, call_site: Span) -> Self {
        Self { lo: self.lo, hi: self.hi, ctxt: fresh_ctxt(), parent: Some(intern_expansion(call_site, *self)) }
    }
    pub fn copied_arg(arg: Span, call_site: Span) -> Self {
        Self { lo: arg.lo, hi: arg.hi, ctxt: arg.ctxt, parent: Some(intern_expansion(call_site, arg)) }
    }
    pub fn expansion(&self) -> Option<ExpnData> {
        self.parent.map(|id| expn_store().lock().unwrap_or_else(|e| panic!("expn store poisoned: {e}"))[id.0 as usize])
    }
    pub fn chain(&self) -> String {
        let mut out = format!("[{}..{}@c{}]", self.lo, self.hi, self.ctxt.0);
        let mut cur = *self;
        while let Some(d) = cur.expansion() {
            out.push_str(&format!(" <- call[{}..{}]", d.call_site.lo, d.call_site.hi));
            cur = d.parent_span;
        }
        out
    }
    pub fn try_snippet<'a>(&self, src: &'a str) -> Result<&'a str, SpanError> {
        let len = src.len();
        if self.lo > self.hi || self.hi > len {
            return Err(SpanError { lo: self.lo, hi: self.hi, len, reason: SpanErrorKind::OutOfRange });
        }
        src.get(self.lo..self.hi).ok_or(SpanError { lo: self.lo, hi: self.hi, len, reason: SpanErrorKind::NotCharBoundary })
    }
    pub fn snippet<'a>(&self, src: &'a str) -> &'a str {
        self.try_snippet(src).unwrap_or_else(|e| panic!("span snippet failed: {e} (src_len={})", src.len()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanErrorKind {
    OutOfRange,
    NotCharBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanError {
    pub lo: usize,
    pub hi: usize,
    pub len: usize,
    pub reason: SpanErrorKind,
}

impl std::fmt::Display for SpanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let why = match self.reason {
            SpanErrorKind::OutOfRange => "range out of bounds",
            SpanErrorKind::NotCharBoundary => "not a char boundary",
        };
        write!(f, "span [{}..{}] invalid for src len {}: {why}", self.lo, self.hi, self.len)
    }
}

impl std::error::Error for SpanError {}

pub fn offset_to_line_col(src: &str, offset: usize) -> Option<(usize, usize)> {
    if offset > src.len() || !src.is_char_boundary(offset) {
        return None;
    }
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (i, ch) in src.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let col = src[line_start..offset].chars().count() + 1;
    Some((line, col))
}

pub fn line_text(src: &str, line: usize) -> Option<(&str, usize)> {
    if line == 0 {
        return None;
    }
    let mut cur = 1usize;
    let mut start = 0usize;
    for (i, ch) in src.char_indices() {
        if cur == line && ch == '\n' {
            return Some((&src[start..i], start));
        }
        if ch == '\n' {
            cur += 1;
            start = i + 1;
        }
    }
    if cur == line {
        return Some((&src[start..], start));
    }
    None
}

pub fn caret_line(src: &str, span: Span) -> Option<(usize, String, String)> {
    let (line, _) = offset_to_line_col(src, span.lo)?;
    let (text, line_start) = line_text(src, line)?;
    if span.lo < line_start || span.lo > line_start + text.len() {
        return None;
    }
    let rel_start_bytes = span.lo - line_start;
    let line_end = line_start + text.len();
    let rel_end_bytes = span.hi.min(line_end).saturating_sub(line_start).max(rel_start_bytes);
    if !text.is_char_boundary(rel_start_bytes) || !text.is_char_boundary(rel_end_bytes) {
        return None;
    }
    let rel_start = text[..rel_start_bytes].chars().count();
    let mut width = text[rel_start_bytes..rel_end_bytes].chars().count().max(1);
    if span.hi > line_end {
        width = width.max(1);
    }
    let mut caret = String::new();
    caret.push_str(&" ".repeat(rel_start));
    caret.push_str(&"^".repeat(width));
    Some((line, text.to_string(), caret))
}

#[must_use]
pub fn same_var(a: &str, a_ctxt: SyntaxContext, b: &str, b_ctxt: SyntaxContext) -> bool {
    a == b && a_ctxt == b_ctxt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_roundtrip() {
        let src = "fn main() { 42 }";
        assert_eq!(Span::root(0, 2).snippet(src), "fn");
        assert_eq!(Span::root(0, 0), Span::new(0, 0, SyntaxContext::root(), None));
    }

    #[test]
    fn caret_has_no_prefix() {
        let src = "fn main() {}";
        let (_, _, caret) = caret_line(src, Span::root(4, 8)).unwrap();
        assert_eq!(caret, "    ^^^^");
        assert!(!caret.contains("|"));
    }

    #[test]
    fn expansion_keeps_parent_chain() {
        let call = Span::root(0, 9);
        let arg = Span::root(6, 7);
        let lhs = Span::copied_arg(arg, call);
        let plus = arg.fresh_child(call);
        assert!(lhs.parent.is_some());
        assert_eq!(lhs.ctxt, SyntaxContext::root());
        assert_ne!(plus.ctxt, SyntaxContext::root());
        assert!(plus.chain().contains("call[0..9]"));
    }

    #[test]
    fn hygiene_separates_macro_tmp() {
        let call = Span::root(10, 20);
        let user_tmp = Span::root(0, 3);
        let macro_tmp = user_tmp.fresh_child(call);
        assert!(!same_var("tmp", user_tmp.ctxt, "tmp", macro_tmp.ctxt));
    }

    #[test]
    fn caret_multibyte_char_boundary() {
        let src = "fn 한글(x: i32) {}";
        let s = src.find("x:").unwrap();
        let (_, _, caret) = caret_line(src, Span::root(s, s + 1)).unwrap();
        assert!(caret.trim_start().starts_with('^'));
    }
}
