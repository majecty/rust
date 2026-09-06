//! rtoy tokenstream-lowering — token stream → AST.
//! Original: compiler/rustc_parse (parser/expr.rs, item.rs).

use rtoy_ast::{Block, Crate, Expr, ExprKind, FnItem, Ident, Item, ItemKind, LetStmt, Stmt, Ty};
use rtoy_span::{Span, SpanError};
use rtoy_lexer::{Token, TokenKind};

/// lowering 실패. 가장 안쪽 정보(expected/found/span/pos/원인)까지 Display에 노출한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerError {
    pub context: &'static str,
    pub expected: String,
    pub found: Option<(TokenKind, String, Span)>,
    pub pos: usize,
    pub source: Option<SpanError>,
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.found {
            Some((kind, text, span)) => write!(
                f,
                "{}: expected {} but found {:?} {:?} at span [{}..{}] (token #{})",
                self.context, self.expected, kind, text, span.start, span.end, self.pos
            ),
            None => write!(f, "{}: expected {} but found end of input (token #{})", self.context, self.expected, self.pos),
        }
    }
}

impl std::error::Error for LowerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e as &dyn std::error::Error)
    }
}

pub struct Lowering<'a> {
    tokens: &'a [Token],
    src: &'a str,
    pos: usize,
}

impl<'a> Lowering<'a> {
    pub fn new(tokens: &'a [Token], src: &'a str) -> Self {
        Self { tokens, src, pos: 0 }
    }

    /// 크레이트 전체. 예: `fn main() { 42 } fn foo() { 1 }`.
    pub fn parse_crate(&mut self) -> Crate {
        self.try_parse_crate().unwrap_or_else(|e| panic!("lowering failed: {e}"))
    }

    /// 크레이트 전체 (엄격). 예: `fn main() { 42 } fn foo() { 1 }`.
    pub fn try_parse_crate(&mut self) -> Result<Crate, LowerError> {
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek().is_none() { break; }
            items.push(self.parse_item()?);
        }
        Ok(Crate { items })
    }

    /// 아이템 하나. 예: `fn main() { 42 }`.
    fn parse_item(&mut self) -> Result<Item, LowerError> {
        let (fn_tok, name) = self.parse_fn_head()?;
        let body = self.parse_block()?;
        let span = Span::new(fn_tok.span.start, body.span.end);
        Ok(Item { name, kind: ItemKind::Fn(FnItem { body }), span })
    }

    /// fn 헤더. 예: `fn main()`.
    fn parse_fn_head(&mut self) -> Result<(Token, Ident), LowerError> {
        self.skip_whitespace();
        let t = self.expect_ident("fn keyword", "fn")?;
        self.skip_whitespace();
        let name_tok = self.expect("fn name", TokenKind::Ident, "function name")?;
        let name = self.peek_text(&name_tok).map_err(|e| LowerError { context: "fn name", expected: "valid function name".into(), found: Some((name_tok.kind, "<invalid span>".into(), name_tok.span)), pos: self.pos, source: Some(e) })?;
        let ident = Ident { name, span: name_tok.span };
        self.expect_punct("fn params", '(')?;
        self.expect_punct("fn params", ')')?;
        Ok((t, ident))
    }

    /// 블록. 예: `{ let x = 1; 42 }`, `{ 42 }`.
    fn parse_block(&mut self) -> Result<Block, LowerError> {
        let open = self.expect_punct("block", '{')?;
        let mut stmts = Vec::new();
        let mut tail: Option<Expr> = None;
        loop {
            self.skip_trivia();
            if self.peek_is_punct('}') {
                let close = self.bump().expect("peeked `}`");
                let span = Span::new(open.span.start, close.span.end);
                return Ok(Block { stmts, tail, span });
            }
            if self.peek_is_ident("let") {
                let stmt = self.parse_let_stmt()?;
                self.expect_punct("let semi", ';')?;
                stmts.push(Stmt::Let(stmt));
                continue;
            }
            let expr = self.parse_int_expr()?;
            self.skip_trivia();
            if self.peek_is_punct(';') {
                self.bump();
                stmts.push(Stmt::Expr(expr));
            } else {
                tail = Some(expr);
            }
        }
    }

    /// let문. 예: `let x = 42`, `let x: i32 = 42` (뒤 `;`는 호출자가 소비).
    fn parse_let_stmt(&mut self) -> Result<LetStmt, LowerError> {
        let let_tok = self.expect_ident("let stmt", "let")?;
        let name_tok = self.expect("let name", TokenKind::Ident, "variable name")?;
        let name = self.peek_text(&name_tok).map_err(|e| LowerError { context: "let name", expected: "valid variable name".into(), found: Some((name_tok.kind, "<invalid span>".into(), name_tok.span)), pos: self.pos, source: Some(e) })?;
        let name = Ident { name, span: name_tok.span };
        self.skip_trivia();
        let mut ty: Option<Ty> = None;
        if self.peek_is_punct(':') {
            self.bump();
            let ty_tok = self.expect("let type", TokenKind::Ident, "type name")?;
            let ty_text = self.peek_text(&ty_tok).map_err(|e| LowerError { context: "let type", expected: "valid type name".into(), found: Some((ty_tok.kind, "<invalid span>".into(), ty_tok.span)), pos: self.pos, source: Some(e) })?;
            ty = Some(Ty { name: ty_text, span: ty_tok.span });
        }
        self.expect_punct("let eq", '=')?;
        let init = self.parse_int_expr()?;
        let span = Span::new(let_tok.span.start, init.span.end);
        Ok(LetStmt { name, ty, init: Some(init), span })
    }

    /// 정수식. 예: `42`.
    fn parse_int_expr(&mut self) -> Result<Expr, LowerError> {
        self.skip_trivia();
        let Some(t) = self.bump() else {
            return Err(self.fail("block body", "expression or `}`"));
        };
        match t.kind {
            TokenKind::Int => {
                let text = self.peek_text(&t).map_err(|e| LowerError { context: "int literal", expected: "valid int text".into(), found: Some((t.kind, "<invalid span>".into(), t.span)), pos: self.pos, source: Some(e) })?;
                match text.parse::<i64>() {
                    Ok(n) => Ok(Expr { kind: ExprKind::Int(n), span: t.span }),
                    Err(inner) => Err(LowerError {
                        context: "int literal",
                        expected: format!("i64 (got {text:?}; parse failed: {inner})"),
                        found: Some((t.kind, text, t.span)),
                        pos: self.pos,
                        source: None,
                    }),
                }
            }
            _ => Err(LowerError {
                context: "block body",
                expected: "int literal or `}`".into(),
                found: Some((t.kind, self.peek_text(&t).unwrap_or_default(), t.span)),
                pos: self.pos,
                source: None,
            }),
        }
    }

    fn peek_is_punct(&self, want: char) -> bool {
        match self.peek() {
            Some(t) if t.kind == TokenKind::Punct => t.span.try_snippet(self.src).as_deref() == Ok(want.to_string().as_str()),
            _ => false,
        }
    }

    fn peek_is_ident(&self, want: &str) -> bool {
        match self.peek() {
            Some(t) if t.kind == TokenKind::Ident => t.span.try_snippet(self.src).as_deref() == Ok(want),
            _ => false,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).copied();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn skip_whitespace(&mut self) {
        self.skip_trivia();
    }

    /// whitespace + `//` 주석 스킵 (rustc 파서가 주석을 건너뛰듯).
    fn skip_trivia(&mut self) {
        while matches!(self.peek(), Some(t) if t.kind == TokenKind::Whitespace || t.kind == TokenKind::Comment) {
            self.bump();
        }
    }

    fn peek_text(&self, t: &Token) -> Result<String, SpanError> {
        t.span.try_snippet(self.src).map(str::to_string)
    }

    fn found_here(&self) -> Option<(TokenKind, String, Span)> {
        self.peek().map(|t| {
            let text = t.span.try_snippet(self.src).unwrap_or("<invalid span>").to_string();
            (t.kind, text, t.span)
        })
    }

    fn fail(&self, context: &'static str, expected: impl Into<String>) -> LowerError {
        LowerError { context, expected: expected.into(), found: self.found_here(), pos: self.pos, source: None }
    }

    fn expect(&mut self, context: &'static str, kind: TokenKind, expected: impl Into<String>) -> Result<Token, LowerError> {
        self.skip_whitespace();
        match self.bump() {
            Some(t) if t.kind == kind => Ok(t),
            _ => {
                // bump로 소비된 토큰이 있으면 pos를 되돌려 found가 실제 위치를 가리키게 한다.
                self.pos = self.pos.saturating_sub(1);
                Err(self.fail(context, expected))
            }
        }
    }

    fn expect_ident(&mut self, context: &'static str, want: &str) -> Result<Token, LowerError> {
        let pos = self.pos;
        let t = self.expect(context, TokenKind::Ident, "identifier")?;
        match self.peek_text(&t) {
            Ok(s) if s == want => Ok(t),
            Ok(_) => Err(LowerError { context, expected: format!("identifier `{want}`"), found: Some((t.kind, self.peek_text(&t).unwrap_or_default(), t.span)), pos: self.pos, source: None }),
            Err(e) => Err(LowerError { context, expected: "valid identifier text".into(), found: Some((t.kind, "<invalid span>".into(), t.span)), pos, source: Some(e) }),
        }
    }

    fn expect_punct(&mut self, context: &'static str, want: char) -> Result<Token, LowerError> {
        let pos = self.pos;
        let t = self.expect(context, TokenKind::Punct, "punct")?;
        match self.peek_text(&t) {
            Ok(s) if s == want.to_string() => Ok(t),
            Ok(_) => Err(LowerError { context, expected: format!("'{want}'"), found: Some((t.kind, self.peek_text(&t).unwrap_or_default(), t.span)), pos: self.pos, source: None }),
            Err(e) => Err(LowerError { context, expected: "valid punct text".into(), found: Some((t.kind, "<invalid span>".into(), t.span)), pos, source: Some(e) }),
        }
    }
}

pub fn lower(tokens: &[Token], src: &str) -> Crate {
    try_lower(tokens, src).unwrap_or_else(|e| panic!("lowering failed: {e}"))
}

/// 실패 원인을 호출자까지 전달하는 엄격 lowering.
/// driver는 이 함수를 쓰고 anyhow 없이 `source()` 체인까지 출력한다.
/// (빈 Crate를 조용히 반환하던 기존 동작은 [`lower`]에만 남는다.)
pub fn try_lower(tokens: &[Token], src: &str) -> Result<Crate, LowerError> {
    Lowering::new(tokens, src).try_parse_crate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtoy_lexer::tokenize;

    #[test]
    fn lowers_fn_with_int() {
        let src = "fn main() { 42 }";
        let toks = tokenize(src);
        let krate = lower(&toks, src);
        assert_eq!(krate.items.len(), 1);
        assert_eq!(krate.items[0].name.name, "main");
        match &krate.items[0].kind {
            ItemKind::Fn(f) => match &f.body.tail.as_ref().unwrap().kind {
                ExprKind::Int(42) => {}
                other => panic!("expected Int(42), got {other:?}"),
            },
        }
    }

    #[test]
    fn spans_cover_source() {
        let src = "fn main() { let x: u32 = 42; }";
        let toks = tokenize(src);
        let krate = lower(&toks, src);
        let item = &krate.items[0];
        assert_eq!(item.span.snippet(src), src);
        assert_eq!(item.name.span.snippet(src), "main");
        match &item.kind {
            ItemKind::Fn(f) => {
                assert_eq!(f.body.span.snippet(src), "{ let x: u32 = 42; }");
                let stmt = match &f.body.stmts[0] {
                    Stmt::Let(l) => l,
                    other => panic!("expected let stmt, got {other:?}"),
                };
                assert_eq!(stmt.name.span.snippet(src), "x");
                assert_eq!(stmt.ty.as_ref().unwrap().span.snippet(src), "u32");
                assert_eq!(stmt.span.snippet(src), "let x: u32 = 42");
            }
        }
    }
}
