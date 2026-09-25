//! MIR 인터프리터 — `rtoy-mir`의 basic block CFG를 실행한다.
//! AST eval과 같은 arena/`Frame`/`Value`/`Memory`를 재사용한다 (호출당 할당 없음).
//! 차이: AST를 재귀 순회하지 않고 블록을 선형 스캔한다 (Place는 이미 index).
//! `eval_mir_traced`는 스텝 예산을 받아 실행 경로와 현재 위치를 남긴다 (웹 한 줄 실행용).

use crate::{
    apply_binop, format_value, push_frame, EvalError, Frame, Memory, Runtime, StructLayout, Value,
};
use rtoy_mir::{
    BlockId, Body, ConstValue, FnId, FnRef, MirCrate, Operand, Place, ProjectionElem, Rvalue, Stmt,
    StmtKind, StructDef, TerminatorKind, Ty,
};
use std::collections::HashMap;

/// 스텝 1개의 종류 — MIR 문장 또는 블록 끝 terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Stmt,
    Term,
}

impl StepKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StepKind::Stmt => "stmt",
            StepKind::Term => "term",
        }
    }
}

/// 실행한 스텝 1개 — 위치(fn/bb/문장 번호)와 그 줄 텍스트.
#[derive(Debug, Clone, PartialEq)]
pub struct StepEvent {
    pub index: u64,
    pub fn_id: FnId,
    pub fn_name: String,
    pub bb: BlockId,
    /// `StepKind::Stmt`면 블록 안 문장 번호, `Term`이면 `None`.
    pub stmt: Option<usize>,
    pub kind: StepKind,
    /// 호출 깊이 (main = 1).
    pub depth: usize,
    pub text: String,
}

/// 마지막으로 실행한 위치와 그 프레임의 지역변수 (실행 후 값).
#[derive(Debug, Clone, PartialEq)]
pub struct Cursor {
    pub fn_id: FnId,
    pub fn_name: String,
    pub bb: BlockId,
    pub depth: usize,
    /// (이름, 값) — 이름 없는 자리는 `_N`.
    pub locals: Vec<(String, String)>,
}

/// 예산만큼(또는 끝까지) 실행한 결과.
pub struct MirRun {
    pub runtime: Runtime,
    /// 실제 실행한 스텝 수.
    pub steps: u64,
    pub budget: u64,
    /// 예산을 다 써서 중간에 멈췄으면 true (프로그램 미완료).
    pub halted: bool,
    pub trace: Vec<StepEvent>,
    pub cursor: Option<Cursor>,
}

/// MIR 크레이트를 끝까지 실행해 main의 반환값과 메모리를 돌려준다.
pub fn eval_mir(mir: &MirCrate) -> Result<Runtime, EvalError> {
    Ok(eval_mir_traced(mir, u64::MAX)?.runtime)
}

/// `budget` 스텝까지만 실행한다. 예산을 다 쓰면 `halted=true`로 멈추고 실행 경로를 남긴다.
pub fn eval_mir_traced(mir: &MirCrate, budget: u64) -> Result<MirRun, EvalError> {
    let main_id = mir.main.ok_or_else(|| EvalError::NotAFunction("main".into()))?;
    // 끝까지 실행할 때는 스텝 기록을 켜지 않는다 (오버헤드 0).
    let trace_on = budget != u64::MAX;
    let mut vm = MirVm::new(mir, budget, trace_on)?;
    let mut arena = Vec::new();
    let value = vm.exec(main_id, &[], &mut arena, 0)?;
    let cursor = vm.cursor(&arena);
    Ok(MirRun {
        runtime: Runtime { value, memory: vm.memory, layouts: vm.layouts },
        steps: vm.steps,
        budget,
        halted: vm.halted,
        trace: vm.trace.take().unwrap_or_default(),
        cursor,
    })
}

/// MIR 타입 → 필드 byte 크기. 중첩 구조체 필드는 아직 없다.
fn mir_ty_size(ty: &Ty) -> Result<usize, EvalError> {
    match ty {
        Ty::I64 => Ok(8),
        Ty::Unit => Ok(0),
        Ty::Struct(_) => Err(EvalError::NotImplemented("struct field of struct type (구조체 중첩 미지원)".into())),
    }
}

fn layout_of(def: &StructDef) -> Result<StructLayout, EvalError> {
    let mut sizes = Vec::with_capacity(def.fields.len());
    for f in &def.fields {
        sizes.push((f.name.as_str(), mir_ty_size(&f.ty)?));
    }
    Ok(StructLayout::from_sizes(&sizes))
}

/// 마지막 스텝의 위치 — halt 시 지역변수를 어느 프레임에서 읽을지 알려준다.
struct LastPos {
    fn_id: FnId,
    bb: BlockId,
    depth: usize,
    frame: Frame,
}

/// MIR 실행기 — 함수 표는 `mir`에서 빌리고, 메모리/layout만 들고 있다.
struct MirVm<'a> {
    mir: &'a MirCrate,
    memory: Memory,
    layouts: HashMap<String, StructLayout>,
    /// StructId → layout (MIR은 struct를 번호로 참조한다).
    by_id: Vec<StructLayout>,
    struct_names: Vec<String>,
    budget: u64,
    steps: u64,
    trace: Option<Vec<StepEvent>>,
    halted: bool,
    /// 현재 프레임 깊이 (main 본문 = 1).
    depth: usize,
    last: Option<LastPos>,
}

impl<'a> MirVm<'a> {
    fn new(mir: &'a MirCrate, budget: u64, trace_on: bool) -> Result<Self, EvalError> {
        let mut layouts = HashMap::new();
        let mut by_id = Vec::new();
        let mut struct_names = Vec::new();
        for def in &mir.structs {
            let layout = layout_of(def)?;
            layouts.insert(def.name.clone(), layout.clone());
            by_id.push(layout);
            struct_names.push(def.name.clone());
        }
        Ok(MirVm {
            mir,
            memory: Memory::default(),
            layouts,
            by_id,
            struct_names,
            budget,
            steps: 0,
            trace: trace_on.then(Vec::new),
            halted: false,
            depth: 0,
            last: None,
        })
    }

    /// 마지막 스텝 위치의 지역변수까지 담은 커서. 아직 아무것도 안 했으면 main 진입점이다.
    fn cursor(&self, arena: &[Value]) -> Option<Cursor> {
        let last = self.last.as_ref()?;
        let body = self.mir.fns.get(last.fn_id as usize)?;
        Some(Cursor {
            fn_id: last.fn_id,
            fn_name: body.name.clone(),
            bb: last.bb,
            depth: last.depth,
            locals: self.locals_of(body, arena, last.frame),
        })
    }

    /// 프레임 하나의 지역변수를 (이름, 값) 목록으로 만든다.
    fn locals_of(&self, body: &Body, arena: &[Value], frame: Frame) -> Vec<(String, String)> {
        (0..frame.size)
            .map(|i| {
                let name = body
                    .locals
                    .get(i)
                    .and_then(|l| l.name.clone())
                    .unwrap_or_else(|| format!("_{i}"));
                let value = arena.get(frame.base + i).cloned().unwrap_or(Value::Unit);
                (name, format_value(&self.memory, &self.layouts, &value))
            })
            .collect()
    }

    /// 스텝 1개를 시작한다 — 예산이 남아 있으면 위치를 기록하고 true, 다 썼으면 halted로 false.
    /// `text`는 trace가 켜졌을 때만 호출한다 (스텝마다 String을 만들면 실행이 몇 배 느려진다).
    fn begin_step(
        &mut self,
        fn_id: FnId,
        body: &Body,
        bb: BlockId,
        stmt: Option<usize>,
        kind: StepKind,
        frame: Frame,
        text: impl FnOnce() -> String,
    ) -> bool {
        if self.steps >= self.budget {
            self.halted = true;
            return false;
        }
        self.steps += 1;
        self.last = Some(LastPos { fn_id, bb, depth: self.depth, frame });
        if let Some(trace) = self.trace.as_mut() {
            trace.push(StepEvent {
                index: self.steps - 1,
                fn_id,
                fn_name: body.name.clone(),
                bb,
                stmt,
                kind,
                depth: self.depth,
                text: text(),
            });
        }
        true
    }

    /// 함수 하나를 실행한다. `base`는 이 호출 프레임이 시작할 arena 위치.
    fn exec(&mut self, fn_id: FnId, args: &[Value], arena: &mut Vec<Value>, base: usize) -> Result<Value, EvalError> {
        let mir = self.mir;
        let body: &'a Body = mir
            .fns
            .get(fn_id as usize)
            .ok_or_else(|| EvalError::NotAFunction(format!("fn #{fn_id}")))?;
        let frame = push_frame(arena, Frame { base, size: body.locals.len() });
        // 인자는 `arg_locals` 순서로 채운다 (AST eval의 bind_args와 같은 규칙).
        for (i, v) in args.iter().enumerate() {
            if let Some(local) = body.arg_locals.get(i) {
                arena[frame.base + *local as usize] = v.clone();
            }
        }
        // 인자가 모자라면 그 자리만 init으로 채우는 prologue 블록부터 시작한다.
        let start: BlockId = if args.len() < body.arg_locals.len() { args.len() as BlockId } else { body.entry };
        self.depth += 1;
        // 첫 프레임(호출 전)은 "main 진입점"으로 남겨 스텝 0에서도 위치를 보여준다.
        if self.last.is_none() {
            self.last = Some(LastPos { fn_id, bb: start, depth: self.depth, frame });
        }
        let out = self.run_blocks(body, fn_id, arena, frame, start);
        self.depth -= 1;
        out
    }

    /// 블록을 선형 스캔한다 — 각 문장/terminator 실행 전에 스텝 1개를 기록한다.
    fn run_blocks(
        &mut self,
        body: &Body,
        fn_id: FnId,
        arena: &mut Vec<Value>,
        frame: Frame,
        start: BlockId,
    ) -> Result<Value, EvalError> {
        let mir = self.mir;
        let mut bb = start;
        loop {
            let block = &body.blocks[bb as usize];
            for (i, stmt) in block.stmts.iter().enumerate() {
                if !self.begin_step(fn_id, body, bb, Some(i), StepKind::Stmt, frame, || mir.stmt_str(stmt)) {
                    return Ok(Value::Unit);
                }
                self.exec_stmt(stmt, arena, frame)?;
            }
            let term = &block.terminator;
            if !self.begin_step(fn_id, body, bb, None, StepKind::Term, frame, || mir.term_str(term)) {
                return Ok(Value::Unit);
            }
            match &term.kind {
                TerminatorKind::Goto(t) => bb = *t,
                TerminatorKind::SwitchInt { discr, targets, otherwise } => {
                    let Value::Int(v) = self.read_operand(discr, arena, frame)? else {
                        return Err(EvalError::NotImplemented("switchInt on non-int".into()));
                    };
                    bb = targets.iter().find(|(k, _)| *k == v).map(|(_, b)| *b).unwrap_or(*otherwise);
                }
                TerminatorKind::Call { func, args: ops, dest, target } => {
                    let vals = ops
                        .iter()
                        .map(|o| self.read_operand(o, arena, frame))
                        .collect::<Result<Vec<_>, _>>()?;
                    let ret = match func {
                        FnRef::Print => {
                            self.print_vals(&vals);
                            Value::Unit
                        }
                        // callee 프레임은 부모 프레임 바로 위 (arena 스택 규율).
                        FnRef::User(id) => self.exec(*id, &vals, arena, frame.base + frame.size)?,
                    };
                    // 예산이 callee 안에서 끝났으면 반환값을 dest에 남기지 않는다 (halt 상태 보존).
                    if self.halted {
                        return Ok(Value::Unit);
                    }
                    self.write_place(dest, ret, arena, frame)?;
                    bb = *target;
                }
                TerminatorKind::Return => return Ok(arena[frame.base].clone()),
                TerminatorKind::Unreachable => {
                    return Err(EvalError::NotImplemented(format!("unreachable bb{bb} in fn {}", body.name)))
                }
            }
        }
    }

    fn exec_stmt(&mut self, stmt: &Stmt, arena: &mut Vec<Value>, frame: Frame) -> Result<(), EvalError> {
        match &stmt.kind {
            StmtKind::Assign(place, rvalue) => {
                let value = self.eval_rvalue(rvalue, arena, frame)?;
                self.write_place(place, value, arena, frame)
            }
        }
    }

    fn eval_rvalue(&mut self, rv: &Rvalue, arena: &mut Vec<Value>, frame: Frame) -> Result<Value, EvalError> {
        match rv {
            Rvalue::Use(op) => self.read_operand(op, arena, frame),
            Rvalue::BinaryOp(op, l, r) => {
                let a = self.read_operand(l, arena, frame)?;
                let b = self.read_operand(r, arena, frame)?;
                apply_binop(op, &a, &b)
            }
            Rvalue::Aggregate { struct_id, fields } => {
                let layout = self
                    .by_id
                    .get(*struct_id as usize)
                    .cloned()
                    .ok_or_else(|| EvalError::NotImplemented(format!("unknown struct id {struct_id}")))?;
                let name = self.struct_names.get(*struct_id as usize).cloned().unwrap_or_default();
                let offset = self.memory.alloc(layout.size);
                for (index, op) in fields {
                    let slot = layout.slot(*index as u16).ok_or_else(|| {
                        EvalError::NotImplemented(format!("field {index} out of range on {name}"))
                    })?;
                    let Value::Int(n) = self.read_operand(op, arena, frame)? else {
                        return Err(EvalError::NotImplemented(format!(
                            "non-int field on {name} (구조체 중첩 미지원)"
                        )));
                    };
                    self.memory.write_int(offset + slot.offset, slot.size, n);
                }
                Ok(Value::Struct { name, offset })
            }
        }
    }

    fn read_operand(&self, op: &Operand, arena: &[Value], frame: Frame) -> Result<Value, EvalError> {
        match op {
            Operand::Const(ConstValue::Int(n)) => Ok(Value::Int(*n)),
            Operand::Const(ConstValue::Unit) => Ok(Value::Unit),
            Operand::Copy(place) => self.read_place(place, arena, frame),
        }
    }

    fn read_place(&self, place: &Place, arena: &[Value], frame: Frame) -> Result<Value, EvalError> {
        let local = frame.base + place.local as usize;
        let base = arena
            .get(local)
            .ok_or_else(|| EvalError::UnresolvedLocal(format!("_{} out of range", place.local)))?;
        match place.proj.as_slice() {
            [] => Ok(base.clone()),
            [ProjectionElem::Field(_, index)] => {
                let Value::Struct { name, offset } = base else {
                    return Err(EvalError::NotImplemented(format!("field {index} on non-struct `_{}`", place.local)));
                };
                let slot = self.field_slot(name, *index)?;
                Ok(Value::Int(self.memory.read_int(offset + slot.offset, slot.size)))
            }
            _ => Err(EvalError::NotImplemented("nested field projection (구조체 중첩 미지원)".into())),
        }
    }

    fn write_place(&mut self, place: &Place, value: Value, arena: &mut [Value], frame: Frame) -> Result<(), EvalError> {
        let local = frame.base + place.local as usize;
        match place.proj.as_slice() {
            [] => {
                let slot = arena
                    .get_mut(local)
                    .ok_or_else(|| EvalError::UnresolvedLocal(format!("_{} out of range", place.local)))?;
                *slot = value;
                Ok(())
            }
            [ProjectionElem::Field(_, index)] => {
                let Value::Int(n) = value else {
                    return Err(EvalError::NotImplemented("non-int field write (구조체 중첩 미지원)".into()));
                };
                let Value::Struct { name, offset } = arena.get(local).cloned().ok_or_else(|| {
                    EvalError::UnresolvedLocal(format!("_{} out of range", place.local))
                })? else {
                    return Err(EvalError::NotImplemented(format!("field {index} on non-struct `_{}`", place.local)));
                };
                let slot = self.field_slot(&name, *index)?;
                self.memory.write_int(offset + slot.offset, slot.size, n);
                Ok(())
            }
            _ => Err(EvalError::NotImplemented("nested field projection (구조체 중첩 미지원)".into())),
        }
    }

    fn field_slot(&self, name: &str, index: u32) -> Result<crate::FieldSlot, EvalError> {
        let layout = self
            .layouts
            .get(name)
            .ok_or_else(|| EvalError::NotImplemented(format!("unknown struct {name}")))?;
        layout
            .slot(index as u16)
            .ok_or_else(|| EvalError::NotImplemented(format!("field {index} out of range on {name}")))
    }

    fn print_vals(&self, vals: &[Value]) {
        for v in vals {
            print!("{} ", format_value(&self.memory, &self.layouts, v));
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtoy_lexer::tokenize;

    /// lowering → resolve → MIR (스텝 실행 테스트용 최소 파이프라인).
    fn mir_of(src: &str) -> MirCrate {
        let mut krate = rtoy_tokenstream_lowering::try_lower(&tokenize(src), src).expect("lowering");
        rtoy_resolve::resolve(&mut krate, src).expect("resolve");
        rtoy_ast_lowering::lower_crate(&krate).expect("mir lowering")
    }

    #[test]
    fn budget_stops_early_and_records_trace() {
        let mir = mir_of("fn main() { let x = 1; x }");
        let run = eval_mir_traced(&mir, 1).expect("budget 1 실행");
        assert!(run.halted, "예산을 다 썼으면 halted");
        assert_eq!(run.steps, 1);
        assert_eq!(run.trace.len(), 1, "실행한 스텝만 기록");
        assert_eq!(run.trace[0].fn_name, "main");
        assert!(run.cursor.is_some(), "현재 위치는 항상 있다");
    }

    #[test]
    fn zero_budget_shows_entry_without_running() {
        let mir = mir_of("fn main() { let x = 1; x }");
        let run = eval_mir_traced(&mir, 0).expect("budget 0 실행");
        assert!(run.halted);
        assert_eq!(run.steps, 0);
        assert!(run.trace.is_empty());
        assert_eq!(run.cursor.map(|c| c.fn_name), Some("main".to_string()));
    }

    #[test]
    fn large_budget_matches_full_eval() {
        let src = "fn fib() { let n = 0; if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }\nfn main() { let n = 5; fib(n) }";
        let mir = mir_of(src);
        let run = eval_mir_traced(&mir, 100_000).expect("큰 예산 실행");
        assert!(!run.halted, "끝까지 실행되면 halted=false");
        assert_eq!(run.steps as usize, run.trace.len());
        assert_eq!(run.runtime.value, eval_mir(&mir).expect("eval_mir").value);
    }
}
