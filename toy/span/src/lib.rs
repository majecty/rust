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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_roundtrip() {
        let src = "fn main() { 42 }";
        assert_eq!(Span::new(0, 2).snippet(src), "fn");
        assert_eq!(Span::dummy(), Span::new(0, 0));
    }
}
