//! rtoy ast — 최소 얼개 (채워넣기용).
//! Original: compiler/rustc_ast (ast.rs, token.rs).

pub use rtoy_span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ty {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Crate {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub name: Ident,
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
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>, // `fn main(){42}`의 42
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Expr(Expr),
    Let(LetStmt),
}

/// rustc처럼 stmt 자체가 span을 가진다 (`Stmt{kind, span}`).
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub name: Ident,
    pub ty: Option<Ty>,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// 이항 연산자. 우선순위는 lowering 파서가 처리.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Var(Ident),
    /// `foo(a, b)` — callee는 함수 이름만, 인자는 식.
    Call { callee: Ident, args: Vec<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    /// `twice!(y)` — 함수형 매크로 호출 (expand 전).
    Macro { name: Ident, args: Vec<Expr> },
    // TODO: picks: Block
}

/// 빈 `fn main(){}` 더미 — driver 배선 확인용.
pub fn dummy_crate() -> Crate {
    let s = Span::root(0, 0);
    Crate {
        span: s,
        items: vec![Item {
            name: Ident { name: "main".into(), span: s },
            kind: ItemKind::Fn(FnItem {
                body: Block { stmts: vec![], tail: None, span: s },
                span: s,
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
        assert_eq!(dummy_crate().items[0].name.name, "main");
    }
}
