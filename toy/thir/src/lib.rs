//! rtoy thir — 타입이 붙은 HIR (rustc_middle::thir 부분집합 + thir lowering/typeck).
//! Original: compiler/rustc_middle/src/thir.rs, compiler/rustc_hir_analysis/src/thir/ (cx/expr.rs).
//!
//! HIR과 다른 점 (모두 `lower_crate`가 만든다):
//! - 표현식이 arena(`exprs`)에 평탄화되고 `ExprId`로 참조된다 (rustc THIR과 같음).
//! - 모든 표현식이 `Ty`를 확정한다 — toy 첫 타입검사다. rustc는 typeck 결과를 받지만
//!   여기서는 lowering이 직접 추론한다(최소형). 모르는 타입은 `Ty::Infer`.
//! - 블록/문장/파라미터가 `BlockId`/`StmtId`/`ParamId`로 분리된다.
//! - HIR `Field`는 이름만 가지므로 필드 순번과 구조체 id는 여기서 정한다
//!   (rustc THIR `Field{lhs, variant_index, name: FieldIdx}`와 같은 자리).
//! - `Param::default`는 toy 편차다 (rustc THIR `Param`에는 기본값이 없다).

use rtoy_hir::{BinOp, Callee, ExprKind as HirExprKind, FnId, HirCrate, LocalId, StmtKind as HirStmtKind, StructId, TyKind};
use rtoy_span::Span;
use std::collections::HashMap;

pub type ExprId = u32;
pub type StmtId = u32;
pub type BlockId = u32;
pub type ParamId = u32;

/// THIR 타입 — toy는 i64/()/구조체 + 추론 변수(`Infer`)뿐이다.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    I64,
    Unit,
    Struct(StructId),
    /// 아직 모름 (rustc `TyKind::Infer`). 남아 있으면 덤프에 `_`로 나온다.
    Infer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal(i64),
    VarRef { local: LocalId, name: String },
    Binary { op: BinOp, lhs: ExprId, rhs: ExprId },
    Call { callee: Callee, name: String, args: Vec<ExprId> },
    If { cond: ExprId, then_block: ExprId, else_opt: Option<ExprId> },
    Block { block: BlockId },
    /// `struct_id`/`index`는 기반 타입을 알아낸 뒤에만 채워진다 (모르면 `Infer`).
    Field { lhs: ExprId, name: String, struct_id: Option<StructId>, index: Option<u32> },
    Adt { struct_id: StructId, fields: Vec<(u32, ExprId)> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<StmtId>,
    pub expr: Option<ExprId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Expr { expr: ExprId },
    /// rustc THIR `StmtKind::Let` — `initializer: None`이면 `let x;`.
    Let { local: LocalId, name: String, ty: Ty, initializer: Option<ExprId> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub local: LocalId,
    pub name: String,
    pub ty: Ty,
    /// toy 편차: HIR에서 올라온 기본값(초기화식) 표현식.
    pub default: Option<ExprId>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub params: Vec<ParamId>,
    pub value: ExprId,
    pub ty: Ty,
}

/// 함수 하나의 THIR (rustc `Thir` + `Body`).
#[derive(Debug, Clone, PartialEq)]
pub struct Thir {
    pub name: String,
    pub fn_id: FnId,
    pub body: Body,
    pub params: Vec<Param>,
    pub blocks: Vec<Block>,
    pub stmts: Vec<Stmt>,
    pub exprs: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThirCrate {
    pub fns: Vec<Thir>,
    pub fn_by_name: HashMap<String, FnId>,
    pub structs: Vec<StructDef>,
    pub main: Option<FnId>,
}

/// 타입검사 실패 — 기대/발견 타입과 위치를 그대로 담는다.
#[derive(Debug, Clone, PartialEq)]
pub enum ThirLowerError {
    MismatchedTypes { context: &'static str, expected: String, found: String, span: Span },
    NotAStruct { context: &'static str, found: String, span: Span },
    UnknownField { struct_name: String, field: String, span: Span },
    WrongArgCount { name: String, expected: usize, found: usize, span: Span },
}

impl std::fmt::Display for ThirLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let loc = |span: &Span| format!("at [{}..{}]", span.lo, span.hi);
        match self {
            ThirLowerError::MismatchedTypes { context, expected, found, span } => write!(
                f,
                "mismatched types in {context}: expected `{expected}`, found `{found}` {}",
                loc(span)
            ),
            ThirLowerError::NotAStruct { context, found, span } => {
                write!(f, "expected struct in {context}, found `{found}` {}", loc(span))
            }
            ThirLowerError::UnknownField { struct_name, field, span } => {
                write!(f, "no field `{field}` on struct `{struct_name}` {}", loc(span))
            }
            ThirLowerError::WrongArgCount { name, expected, found, span } => {
                write!(f, "this function takes {expected} arguments but {found} were supplied to `{name}` {}", loc(span))
            }
        }
    }
}

impl std::error::Error for ThirLowerError {}

/// HIR을 THIR로 내린다(타입검사 포함). 에러가 있으면 모두 모아 돌려준다.
pub fn lower_crate(krate: &HirCrate) -> Result<ThirCrate, Vec<ThirLowerError>> {
    let structs = build_structs(krate);
    let sigs = build_param_sigs(krate);
    let mut ret: Vec<Ty> = vec![Ty::Infer; krate.fns.len()];
    // 반환 타입 fixpoint — 재귀(`fib`→`fib`)에서는 한 바퀴로 모자란다 (rustc는 typeck가 채운다).
    for _ in 0..=krate.fns.len() {
        let (probe, _) = lower_all(krate, &structs, &sigs, &ret);
        let next: Vec<Ty> = probe.iter().map(|f| f.body.ty.clone()).collect();
        if next == ret {
            break;
        }
        ret = next;
    }
    let (fns, errors) = lower_all(krate, &structs, &sigs, &ret);
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(ThirCrate { fns, fn_by_name: krate.fn_by_name.clone(), structs, main: krate.main })
}

fn build_structs(krate: &HirCrate) -> Vec<StructDef> {
    krate
        .structs
        .iter()
        .map(|s| StructDef {
            name: s.name.clone(),
            fields: s.fields.iter().map(|f| FieldDef { name: f.name.clone(), ty: ty_from_hir(&f.ty.kind) }).collect(),
        })
        .collect()
}

/// HIR `TyKind` → THIR `Ty`. 표에 없는 이름은 `Infer` (타입검사를 건너뛴다).
fn ty_from_hir(kind: &TyKind) -> Ty {
    match kind {
        TyKind::I64 => Ty::I64,
        TyKind::Struct(id) => Ty::Struct(*id),
        TyKind::Unknown(_) => Ty::Infer,
    }
}

/// fn별 파라미터 타입 — 선언된 타입만 본다 (기본값에서 유추하는 건 본문 lowering 몫).
fn build_param_sigs(krate: &HirCrate) -> Vec<Vec<Ty>> {
    krate
        .fns
        .iter()
        .map(|f| f.body.params.iter().map(|p| p.ty.as_ref().map(|t| ty_from_hir(&t.kind)).unwrap_or(Ty::Infer)).collect())
        .collect()
}

fn lower_all(krate: &HirCrate, structs: &[StructDef], sigs: &[Vec<Ty>], ret: &[Ty]) -> (Vec<Thir>, Vec<ThirLowerError>) {
    let mut bodies = Vec::with_capacity(krate.fns.len());
    let mut errors = Vec::new();
    for (id, f) in krate.fns.iter().enumerate() {
        let mut cx = BodyLower {
            structs,
            sigs,
            ret,
            fn_id: id as FnId,
            locals: HashMap::new(),
            exprs: Vec::new(),
            stmts: Vec::new(),
            blocks: Vec::new(),
            errors: Vec::new(),
        };
        bodies.push(cx.lower_fn(f));
        errors.append(&mut cx.errors);
    }
    (bodies, errors)
}

/// 함수 하나를 내리는 동안의 상태 (rustc `thir::cx::Cx` 자리).
struct BodyLower<'a> {
    structs: &'a [StructDef],
    sigs: &'a [Vec<Ty>],
    ret: &'a [Ty],
    fn_id: FnId,
    locals: HashMap<LocalId, Ty>,
    exprs: Vec<Expr>,
    stmts: Vec<Stmt>,
    blocks: Vec<Block>,
    errors: Vec<ThirLowerError>,
}

impl<'a> BodyLower<'a> {
    fn lower_fn(&mut self, f: &rtoy_hir::Fn) -> Thir {
        let mut params = Vec::with_capacity(f.body.params.len());
        for p in &f.body.params {
            let default = p.init.as_ref().map(|e| self.lower_expr(e));
            let declared = p.ty.as_ref().map(|t| ty_from_hir(&t.kind));
            let ty = match (declared, default) {
                (Some(declared), Some(init)) => {
                    let found = self.ty_of(init);
                    self.expect(&found, &declared, "parameter", p.span);
                    declared
                }
                (Some(declared), None) => declared,
                (None, Some(init)) => self.ty_of(init),
                (None, None) => Ty::Infer,
            };
            self.locals.insert(p.id, ty.clone());
            params.push(Param { local: p.id, name: p.name.clone(), ty, default, span: p.span });
        }
        let value = self.lower_expr(&f.body.value);
        let ty = self.ty_of(value);
        let body = Body { params: (0..params.len() as ParamId).collect(), value, ty };
        Thir {
            name: f.name.clone(),
            fn_id: self.fn_id,
            body,
            params,
            blocks: std::mem::take(&mut self.blocks),
            stmts: std::mem::take(&mut self.stmts),
            exprs: std::mem::take(&mut self.exprs),
            span: f.span,
        }
    }

    /// 자식 표현식을 먼저 arena에 넣고 부모를 붙인다 (rustc THIR도 자식이 먼저다).
    fn lower_expr(&mut self, e: &rtoy_hir::Expr) -> ExprId {
        let (kind, ty) = match &e.kind {
            HirExprKind::Literal(n) => (ExprKind::Literal(*n), Ty::I64),
            HirExprKind::Var { local, name } => {
                let ty = self.locals.get(local).cloned().unwrap_or(Ty::Infer);
                (ExprKind::VarRef { local: *local, name: name.clone() }, ty)
            }
            HirExprKind::Binary { op, lhs, rhs } => {
                let lhs_id = self.lower_expr(lhs);
                let rhs_id = self.lower_expr(rhs);
                let (lhs_ty, rhs_ty) = (self.ty_of(lhs_id), self.ty_of(rhs_id));
                self.expect(&lhs_ty, &Ty::I64, "binary op lhs", lhs.span);
                self.expect(&rhs_ty, &Ty::I64, "binary op rhs", rhs.span);
                (ExprKind::Binary { op: *op, lhs: lhs_id, rhs: rhs_id }, Ty::I64)
            }
            HirExprKind::Call { callee, name, args } => {
                let ids: Vec<ExprId> = args.iter().map(|a| self.lower_expr(a)).collect();
                (ExprKind::Call { callee: *callee, name: name.clone(), args: ids.clone() }, self.call_ty(*callee, name, &ids, args, e.span))
            }
            HirExprKind::Field { base, name } => {
                let lhs = self.lower_expr(base);
                match self.ty_of(lhs) {
                    Ty::Struct(struct_id) => match self.field_index(struct_id, name) {
                        Some(index) => (
                            ExprKind::Field { lhs, name: name.clone(), struct_id: Some(struct_id), index: Some(index) },
                            self.field_ty(struct_id, index),
                        ),
                        None => {
                            self.errors.push(ThirLowerError::UnknownField {
                                struct_name: self.struct_name(struct_id),
                                field: name.clone(),
                                span: e.span,
                            });
                            (ExprKind::Field { lhs, name: name.clone(), struct_id: Some(struct_id), index: None }, Ty::Infer)
                        }
                    },
                    other => {
                        if other != Ty::Infer {
                            self.errors.push(ThirLowerError::NotAStruct {
                                context: "field access",
                                found: self.ty_str(&other),
                                span: e.span,
                            });
                        }
                        (ExprKind::Field { lhs, name: name.clone(), struct_id: None, index: None }, Ty::Infer)
                    }
                }
            }
            HirExprKind::Struct { struct_id, fields } => {
                let mut lowered = Vec::with_capacity(fields.len());
                for (index, value) in fields {
                    let id = self.lower_expr(value);
                    let found = self.ty_of(id);
                    let expected = self.field_ty(*struct_id, *index);
                    self.expect(&found, &expected, "struct field", value.span);
                    lowered.push((*index, id));
                }
                (ExprKind::Adt { struct_id: *struct_id, fields: lowered }, Ty::Struct(*struct_id))
            }
            HirExprKind::If { cond, then_block, else_block } => {
                let cond_id = self.lower_expr(cond);
                let cond_ty = self.ty_of(cond_id);
                self.expect(&cond_ty, &Ty::I64, "if condition", cond.span);
                let then_id = self.lower_expr(then_block);
                let then_ty = self.ty_of(then_id);
                let (else_opt, ty) = match else_block {
                    Some(block) => {
                        let else_id = self.lower_expr(block);
                        let else_ty = self.ty_of(else_id);
                        let ty = self.unify(&then_ty, &else_ty, "if branches", e.span);
                        (Some(else_id), ty)
                    }
                    None => {
                        self.expect(&then_ty, &Ty::Unit, "if without else", then_block.span);
                        (None, Ty::Unit)
                    }
                };
                (ExprKind::If { cond: cond_id, then_block: then_id, else_opt }, ty)
            }
            HirExprKind::Block { stmts, tail } => {
                let ids: Vec<StmtId> = stmts.iter().map(|s| self.lower_stmt(s)).collect();
                let tail_id = tail.as_ref().map(|t| self.lower_expr(t));
                let ty = tail_id.map(|id| self.ty_of(id)).unwrap_or(Ty::Unit);
                let block = self.blocks.len() as BlockId;
                self.blocks.push(Block { stmts: ids, expr: tail_id });
                (ExprKind::Block { block }, ty)
            }
        };
        self.new_expr(kind, ty, e.span)
    }

    fn lower_stmt(&mut self, s: &rtoy_hir::Stmt) -> StmtId {
        let kind = match &s.kind {
            HirStmtKind::Semi(e) => StmtKind::Expr { expr: self.lower_expr(e) },
            HirStmtKind::Let(l) => {
                let initializer = l.init.as_ref().map(|e| self.lower_expr(e));
                let declared = l.ty.as_ref().map(|t| ty_from_hir(&t.kind));
                let ty = match (declared, initializer) {
                    (Some(declared), Some(init)) => {
                        let found = self.ty_of(init);
                        self.expect(&found, &declared, "let initializer", l.span);
                        declared
                    }
                    (Some(declared), None) => declared,
                    (None, Some(init)) => self.ty_of(init),
                    (None, None) => Ty::Infer,
                };
                self.locals.insert(l.id, ty.clone());
                StmtKind::Let { local: l.id, name: l.name.clone(), ty, initializer }
            }
        };
        let id = self.stmts.len() as StmtId;
        self.stmts.push(Stmt { kind, span: s.span });
        id
    }

    /// 호출 타입 — 인자 개수/타입을 보고 반환 타입을 고른다.
    fn call_ty(&mut self, callee: Callee, name: &str, ids: &[ExprId], args: &[rtoy_hir::Expr], span: Span) -> Ty {
        match callee {
            Callee::Print => {
                for (arg, id) in args.iter().zip(ids) {
                    let found = self.ty_of(*id);
                    self.expect(&found, &Ty::I64, "print argument", arg.span);
                }
                Ty::Unit
            }
            Callee::User(fn_id) => {
                let params = self.sigs.get(fn_id as usize).cloned().unwrap_or_default();
                if params.len() != ids.len() {
                    self.errors.push(ThirLowerError::WrongArgCount {
                        name: name.to_string(),
                        expected: params.len(),
                        found: ids.len(),
                        span,
                    });
                }
                for (expected, id) in params.iter().zip(ids) {
                    let found = self.ty_of(*id);
                    self.expect(&found, expected, "call argument", span);
                }
                self.ret.get(fn_id as usize).cloned().unwrap_or(Ty::Infer)
            }
        }
    }

    /// `p.x` — 기반 타입이 구조체일 때만 필드 순번을 확정한다 (위 `lower_expr` 안에서 쓴다).
    fn field_index(&self, struct_id: StructId, name: &str) -> Option<u32> {
        self.structs
            .get(struct_id as usize)
            .and_then(|s| s.fields.iter().position(|f| f.name == name))
            .map(|i| i as u32)
    }

    fn field_ty(&self, struct_id: StructId, index: u32) -> Ty {
        self.structs
            .get(struct_id as usize)
            .and_then(|s| s.fields.get(index as usize))
            .map(|f| f.ty.clone())
            .unwrap_or(Ty::Infer)
    }

    fn struct_name(&self, id: StructId) -> String {
        self.structs.get(id as usize).map(|s| s.name.clone()).unwrap_or_else(|| format!("<struct {id}>"))
    }

    /// 두 타입을 맞춘다 — `Infer`는 상대를 따른다 (rustc typeck의 unify 최소형).
    fn unify(&mut self, a: &Ty, b: &Ty, context: &'static str, span: Span) -> Ty {
        if a == b {
            return a.clone();
        }
        if *a == Ty::Infer {
            return b.clone();
        }
        if *b == Ty::Infer {
            return a.clone();
        }
        self.push_mismatch(context, a, b, span);
        a.clone()
    }

    fn expect(&mut self, found: &Ty, expected: &Ty, context: &'static str, span: Span) {
        if found == expected || *found == Ty::Infer || *expected == Ty::Infer {
            return;
        }
        self.push_mismatch(context, expected, found, span);
    }

    fn push_mismatch(&mut self, context: &'static str, expected: &Ty, found: &Ty, span: Span) {
        self.errors.push(ThirLowerError::MismatchedTypes {
            context,
            expected: self.ty_str(expected),
            found: self.ty_str(found),
            span,
        });
    }

    fn ty_str(&self, ty: &Ty) -> String {
        match ty {
            Ty::I64 => "i64".to_string(),
            Ty::Unit => "()".to_string(),
            Ty::Struct(id) => self.struct_name(*id),
            Ty::Infer => "_".to_string(),
        }
    }

    fn ty_of(&self, id: ExprId) -> Ty {
        self.exprs.get(id as usize).map(|e| e.ty.clone()).unwrap_or(Ty::Infer)
    }

    fn new_expr(&mut self, kind: ExprKind, ty: Ty, span: Span) -> ExprId {
        let id = self.exprs.len() as ExprId;
        self.exprs.push(Expr { kind, ty, span });
        id
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

impl ThirCrate {
    /// 사람이 읽는 THIR 덤프 — `rustc -Zunpretty=thir-tree` 흉내 (arena + 확정 타입).
    pub fn dump(&self) -> String {
        let mut out = String::from("// thir — arena 평탄화 · 타입확정 (`_` = Infer) · `#n`=slot\n");
        for f in &self.fns {
            out.push_str(&self.fn_str(f));
        }
        out
    }

    fn fn_str(&self, f: &Thir) -> String {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| {
                let default = p.default.map(|e| format!(" = e{e}")).unwrap_or_default();
                format!("#{} {}: {}{default}", p.local, p.name, self.ty_str(&p.ty))
            })
            .collect();
        let mut out = format!("fn {} -> {} {{ // fn_id={}\n", f.name, self.ty_str(&f.body.ty), f.fn_id);
        out.push_str(&format!("    params: {}\n", if params.is_empty() { "(없음)".to_string() } else { params.join(", ") }));
        out.push_str(&format!("    body: value=e{} ty={}\n", f.body.value, self.ty_str(&f.body.ty)));
        for (i, b) in f.blocks.iter().enumerate() {
            let stmts: Vec<String> = b.stmts.iter().map(|s| format!("s{s}")).collect();
            let expr = b.expr.map(|e| format!("e{e}")).unwrap_or_else(|| "none".to_string());
            out.push_str(&format!("    b{i}: stmts=[{}] expr={expr}\n", stmts.join(", ")));
        }
        for (i, s) in f.stmts.iter().enumerate() {
            out.push_str(&format!("    s{i}: {}\n", self.stmt_str(s)));
        }
        for (i, e) in f.exprs.iter().enumerate() {
            out.push_str(&format!("    e{i}: {} = {}\n", self.ty_str(&e.ty), self.expr_str(e)));
        }
        out.push_str("}\n");
        out
    }

    fn stmt_str(&self, s: &Stmt) -> String {
        match &s.kind {
            StmtKind::Expr { expr } => format!("Expr {{ expr: e{expr} }}"),
            StmtKind::Let { local, name, ty, initializer } => {
                let init = initializer.map(|e| format!("e{e}")).unwrap_or_else(|| "none".to_string());
                format!("Let {{ local: #{local} {name}: {}, initializer: {init} }}", self.ty_str(ty))
            }
        }
    }

    fn expr_str(&self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Literal(n) => format!("Literal({n})"),
            ExprKind::VarRef { local, name } => format!("VarRef(#{local} {name})"),
            ExprKind::Binary { op, lhs, rhs } => format!("Binary({} e{lhs} e{rhs})", binop_str(*op)),
            ExprKind::Call { name, args, .. } => {
                let a: Vec<String> = args.iter().map(|x| format!("e{x}")).collect();
                format!("Call({name}({}))", a.join(", "))
            }
            ExprKind::If { cond, then_block, else_opt } => match else_opt {
                Some(else_id) => format!("If(cond=e{cond} then=e{then_block} else=e{else_id})"),
                None => format!("If(cond=e{cond} then=e{then_block})"),
            },
            ExprKind::Block { block } => format!("Block(b{block})"),
            ExprKind::Field { lhs, name, struct_id, index } => match (struct_id, index) {
                (Some(id), Some(i)) => format!("Field(e{lhs} .{name} 순번={i} struct={})", self.struct_name(*id)),
                _ => format!("Field(e{lhs} .{name})"),
            },
            ExprKind::Adt { struct_id, fields } => {
                let parts: Vec<String> = fields.iter().map(|(i, e)| format!("f{i}:e{e}")).collect();
                format!("Adt({} {{ {} }})", self.struct_name(*struct_id), parts.join(", "))
            }
        }
    }

    fn struct_name(&self, id: StructId) -> String {
        self.structs.get(id as usize).map(|s| s.name.clone()).unwrap_or_else(|| format!("<struct {id}>"))
    }

    fn ty_str(&self, ty: &Ty) -> String {
        match ty {
            Ty::I64 => "i64".to_string(),
            Ty::Unit => "()".to_string(),
            Ty::Struct(id) => self.struct_name(*id),
            Ty::Infer => "_".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 소스 → (lex → lowering → HIR). resolve는 거치지 않는다 —
    /// HIR lowering이 slot을 이름으로 fallback 해석한다.
    fn hir_of(src: &str) -> HirCrate {
        let tokens = rtoy_lexer::tokenize(src);
        let ast = rtoy_tokenstream_lowering::try_lower(&tokens, src).expect("ast");
        rtoy_hir::lower(&ast).expect("hir")
    }

    fn thir_of(src: &str) -> Result<ThirCrate, Vec<ThirLowerError>> {
        lower_crate(&hir_of(src))
    }

    fn fn0(thir: &ThirCrate) -> &Thir {
        &thir.fns[0]
    }

    #[test]
    fn infers_literal_and_binary() {
        let thir = thir_of("fn main() { let a = 1; a + 2 }").expect("thir");
        let f = fn0(&thir);
        assert_eq!(f.body.ty, Ty::I64);
        let root = &f.exprs[f.body.value as usize];
        assert_eq!(root.ty, Ty::I64);
        let ExprKind::Block { block } = &root.kind else { panic!("블록") };
        let tail = f.blocks[*block as usize].expr.expect("tail");
        assert!(matches!(f.exprs[tail as usize].kind, ExprKind::Binary { .. }));
    }

    #[test]
    fn top_level_let_becomes_param_with_default() {
        let thir = thir_of("fn helper() { 1 } fn main() { let n = helper(); n }").expect("thir");
        let f = &thir.fns[thir.fn_by_name["main"] as usize];
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].name, "n");
        assert_eq!(f.params[0].ty, Ty::I64, "기본값(호출)에서 파라미터 타입을 유추한다");
        assert!(f.params[0].default.is_some());
    }

    #[test]
    fn recursive_call_gets_return_ty_by_fixpoint() {
        // fib: 자기 자신을 부르므로 반환 타입은 fixpoint로만 나온다.
        let src = "fn fib() { let n = 0; if n < 2 { n } else { fib(n) + fib(n) } } fn main() { fib(10) }";
        let thir = thir_of(src).expect("thir");
        assert_eq!(thir.fns[thir.fn_by_name["fib"] as usize].body.ty, Ty::I64);
        assert_eq!(thir.fns[thir.fn_by_name["main"] as usize].body.ty, Ty::I64);
    }

    #[test]
    fn struct_field_and_literal_get_struct_ty() {
        let src = "struct Point { x: i64, y: i64 } fn main() { let p = Point { x: 1, y: 2 }; p.x + p.y }";
        let thir = thir_of(src).expect("thir");
        let f = &thir.fns[thir.fn_by_name["main"] as usize];
        assert_eq!(f.params[0].ty, Ty::Struct(0));
        assert!(f.exprs.iter().any(|e| matches!(&e.kind, ExprKind::Adt { struct_id: 0, .. })));
        let field = f
            .exprs
            .iter()
            .find(|e| matches!(e.kind, ExprKind::Field { index: Some(0), .. }))
            .expect("필드 접근에 순번이 붙는다");
        assert_eq!(field.ty, Ty::I64);
    }

    #[test]
    fn mismatched_types_are_reported() {
        let src = "struct Point { x: i64 } fn main() { let p = Point { x: 1 }; 1 + p }";
        let errs = thir_of(src).expect_err("타입 불일치");
        assert!(
            matches!(&errs[0], ThirLowerError::MismatchedTypes { context, expected, found, .. }
                if *context == "binary op rhs" && expected.as_str() == "i64" && found.as_str() == "Point"),
            "기대/발견 타입이 그대로 보여야 한다: {errs:?}"
        );
    }

    #[test]
    fn if_without_else_must_be_unit() {
        let errs = thir_of("fn main() { if 1 { 2 } }").expect_err("()가 아님");
        assert!(matches!(&errs[0], ThirLowerError::MismatchedTypes { context, expected, .. }
            if *context == "if without else" && expected.as_str() == "()"));
    }

    #[test]
    fn field_on_non_struct_is_reported() {
        let errs = thir_of("fn main() { let x = 1; x.y }").expect_err("구조체가 아님");
        assert!(matches!(&errs[0], ThirLowerError::NotAStruct { context, found, .. }
            if *context == "field access" && found.as_str() == "i64"));
    }

    #[test]
    fn wrong_arg_count_is_reported() {
        // helper는 파라미터 1개(최상위 let)를 갖지만 0개로 호출한다.
        let errs = thir_of("fn helper() { let n = 1; n } fn main() { helper() }").expect_err("인자 개수");
        assert!(matches!(&errs[0], ThirLowerError::WrongArgCount { expected: 1, found: 0, .. }), "{errs:?}");
    }

    #[test]
    fn dump_shows_types_and_arena() {
        let thir = thir_of("fn main() { 1 + 2 }").expect("thir");
        let dump = thir.dump();
        assert!(dump.contains("fn main -> i64"), "{dump}");
        assert!(dump.contains("Binary(+"), "{dump}");
        assert!(dump.contains("b0: stmts=[]"), "{dump}");
        // 정수 리터럴 span은 파서가 채운다 (Span 배선 확인).
        let f = fn0(&thir);
        assert_ne!(f.exprs[0].span, Span::root(0, 0));    }
}
