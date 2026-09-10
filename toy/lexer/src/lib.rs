//! rtoy lexer — 토크나이저.
//! Original: compiler/rustc_lexer (cursor.rs, token.rs).

use rtoy_span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Int,
    Punct,
    Whitespace,
    Comment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// 입력 전체를 토큰 리스트로 변환.
pub fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        let kind = if c.is_alphabetic() || c == '_' {
            while i < chars.len() && (chars[i].1.is_alphanumeric() || chars[i].1 == '_') {
                i += 1;
            }
            TokenKind::Ident
        } else if c.is_ascii_digit() {
            while i < chars.len() && chars[i].1.is_ascii_digit() {
                i += 1;
            }
            TokenKind::Int
        } else if c.is_whitespace() {
            while i < chars.len() && chars[i].1.is_whitespace() {
                i += 1;
            }
            TokenKind::Whitespace
        } else if c == '/' && chars.get(i + 1).is_some_and(|(_, n)| *n == '/') {
            // `//` 라인 주석: 개행 전까지 Comment 1개 (개행은 Whitespace로 남김).
            i += 2;
            while i < chars.len() && chars[i].1 != '\n' {
                i += 1;
            }
            TokenKind::Comment
        } else {
            i += 1;
            TokenKind::Punct
        };
        let end = chars.get(i).map(|(p, _)| *p).unwrap_or(src.len());
        tokens.push(Token { kind, span: Span::root(start, end) });
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_simple_fn() {
        let toks = tokenize("fn main() {}");
        let kinds: Vec<_> = toks.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident,
                TokenKind::Whitespace,
                TokenKind::Ident,
                TokenKind::Punct,
                TokenKind::Punct,
                TokenKind::Whitespace,
                TokenKind::Punct,
                TokenKind::Punct,
            ]
        );
    }
}
