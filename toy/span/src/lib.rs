//! rtoy span — 최소 위치 정보.
//! Original: compiler/rustc_span (SpanData/Span 간소형).

/// rustc_span 대비: 일단 start/end만.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
    /// src에서 해당 구간 스니펫 반환. 실패 시 가장 안쪽 값까지 담은 에러.
    pub fn try_snippet<'a>(&self, src: &'a str) -> Result<&'a str, SpanError> {
        let len = src.len();
        if self.start > self.end || self.end > len {
            return Err(SpanError { start: self.start, end: self.end, len, reason: SpanErrorKind::OutOfRange });
        }
        src.get(self.start..self.end).ok_or(SpanError { start: self.start, end: self.end, len, reason: SpanErrorKind::NotCharBoundary })
    }
    /// src에서 해당 구간 스니펫 반환.
    pub fn snippet<'a>(&self, src: &'a str) -> &'a str {
        self.try_snippet(src).unwrap_or_else(|e| panic!("span snippet failed: {e} (src_len={})", src.len()))
    }
}

/// span이 src를 벗어난 이유. 가장 안쪽 정보까지 Display에 노출한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanErrorKind {
    OutOfRange,
    NotCharBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanError {
    pub start: usize,
    pub end: usize,
    pub len: usize,
    pub reason: SpanErrorKind,
}

impl std::fmt::Display for SpanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let why = match self.reason {
            SpanErrorKind::OutOfRange => "range out of bounds",
            SpanErrorKind::NotCharBoundary => "not a char boundary",
        };
        write!(f, "span [{}..{}] invalid for src len {}: {why}", self.start, self.end, self.len)
    }
}

impl std::error::Error for SpanError {}

/// byte offset → 1-based (line, col). col은 해당 줄 내 char 수 기준.
/// 범위 밖이면 None (가장 안쪽 원인을 호출자가 문구로 노출).
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

/// 1-based line 번호의 원문 한 줄 (개행 제외) + 줄 시작 offset.
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

/// 한 줄 스니펫 + 캐럿 본문. span이 여러 줄이면 첫 줄만 `^^^`로 표시.
/// 반환값: (line, 원문 한 줄, 캐럿 본문). 캐럿 본문은 접두사 없이
/// " "*rel_start + "^"*width — 접두사 폭은 caller가 줄번호 렌더링과 맞춘다.
pub fn caret_line(src: &str, span: Span) -> Option<(usize, String, String)> {
    let (line, _) = offset_to_line_col(src, span.start)?;
    let (text, line_start) = line_text(src, line)?;
    if span.start < line_start || span.start > line_start + text.len() {
        return None;
    }
    let rel_start_bytes = span.start - line_start;
    let line_end = line_start + text.len();
    let rel_end_bytes = span.end.min(line_end).saturating_sub(line_start).max(rel_start_bytes);
    if !text.is_char_boundary(rel_start_bytes) || !text.is_char_boundary(rel_end_bytes) {
        return None;
    }
    let rel_start = text[..rel_start_bytes].chars().count();
    let mut width = text[rel_start_bytes..rel_end_bytes].chars().count().max(1);
    if span.end > line_end {
        width = width.max(1);
    }
    let mut caret = String::new();
    caret.push_str(&" ".repeat(rel_start));
    caret.push_str(&"^".repeat(width));
    Some((line, text.to_string(), caret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_roundtrip() {
        let src = "fn main() { 42 }";
        assert_eq!(Span::new(0, 2).snippet(src), "fn");
        assert_eq!(Span::dummy(), Span::new(0, 0));
    }

    #[test]
    fn caret_has_no_prefix() {
        let src = "fn main() {}";
        let (_, _, caret) = caret_line(src, Span::new(4, 8)).unwrap();
        assert_eq!(caret, "    ^^^^");
        assert!(!caret.contains("|"));
    }

    #[test]
    fn caret_multibyte_char_boundary() {
        // 전각 1자가 span 앞에 있어도 자르기/개수 계산이 char 경계에서 안전
        let src = "fn 한글(x: i32) {}";
        let s = src.find("x:").unwrap();
        let (_, _, caret) = caret_line(src, Span::new(s, s + 1)).unwrap();
        assert!(caret.trim_start().starts_with('^'));
    }
}
