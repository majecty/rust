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
    Struct(StructItem),
    /// `def_fn!(foo)` — 아이템 위치 매크로 호출 (expand 전).
    Macro { name: Ident, args: Vec<Expr> },
    /// `macro_rules! twice { ($x:expr) => { $x + $x } }`
    MacroDef(MacroDef),
    // TODO: picks: Const/Mod
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructItem {
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: Ident,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Vis {
    Private,
    PubCrate,
    PubSuper,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    pub name: Ident,
    pub arms: Vec<MacroArm>,
    pub vis: Vis,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroArm {
    pub pattern: TokenTree,
    pub template: TokenTree,
    pub span: Span,
}

/// 미니 토큰 트리 — 패턴 매칭/전개에 사용.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenTree {
    Token(TokenNode),
    Delimited { kind: DelimKind, trees: Vec<TokenTree> },
    /// `$x:expr` 같은 패턴 와일드카드.
    Placeholder { name: String, frag: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenNode {
    Ident(String),
    Literal(String),
    Punct(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelimKind {
    Paren,  // ()
    Brace,  // {}
    Bracket, // []
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
    /// `<` — 지금은 Int 비교만, 결과는 0/1 (bool 타입 없음).
    Lt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Var(Ident),
    /// `foo(a, b)` — callee는 함수 이름만, 인자는 식.
    Call { callee: Ident, args: Vec<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    /// `Point { x: 1, y: 2 }` — 구조체 리터럴.
    StructLiteral { name: Ident, fields: Vec<FieldInit> },
    /// `p.x` — 필드 접근. `slot`은 resolve가 채우는 필드 순번(미해결이면 None).
    FieldAccess { base: Box<Expr>, field: Ident, slot: Option<u16> },
    /// `if cond { .. } else { .. }` — 식으로 평가된다 (else는 없을 수 있음).
    /// Block이 Expr을 품으므로 Box로 끊는다 (재귀 크기 방지).
    If { cond: Box<Expr>, then_block: Box<Block>, else_block: Option<Box<Block>> },
    /// `twice!(y)` — 함수형 매크로 호출 (expand 전).
    Macro { name: Ident, args: Vec<Expr> },
    // TODO: picks: Block
}

/// 구조체 리터럴의 필드 초기화.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
    /// resolve가 채우는 필드 순번(선언 순서). 미해결이면 None.
    pub slot: Option<u16>,
    pub span: Span,
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
