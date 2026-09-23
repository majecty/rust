//! rtoy mir — AST(resolve 후)를 basic block CFG 기반 MIR로 내린다.
//! Original: compiler/rustc_middle/src/mir (body.rs, visit.rs) — 부분집합.
//!
//! rustc와의 대응:
//! - `Body` = 함수 하나. `_0` = 반환 place, `_1..` = 지역/인자 (rustc 관례).
//! - `BasicBlock { stmts, terminator }`, `Place { local, proj }`, `Rvalue`, `Operand`.
//! - `TerminatorKind::{Goto, SwitchInt, Call, Return}` = rustc TerminatorKind의 부분집합.
//!
//! 부분집합이라 다른 점:
//! - 타입 추론이 없다 — 지역 타입은 리터럴 초기화에서만 `Struct`로 알아낸다.
//! - unwind/StorageLive/StorageDead 없음.
//! - `fn` 파라미터 AST가 없어 최상위 `let`을 "인자 자리"로 쓴다 (eval `bind_args` 관례).
//!   인자 n개로 호출되면 `bb n`부터 실행하면 되도록 prologue 블록을 앞에 깔아 둔다.

use rtoy_ast::{BinOp, Block, Crate, Expr, ExprKind, FnItem, Ident, ItemKind, StmtKind as AstStmtKind};
use rtoy_span::Span;
use std::collections::HashMap;

pub type LocalId = u32;
pub type BlockId = u32;
pub type FnId = u32;
pub type StructId = u32;

/// MIR 타입 — 타입 추론이 없어 리터럴 초기화에서만 `Struct`를 알아낸다.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    I64,
    Unit,
    Struct(StructId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalDecl {
    pub ty: Ty,
    pub name: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectionElem {
    /// `base.field` — 필드 순번 (byte offset은 평가 시 struct 표에서 구한다).
    Field(StructId, u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub local: LocalId,
    pub proj: Vec<ProjectionElem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Int(i64),
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Copy(Place),
    Const(ConstValue),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Operand, Operand),
    /// 구조체 리터럴. `(필드 순번, 값)` 쌍을 **원문 순서**로 유지한다 (평가 순서 보존).
    Aggregate { struct_id: StructId, fields: Vec<(u32, Operand)> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Assign(Place, Rvalue),
}

/// 호출 대상 — 함수 포인터가 없어 사용자 함수 id 아니면 `print` 내장이다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FnRef {
    User(FnId),
    Print,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminatorKind {
    Goto(BlockId),
    /// `switchInt(discr) -> [0: .., otherwise: ..]`
    SwitchInt { discr: Operand, targets: Vec<(i64, BlockId)>, otherwise: BlockId },
    Call { func: FnRef, args: Vec<Operand>, dest: Place, target: BlockId },
    Return,
    /// lowering이 채우지 못한 자리 (남아 있으면 버그).
    Unreachable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub stmts: Vec<Stmt>,
    pub terminator: Terminator,
}

/// 함수 하나의 MIR 본문.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub name: String,
    /// `_0` = 반환, `_1..` = 지역/인자.
    pub locals: Vec<LocalDecl>,
    /// 최상위 `let` 순서의 local (인자 자리). `_1..` 이지만 shadowing이면 같은 id가 반복된다.
    pub arg_locals: Vec<LocalId>,
    /// 본문 첫 블록. prologue 블록이 있으면 그 뒤 번호다.
    pub entry: BlockId,
    pub blocks: Vec<BasicBlock>,
    pub span: Span,
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
pub struct MirCrate {
    pub fns: Vec<Body>,
    pub by_name: HashMap<String, FnId>,
    pub structs: Vec<StructDef>,
    pub struct_by_name: HashMap<String, StructId>,
    pub main: Option<FnId>,
}

/// MIR lowering 실패. 어디서 무엇이 문제인지 span과 함께 노출한다.
#[derive(Debug, Clone, PartialEq)]
pub enum MirLowerError {
    UnknownFn { name: String, span: Span },
    UnknownStruct { name: String, span: Span },
    UnknownField { name: String, span: Span },
    UnknownLocal { name: String, span: Span },
    /// 아직 MIR로 내릴 수 없는 노드 (예: 전개되지 않은 매크로, place가 아닌 곳의 필드 접근).
    Unsupported { what: String, span: Span },
}

impl std::fmt::Display for MirLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (what, name, span) = match self {
            MirLowerError::UnknownFn { name, span } => ("unknown function", name.as_str(), span),
            MirLowerError::UnknownStruct { name, span } => ("unknown struct", name.as_str(), span),
            MirLowerError::UnknownField { name, span } => ("unknown field", name.as_str(), span),
            MirLowerError::UnknownLocal { name, span } => ("unknown local", name.as_str(), span),
            MirLowerError::Unsupported { what, span } => (what.as_str(), "", span),
        };
        if name.is_empty() {
            write!(f, "mir lowering failed: {what} at [{}..{}]", span.lo, span.hi)
        } else {
            write!(f, "mir lowering failed: {what} `{name}` at [{}..{}]", span.lo, span.hi)
        }
    }
}

impl std::error::Error for MirLowerError {}

/// 기본 타입 이름 → MIR 타입. 모르는 이름은 `Unit` (타입 검사는 아직 없다).
fn ty_from_name(name: &str, structs: &HashMap<String, StructId>) -> Ty {
    match name {
        "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "bool" => Ty::I64,
        other => structs.get(other).map(|id| Ty::Struct(*id)).unwrap_or(Ty::Unit),
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

/// 크레이트 전체를 MIR로 내린다 (resolve 이후 AST를 전제).
pub fn lower_crate(krate: &Crate) -> Result<MirCrate, MirLowerError> {
    let (structs, struct_by_name) = collect_structs(krate);
    let (fns, by_name) = collect_fns(krate);
    let main = by_name.get("main").copied();
    let mut bodies = Vec::with_capacity(fns.len());
    for (name, item) in &fns {
        bodies.push(lower_fn(name, item, &struct_by_name, &structs, &by_name)?);
    }
    Ok(MirCrate { fns: bodies, by_name, structs, struct_by_name, main })
}

/// struct 표 (선언 순서 = StructId) — 필드 이름과 타입만 담는다.
fn collect_structs(krate: &Crate) -> (Vec<StructDef>, HashMap<String, StructId>) {
    let mut structs = Vec::new();
    let mut by_name = HashMap::new();
    for item in &krate.items {
        if let ItemKind::Struct(s) = &item.kind {
            by_name.insert(item.name.name.clone(), structs.len() as StructId);
            let fields = s
                .fields
                .iter()
                .map(|f| FieldDef { name: f.name.name.clone(), ty: Ty::Unit })
                .collect();
            structs.push(StructDef { name: item.name.name.clone(), fields });
        }
    }
    // 필드 타입은 struct 표가 다 모인 뒤에 채운다 (구조체끼리 참조 가능).
    for (id, item) in struct_items(krate).into_iter().enumerate() {
        if let ItemKind::Struct(s) = &item.kind {
            for (fid, f) in s.fields.iter().enumerate() {
                structs[id].fields[fid].ty = ty_from_name(&f.ty.name, &by_name);
            }
        }
    }
    (structs, by_name)
}

fn struct_items(krate: &Crate) -> Vec<&rtoy_ast::Item> {
    krate.items.iter().filter(|i| matches!(i.kind, ItemKind::Struct(_))).collect()
}

/// fn 표 (크레이트 순서 = FnId).
fn collect_fns(krate: &Crate) -> (Vec<(String, &FnItem)>, HashMap<String, FnId>) {
    let mut fns = Vec::new();
    let mut by_name = HashMap::new();
    for item in &krate.items {
        if let ItemKind::Fn(f) = &item.kind {
            by_name.insert(item.name.name.clone(), fns.len() as FnId);
            fns.push((item.name.name.clone(), f));
        }
    }
    (fns, by_name)
}

/// 함수 하나를 MIR `Body`로 내린다.
fn lower_fn(
    name: &str,
    f: &FnItem,
    struct_by_name: &HashMap<String, StructId>,
    structs: &[StructDef],
    fn_by_name: &HashMap<String, FnId>,
) -> Result<Body, MirLowerError> {
    let mut names = Vec::new();
    rtoy_ast::collect_local_names(&f.body, &mut names);
    let mut fl = FnLower::new(names, struct_by_name, structs, fn_by_name, f.span);

    // 최상위 let = 인자 자리. prologue 블록을 앞에 깔아, 인자 n개면 `bb n`부터 실행하면 되게 한다.
    let top_lets: Vec<&rtoy_ast::LetStmt> = f
        .body
        .stmts
        .iter()
        .filter_map(|s| match &s.kind {
            AstStmtKind::Let(l) => Some(l),
            AstStmtKind::Expr(_) => None,
        })
        .collect();
    let mut arg_locals = Vec::with_capacity(top_lets.len());
    for l in &top_lets {
        arg_locals.push(fl.local_of(&l.name.name, l.name.span)?);
    }
    for _ in 0..top_lets.len() {
        fl.new_block();
    }
    let entry = fl.new_block();
    for (i, l) in top_lets.iter().enumerate() {
        fl.current = i as BlockId;
        let dest = Place { local: arg_locals[i], proj: vec![] };
        match &l.init {
            Some(init) => {
                fl.lower_expr(init, &dest)?;
                fl.note_local_ty(arg_locals[i], init)?;
            }
            None => fl.push(StmtKind::Assign(dest, Rvalue::Use(Operand::Const(ConstValue::Unit))), l.span),
        }
        let target = if i + 1 < top_lets.len() { (i + 1) as BlockId } else { entry };
        fl.terminate(TerminatorKind::Goto(target), l.span);
    }

    // 본문 — 최상위 let은 prologue가 이미 처리했으므로 Expr stmt만 실행한다.
    fl.current = entry;
    for stmt in &f.body.stmts {
        if let AstStmtKind::Expr(e) = &stmt.kind {
            let ty = fl.expr_ty(e)?;
            let local = fl.temp(ty, e.span);
            fl.lower_expr(e, &Place { local, proj: vec![] })?;
        }
    }
    let ret = Place { local: 0, proj: vec![] };
    match &f.body.tail {
        Some(tail) => fl.lower_expr(tail, &ret)?,
        None => fl.push(StmtKind::Assign(ret, Rvalue::Use(Operand::Const(ConstValue::Unit))), f.span),
    }
    fl.terminate(TerminatorKind::Return, f.span);

    Ok(Body { name: name.to_string(), locals: fl.locals, arg_locals, entry, blocks: fl.blocks, span: f.span })
}

/// 함수 하나를 내리는 동안의 블록 빌더 (rustc의 BlockAndBuilder에 해당).
struct FnLower<'a> {
    locals: Vec<LocalDecl>,
    /// local id → 타입 (필드 projection에 필요).
    local_ty: Vec<Ty>,
    names: Vec<String>,
    blocks: Vec<BasicBlock>,
    current: BlockId,
    struct_by_name: &'a HashMap<String, StructId>,
    structs: &'a [StructDef],
    fn_by_name: &'a HashMap<String, FnId>,
}

impl<'a> FnLower<'a> {
    fn new(
        names: Vec<String>,
        struct_by_name: &'a HashMap<String, StructId>,
        structs: &'a [StructDef],
        fn_by_name: &'a HashMap<String, FnId>,
        span: Span,
    ) -> Self {
        let mut locals = vec![LocalDecl { ty: Ty::Unit, name: None, span }];
        let mut local_ty = vec![Ty::Unit];
        for n in &names {
            locals.push(LocalDecl { ty: Ty::Unit, name: Some(n.clone()), span });
            local_ty.push(Ty::Unit);
        }
        FnLower { locals, local_ty, names, blocks: Vec::new(), current: 0, struct_by_name, structs, fn_by_name }
    }

    /// 지역변수 이름 → local id (`_1..` = names 순서).
    fn local_of(&self, name: &str, span: Span) -> Result<LocalId, MirLowerError> {
        self.names
            .iter()
            .position(|n| n == name)
            .map(|i| i as LocalId + 1)
            .ok_or_else(|| MirLowerError::UnknownLocal { name: name.to_string(), span })
    }

    fn field_index(&self, struct_id: StructId, field: &str, span: Span) -> Result<u32, MirLowerError> {
        self.structs
            .get(struct_id as usize)
            .and_then(|s| s.fields.iter().position(|f| f.name == field))
            .map(|i| i as u32)
            .ok_or_else(|| MirLowerError::UnknownField { name: field.to_string(), span })
    }

    fn new_block(&mut self) -> BlockId {
        let id = self.blocks.len() as BlockId;
        self.blocks.push(BasicBlock {
            stmts: Vec::new(),
            terminator: Terminator { kind: TerminatorKind::Unreachable, span: Span::root(0, 0) },
        });
        id
    }

    fn temp(&mut self, ty: Ty, span: Span) -> LocalId {
        let id = self.locals.len() as LocalId;
        self.locals.push(LocalDecl { ty: ty.clone(), name: None, span });
        self.local_ty.push(ty);
        id
    }

    fn push(&mut self, kind: StmtKind, span: Span) {
        self.blocks[self.current as usize].stmts.push(Stmt { kind, span });
    }

    fn terminate(&mut self, kind: TerminatorKind, span: Span) {
        self.blocks[self.current as usize].terminator = Terminator { kind, span };
    }

    /// `let x = init` 뒤에 x의 타입을 기록한다 (타입 추론이 없어 필드 projection에 필요).
    /// 추론이 없으므로 `expr_ty`의 추정치를 그대로 쓴다.
    fn note_local_ty(&mut self, local: LocalId, init: &Expr) -> Result<(), MirLowerError> {
        let ty = self.expr_ty(init)?;
        self.local_ty[local as usize] = ty.clone();
        self.locals[local as usize].ty = ty;
        Ok(())
    }

    /// 식의 MIR 타입 — 추론이 없어 Var의 local_ty와 리터럴/필드만 본다.
    fn expr_ty(&self, expr: &Expr) -> Result<Ty, MirLowerError> {
        match &expr.kind {
            ExprKind::Var { name, .. } => {
                let local = self.local_of(&name.name, name.span)?;
                Ok(self.local_ty[local as usize].clone())
            }
            ExprKind::FieldAccess { base, field, .. } => {
                let Ty::Struct(id) = self.expr_ty(base)? else { return Ok(Ty::Unit) };
                let idx = self.field_index(id, &field.name, field.span)?;
                Ok(self
                    .structs
                    .get(id as usize)
                    .and_then(|s| s.fields.get(idx as usize))
                    .map(|f| f.ty.clone())
                    .unwrap_or(Ty::Unit))
            }
            ExprKind::StructLiteral { name, .. } => {
                Ok(self.struct_by_name.get(&name.name).map(|id| Ty::Struct(*id)).unwrap_or(Ty::Unit))
            }
            _ => Ok(Ty::I64),
        }
    }

    /// place 식(Var/FieldAccess)을 `Place`로 만든다.
    fn place_of(&mut self, expr: &Expr) -> Result<Place, MirLowerError> {
        match &expr.kind {
            ExprKind::Var { name, .. } => Ok(Place { local: self.local_of(&name.name, name.span)?, proj: vec![] }),
            ExprKind::FieldAccess { base, field, .. } => {
                let Ty::Struct(id) = self.expr_ty(base)? else {
                    return Err(MirLowerError::Unsupported {
                        what: format!("field `{}` on non-struct", field.name),
                        span: expr.span,
                    });
                };
                let index = self.field_index(id, &field.name, field.span)?;
                let mut place = self.place_of(base)?;
                place.proj.push(ProjectionElem::Field(id, index));
                Ok(place)
            }
            _ => Err(MirLowerError::Unsupported { what: "not a place expression".into(), span: expr.span }),
        }
    }

    /// 식을 temp에 담아 `Operand::Copy`로 돌려준다.
    fn lower_operand(&mut self, expr: &Expr) -> Result<Operand, MirLowerError> {
        let ty = self.expr_ty(expr)?;
        let local = self.temp(ty, expr.span);
        let place = Place { local, proj: vec![] };
        self.lower_expr(expr, &place)?;
        Ok(Operand::Copy(place))
    }

    /// 식을 `dest`에 담는다. 돌아올 때 `self.current`는 dest가 채워진 뒤 실행할 블록이다.
    fn lower_expr(&mut self, expr: &Expr, dest: &Place) -> Result<(), MirLowerError> {
        match &expr.kind {
            ExprKind::Int(n) => {
                self.push(StmtKind::Assign(dest.clone(), Rvalue::Use(Operand::Const(ConstValue::Int(*n)))), expr.span);
                Ok(())
            }
            ExprKind::Var { .. } | ExprKind::FieldAccess { .. } => {
                let place = self.place_of(expr)?;
                self.push(StmtKind::Assign(dest.clone(), Rvalue::Use(Operand::Copy(place))), expr.span);
                Ok(())
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.lower_operand(lhs)?;
                let r = self.lower_operand(rhs)?;
                self.push(StmtKind::Assign(dest.clone(), Rvalue::BinaryOp(*op, l, r)), expr.span);
                Ok(())
            }
            ExprKind::Call { callee, args, .. } => self.lower_call(callee, args, dest, expr.span),
            ExprKind::StructLiteral { name, fields } => self.lower_struct_lit(name, fields, dest, expr.span),
            ExprKind::If { cond, then_block, else_block } => {
                self.lower_if(cond, then_block, else_block.as_deref(), dest, expr.span)
            }
            ExprKind::Macro { name, .. } => Err(MirLowerError::Unsupported {
                what: format!("unexpanded macro `{}!`", name.name),
                span: expr.span,
            }),
        }
    }

    fn lower_call(&mut self, callee: &Ident, args: &[Expr], dest: &Place, span: Span) -> Result<(), MirLowerError> {
        let mut operands = Vec::with_capacity(args.len());
        for a in args {
            operands.push(self.lower_operand(a)?);
        }
        let func = if callee.name == "print" {
            FnRef::Print
        } else {
            match self.fn_by_name.get(&callee.name) {
                Some(id) => FnRef::User(*id),
                None => return Err(MirLowerError::UnknownFn { name: callee.name.clone(), span: callee.span }),
            }
        };
        let target = self.new_block();
        self.terminate(TerminatorKind::Call { func, args: operands, dest: dest.clone(), target }, span);
        self.current = target;
        Ok(())
    }

    fn lower_struct_lit(
        &mut self,
        name: &Ident,
        fields: &[rtoy_ast::FieldInit],
        dest: &Place,
        span: Span,
    ) -> Result<(), MirLowerError> {
        let struct_id = match self.struct_by_name.get(&name.name) {
            Some(id) => *id,
            None => return Err(MirLowerError::UnknownStruct { name: name.name.clone(), span: name.span }),
        };
        let mut ops = Vec::with_capacity(fields.len());
        for f in fields {
            let index = self.field_index(struct_id, &f.name.name, f.name.span)?;
            let op = self.lower_operand(&f.value)?;
            ops.push((index, op));
        }
        self.push(StmtKind::Assign(dest.clone(), Rvalue::Aggregate { struct_id, fields: ops }), span);
        Ok(())
    }

    /// `if` → `SwitchInt` + 두 분기 + join. 두 분기 모두 같은 dest에 쓴다 (rustc와 같음).
    fn lower_if(
        &mut self,
        cond: &Expr,
        then_block: &Block,
        else_block: Option<&Block>,
        dest: &Place,
        span: Span,
    ) -> Result<(), MirLowerError> {
        let discr = self.lower_operand(cond)?;
        let then_bb = self.new_block();
        let else_bb = self.new_block();
        let join_bb = self.new_block();
        self.terminate(
            TerminatorKind::SwitchInt { discr, targets: vec![(0, else_bb)], otherwise: then_bb },
            span,
        );

        self.current = then_bb;
        self.lower_block(then_block, dest)?;
        self.terminate(TerminatorKind::Goto(join_bb), then_block.span);

        self.current = else_bb;
        match else_block {
            Some(b) => self.lower_block(b, dest)?,
            None => self.push(StmtKind::Assign(dest.clone(), Rvalue::Use(Operand::Const(ConstValue::Unit))), span),
        }
        self.terminate(TerminatorKind::Goto(join_bb), span);

        self.current = join_bb;
        Ok(())
    }

    /// 블록을 내린다 (분기 안 `let`도 여기서 처리 — AST에 블록 스코프가 없어 같은 프레임을 쓴다).
    fn lower_block(&mut self, block: &Block, dest: &Place) -> Result<(), MirLowerError> {
        for stmt in &block.stmts {
            match &stmt.kind {
                AstStmtKind::Let(l) => {
                    let local = self.local_of(&l.name.name, l.name.span)?;
                    let place = Place { local, proj: vec![] };
                    match &l.init {
                        Some(init) => {
                            self.lower_expr(init, &place)?;
                            self.note_local_ty(local, init)?;
                        }
                        None => self.push(
                            StmtKind::Assign(place, Rvalue::Use(Operand::Const(ConstValue::Unit))),
                            stmt.span,
                        ),
                    }
                }
                AstStmtKind::Expr(e) => {
                    let ty = self.expr_ty(e)?;
                    let local = self.temp(ty, e.span);
                    self.lower_expr(e, &Place { local, proj: vec![] })?;
                }
            }
        }
        match &block.tail {
            Some(tail) => self.lower_expr(tail, dest)?,
            None => self.push(
                StmtKind::Assign(dest.clone(), Rvalue::Use(Operand::Const(ConstValue::Unit))),
                block.span,
            ),
        }
        Ok(())
    }
}

impl MirCrate {
    /// rustc `-Zunpretty=mir` 흉내 — 사람이 읽는 덤프.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        for (id, body) in self.fns.iter().enumerate() {
            out.push_str(&format!("fn {}() -> () {{\n", body.name));
            for (i, l) in body.locals.iter().enumerate() {
                let comment = l.name.as_ref().map(|n| format!(" // {n}")).unwrap_or_default();
                out.push_str(&format!("    let mut _{i}: {};{comment}\n", self.ty_str(&l.ty)));
            }
            out.push('\n');
            for (b, bb) in body.blocks.iter().enumerate() {
                out.push_str(&format!("    bb{b}: {{\n"));
                for s in &bb.stmts {
                    out.push_str(&format!("        {};\n", self.stmt_str(s)));
                }
                out.push_str(&format!("        {}\n", self.term_str(&bb.terminator)));
                out.push_str("    }\n");
            }
            out.push_str("}\n");
            if id + 1 < self.fns.len() {
                out.push('\n');
            }
        }
        out
    }

    fn ty_str(&self, ty: &Ty) -> String {
        match ty {
            Ty::I64 => "i64".into(),
            Ty::Unit => "()".into(),
            Ty::Struct(id) => self.structs.get(*id as usize).map(|s| s.name.clone()).unwrap_or_else(|| format!("<struct {id}>")),
        }
    }

    fn place_str(&self, p: &Place) -> String {
        let mut s = format!("_{}", p.local);
        for e in &p.proj {
            match e {
                ProjectionElem::Field(_, i) => s.push_str(&format!(".f{i}")),
            }
        }
        s
    }

    fn operand_str(&self, o: &Operand) -> String {
        match o {
            Operand::Copy(p) => format!("copy {}", self.place_str(p)),
            Operand::Const(ConstValue::Int(n)) => format!("const {n}_i64"),
            Operand::Const(ConstValue::Unit) => "const ()".into(),
        }
    }

    fn rvalue_str(&self, rv: &Rvalue) -> String {
        match rv {
            Rvalue::Use(o) => self.operand_str(o),
            Rvalue::BinaryOp(op, l, r) => {
                format!("{} {} {}", self.operand_str(l), binop_str(*op), self.operand_str(r))
            }
            Rvalue::Aggregate { struct_id, fields } => {
                let def = self.structs.get(*struct_id as usize);
                let name = def.map(|s| s.name.clone()).unwrap_or_else(|| format!("<struct {struct_id}>"));
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(i, o)| {
                        let fname = def
                            .and_then(|s| s.fields.get(*i as usize))
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| format!("f{i}"));
                        format!("{fname}: {}", self.operand_str(o))
                    })
                    .collect();
                format!("{name} {{ {} }}", parts.join(", "))
            }
        }
    }

    fn stmt_str(&self, s: &Stmt) -> String {
        match &s.kind {
            StmtKind::Assign(place, rv) => format!("{} = {}", self.place_str(place), self.rvalue_str(rv)),
        }
    }

    fn term_str(&self, t: &Terminator) -> String {
        match &t.kind {
            TerminatorKind::Goto(b) => format!("goto -> bb{b};"),
            TerminatorKind::SwitchInt { discr, targets, otherwise } => {
                let mut parts: Vec<String> = targets.iter().map(|(v, b)| format!("{v}: bb{b}")).collect();
                parts.push(format!("otherwise: bb{otherwise}"));
                format!("switchInt({}) -> [{}];", self.operand_str(discr), parts.join(", "))
            }
            TerminatorKind::Call { func, args, dest, target } => {
                let f = match func {
                    FnRef::User(id) => self
                        .fns
                        .get(*id as usize)
                        .map(|b| b.name.clone())
                        .unwrap_or_else(|| format!("<fn {id}>")),
                    FnRef::Print => "print".to_string(),
                };
                let a: Vec<String> = args.iter().map(|o| self.operand_str(o)).collect();
                format!("{} = {}({}) -> bb{};", self.place_str(dest), f, a.join(", "), target)
            }
            TerminatorKind::Return => "return;".into(),
            TerminatorKind::Unreachable => "unreachable;".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtoy_lexer::tokenize;
    use rtoy_tokenstream_lowering::try_lower;

    /// lowering → (선택) expand → resolve를 거쳐 MIR을 만든다.
    fn mir_of(src: &str) -> MirCrate {
        let krate = try_lower(&tokenize(src), src).expect("lowering");
        let krate = rtoy_expand::expand_crate(krate).expect("expand");
        let mut krate = krate;
        rtoy_resolve::resolve(&mut krate, src).expect("resolve");
        lower_crate(&krate).expect("mir lowering")
    }

    #[test]
    fn lowers_int_tail() {
        let m = mir_of("fn main() { 42 }");
        assert_eq!(m.fns.len(), 1);
        let body = &m.fns[0];
        assert_eq!(body.name, "main");
        assert_eq!(body.arg_locals.len(), 0);
        assert_eq!(body.entry, 0);
        assert_eq!(body.blocks.len(), 1);
        match &body.blocks[0].stmts[0].kind {
            StmtKind::Assign(place, Rvalue::Use(Operand::Const(ConstValue::Int(42)))) => {
                assert_eq!(place.local, 0, "tail은 반환 place `_0`에 쓴다");
            }
            other => panic!("expected `_0 = const 42`, got {other:?}"),
        }
        assert_eq!(body.blocks[0].terminator.kind, TerminatorKind::Return);
    }

    #[test]
    fn top_level_lets_become_prologue_blocks() {
        // 최상위 let 2개 → prologue bb0/bb1, 본문 entry는 bb2.
        let m = mir_of("fn main() { let a = 1; let b = 2; a + b }");
        let body = &m.fns[0];
        assert_eq!(body.arg_locals, vec![1, 2]);
        assert_eq!(body.entry, 2);
        assert_eq!(body.blocks.len(), 3);
        assert_eq!(body.blocks[0].terminator.kind, TerminatorKind::Goto(1));
        assert_eq!(body.blocks[1].terminator.kind, TerminatorKind::Goto(2));
    }

    #[test]
    fn if_becomes_switch_int_with_join() {
        let m = mir_of("fn main() { if 1 < 2 { 10 } else { 20 } }");
        let body = &m.fns[0];
        let switch = body
            .blocks
            .iter()
            .find_map(|b| match &b.terminator.kind {
                TerminatorKind::SwitchInt { targets, otherwise, .. } => Some((targets.clone(), *otherwise)),
                _ => None,
            })
            .expect("SwitchInt 없음");
        assert_eq!(switch.0, vec![(0, 2)], "0이면 else 분기");
        assert_eq!(switch.1, 1, "otherwise는 then 분기");
        // 두 분기 모두 같은 join 블록으로 모인다.
        let gotos: Vec<BlockId> = body
            .blocks
            .iter()
            .filter_map(|b| match b.terminator.kind {
                TerminatorKind::Goto(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(gotos, vec![3, 3]);
    }

    #[test]
    fn call_carries_fn_id() {
        let m = mir_of("fn main() { foo(1) } fn foo() { 2 }");
        assert_eq!(m.by_name["foo"], 1);
        assert_eq!(m.main, Some(0));
        let body = &m.fns[0];
        let (func, target) = body
            .blocks
            .iter()
            .find_map(|b| match &b.terminator.kind {
                TerminatorKind::Call { func, target, .. } => Some((*func, *target)),
                _ => None,
            })
            .expect("Call 없음");
        assert_eq!(func, FnRef::User(1));
        assert_eq!(target, body.entry + 1, "Call 뒤에 이어지는 블록");
    }

    #[test]
    fn struct_literal_and_field_projection() {
        let m = mir_of("struct P { x: i64, y: i64 }\nfn main() { let p = P { x: 1, y: 2 }; p.y }");
        let body = &m.fns[0];
        let agg = body
            .blocks
            .iter()
            .flat_map(|b| &b.stmts)
            .find_map(|s| match &s.kind {
                StmtKind::Assign(_, Rvalue::Aggregate { struct_id, fields }) => Some((*struct_id, fields.clone())),
                _ => None,
            })
            .expect("Aggregate 없음");
        assert_eq!(agg.0, 0, "P는 첫 struct");
        assert_eq!(agg.1.len(), 2);
        assert_eq!(agg.1[0].0, 0);
        assert_eq!(agg.1[1].0, 1, "원문 순서 유지");
        let proj = body
            .blocks
            .iter()
            .flat_map(|b| &b.stmts)
            .find_map(|s| match &s.kind {
                StmtKind::Assign(_, Rvalue::Use(Operand::Copy(p))) if !p.proj.is_empty() => Some(p.proj.clone()),
                _ => None,
            })
            .expect("field projection 없음");
        assert_eq!(proj, vec![ProjectionElem::Field(0, 1)], "p.y = 필드 1");
    }

    #[test]
    fn unknown_fn_is_an_error() {
        let src = "fn main() { nope(1) }";
        let mut krate = try_lower(&tokenize(src), src).expect("lowering");
        rtoy_resolve::resolve(&mut krate, src).expect("resolve");
        let err = lower_crate(&krate).expect_err("에러여야 함");
        assert!(matches!(err, MirLowerError::UnknownFn { ref name, .. } if name == "nope"), "{err}");
    }

    #[test]
    fn dump_mentions_blocks_and_switch() {
        let dump = mir_of("fn main() { if 1 < 2 { 10 } else { 20 } }").dump();
        assert!(dump.contains("bb0: {"), "{dump}");
        assert!(dump.contains("switchInt("), "{dump}");
        assert!(dump.contains("return;"), "{dump}");
    }
}
