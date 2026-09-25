//! rtoy hir — desugar된 AST + 이름 해석 결과 (rustc_hir 부분집합).
//! Original: compiler/rustc_ast_lowering (AST → HIR), compiler/rustc_hir/src/hir.rs.
//!
//! AST와 다른 점 (모두 `lower`가 만든다):
//! - 매크로가 없다 — `Macro` 노드가 남아 있으면 `HirLowerError::MacroNotExpanded`.
//! - 모든 노드가 `HirId{owner, local}`를 가진다 (rustc HIR과 같음).
//! - 표현식 문장이 `StmtKind::Semi`로 명시된다 (rustc `StmtKind::Semi`).
//! - fn 본문 최상위 `let`은 `Body::params`로 올라간다 (toy 인자 관례: eval `bind_args`/
//!   MIR prologue와 같다). 초기화식은 파라미터 기본값 `Local::init`으로 남는다
//!   (rustc `Param`에는 기본값이 없으므로 이 부분만 toy 편차다).
//! - 참조가 해석된 id로 바뀐다: Var→`LocalId`, Call→`Callee::User(FnId)`, 구조체→`StructId`.
//!   필드는 rustc처럼 이름만 남긴다 (필드 순번은 타입검사(THIR)가 정한다).
//! - 타입은 선언된 이름만 담는다 (`TyKind::Unknown` = 표에 없는 이름).

use rtoy_ast::{Block, Crate, Expr as AstExpr, ExprKind as AstExprKind, ItemKind, StmtKind as AstStmtKind};
use rtoy_span::Span;
use std::collections::HashMap;

pub use rtoy_ast::BinOp;

/// 아이템(구조체/함수)마다 하나 — rustc `OwnerId`.
pub type OwnerId = u32;
pub type StructId = u32;
pub type FnId = u32;
/// 프레임 slot 번호 = HIR에서 바인딩 id (rustc `Res::Local`의 id 자리).
pub type LocalId = u32;

/// 노드 하나를 가리키는 id (rustc `HirId{owner, local}`과 같은 얼개).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirId {
    pub owner: OwnerId,
    pub local: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TyKind {
    I64,
    Struct(StructId),
    /// 표에 없는 이름 — THIR이 `Ty::Infer`로 받는다.
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub owner: OwnerId,
    pub fields: Vec<FieldDef>,
    pub span: Span,
}

/// `let` 바인딩 또는 `fn` 파라미터 (파라미터는 `init` = 기본값).
#[derive(Debug, Clone, PartialEq)]
pub struct Local {
    pub hir_id: HirId,
    pub id: LocalId,
    pub name: String,
    pub ty: Option<Ty>,
    pub init: Option<Expr>,
    pub span: Span,
}

/// rustc HIR `Body{params, value}` — `value`는 블록 표현식이다.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub params: Vec<Local>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fn {
    pub name: String,
    pub owner: OwnerId,
    pub body: Body,
    pub span: Span,
}

/// 호출 대상 — 이름 해석 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Callee {
    User(FnId),
    Print,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub hir_id: HirId,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// rustc `ExprKind::Block(&Block, _)` — `tail`은 `Block::expr`.
    Block { stmts: Vec<Stmt>, tail: Option<Box<Expr>> },
    /// rustc `ExprKind::Lit` — toy는 정수 리터럴만.
    Literal(i64),
    /// rustc `ExprKind::Path(Res::Local)`.
    Var { local: LocalId, name: String },
    /// rustc `ExprKind::Binary` — 오버로딩 없음.
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    /// rustc `ExprKind::Call(callee, args)`.
    Call { callee: Callee, name: String, args: Vec<Expr> },
    /// rustc `ExprKind::Field(base, Ident)` — 필드 순번은 THIR이 정한다.
    Field { base: Box<Expr>, name: String },
    /// rustc `ExprKind::Struct(&QPath, &[ExprField], _)` — `(필드 순번, 값)`.
    Struct { struct_id: StructId, fields: Vec<(u32, Expr)> },
    /// rustc `ExprKind::If(cond, then, Option<else>)` — 두 분기 모두 블록 표현식.
    If { cond: Box<Expr>, then_block: Box<Expr>, else_block: Option<Box<Expr>> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub hir_id: HirId,
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let(Box<Local>),
    /// `expr;` — 값 버림 (rustc `StmtKind::Semi`).
    Semi(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirCrate {
    pub structs: Vec<StructDef>,
    pub fns: Vec<Fn>,
    pub fn_by_name: HashMap<String, FnId>,
    pub struct_by_name: HashMap<String, StructId>,
    pub main: Option<FnId>,
}

/// HIR lowering 실패 — AST 단계에서 남은 것과 해석 실패를 span과 함께 노출한다.
#[derive(Debug, Clone, PartialEq)]
pub enum HirLowerError {
    MacroNotExpanded { name: String, span: Span },
    UnknownFn { name: String, span: Span },
    UnknownStruct { name: String, span: Span },
    UnknownLocal { name: String, span: Span },
}

impl std::fmt::Display for HirLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (what, name, span) = match self {
            HirLowerError::MacroNotExpanded { name, span } => ("macro was not expanded", name, span),
            HirLowerError::UnknownFn { name, span } => ("unknown function", name, span),
            HirLowerError::UnknownStruct { name, span } => ("unknown struct", name, span),
            HirLowerError::UnknownLocal { name, span } => ("unknown local", name, span),
        };
        write!(f, "{} `{}` at [{}..{}]", what, name, span.lo, span.hi)
    }
}

impl std::error::Error for HirLowerError {}

/// 타입 이름 → HIR 타입. 모르는 이름은 `Unknown`(타입검사는 THIR 몫).
fn resolve_ty(name: &str, span: Span, structs: &HashMap<String, StructId>) -> Ty {
    let kind = match name {
        "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "bool" => TyKind::I64,
        other => match structs.get(other) {
            Some(id) => TyKind::Struct(*id),
            None => TyKind::Unknown(other.to_string()),
        },
    };
    Ty { kind, span }
}

/// AST(resolve 후)를 HIR로 내린다.
pub fn lower(krate: &Crate) -> Result<HirCrate, HirLowerError> {
    let (structs, struct_by_name) = collect_structs(krate);
    let (fn_items, fn_by_name) = collect_fns(krate);
    let main = fn_by_name.get("main").copied();
    let mut fns = Vec::with_capacity(fn_items.len());
    for (name, item, owner) in &fn_items {
        let mut cx = Lower::new(&struct_by_name, &fn_by_name);
        fns.push(cx.lower_fn(name, item, *owner)?);
    }
    Ok(HirCrate { structs, fns, fn_by_name, struct_by_name, main })
}

/// struct 표 (선언 순서 = StructId, 아이템 순서 = OwnerId).
fn collect_structs(krate: &Crate) -> (Vec<StructDef>, HashMap<String, StructId>) {
    let mut defs: Vec<StructDef> = Vec::new();
    let mut by_name = HashMap::new();
    for (owner, item) in krate.items.iter().enumerate() {
        if !matches!(item.kind, ItemKind::Struct(_)) {
            continue;
        }
        by_name.insert(item.name.name.clone(), defs.len() as StructId);
        // 필드 타입은 표가 다 모인 뒤에 채운다 (구조체끼리 참조 가능).
        defs.push(StructDef { name: item.name.name.clone(), owner: owner as OwnerId, fields: Vec::new(), span: item.span });
    }
    for item in &krate.items {
        let ItemKind::Struct(s) = &item.kind else { continue };
        let id = by_name[&item.name.name];
        defs[id as usize].fields = s
            .fields
            .iter()
            .map(|f| FieldDef { name: f.name.name.clone(), ty: resolve_ty(&f.ty.name, f.ty.span, &by_name), span: f.span })
            .collect();
    }
    (defs, by_name)
}

/// fn 표 (크레이트 순서 = FnId).
fn collect_fns<'a>(krate: &'a Crate) -> (Vec<(String, &'a rtoy_ast::FnItem, OwnerId)>, HashMap<String, FnId>) {
    let mut items = Vec::new();
    let mut by_name = HashMap::new();
    for (owner, item) in krate.items.iter().enumerate() {
        if let ItemKind::Fn(f) = &item.kind {
            by_name.insert(item.name.name.clone(), items.len() as FnId);
            items.push((item.name.name.clone(), f, owner as OwnerId));
        }
    }
    (items, by_name)
}

/// 함수 하나를 내리는 동안의 상태 (rustc `ast_lowering::LoweringContext` 자리).
struct Lower<'a> {
    struct_by_name: &'a HashMap<String, StructId>,
    fn_by_name: &'a HashMap<String, FnId>,
    owner: OwnerId,
    next_local: u32,
    /// 프레임 slot 순서의 지역변수 이름 (resolve의 `collect_local_names`와 같은 순서).
    names: Vec<String>,
}

impl<'a> Lower<'a> {
    fn new(struct_by_name: &'a HashMap<String, StructId>, fn_by_name: &'a HashMap<String, FnId>) -> Self {
        Lower { struct_by_name, fn_by_name, owner: 0, next_local: 0, names: Vec::new() }
    }

    fn hir_id(&mut self) -> HirId {
        let local = self.next_local;
        self.next_local += 1;
        HirId { owner: self.owner, local }
    }

    fn lower_fn(&mut self, name: &str, f: &rtoy_ast::FnItem, owner: OwnerId) -> Result<Fn, HirLowerError> {
        let mut names = Vec::new();
        rtoy_ast::collect_local_names(&f.body, &mut names);
        self.owner = owner;
        self.next_local = 0;
        self.names = names;

        // 최상위 let = 파라미터 (toy 관례). 순서는 소스 순서 그대로.
        let mut params = Vec::new();
        for stmt in &f.body.stmts {
            if let AstStmtKind::Let(l) = &stmt.kind {
                params.push(self.lower_local(l)?);
            }
        }
        let value = self.lower_block(&f.body, true)?;
        Ok(Fn { name: name.to_string(), owner, body: Body { params, value, span: f.body.span }, span: f.span })
    }

    fn lower_local(&mut self, l: &rtoy_ast::LetStmt) -> Result<Local, HirLowerError> {
        let hir_id = self.hir_id();
        let id = match l.slot {
            Some(slot) => slot as LocalId,
            None => self.slot_of(&l.name.name, l.name.span)?,
        };
        let ty = l.ty.as_ref().map(|t| resolve_ty(&t.name, t.span, self.struct_by_name));
        let init = match &l.init {
            Some(e) => Some(self.lower_expr(e)?),
            None => None,
        };
        Ok(Local { hir_id, id, name: l.name.name.clone(), ty, init, span: l.span })
    }

    /// 블록 → 블록 표현식. `skip_lets`면 `let`을 건너뛴다 (fn 본문은 이미 params로 올렸다).
    fn lower_block(&mut self, block: &Block, skip_lets: bool) -> Result<Expr, HirLowerError> {
        let hir_id = self.hir_id();
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            let stmt_id = self.hir_id();
            let kind = match &stmt.kind {
                AstStmtKind::Let(l) if skip_lets => continue,
                AstStmtKind::Let(l) => StmtKind::Let(Box::new(self.lower_local(l)?)),
                AstStmtKind::Expr(e) => StmtKind::Semi(self.lower_expr(e)?),
            };
            stmts.push(Stmt { hir_id: stmt_id, kind, span: stmt.span });
        }
        let tail = match &block.tail {
            Some(t) => Some(Box::new(self.lower_expr(t)?)),
            None => None,
        };
        Ok(Expr { hir_id, kind: ExprKind::Block { stmts, tail }, span: block.span })
    }

    fn lower_expr(&mut self, e: &AstExpr) -> Result<Expr, HirLowerError> {
        let hir_id = self.hir_id();
        let kind = match &e.kind {
            AstExprKind::Int(n) => ExprKind::Literal(*n),
            AstExprKind::Var { name, slot } => {
                let local = match slot {
                    Some(slot) => *slot as LocalId,
                    None => self.slot_of(&name.name, name.span)?,
                };
                ExprKind::Var { local, name: name.name.clone() }
            }
            AstExprKind::Binary { op, lhs, rhs } => ExprKind::Binary {
                op: *op,
                lhs: Box::new(self.lower_expr(lhs)?),
                rhs: Box::new(self.lower_expr(rhs)?),
            },
            AstExprKind::Call { callee, args, .. } => {
                let callee_kind = if callee.name == "print" {
                    Callee::Print
                } else {
                    match self.fn_by_name.get(&callee.name) {
                        Some(id) => Callee::User(*id),
                        None => return Err(HirLowerError::UnknownFn { name: callee.name.clone(), span: callee.span }),
                    }
                };
                let mut lowered = Vec::with_capacity(args.len());
                for a in args {
                    lowered.push(self.lower_expr(a)?);
                }
                ExprKind::Call { callee: callee_kind, name: callee.name.clone(), args: lowered }
            }
            AstExprKind::FieldAccess { base, field, .. } => {
                ExprKind::Field { base: Box::new(self.lower_expr(base)?), name: field.name.clone() }
            }
            AstExprKind::StructLiteral { name, fields } => {
                let Some(struct_id) = self.struct_by_name.get(&name.name).copied() else {
                    return Err(HirLowerError::UnknownStruct { name: name.name.clone(), span: name.span });
                };
                let mut lowers = Vec::with_capacity(fields.len());
                for (index, f) in fields.iter().enumerate() {
                    let slot = f.slot.map(u32::from).unwrap_or(index as u32);
                    lowers.push((slot, self.lower_expr(&f.value)?));
                }
                ExprKind::Struct { struct_id, fields: lowers }
            }
            AstExprKind::If { cond, then_block, else_block } => {
                let cond = Box::new(self.lower_expr(cond)?);
                let then_block = Box::new(self.lower_block(then_block, false)?);
                let else_block = match else_block {
                    Some(b) => Some(Box::new(self.lower_block(b, false)?)),
                    None => None,
                };
                ExprKind::If { cond, then_block, else_block }
            }
            AstExprKind::Macro { name, .. } => {
                return Err(HirLowerError::MacroNotExpanded { name: name.name.clone(), span: e.span })
            }
        };
        Ok(Expr { hir_id, kind, span: e.span })
    }

    /// 변수 이름 → 프레임 slot (resolve가 slot을 안 채운 AST용 fallback).
    fn slot_of(&self, name: &str, span: Span) -> Result<LocalId, HirLowerError> {
        self.names
            .iter()
            .position(|n| n == name)
            .map(|i| i as LocalId)
            .ok_or_else(|| HirLowerError::UnknownLocal { name: name.to_string(), span })
    }
}

fn binop_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Lt => "<",
    }
}

impl HirCrate {
    /// 사람이 읽는 HIR 덤프 — `rustc -Zunpretty=hir` 흉내 (desugar된 소스 + id 주석).
    pub fn dump(&self) -> String {
        let mut out = String::from("// hir — macro 없음 · Semi 명시 · fn 최상위 let = param · `#n`=HirId\n");
        for s in &self.structs {
            let fields: Vec<String> = s.fields.iter().map(|f| format!("{}: {}", f.name, self.ty_str(&f.ty))).collect();
            out.push_str(&format!("struct {} {{ {} }} // #{}\n", s.name, fields.join(", "), s.owner));
        }
        for f in &self.fns {
            out.push_str(&self.fn_str(f));
        }
        out
    }

    fn fn_str(&self, f: &Fn) -> String {
        let params: Vec<String> = f
            .body
            .params
            .iter()
            .map(|p| {
                let default = p.init.as_ref().map(|e| format!(" = {}", self.expr_str(e))).unwrap_or_default();
                format!("{}: {}{default}", p.name, self.local_ty_str(p))
            })
            .collect();
        let mut out = format!("fn {}({}) {{ // #{}\n", f.name, params.join(", "), f.owner);
        if let ExprKind::Block { stmts, tail } = &f.body.value.kind {
            for s in stmts {
                out.push_str(&format!("    {}\n", self.stmt_str(s)));
            }
            if let Some(t) = tail {
                out.push_str(&format!("    {}\n", self.expr_str(t)));
            }
        }
        out.push_str("}\n");
        out
    }

    fn stmt_str(&self, s: &Stmt) -> String {
        match &s.kind {
            StmtKind::Let(l) => format!("let {}: {} = {};", l.name, self.local_ty_str(l), self.init_str(l)),
            StmtKind::Semi(e) => format!("{};", self.expr_str(e)),
        }
    }

    fn expr_str(&self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Literal(n) => n.to_string(),
            ExprKind::Var { local, name } => format!("{name}#{local}"),
            ExprKind::Binary { op, lhs, rhs } => {
                format!("{} {} {}", self.expr_str(lhs), binop_str(*op), self.expr_str(rhs))
            }
            ExprKind::Call { name, args, .. } => {
                let a: Vec<String> = args.iter().map(|x| self.expr_str(x)).collect();
                format!("{}({})", name, a.join(", "))
            }
            ExprKind::Field { base, name } => format!("{}.{}", self.expr_str(base), name),
            ExprKind::Struct { struct_id, fields } => {
                let parts: Vec<String> = fields.iter().map(|(_, v)| self.expr_str(v)).collect();
                format!("{} {{ {} }}", self.struct_name(*struct_id), parts.join(", "))
            }
            ExprKind::If { cond, then_block, else_block } => {
                let mut s = format!("if {} {}", self.expr_str(cond), self.expr_str(then_block));
                if let Some(e) = else_block {
                    s.push_str(&format!(" else {}", self.expr_str(e)));
                }
                s
            }
            ExprKind::Block { stmts, tail } => {
                let mut parts: Vec<String> = stmts.iter().map(|s| self.stmt_str(s)).collect();
                if let Some(t) = tail {
                    parts.push(self.expr_str(t));
                }
                format!("{{ {} }}", parts.join(" "))
            }
        }
    }

    fn init_str(&self, l: &Local) -> String {
        match &l.init {
            Some(e) => self.expr_str(e),
            None => "_".to_string(),
        }
    }

    fn local_ty_str(&self, l: &Local) -> String {
        match &l.ty {
            Some(t) => self.ty_str(t),
            None => "?".to_string(),
        }
    }

    fn ty_str(&self, t: &Ty) -> String {
        match &t.kind {
            TyKind::I64 => "i64".to_string(),
            TyKind::Struct(id) => self.struct_name(*id),
            TyKind::Unknown(name) => name.clone(),
        }
    }

    fn struct_name(&self, id: StructId) -> String {
        self.structs.get(id as usize).map(|s| s.name.clone()).unwrap_or_else(|| format!("<struct {id}>"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 소스 → (lex → tokenstream lowering → HIR). expand/resolve는 거치지 않는다 —
    /// `lower`가 이름으로 표와 slot을 다시 찾는다.
    fn hir_of(src: &str) -> HirCrate {
        let tokens = rtoy_lexer::tokenize(src);
        let ast = rtoy_tokenstream_lowering::try_lower(&tokens, src).expect("ast");
        lower(&ast).expect("hir")
    }

    fn main_fn(hir: &HirCrate) -> &Fn {
        &hir.fns[hir.fn_by_name["main"] as usize]
    }

    /// fn 본문 블록의 (문장들, tail).
    fn body_parts(f: &Fn) -> (&[Stmt], Option<&Expr>) {
        match &f.body.value.kind {
            ExprKind::Block { stmts, tail } => (stmts, tail.as_deref()),
            other => panic!("fn body value는 블록이어야 한다: {other:?}"),
        }
    }

    #[test]
    fn top_level_let_becomes_param() {
        // `fn main(){ let n: i64 = 0; n }` → param 1개 + 본문 tail
        let hir = hir_of("fn main() { let n: i64 = 0; n }");
        let f = main_fn(&hir);
        assert_eq!(f.body.params.len(), 1);
        let p = &f.body.params[0];
        assert_eq!(p.name, "n");
        assert_eq!(p.ty.as_ref().map(|t| t.kind.clone()), Some(TyKind::I64));
        assert!(p.init.is_some(), "초기화식은 프라미터 기본값으로 남는다");
        let (stmts, tail) = body_parts(f);
        assert!(stmts.is_empty(), "승격된 let은 본문에서 사라진다");
        assert_eq!(tail.map(|t| t.kind.clone()), Some(ExprKind::Var { local: 0, name: "n".into() }));
    }

    #[test]
    fn expr_stmt_becomes_semi() {
        // `fn main(){ 1; 2 }` → Semi(1) + tail(2)
        let hir = hir_of("fn main() { 1; 2 }");
        let (stmts, tail) = body_parts(main_fn(&hir));
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0].kind, StmtKind::Semi(e) if e.kind == ExprKind::Literal(1)));
        assert_eq!(tail.map(|t| t.kind.clone()), Some(ExprKind::Literal(2)));
    }

    #[test]
    fn unexpanded_macro_is_rejected() {
        // `twice!`는 expand 전이므로 HIR은 매크로를 받지 않는다.
        let src = "fn main() { twice!(1) }";
        let tokens = rtoy_lexer::tokenize(src);
        let ast = rtoy_tokenstream_lowering::try_lower(&tokens, src).expect("ast");
        match lower(&ast) {
            Err(HirLowerError::MacroNotExpanded { name, .. }) => assert_eq!(name, "twice"),
            other => panic!("MacroNotExpanded를 기대: {other:?}"),
        }
    }

    #[test]
    fn call_and_struct_resolve_to_ids() {
        let src = "struct Point { x: i64, y: i64 } fn helper() { 1 } \
                   fn main() { let p = Point { x: 1, y: 2 }; helper(); p.x }";
        let hir = hir_of(src);
        assert_eq!(hir.structs.len(), 1);
        assert_eq!(hir.structs[0].fields[1].name, "y");
        assert_eq!(hir.fn_by_name.get("helper").copied(), Some(0));
        let f = main_fn(&hir);
        assert_eq!(f.body.params[0].name, "p");
        assert!(
            matches!(f.body.params[0].init.as_ref().map(|e| &e.kind), Some(ExprKind::Struct { struct_id: 0, .. })),
            "리터럴이 StructId로 해석된다"
        );
        let (stmts, tail) = body_parts(f);
        assert!(
            matches!(&stmts[0].kind, StmtKind::Semi(Expr { kind: ExprKind::Call { callee: Callee::User(0), .. }, .. })),
            "호출이 FnId로 해석된다: {:?}",
            stmts[0]
        );
        assert!(matches!(&tail.unwrap().kind, ExprKind::Field { name, .. } if name == "x"));
        // 덤프: 프라미터 기본값과 Semi 문장이 그대로 보인다.
        let dump = hir.dump();
        assert!(dump.contains("Point { 1, 2 }"), "{dump}");
        assert!(dump.contains("helper();"), "{dump}");
    }

    #[test]
    fn hir_ids_are_unique_within_owner() {
        let hir = hir_of("fn main() { let a = 1; a + 2 }");
        let f = main_fn(&hir);
        let mut seen = Vec::new();
        collect_ids(&f.body.value, &mut seen);
        let unique: std::collections::HashSet<u32> = seen.iter().map(|id| id.local).collect();
        assert_eq!(unique.len(), seen.len(), "owner 안에서 HirId는 서로 달라야 한다");
        assert!(seen.len() > 3, "블록/문장/식 id가 모두 잡혀야 한다: {seen:?}");
    }

    /// 블록 표현식의 HirId를 전부 모은다 (트리 크기 = id 개수 확인용).
    fn collect_ids(e: &Expr, out: &mut Vec<HirId>) {
        out.push(e.hir_id);
        match &e.kind {
            ExprKind::Block { stmts, tail } => {
                for s in stmts {
                    out.push(s.hir_id);
                    match &s.kind {
                        StmtKind::Let(l) => {
                            out.push(l.hir_id);
                            if let Some(init) = &l.init {
                                collect_ids(init, out);
                            }
                        }
                        StmtKind::Semi(e) => collect_ids(e, out),
                    }
                }
                if let Some(t) = tail {
                    collect_ids(t, out);
                }
            }
            ExprKind::Binary { lhs, rhs, .. } => {
                collect_ids(lhs, out);
                collect_ids(rhs, out);
            }
            ExprKind::Call { args, .. } => args.iter().for_each(|a| collect_ids(a, out)),
            ExprKind::Field { base, .. } => collect_ids(base, out),
            ExprKind::Struct { fields, .. } => fields.iter().for_each(|(_, v)| collect_ids(v, out)),
            ExprKind::If { cond, then_block, else_block } => {
                collect_ids(cond, out);
                collect_ids(then_block, out);
                if let Some(b) = else_block {
                    collect_ids(b, out);
                }
            }
            ExprKind::Literal(_) | ExprKind::Var { .. } => {}
        }
    }
}
