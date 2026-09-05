//! rtoy ast — 최소 얼개 (채워넣기용).
//! Original: compiler/rustc_ast (ast.rs, token.rs).

/// rustc_span 대비: 일단 start/end만.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Crate {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub name: String,
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Fn(FnItem),
    // TODO: picks: Struct/Const/Mod
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnItem {
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>, // `fn main(){42}`의 42
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    // TODO: picks: Let(..)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    // TODO: picks: Var, Call, Block
}

/// 빈 `fn main(){}` 더미 — driver 배선 확인용.
pub fn dummy_crate() -> Crate {
    let s = Span { start: 0, end: 0 };
    Crate {
        items: vec![Item {
            name: "main".into(),
            kind: ItemKind::Fn(FnItem {
                body: Block { stmts: vec![], tail: None },
            }),
            span: s,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_dummy() {
        assert_eq!(dummy_crate().items[0].name, "main");
    }
}
