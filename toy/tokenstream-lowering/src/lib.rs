//! rtoy tokenstream-lowering — token stream → AST.
//! Original: compiler/rustc_parse (parser/expr.rs, item.rs).

use rtoy_ast::{Block, Crate, Expr, ExprKind, FnItem, Item, ItemKind, Span};
use rtoy_lexer::{Token, TokenKind};

pub struct Lowering<'a> {
    tokens: &'a [Token],
    src: &'a str,
    pos: usize,
}

impl<'a> Lowering<'a> {
    pub fn new(tokens: &'a [Token], src: &'a str) -> Self {
        Self { tokens, src, pos: 0 }
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
        while matches!(self.peek(), Some(t) if t.kind == TokenKind::Whitespace) {
            self.bump();
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Option<Token> {
        self.skip_whitespace();
        let t = self.bump()?;
        if t.kind == kind { Some(t) } else { None }
    }

    fn expect_ident(&mut self, want: &str) -> Option<Token> {
        let t = self.expect(TokenKind::Ident)?;
        if self.text(&t) == want { Some(t) } else { None }
    }

    fn expect_punct(&mut self, want: char) -> Option<Token> {
        let t = self.expect(TokenKind::Punct)?;
        if self.text(&t) == want.to_string() { Some(t) } else { None }
    }

    fn text(&self, t: &Token) -> &str {
        &self.src[t.start..t.end]
    }

    pub fn parse_crate(&mut self) -> Crate {
        let mut items = Vec::new();
        while self.peek().is_some() {
            self.skip_whitespace();
            if self.peek().is_none() { break; }
            if let Some(item) = self.parse_item() {
                items.push(item);
            } else {
                self.bump();
            }
        }
        Crate { items }
    }

    fn parse_item(&mut self) -> Option<Item> {
        let (fn_tok, name) = self.parse_fn_head()?;
        let body = self.parse_block()?;
        let span = Span { start: fn_tok.start, end: 0 };
        Some(Item { name, kind: ItemKind::Fn(FnItem { body }), span })
    }

    fn parse_fn_head(&mut self) -> Option<(Token, String)> {
        self.skip_whitespace();
        let t = self.expect_ident("fn")?;
        self.skip_whitespace();
        let name_tok = self.expect(TokenKind::Ident)?;
        let name = self.text(&name_tok).to_string();
        self.expect_punct('(')?;
        self.expect_punct(')')?;
        Some((t, name))
    }

    fn parse_block(&mut self) -> Option<Block> {
        self.expect_punct('{')?;
        let tail = self.parse_expr();
        self.expect_punct('}')?;
        Some(Block { stmts: vec![], tail })
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        self.skip_whitespace();
        let t = self.bump()?;
        match t.kind {
            TokenKind::Int => {
                let n: i64 = self.text(&t).parse().ok()?;
                Some(Expr { kind: ExprKind::Int(n), span: Span { start: t.start, end: t.end } })
            }
            _ => None,
        }
    }
}

pub fn lower(tokens: &[Token], src: &str) -> Crate {
    Lowering::new(tokens, src).parse_crate()
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
        assert_eq!(krate.items[0].name, "main");
        match &krate.items[0].kind {
            ItemKind::Fn(f) => match &f.body.tail.as_ref().unwrap().kind {
                ExprKind::Int(42) => {}
                other => panic!("expected Int(42), got {other:?}"),
            },
        }
    }
}
