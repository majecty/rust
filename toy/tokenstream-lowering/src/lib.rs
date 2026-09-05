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

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(t) if t.kind == TokenKind::Whitespace) {
            self.bump();
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Option<Token> {
        self.skip_ws();
        let t = self.bump()?;
        if t.kind == kind { Some(t) } else { None }
    }

    fn text(&self, t: &Token) -> &str {
        &self.src[t.start..t.end]
    }

    pub fn parse_crate(&mut self) -> Crate {
        let mut items = Vec::new();
        while self.peek().is_some() {
            self.skip_ws();
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
        self.skip_ws();
        let t = self.bump()?;
        if t.kind != TokenKind::Ident || self.text(&t) != "fn" { return None; }
        self.skip_ws();
        let name_tok = self.bump()?;
        if name_tok.kind != TokenKind::Ident { return None; }
        let name = self.text(&name_tok).to_string();
        self.expect(TokenKind::Punct)?; // (
        self.expect(TokenKind::Punct)?; // )
        self.expect(TokenKind::Punct)?; // {
        let tail = self.parse_expr();
        self.expect(TokenKind::Punct)?; // }
        let span = Span { start: t.start, end: 0 };
        Some(Item {
            name,
            kind: ItemKind::Fn(FnItem { body: Block { stmts: vec![], tail } }),
            span,
        })
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        self.skip_ws();
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
            ItemKind::Fn(f) => match f.body.tail.as_ref().unwrap().kind {
                ExprKind::Int(42) => {}
                other => panic!("expected Int(42), got {other:?}"),
            },
        }
    }
}
