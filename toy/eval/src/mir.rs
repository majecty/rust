//! MIR 인터프리터 — `rtoy-mir`의 basic block CFG를 실행한다.
//! AST eval과 같은 arena/`Frame`/`Value`/`Memory`를 재사용한다 (호출당 할당 없음).
//! 차이: AST를 재귀 순회하지 않고 블록을 선형 스캔한다 (Place는 이미 index).

use crate::{
    apply_binop, format_value, push_frame, EvalError, Frame, Memory, Runtime, StructLayout, Value,
};
use rtoy_mir::{
    BlockId, Body, ConstValue, FnId, FnRef, MirCrate, Operand, Place, ProjectionElem, Rvalue, Stmt,
    StmtKind, StructDef, TerminatorKind, Ty,
};
use std::collections::HashMap;

/// MIR 크레이트를 실행해 main의 반환값과 메모리를 돌려준다.
pub fn eval_mir(mir: &MirCrate) -> Result<Runtime, EvalError> {
    let main_id = mir.main.ok_or_else(|| EvalError::NotAFunction("main".into()))?;
    let mut vm = MirVm::new(mir)?;
    let mut arena = Vec::new();
    let value = vm.exec(main_id, &[], &mut arena, 0)?;
    Ok(Runtime { value, memory: vm.memory, layouts: vm.layouts })
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

/// MIR 실행기 — 함수 표는 `mir`에서 빌리고, 메모리/layout만 들고 있다.
struct MirVm<'a> {
    mir: &'a MirCrate,
    memory: Memory,
    layouts: HashMap<String, StructLayout>,
    /// StructId → layout (MIR은 struct를 번호로 참조한다).
    by_id: Vec<StructLayout>,
    struct_names: Vec<String>,
}

impl<'a> MirVm<'a> {
    fn new(mir: &'a MirCrate) -> Result<Self, EvalError> {
        let mut layouts = HashMap::new();
        let mut by_id = Vec::new();
        let mut struct_names = Vec::new();
        for def in &mir.structs {
            let layout = layout_of(def)?;
            layouts.insert(def.name.clone(), layout.clone());
            by_id.push(layout);
            struct_names.push(def.name.clone());
        }
        Ok(MirVm { mir, memory: Memory::default(), layouts, by_id, struct_names })
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
        let mut bb: BlockId = if args.len() < body.arg_locals.len() { args.len() as BlockId } else { body.entry };
        loop {
            let block = &body.blocks[bb as usize];
            for stmt in &block.stmts {
                self.exec_stmt(stmt, arena, frame)?;
            }
            match &block.terminator.kind {
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
