//! rtoy mir — basic block CFG 기반 MIR 데이터 (rustc_middle/mir 부분집합).
//! Original: compiler/rustc_middle/src/mir (body.rs, visit.rs).
//!
//! rustc와의 대응:
//! - `Body` = 함수 하나. `_0` = 반환 place, `_1..` = 지역/인자 (rustc 관례).
//! - `BasicBlock { stmts, terminator }`, `Place { local, proj }`, `Rvalue`, `Operand`.
//! - `TerminatorKind::{Goto, SwitchInt, Call, Return}` = rustc TerminatorKind의 부분집합.
//!
//! 부분집합이라 다른 점:
//! - 타입 추론이 없다 — 지역 타입은 리터럴 초기화에서만 `Struct`로 알아낸다.
//! - unwind/StorageLive/StorageDead 없음.
//!
//! AST(resolve 후) → MIR lowering은 `rtoy-ast-lowering` 크레이트에 있다(`lower_crate`).
//! `BinOp`만 `rtoy_ast` 것을 그대로 쓴다 (IR에 이미 박힌 이름).

use rtoy_ast::BinOp;
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

    /// MIR 문장 1개를 덤프와 같은 문자열로 만든다 (eval 트레이스 공용).
    pub fn stmt_str(&self, s: &Stmt) -> String {
        match &s.kind {
            StmtKind::Assign(place, rv) => format!("{} = {}", self.place_str(place), self.rvalue_str(rv)),
        }
    }

    /// terminator 1개를 덤프와 같은 문자열로 만든다 (eval 트레이스 공용).
    pub fn term_str(&self, t: &Terminator) -> String {
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

    /// 손으로 만든 최소 Body로 dump 형식을 고정한다 (lowering은 `rtoy-ast-lowering`에서 검증).
    fn tiny_crate() -> MirCrate {
        let span = Span::root(0, 0);
        let body = Body {
            name: "main".into(),
            locals: vec![LocalDecl { ty: Ty::Unit, name: None, span }],
            arg_locals: vec![],
            entry: 0,
            blocks: vec![BasicBlock {
                stmts: vec![Stmt {
                    kind: StmtKind::Assign(
                        Place { local: 0, proj: vec![] },
                        Rvalue::Use(Operand::Const(ConstValue::Int(42))),
                    ),
                    span,
                }],
                terminator: Terminator { kind: TerminatorKind::Return, span },
            }],
            span,
        };
        MirCrate {
            fns: vec![body],
            by_name: HashMap::new(),
            structs: vec![],
            struct_by_name: HashMap::new(),
            main: Some(0),
        }
    }

    #[test]
    fn dump_mentions_blocks_and_return() {
        let dump = tiny_crate().dump();
        assert!(dump.contains("bb0: {"), "{dump}");
        assert!(dump.contains("const 42_i64"), "{dump}");
        assert!(dump.contains("return;"), "{dump}");
    }
}
