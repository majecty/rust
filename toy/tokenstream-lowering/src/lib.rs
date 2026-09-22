//! rtoy tokenstream-lowering — token stream → AST.
//! Original: compiler/rustc_parse (parser/expr.rs, item.rs).

use rtoy_ast::{BinOp, Block, Crate, Expr, ExprKind, FieldInit, FnItem, Ident, Item, ItemKind, LetStmt, Stmt, StmtKind, StructField, StructItem, Ty};
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
                self.context, self.expected, kind, text, span.lo, span.hi, self.pos
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
        self.skip_whitespace();
        let start = self.peek().map(|t| t.span.lo).unwrap_or(0);
        loop {
            self.skip_whitespace();
            if self.peek().is_none() { break; }
            items.push(self.parse_item()?);
        }
        let end = items.last().map(|i| i.span.hi).unwrap_or(start);
        Ok(Crate { items, span: Span::root(start, end) })
    }

    /// 아이템 하나. 예: `fn main() { 42 }`, `def_fn!(foo)`.
    fn parse_item(&mut self) -> Result<Item, LowerError> {
        if self.is_macro_item() {
            return self.parse_macro_item();
        }
        if self.peek_is_ident("struct") {
            return self.parse_struct_item();
        }
        let (fn_tok, name) = self.parse_fn_head()?;
        let body = self.parse_block()?;
        let span = Span::root(fn_tok.span.lo, body.span.hi);
        Ok(Item { name, kind: ItemKind::Fn(FnItem { body, span }), span })
    }

    /// `def_fn!(foo)`처럼 아이템 위치 매크로 호출인지 미리보기 (소비 없음).
    fn is_macro_item(&self) -> bool {
        let mut idx = self.pos;
        while let Some(t) = self.tokens.get(idx) {
            if matches!(t.kind, TokenKind::Whitespace | TokenKind::Comment) {
                idx += 1;
                continue;
            }
            break;
        }
        let Some(name_tok) = self.tokens.get(idx) else { return false; };
        if name_tok.kind != TokenKind::Ident {
            return false;
        }
        idx += 1;
        while let Some(t) = self.tokens.get(idx) {
            if matches!(t.kind, TokenKind::Whitespace | TokenKind::Comment) {
                idx += 1;
                continue;
            }
            break;
        }
        matches!(self.tokens.get(idx), Some(t) if t.kind == TokenKind::Punct && t.span.try_snippet(self.src).as_deref() == Ok("!"))
    }

    /// 아이템 매크로. 예: `def_fn!(foo)` — 뒤 `;`는 있어도 없어도 된다.
    fn parse_macro_item(&mut self) -> Result<Item, LowerError> {
        self.skip_trivia();
        let name_tok = self.expect("item macro name", TokenKind::Ident, "macro name")?;
        let name = self.peek_text(&name_tok).map_err(|e| LowerError { context: "item macro name", expected: "valid macro name".into(), found: Some((name_tok.kind, "<invalid span>".into(), name_tok.span)), pos: self.pos, source: Some(e) })?;
        let ident = Ident { name, span: name_tok.span };
        self.skip_trivia();
        self.expect_punct("item macro bang", '!')?;
        self.skip_trivia();
        self.expect_punct("item macro args", '(')?;
        let (args, end) = self.parse_call_args()?;
        let mut hi = end;
        self.skip_trivia();
        if self.peek_is_punct(';') {
            hi = self.bump().expect("peeked `;`").span.hi;
        }
        let span = Span::root(name_tok.span.lo, hi);
        Ok(Item { name: ident.clone(), kind: ItemKind::Macro { name: ident, args }, span })
    }

    /// 구조체 정의. 예: `struct Point { x: i64, y: i64 }`.
    fn parse_struct_item(&mut self) -> Result<Item, LowerError> {
        let kw = self.expect_ident("struct keyword", "struct")?;
        let name_tok = self.expect("struct name", TokenKind::Ident, "struct name")?;
        let name = Ident { name: self.ident_text("struct name", &name_tok)?, span: name_tok.span };
        self.expect_punct("struct body", '{')?;
        let fields = self.parse_struct_fields()?;
        let close = self.expect_punct("struct body", '}')?;
        let mut hi = close.span.hi;
        self.skip_trivia();
        if self.peek_is_punct(';') {
            hi = self.bump().expect("peeked `;`").span.hi;
        }
        let span = Span::root(kw.span.lo, hi);
        Ok(Item { name, kind: ItemKind::Struct(StructItem { fields, span }), span })
    }

    /// 구조체 필드 목록. 예: `x: i64, y: i64,` — 닫는 `}`는 소비하지 않는다.
    fn parse_struct_fields(&mut self) -> Result<Vec<StructField>, LowerError> {
        let mut fields = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek_is_punct('}') {
                return Ok(fields);
            }
            let f_tok = self.expect("struct field name", TokenKind::Ident, "field name")?;
            let field_name = Ident { name: self.ident_text("struct field name", &f_tok)?, span: f_tok.span };
            self.expect_punct("struct field colon", ':')?;
            let ty_tok = self.expect("struct field type", TokenKind::Ident, "type name")?;
            let ty = Ty { name: self.ident_text("struct field type", &ty_tok)?, span: ty_tok.span };
            let span = Span::root(f_tok.span.lo, ty_tok.span.hi);
            fields.push(StructField { name: field_name, ty, span });
            self.skip_trivia();
            if self.peek_is_punct(',') {
                self.bump();
            }
        }
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
                let span = Span::root(open.span.lo, close.span.hi);
                return Ok(Block { stmts, tail, span });
            }
            if self.peek_is_ident("let") {
                let let_tok = self.peek().copied();
                let stmt = self.parse_let_stmt()?;
                let semi = self.expect_punct("let semi", ';')?;
                let span = Span::root(let_tok.map(|t| t.span.lo).unwrap_or(stmt.span.lo), semi.span.hi);
                stmts.push(Stmt { kind: StmtKind::Let(stmt), span });
                continue;
            }
            let expr = self.parse_expr()?;
            self.skip_trivia();
            if self.peek_is_punct(';') {
                let semi = self.bump().expect("peeked `;`");
                let span = Span::root(expr.span.lo, semi.span.hi);
                stmts.push(Stmt { kind: StmtKind::Expr(expr), span });
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
        let init = self.parse_expr()?;
        let span = Span::root(let_tok.span.lo, init.span.hi);
        Ok(LetStmt { name, ty, init: Some(init), span })
    }

    /// 식. 예: `42`, `x`, `foo(1, x)`, `1 + 2 * 3`, `if c { .. } else { .. }`.
    fn parse_expr(&mut self) -> Result<Expr, LowerError> {
        self.skip_trivia();
        if self.peek_is_ident("if") {
            return self.parse_if();
        }
        let lhs = self.parse_additive()?;
        self.skip_trivia();
        let Some(op) = self.peek_binop(&[("<", BinOp::Lt)]) else { return Ok(lhs) };
        self.bump();
        let rhs = self.parse_additive()?;
        Ok(self.join_binary(op, lhs, rhs))
    }

    /// if 식. 예: `if n < 2 { n } else { 0 }` (else 생략 가능).
    fn parse_if(&mut self) -> Result<Expr, LowerError> {
        let if_tok = self.expect_ident("if expr", "if")?;
        let cond = self.parse_expr()?;
        let then_block = self.parse_block()?;
        self.skip_trivia();
        let else_block = if self.peek_is_ident("else") {
            self.bump();
            Some(self.parse_block()?)
        } else {
            None
        };
        let end = else_block.as_ref().map(|b| b.span.hi).unwrap_or(then_block.span.hi);
        let kind = ExprKind::If {
            cond: Box::new(cond),
            then_block: Box::new(then_block),
            else_block: else_block.map(Box::new),
        };
        Ok(Expr { kind, span: Span::root(if_tok.span.lo, end) })
    }

    /// 덧셈급. 예: `1 + 2`, `a - b` (좌결합).
    fn parse_additive(&mut self) -> Result<Expr, LowerError> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            self.skip_trivia();
            let Some(op) = self.peek_binop(&[("+", BinOp::Add), ("-", BinOp::Sub)]) else { break };
            self.bump();
            let rhs = self.parse_multiplicative()?;
            lhs = self.join_binary(op, lhs, rhs);
        }
        Ok(lhs)
    }

    /// 곱셈급. 예: `2 * 3`, `8 / 4`, `7 % 2` (좌결합).
    fn parse_multiplicative(&mut self) -> Result<Expr, LowerError> {
        let mut lhs = self.parse_postfix()?;
        loop {
            self.skip_trivia();
            let Some(op) = self.peek_binop(&[("*", BinOp::Mul), ("/", BinOp::Div), ("%", BinOp::Mod)]) else { break };
            self.bump();
            let rhs = self.parse_postfix()?;
            lhs = self.join_binary(op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn join_binary(&self, op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
        let span = Span::root(lhs.span.lo, rhs.span.hi);
        Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span }
    }

    /// peek가 지정 punct면 (op, true) — 소비는 호출자가 bump로.
    fn peek_binop(&self, ops: &[(&str, BinOp)]) -> Option<BinOp> {
        let t = self.peek()?;
        if t.kind != TokenKind::Punct { return None; }
        let text = t.span.try_snippet(self.src).ok()?;
        ops.iter().find(|(s, _)| *s == text).map(|(_, op)| *op)
    }

    /// 후위식. 예: `p.x.y` — 원자식 뒤 `.field`를 반복 적용.
    fn parse_postfix(&mut self) -> Result<Expr, LowerError> {
        let mut e = self.parse_atom()?;
        loop {
            self.skip_trivia();
            if !self.peek_is_punct('.') {
                break;
            }
            self.bump();
            let f_tok = self.expect("field access", TokenKind::Ident, "field name")?;
            let field = Ident { name: self.ident_text("field access", &f_tok)?, span: f_tok.span };
            let span = Span::root(e.span.lo, f_tok.span.hi);
            e = Expr { kind: ExprKind::FieldAccess { base: Box::new(e), field, slot: None }, span };
        }
        Ok(e)
    }

    /// 원자식. 예: `42`, `x`, `foo(1)`.
    fn parse_atom(&mut self) -> Result<Expr, LowerError> {
        self.skip_trivia();
        let Some(t) = self.bump() else {
            return Err(self.fail("block body", "expression or `}`"));
        };
        match t.kind {
            TokenKind::Int => self.parse_int_lit(t),
            TokenKind::Ident => self.parse_ident_expr(t),
            _ => Err(LowerError {
                context: "block body",
                expected: "expression (int/ident/call)".into(),
                found: Some((t.kind, self.peek_text(&t).unwrap_or_default(), t.span)),
                pos: self.pos,
                source: None,
            }),
        }
    }

    /// 정수 리터럴. 예: `42`.
    fn parse_int_lit(&mut self, t: Token) -> Result<Expr, LowerError> {
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

    /// 식별자식. 다음 토큰이 `(`면 호출, 아니면 변수 참조.
    fn parse_ident_expr(&mut self, t: Token) -> Result<Expr, LowerError> {
        let name = self.peek_text(&t).map_err(|e| LowerError { context: "ident expr", expected: "valid identifier text".into(), found: Some((t.kind, "<invalid span>".into(), t.span)), pos: self.pos, source: Some(e) })?;
        let ident = Ident { name, span: t.span };
        let save = self.pos;
        self.skip_trivia();
        if self.peek_is_punct('(') {
            self.bump();
            let (args, end) = self.parse_call_args()?;
            let span = Span::root(t.span.lo, end);
            return Ok(Expr { kind: ExprKind::Call { callee: ident, args }, span });
        }
        if self.peek_is_punct('!') {
            self.bump();
            self.skip_trivia();
            self.expect_punct("macro args", '(')?;
            let (args, end) = self.parse_call_args()?;
            let span = Span::root(t.span.lo, end);
            return Ok(Expr { kind: ExprKind::Macro { name: ident, args }, span });
        }
        if self.peek_is_punct('{') {
            self.bump();
            let fields = self.parse_field_inits()?;
            let close = self.expect_punct("struct literal", '}')?;
            let span = Span::root(t.span.lo, close.span.hi);
            return Ok(Expr { kind: ExprKind::StructLiteral { name: ident, fields }, span });
        }
        self.pos = save;
        Ok(Expr { kind: ExprKind::Var(ident), span: t.span })
    }

    /// 구조체 리터럴 필드 목록. 예: `x: 1, y: 2` — 닫는 `}`는 소비하지 않는다.
    fn parse_field_inits(&mut self) -> Result<Vec<FieldInit>, LowerError> {
        let mut fields = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek_is_punct('}') {
                return Ok(fields);
            }
            let f_tok = self.expect("struct literal field", TokenKind::Ident, "field name")?;
            let name = Ident { name: self.ident_text("struct literal field", &f_tok)?, span: f_tok.span };
            self.expect_punct("struct literal colon", ':')?;
            let value = self.parse_expr()?;
            let span = Span::root(f_tok.span.lo, value.span.hi);
            fields.push(FieldInit { name, value, slot: None, span });
            self.skip_trivia();
            if self.peek_is_punct(',') {
                self.bump();
            }
        }
    }

    /// 호출 인자. 예: `(1, x)` — `(foo())`의 `)` 끝 오프셋까지 소비.
    fn parse_call_args(&mut self) -> Result<(Vec<Expr>, usize), LowerError> {
        let mut args = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek_is_punct(')') {
                let close = self.bump().expect("peeked `)`");
                return Ok((args, close.span.hi));
            }
            args.push(self.parse_expr()?);
            self.skip_trivia();
            if self.peek_is_punct(',') {
                self.bump();
                continue;
            }
            let close = self.expect_punct("call args", ')')?;
            return Ok((args, close.span.hi));
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

    /// 토큰 텍스트를 얻고, 실패 시 LowerError(source=SpanError)로 감싼다.
    fn ident_text(&self, context: &'static str, t: &Token) -> Result<String, LowerError> {
        self.peek_text(t).map_err(|e| LowerError {
            context,
            expected: "valid identifier text".into(),
            found: Some((t.kind, "<invalid span>".into(), t.span)),
            pos: self.pos,
            source: Some(e),
        })
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
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn lowers_binary_precedence() {
        let src = "fn main() { 1 + 2 * 3 }";
        let toks = tokenize(src);
        let krate = lower(&toks, src);
        match &krate.items[0].kind {
            ItemKind::Fn(f) => {
                let tail = f.body.tail.as_ref().unwrap();
                assert_eq!(tail.span.snippet(src), "1 + 2 * 3");
                match &tail.kind {
                    ExprKind::Binary { op: BinOp::Add, lhs, rhs } => {
                        assert_eq!(lhs.span.snippet(src), "1");
                        assert_eq!(rhs.span.snippet(src), "2 * 3");
                        assert!(matches!(&rhs.kind, ExprKind::Binary { op: BinOp::Mul, .. }));
                    }
                    other => panic!("expected add, got {other:?}"),
                }
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn lowers_var_and_call() {
        let src = "fn main() { let x = 1; foo(x, 2) }";
        let toks = tokenize(src);
        let krate = lower(&toks, src);
        match &krate.items[0].kind {
            ItemKind::Fn(f) => {
                let tail = f.body.tail.as_ref().unwrap();
                assert_eq!(tail.span.snippet(src), "foo(x, 2)");
                match &tail.kind {
                    ExprKind::Call { callee, args } => {
                        assert_eq!(callee.name, "foo");
                        assert_eq!(callee.span.snippet(src), "foo");
                        assert_eq!(args.len(), 2);
                        assert_eq!(args[0].span.snippet(src), "x");
                        assert!(matches!(&args[0].kind, ExprKind::Var(v) if v.name == "x"));
                        assert!(matches!(&args[1].kind, ExprKind::Int(2)));
                    }
                    other => panic!("expected call, got {other:?}"),
                }
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn spans_cover_source() {
        let src = "fn main() { let x: u32 = 42; }";
        let toks = tokenize(src);
        let krate = lower(&toks, src);
        let item = &krate.items[0];
        assert_eq!(krate.span.snippet(src), "fn main() { let x: u32 = 42; }");
        assert_eq!(item.span.snippet(src), src);
        assert_eq!(item.name.span.snippet(src), "main");
        match &item.kind {
            ItemKind::Fn(f) => {
                assert_eq!(f.span.snippet(src), "fn main() { let x: u32 = 42; }");
                assert_eq!(f.body.span.snippet(src), "{ let x: u32 = 42; }");
                let stmt = match &f.body.stmts[0] {
                    Stmt { kind: StmtKind::Let(l), span: s } => {
                        assert_eq!(s.snippet(src), "let x: u32 = 42;");
                        l
                    }
                    other => panic!("expected let stmt, got {other:?}"),
                };
                assert_eq!(stmt.name.span.snippet(src), "x");
                assert_eq!(stmt.ty.as_ref().unwrap().span.snippet(src), "u32");
                assert_eq!(stmt.span.snippet(src), "let x: u32 = 42");
                assert_eq!(stmt.init.as_ref().unwrap().span.snippet(src), "42");
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn lowers_def_fn_item() {
        let src = "def_fn!(foo)";
        let toks = tokenize(src);
        let krate = lower(&toks, src);
        assert_eq!(krate.items.len(), 1);
        assert_eq!(krate.items[0].span.snippet(src), src);
        match &krate.items[0].kind {
            ItemKind::Macro { name, args } => {
                assert_eq!(name.name, "def_fn");
                assert_eq!(args.len(), 1);
                assert_eq!(args[0].span.snippet(src), "foo");
            }
            other => panic!("expected item macro, got {other:?}"),
        }
    }

    #[test]
    fn lowers_struct_def() {
        let src = "struct Point { x: i64, y: i64 }";
        let toks = tokenize(src);
        let krate = lower(&toks, src);
        assert_eq!(krate.items[0].name.name, "Point");
        match &krate.items[0].kind {
            ItemKind::Struct(s) => {
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].name.name, "x");
                assert_eq!(s.fields[0].ty.name, "i64");
                assert_eq!(s.fields[1].name.name, "y");
            }
            other => panic!("expected struct, got {other:?}"),
        }
    }

    #[test]
    fn lowers_struct_literal_and_field() {
        let src = "fn main() { let p = Point { x: 1, y: 2 }; p.x + p.y }";
        let toks = tokenize(src);
        let krate = lower(&toks, src);
        let ItemKind::Fn(f) = &krate.items[0].kind else { panic!("expected Fn") };
        let StmtKind::Let(l) = &f.body.stmts[0].kind else { panic!("expected let") };
        match &l.init.as_ref().unwrap().kind {
            ExprKind::StructLiteral { name, fields } => {
                assert_eq!(name.name, "Point");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name.name, "x");
            }
            other => panic!("expected struct literal, got {other:?}"),
        }
        let tail = f.body.tail.as_ref().unwrap();
        assert_eq!(tail.span.snippet(src), "p.x + p.y");
        match &tail.kind {
            ExprKind::Binary { op: BinOp::Add, lhs, rhs } => {
                assert!(matches!(&lhs.kind, ExprKind::FieldAccess { field, .. } if field.name == "x"));
                assert!(matches!(&rhs.kind, ExprKind::FieldAccess { field, .. } if field.name == "y"));
            }
            other => panic!("expected add, got {other:?}"),
        }
    }

    #[test]
    fn lowers_if_else_with_lt() {
        let src = "fn main() { if 1 < 2 { 10 } else { 20 } }";
        let krate = lower(&tokenize(src), src);
        let ItemKind::Fn(f) = &krate.items[0].kind else { panic!("expected Fn") };
        let tail = f.body.tail.as_ref().expect("tail");
        match &tail.kind {
            ExprKind::If { cond, then_block, else_block } => {
                assert!(matches!(cond.kind, ExprKind::Binary { op: BinOp::Lt, .. }));
                assert_eq!(then_block.tail.as_ref().unwrap().span.snippet(src), "10");
                let else_tail = else_block.as_ref().unwrap().tail.as_ref().unwrap();
                assert_eq!(else_tail.span.snippet(src), "20");
                assert_eq!(tail.span.snippet(src), "if 1 < 2 { 10 } else { 20 }");
            }
            other => panic!("expected if, got {other:?}"),
        }
    }
}
