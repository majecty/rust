//! rtoy eval — 단일 byte array 메모리 위에서 도는 미니 AST 인터프리터.
//! fn main() { let p = Point { x: 1, y: 2 }; p.x + p.y } → 3
//!
//! 구조체 값은 `Value::Struct { offset }` 핸들일 뿐이고, 필드는 StructLayout의
//! byte offset/size로 단일 버퍼에 읽고 쓴다. (Wasm/JVM 선형 메모리 모델 축소판)

use rtoy_ast::*;
use std::collections::HashMap;
use std::rc::Rc;

mod mir;
pub use mir::eval_mir;

#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// resolve를 거치지 않았거나 resolve 결과와 프레임이 어긋난 지역변수.
    UnresolvedLocal(String),
    NotAFunction(String),
    ArgCountMismatch { expected: usize, got: usize },
    EmptyMain,
    NotImplemented(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::UnresolvedLocal(n) => {
                write!(f, "unresolved local: {n} (resolve must run before eval)")
            }
            EvalError::NotAFunction(n) => write!(f, "not a function: {}", n),
            EvalError::ArgCountMismatch { expected, got } => {
                write!(f, "argument count mismatch: expected {} got {}", expected, got)
            }
            EvalError::EmptyMain => write!(f, "fn main() is empty"),
            EvalError::NotImplemented(s) => write!(f, "not implemented: {}", s),
        }
    }
}

impl std::error::Error for EvalError {}

/// 단일 byte array. 모든 구조체 값이 이 버퍼 안에 산다.
#[derive(Debug, Default)]
pub struct Memory {
    bytes: Vec<u8>,
}

impl Memory {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// size 바이트를 0으로 늘리고 시작 offset을 돌려준다.
    fn alloc(&mut self, size: usize) -> usize {
        let offset = self.bytes.len();
        self.bytes.resize(offset + size, 0);
        offset
    }

    fn write_int(&mut self, offset: usize, size: usize, value: i64) {
        let src = value.to_le_bytes();
        self.bytes[offset..offset + size].copy_from_slice(&src[..size]);
    }

    /// size 바이트를 little-endian으로 읽고 부호 확장한다.
    fn read_int(&self, offset: usize, size: usize) -> i64 {
        let mut buf = [0u8; 8];
        buf[..size].copy_from_slice(&self.bytes[offset..offset + size]);
        let shift = 64 - size * 8;
        (i64::from_le_bytes(buf) << shift) >> shift
    }
}

/// 구조체 1개의 메모리 배치.
/// `slots`의 순서가 선언 순서 = resolve가 채우는 slot 번호다.
#[derive(Debug, Clone)]
pub struct StructLayout {
    pub size: usize,
    pub slots: Vec<FieldSlot>,
    /// 표시·fallback용 이름 (slots와 같은 순서)
    names: Vec<String>,
    by_name: HashMap<String, u16>,
}

/// 필드 1개의 byte 위치.
#[derive(Debug, Clone, Copy)]
pub struct FieldSlot {
    pub offset: usize,
    pub size: usize,
}

impl StructLayout {
    /// (필드 이름, 크기) 목록에서 배치를 만든다 (AST eval / MIR eval 공용).
    /// 필드 선언 순서대로 offset을 누적한다 (natural alignment, padding 포함).
    fn from_sizes(fields: &[(&str, usize)]) -> StructLayout {
        let mut size = 0usize;
        let mut slots = Vec::new();
        let mut names = Vec::new();
        let mut by_name = HashMap::new();
        for (name, fsize) in fields {
            let align = (*fsize).clamp(1, 8);
            size = (size + align - 1) / align * align;
            by_name.insert((*name).to_string(), slots.len() as u16);
            names.push((*name).to_string());
            slots.push(FieldSlot { offset: size, size: *fsize });
            size += *fsize;
        }
        StructLayout { size, slots, names, by_name }
    }

    /// 필드 선언 순서대로 offset을 누적한다 (natural alignment, padding 포함).
    fn of(item: &StructItem) -> Result<StructLayout, EvalError> {
        let sizes = item
            .fields
            .iter()
            .map(|f| Ok((f.name.name.as_str(), type_size(&f.ty.name)?)))
            .collect::<Result<Vec<_>, EvalError>>()?;
        Ok(StructLayout::from_sizes(&sizes))
    }

    /// slot 번호로 필드 위치를 얻는다 (resolve가 채운 번호의 조회 경로).
    pub fn slot(&self, index: u16) -> Option<FieldSlot> {
        self.slots.get(index as usize).copied()
    }

    /// 이름으로 slot 번호를 찾는다 (resolve 미해결 노드의 fallback 경로).
    pub fn slot_of(&self, name: &str) -> Option<u16> {
        self.by_name.get(name).copied()
    }

    /// slot 번호의 필드 이름 (표시용).
    pub fn name_of(&self, index: u16) -> Option<&str> {
        self.names.get(index as usize).map(String::as_str)
    }
}

/// 기본 타입 이름 → byte 크기. 구조체 중첩 타입은 아직 없다.
fn type_size(ty: &str) -> Result<usize, EvalError> {
    match ty {
        "i8" | "u8" | "bool" => Ok(1),
        "i16" | "u16" => Ok(2),
        "i32" | "u32" => Ok(4),
        "i64" | "u64" | "usize" | "isize" => Ok(8),
        other => Err(EvalError::NotImplemented(format!("field type {other}"))),
    }
}

/// 값 — 구조체는 메모리 offset 핸들이다.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Struct { name: String, offset: usize },
    Unit,
}

/// 실행 결과: main 반환값 + 그 값이 사는 단일 메모리.
pub struct Runtime {
    pub value: Value,
    pub memory: Memory,
    layouts: HashMap<String, StructLayout>,
}

impl Runtime {
    pub fn layout(&self, name: &str) -> Option<&StructLayout> {
        self.layouts.get(name)
    }

    pub fn format_value(&self, value: &Value) -> String {
        format_value(&self.memory, &self.layouts, value)
    }
}

/// 값 1개를 사람이 읽는 문자열로 만든다 (구조체 필드는 메모리에서 읽음).
fn format_value(memory: &Memory, layouts: &HashMap<String, StructLayout>, value: &Value) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        Value::Unit => "()".into(),
        Value::Struct { name, offset } => {
            let Some(layout) = layouts.get(name) else {
                return format!("{name} {{ .. }}");
            };
            let mut order: Vec<u16> = (0..layout.slots.len() as u16).collect();
            order.sort_by_key(|i| layout.name_of(*i).unwrap_or("?"));
            let body: Vec<String> = order
                .iter()
                .filter_map(|i| {
                    let slot = layout.slot(*i)?;
                    let n = memory.read_int(offset + slot.offset, slot.size);
                    Some(format!("{}: {n}", layout.name_of(*i)?))
                })
                .collect();
            format!("{name} {{ {} }}", body.join(", "))
        }
    }
}

/// Crate를 실행하여 main의 반환값과 메모리를 돌려준다.
pub fn eval_crate(krate: &Crate) -> Result<Runtime, EvalError> {
    let mut vm = Vm::register(krate)?;
    // Rc만 복제한다 (호출마다 AST를 clone하지 않기 위함).
    let main_fn = vm.main_fn()?;
    // 실행 스택: 최대 호출 깊이만큼 자라고, 호출마다 창을 잘라 재사용한다 (호출당 할당 없음).
    let mut arena = vec![Value::Unit; main_fn.locals as usize];
    let value = vm.eval_block(&main_fn.body, &mut arena, Frame::root(main_fn.locals))?;
    Ok(Runtime { value, memory: vm.memory, layouts: vm.layouts })
}

/// 지역변수 프레임 창 — `arena[base .. base+size]`가 이 프레임이다.
/// resolve가 배정한 slot 번호가 `base` 기준 상대 index이고, 크기는 `FnItem.locals`에서 온다.
#[derive(Debug, Clone, Copy)]
struct Frame {
    base: usize,
    size: usize,
}

impl Frame {
    /// 호출 스택 맨 아래 프레임 (main).
    fn root(locals: u16) -> Frame {
        Frame { base: 0, size: locals as usize }
    }

    /// 이 프레임 바로 위에 붙는 callee 프레임 (스택 규율: base는 부모 끝).
    fn child(&self, locals: u16) -> Frame {
        Frame { base: self.base + self.size, size: locals as usize }
    }
}

/// callee 창을 준비한다 — arena를 필요하면 늘리고, 창을 `Value::Unit`으로 되돌린다.
/// 재사용 시 이전 호출의 값이 남지 않도록 반드시 초기화한다.
fn push_frame(arena: &mut Vec<Value>, frame: Frame) -> Frame {
    let end = frame.base + frame.size;
    if arena.len() < end {
        arena.resize(end, Value::Unit);
    }
    for v in &mut arena[frame.base..end] {
        *v = Value::Unit;
    }
    frame
}

/// slot 번호를 프레임 안 상대 index로 바꾼다.
/// resolve를 거치지 않은 AST는 slot이 없거나 범위를 벗어나므로 조용히 넘기지 않는다.
fn slot_index(frame: Frame, name: &str, slot: Option<u16>) -> Result<usize, EvalError> {
    let index = slot.ok_or_else(|| {
        EvalError::UnresolvedLocal(format!("`{name}` has no slot (resolve not run)"))
    })? as usize;
    if index >= frame.size {
        return Err(EvalError::UnresolvedLocal(format!(
            "`{name}` slot {index} out of range (frame has {})",
            frame.size
        )));
    }
    Ok(index)
}

/// 프레임에서 값을 읽는다.
fn frame_get(arena: &[Value], frame: Frame, name: &str, slot: Option<u16>) -> Result<Value, EvalError> {
    let index = slot_index(frame, name, slot)?;
    Ok(arena[frame.base + index].clone())
}

/// 프레임에 값을 쓴다 (같은 이름 재선언은 같은 slot을 덮어쓴다 — 선형 스캔과 같은 의미).
fn frame_set(arena: &mut [Value], frame: Frame, name: &str, slot: Option<u16>, value: Value) -> Result<(), EvalError> {
    let index = slot_index(frame, name, slot)?;
    arena[frame.base + index] = value;
    Ok(())
}

/// 이항 연산 적용 (AST eval / MIR eval 공용).
pub(crate) fn apply_binop(op: &BinOp, l: &Value, r: &Value) -> Result<Value, EvalError> {
    match (op, l, r) {
        (BinOp::Add, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        (BinOp::Sub, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
        (BinOp::Mul, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
        (BinOp::Lt, Value::Int(a), Value::Int(b)) => Ok(Value::Int((a < b) as i64)),
        (BinOp::Div, Value::Int(a), Value::Int(b)) => match b {
            0 => Err(EvalError::NotImplemented("division by zero".into())),
            _ => Ok(Value::Int(a / b)),
        },
        (BinOp::Mod, Value::Int(a), Value::Int(b)) => match b {
            0 => Err(EvalError::NotImplemented("mod by zero".into())),
            _ => Ok(Value::Int(a % b)),
        },
        _ => Err(EvalError::NotImplemented(format!("binary {:?} on non-int", op))),
    }
}

/// 함수 정의 + layout + 단일 메모리.
struct Vm {
    /// 크레이트 순서 그대로의 함수 테이블 — resolve가 심은 `Call.fn_index`가 이 index다.
    /// 호출마다 `FnItem`을 clone하면 AST 전체가 복사되므로 Rc로 공유한다.
    fns: Vec<Rc<FnItem>>,
    /// 이름 → 테이블 index (main 조회와 resolve 미경유 AST의 fallback용).
    by_name: HashMap<String, u16>,
    layouts: HashMap<String, StructLayout>,
    memory: Memory,
}

impl Vm {
    fn main_fn(&self) -> Result<Rc<FnItem>, EvalError> {
        let index = self.by_name.get("main").ok_or_else(|| EvalError::NotAFunction("main".into()))?;
        Ok(Rc::clone(&self.fns[*index as usize]))
    }

    fn register(krate: &Crate) -> Result<Vm, EvalError> {
        let mut vm = Vm { fns: Vec::new(), by_name: HashMap::new(), layouts: HashMap::new(), memory: Memory::default() };
        for item in &krate.items {
            match &item.kind {
                ItemKind::Fn(f) => {
                    vm.by_name.insert(item.name.name.clone(), vm.fns.len() as u16);
                    vm.fns.push(Rc::new(f.clone()));
                }
                ItemKind::Struct(s) => {
                    vm.layouts.insert(item.name.name.clone(), StructLayout::of(s)?);
                }
                ItemKind::MacroDef(_) | ItemKind::Macro { .. } => {} // 매크로는 무시
            }
        }
        Ok(vm)
    }

    fn eval_block(&mut self, block: &Block, arena: &mut Vec<Value>, frame: Frame) -> Result<Value, EvalError> {
        for stmt in &block.stmts {
            match &stmt.kind {
                StmtKind::Let(let_stmt) => {
                    let val = match &let_stmt.init {
                        Some(init) => self.eval_expr(init, arena, frame)?,
                        None => Value::Int(0),
                    };
                    frame_set(arena, frame, &let_stmt.name.name, let_stmt.slot, val)?;
                }
                StmtKind::Expr(expr) => {
                    self.eval_expr(expr, arena, frame)?;
                }
            }
        }
        match &block.tail {
            Some(tail) => self.eval_expr(tail, arena, frame),
            None => Ok(Value::Unit),
        }
    }

    fn eval_expr(&mut self, expr: &Expr, arena: &mut Vec<Value>, frame: Frame) -> Result<Value, EvalError> {
        match &expr.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Var { name, slot } => frame_get(arena, frame, &name.name, *slot),
            ExprKind::Binary { op, lhs, rhs } => self.eval_binary(op, lhs, rhs, arena, frame),
            ExprKind::StructLiteral { name, fields } => self.build_struct(name, fields, arena, frame),
            ExprKind::FieldAccess { base, field, slot } => {
                let val = self.eval_expr(base, arena, frame)?;
                self.read_field(&val, &field.name, *slot)
            }
            ExprKind::Call { callee, args, fn_index } => self.eval_call(callee, args, *fn_index, arena, frame),
            ExprKind::If { cond, then_block, else_block } => {
                self.eval_if(cond, then_block, else_block.as_deref(), arena, frame)
            }
            ExprKind::Macro { .. } => Err(EvalError::NotImplemented("macro call (not expanded)".into())),
        }
    }

    fn eval_binary(
        &mut self,
        op: &BinOp,
        lhs: &Expr,
        rhs: &Expr,
        arena: &mut Vec<Value>,
        frame: Frame,
    ) -> Result<Value, EvalError> {
        let l = self.eval_expr(lhs, arena, frame)?;
        let r = self.eval_expr(rhs, arena, frame)?;
        apply_binop(op, &l, &r)
    }

    /// if 식: 조건이 0이 아니면 then, 0이면 else (else 없으면 Unit).
    /// 블록 레벨 스코프가 없어 분기 안 `let`은 바깥 프레임에 그대로 들어간다.
    fn eval_if(
        &mut self,
        cond: &Expr,
        then_block: &Block,
        else_block: Option<&Block>,
        arena: &mut Vec<Value>,
        frame: Frame,
    ) -> Result<Value, EvalError> {
        match self.eval_expr(cond, arena, frame)? {
            Value::Int(0) => match else_block {
                Some(block) => self.eval_block(block, arena, frame),
                None => Ok(Value::Unit),
            },
            Value::Int(_) => self.eval_block(then_block, arena, frame),
            other => Err(EvalError::NotImplemented(format!(
                "if condition must be int, got {other:?}"
            ))),
        }
    }

    /// 구조체 리터럴: layout 크기만큼 버퍼를 잡고 필드별 byte offset에 쓴다.
    fn build_struct(
        &mut self,
        name: &Ident,
        fields: &[FieldInit],
        arena: &mut Vec<Value>,
        frame: Frame,
    ) -> Result<Value, EvalError> {
        let layout = self
            .layouts
            .get(&name.name)
            .cloned()
            .ok_or_else(|| EvalError::NotImplemented(format!("unknown struct {}", name.name)))?;
        let offset = self.memory.alloc(layout.size);
        for field in fields {
            let slot = self.slot_of_field(&layout, &name.name, field)?;
            let Value::Int(n) = self.eval_expr(&field.value, arena, frame)? else {
                return Err(EvalError::NotImplemented(format!(
                    "non-int field {} (구조체 중첩 미지원)",
                    field.name.name
                )));
            };
            self.memory.write_int(offset + slot.offset, slot.size, n);
        }
        Ok(Value::Struct { name: name.name.clone(), offset })
    }

    /// 리터럴 필드 위치: resolve가 채운 slot을 쓰고, 미해결이면 이름으로 찾는다.
    fn slot_of_field(
        &self,
        layout: &StructLayout,
        struct_name: &str,
        field: &FieldInit,
    ) -> Result<FieldSlot, EvalError> {
        let index = match field.slot {
            Some(i) => i,
            None => layout.slot_of(&field.name.name).ok_or_else(|| {
                EvalError::NotImplemented(format!(
                    "unknown field {} on {struct_name}",
                    field.name.name
                ))
            })?,
        };
        layout.slot(index).ok_or_else(|| {
            EvalError::NotImplemented(format!("slot {index} out of range on {struct_name}"))
        })
    }

    /// 필드 접근: base offset + 필드 offset에서 size바이트를 읽는다.
    fn read_field(&self, base: &Value, field: &str, slot: Option<u16>) -> Result<Value, EvalError> {
        let Value::Struct { name, offset } = base else {
            return Err(EvalError::NotImplemented("field access on non-struct".into()));
        };
        let layout = self
            .layouts
            .get(name)
            .ok_or_else(|| EvalError::NotImplemented(format!("unknown struct {name}")))?;
        let index = match slot {
            Some(i) => i,
            None => layout
                .slot_of(field)
                .ok_or_else(|| EvalError::NotImplemented(format!("unknown field {field}")))?,
        };
        let slot = layout
            .slot(index)
            .ok_or_else(|| EvalError::NotImplemented(format!("slot {index} out of range on {name}")))?;
        Ok(Value::Int(self.memory.read_int(offset + slot.offset, slot.size)))
    }

    fn eval_call(
        &mut self,
        callee: &Ident,
        args: &[Expr],
        fn_index: Option<u16>,
        arena: &mut Vec<Value>,
        frame: Frame,
    ) -> Result<Value, EvalError> {
        if callee.name == "print" {
            let vals: Vec<Value> = args
                .iter()
                .map(|a| self.eval_expr(a, arena, frame))
                .collect::<Result<_, _>>()?;
            for v in &vals {
                print!("{} ", format_value(&self.memory, &self.layouts, v));
            }
            println!();
            return Ok(Value::Unit);
        }

        // resolve가 심은 테이블 번호로 바로 찾는다 (이름 해시 제거).
        // 미해결(resolve 미경유)이면 이름으로 fallback한다.
        let f = match fn_index {
            Some(i) => self.fns.get(i as usize).cloned(),
            None => self.by_name.get(&callee.name).and_then(|i| self.fns.get(*i as usize)).cloned(),
        }
        .ok_or_else(|| EvalError::NotAFunction(callee.name.clone()))?;

        // 호출마다 새 Vec을 만들지 않고, 부모 프레임 바로 위 창을 재사용한다.
        let locals = push_frame(arena, frame.child(f.locals));
        self.bind_args(&f, args, arena, frame, locals)?;
        for stmt in &f.body.stmts {
            if let StmtKind::Expr(expr) = &stmt.kind {
                self.eval_expr(expr, arena, locals)?;
            }
        }
        match &f.body.tail {
            Some(tail) => self.eval_expr(tail, arena, locals),
            None => Ok(Value::Unit),
        }
    }

    /// AST에 파라미터가 없어 let 문 순서대로 인자를 바인딩한다 (간이 방식, 남은 let은 init 사용).
    fn bind_args(
        &mut self,
        f: &FnItem,
        args: &[Expr],
        arena: &mut Vec<Value>,
        caller: Frame,
        locals: Frame,
    ) -> Result<(), EvalError> {
        let mut idx = 0;
        for stmt in &f.body.stmts {
            let StmtKind::Let(let_stmt) = &stmt.kind else { continue };
            let value = if idx < args.len() {
                let v = self.eval_expr(&args[idx], arena, caller)?;
                idx += 1;
                v
            } else if let Some(init) = &let_stmt.init {
                self.eval_expr(init, arena, locals)?
            } else {
                Value::Int(0)
            };
            frame_set(arena, locals, &let_stmt.name.name, let_stmt.slot, value)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtoy_span::Span;

    fn int_lit(n: i64) -> Expr {
        Expr { kind: ExprKind::Int(n), span: Span::root(0, 0) }
    }

    /// slot을 주지 않은 참조 — eval이 프레임 이름표로 fallback 해석한다.
    fn var(name: &str) -> Expr {
        var_at(name, None)
    }

    /// resolve가 slot을 채웠다고 가정한 참조 (slot이 있으면 이름보다 우선해야 한다).
    fn var_at(name: &str, slot: Option<u16>) -> Expr {
        Expr {
            kind: ExprKind::Var { name: Ident { name: name.into(), span: Span::root(0, 0) }, slot },
            span: Span::root(0, 0),
        }
    }

    fn add(l: Expr, r: Expr) -> Expr {
        Expr { kind: ExprKind::Binary { op: BinOp::Add, lhs: Box::new(l), rhs: Box::new(r) }, span: Span::root(0, 0) }
    }

    fn let_stmt(name: &str, init: Expr) -> Stmt {
        Stmt {
            kind: StmtKind::Let(LetStmt {
                name: Ident { name: name.into(), span: Span::root(0, 0) },
                ty: None,
                init: Some(init),
                slot: None,
                span: Span::root(0, 0),
            }),
            span: Span::root(0, 0),
        }
    }

    fn main_fn(stmts: Vec<Stmt>, tail: Option<Expr>) -> Item {
        Item {
            name: Ident { name: "main".into(), span: Span::root(0, 0) },
            kind: ItemKind::Fn(FnItem {
                body: Block { stmts, tail, span: Span::root(0, 0) },
                locals: 0,
                span: Span::root(0, 0),
            }),
            span: Span::root(0, 0),
        }
    }

    /// 손으로 만든 AST를 resolve까지 통과시켜 실행한다 (eval은 resolve된 AST를 전제한다).
    fn run(items: Vec<Item>) -> Runtime {
        let mut krate = Crate { items, span: Span::root(0, 0) };
        if let Err(errs) = rtoy_resolve::resolve(&mut krate, "") {
            panic!("resolve 실패: {errs:?}");
        }
        eval_crate(&krate).unwrap()
    }

    /// resolve를 일부러 건너뛴 AST — slot 미지정/오지정 검증용.
    fn run_unresolved(items: Vec<Item>) -> Result<Runtime, EvalError> {
        eval_crate(&Crate { items, span: Span::root(0, 0) })
    }

    #[test]
    fn eval_int_literal() {
        assert_eq!(run(vec![main_fn(vec![], Some(int_lit(42)))]).value, Value::Int(42));
    }

    #[test]
    fn eval_add() {
        let krate = vec![main_fn(vec![], Some(add(int_lit(1), int_lit(2))))];
        assert_eq!(run(krate).value, Value::Int(3));
    }

    #[test]
    fn eval_let_and_var() {
        let items = vec![main_fn(
            vec![let_stmt("x", int_lit(10))],
            Some(add(var("x"), int_lit(5))),
        )];
        assert_eq!(run(items).value, Value::Int(15));
    }

    #[test]
    fn eval_nested_let() {
        let items = vec![main_fn(
            vec![let_stmt("a", int_lit(3)), let_stmt("b", add(var("a"), int_lit(7)))],
            Some(var("b")),
        )];
        assert_eq!(run(items).value, Value::Int(10));
    }

    #[test]
    fn eval_sub_mul() {
        let mul = Expr {
            kind: ExprKind::Binary {
                op: BinOp::Mul,
                lhs: Box::new(int_lit(3)),
                rhs: Box::new(add(int_lit(1), int_lit(2))),
            },
            span: Span::root(0, 0),
        };
        assert_eq!(run(vec![main_fn(vec![], Some(mul))]).value, Value::Int(9));
    }

    #[test]
    fn unresolved_local_is_rejected() {
        // resolve를 건너뛴 Var는 이름으로 찾지 않고 즉시 실패한다.
        let items = vec![main_fn(vec![], Some(var("x")))];
        assert!(matches!(run_unresolved(items), Err(EvalError::UnresolvedLocal(_))));
    }

    #[test]
    fn unresolved_slots_are_rejected() {
        // resolve를 건너뛴 AST: slot이 없거나 프레임 범위를 벗어나면 즉시 실패한다.
        let no_slot = vec![main_fn(vec![let_stmt("x", int_lit(10))], Some(int_lit(1)))];
        let Err(err) = run_unresolved(no_slot) else { panic!("에러가 아님") };
        assert!(err.to_string().contains("has no slot"), "{err}");

        let out_of_range = vec![main_fn(vec![], Some(var_at("x", Some(9))))];
        let Err(err) = run_unresolved(out_of_range) else { panic!("에러가 아님") };
        assert!(err.to_string().contains("out of range"), "{err}");
    }

    #[test]
    fn eval_empty_main() {
        let krate = Crate { items: vec![], span: Span::root(0, 0) };
        assert!(matches!(eval_crate(&krate), Err(EvalError::NotAFunction(_))));
    }

    // 구조체 관련 헬퍼
    fn struct_def(name: &str, fields: Vec<(&str, &str)>) -> Item {
        let s = Span::root(0, 0);
        Item {
            name: Ident { name: name.into(), span: s },
            kind: ItemKind::Struct(StructItem {
                fields: fields.into_iter().map(|(n, t)| StructField {
                    name: Ident { name: n.into(), span: s },
                    ty: Ty { name: t.into(), span: s },
                    span: s,
                }).collect(),
                span: s,
            }),
            span: s,
        }
    }

    fn struct_literal(name: &str, fields: Vec<(&str, Expr)>) -> Expr {
        let s = Span::root(0, 0);
        Expr {
            kind: ExprKind::StructLiteral {
                name: Ident { name: name.into(), span: s },
                fields: fields.into_iter().map(|(n, v)| FieldInit {
                    name: Ident { name: n.into(), span: s },
                    value: v,
                    slot: None,
                    span: s,
                }).collect(),
            },
            span: s,
        }
    }

    fn field_access(base: Expr, field: &str) -> Expr {
        field_access_at(base, field, None)
    }

    /// slot: resolve가 채웠다고 가정한 값 (None이면 이름 fallback 경로).
    fn field_access_at(base: Expr, field: &str, slot: Option<u16>) -> Expr {
        let s = Span::root(0, 0);
        Expr {
            kind: ExprKind::FieldAccess {
                base: Box::new(base),
                field: Ident { name: field.into(), span: s },
                slot,
            },
            span: s,
        }
    }

    fn point_literal() -> Expr {
        struct_literal("Point", vec![("x", int_lit(10)), ("y", int_lit(20))])
    }

    #[test]
    fn struct_layout_is_byte_offsets() {
        let items = vec![struct_def("Point", vec![("x", "i64"), ("y", "i64")])];
        let rt = run(vec![items[0].clone(), main_fn(vec![], Some(point_literal()))]);
        let layout = rt.layout("Point").expect("layout");
        assert_eq!(layout.size, 16);
        assert_eq!(layout.slot_of("x"), Some(0));
        assert_eq!(layout.slot_of("y"), Some(1));
        assert_eq!(layout.slot(0).unwrap().offset, 0);
        assert_eq!(layout.slot(1).unwrap().offset, 8);
        assert_eq!(layout.name_of(1), Some("y"));
    }

    #[test]
    fn struct_literal_writes_single_byte_array() {
        let rt = run(vec![
            struct_def("Point", vec![("x", "i64"), ("y", "i64")]),
            main_fn(vec![], Some(point_literal())),
        ]);
        assert!(matches!(rt.value, Value::Struct { offset: 0, .. }));
        let expected: Vec<u8> = [10i64.to_le_bytes(), 20i64.to_le_bytes()].concat();
        assert_eq!(rt.memory.as_bytes(), expected.as_slice());
        assert_eq!(rt.format_value(&rt.value), "Point { x: 10, y: 20 }");
    }

    #[test]
    fn field_access_reads_at_offset() {
        let items = vec![
            struct_def("Point", vec![("x", "i64"), ("y", "i64")]),
            main_fn(vec![], Some(field_access(point_literal(), "x"))),
        ];
        assert_eq!(run(items).value, Value::Int(10));
        let y = vec![
            struct_def("Point", vec![("x", "i64"), ("y", "i64")]),
            main_fn(vec![], Some(field_access(point_literal(), "y"))),
        ];
        assert_eq!(run(y).value, Value::Int(20));
    }

    #[test]
    fn field_read_uses_declared_byte_size() {
        let items = vec![
            struct_def("Pair", vec![("a", "i8"), ("b", "i8")]),
            main_fn(vec![], Some(field_access(
                struct_literal("Pair", vec![("a", int_lit(1)), ("b", int_lit(2))]),
                "b",
            ))),
        ];
        let rt = run(items);
        assert_eq!(rt.memory.as_bytes(), &[1u8, 2u8]);
        assert_eq!(rt.value, Value::Int(2));
    }

    /// resolve가 필드 순번을 채운 상태를 흥내낸다 (fields 순서와 slots가 어긋나도 slot이 기준).
    fn with_slots(mut lit: Expr, slots: &[u16]) -> Expr {
        if let ExprKind::StructLiteral { fields, .. } = &mut lit.kind {
            for (field, slot) in fields.iter_mut().zip(slots) {
                field.slot = Some(*slot);
            }
        }
        lit
    }

    #[test]
    fn prefilled_slot_wins_over_name() {
        // 이름은 "x"지만 slot이 1 → y(offset 8)를 읽는다.
        let base = with_slots(point_literal(), &[0, 1]);
        let items = vec![
            struct_def("Point", vec![("x", "i64"), ("y", "i64")]),
            main_fn(vec![], Some(field_access_at(base, "x", Some(1)))),
        ];
        assert_eq!(run_unresolved(items).unwrap().value, Value::Int(20));
    }

    #[test]
    fn prefilled_literal_slots_drive_offsets() {
        // 필드 나열은 (y, x)지만 slot이 (1, 0) → 메모리는 x@0, y@8.
        let lit = with_slots(
            struct_literal("Point", vec![("y", int_lit(20)), ("x", int_lit(10))]),
            &[1, 0],
        );
        let rt = run(vec![
            struct_def("Point", vec![("x", "i64"), ("y", "i64")]),
            main_fn(vec![], Some(lit)),
        ]);
        let expected: Vec<u8> = [10i64.to_le_bytes(), 20i64.to_le_bytes()].concat();
        assert_eq!(rt.memory.as_bytes(), expected.as_slice());
        assert_eq!(rt.format_value(&rt.value), "Point { x: 10, y: 20 }");
    }

    fn lt(l: Expr, r: Expr) -> Expr {
        Expr { kind: ExprKind::Binary { op: BinOp::Lt, lhs: Box::new(l), rhs: Box::new(r) }, span: Span::root(0, 0) }
    }

    fn call(name: &str, arg: Expr) -> Expr {
        let s = Span::root(0, 0);
        Expr { kind: ExprKind::Call { callee: Ident { name: name.into(), span: s }, args: vec![arg], fn_index: None }, span: s }
    }

    fn sub(l: Expr, r: Expr) -> Expr {
        Expr { kind: ExprKind::Binary { op: BinOp::Sub, lhs: Box::new(l), rhs: Box::new(r) }, span: Span::root(0, 0) }
    }

    /// `if cond { then } else { else_ }` — 블록은 tail 하나만.
    fn if_expr(cond: Expr, then_tail: Expr, else_tail: Expr) -> Expr {
        let s = Span::root(0, 0);
        let block = |tail: Expr| Block { stmts: vec![], tail: Some(tail), span: s };
        let kind = ExprKind::If {
            cond: Box::new(cond),
            then_block: Box::new(block(then_tail)),
            else_block: Some(Box::new(block(else_tail))),
        };
        Expr { kind, span: s }
    }

    /// `fn fib() { let n = 0; if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }`
    /// — 인자 바인딩이 선두 let을 대체하는 현재 방식을 그대로 쓴다.
    fn fib_fn() -> Item {
        let s = Span::root(0, 0);
        let rec = |n: i64| call("fib", sub(var("n"), int_lit(n)));
        let tail = if_expr(lt(var("n"), int_lit(2)), var("n"), add(rec(1), rec(2)));
        let body = Block { stmts: vec![let_stmt("n", int_lit(0))], tail: Some(tail), span: s };
        Item {
            name: Ident { name: "fib".into(), span: s },
            kind: ItemKind::Fn(FnItem { body, locals: 0, span: s }),
            span: s,
        }
    }

    #[test]
    fn eval_if_else_and_lt() {
        let taken = if_expr(lt(int_lit(1), int_lit(2)), int_lit(10), int_lit(20));
        assert_eq!(run(vec![main_fn(vec![], Some(taken))]).value, Value::Int(10));
        let other = if_expr(lt(int_lit(2), int_lit(1)), int_lit(10), int_lit(20));
        assert_eq!(run(vec![main_fn(vec![], Some(other))]).value, Value::Int(20));
    }

    #[test]
    fn eval_recursive_fib() {
        let items = vec![fib_fn(), main_fn(vec![], Some(call("fib", int_lit(10))))];
        assert_eq!(run(items).value, Value::Int(55));
    }

    #[test]
    fn eval_let_shadows_previous_binding() {
        // 같은 이름 재바인딩은 이전 값을 덮어쓴다 (프레임 = slot index 배열).
        let items = vec![main_fn(
            vec![let_stmt("x", int_lit(1)), let_stmt("x", add(var("x"), int_lit(41)))],
            Some(var("x")),
        )];
        assert_eq!(run(items).value, Value::Int(42));
    }

    #[test]
    fn eval_same_fn_called_twice() {
        // Rc로 공유한 FnItem을 두 번 호출해도 결과가 같아야 한다 (호출이 AST를 변형하지 않음).
        let s = Span::root(0, 0);
        let inc = Item {
            name: Ident { name: "inc".into(), span: s },
            kind: ItemKind::Fn(FnItem {
                body: Block {
                    stmts: vec![let_stmt("n", int_lit(0))],
                    tail: Some(add(var("n"), int_lit(1))),
                    span: s,
                },
                locals: 0,
                span: s,
            }),
            span: s,
        };
        let items = vec![
            inc,
            main_fn(vec![], Some(add(call("inc", int_lit(1)), call("inc", int_lit(2))))),
        ];
        assert_eq!(run(items).value, Value::Int(5));
    }

    #[test]
    fn unknown_field_is_rejected() {
        let items = vec![
            struct_def("Point", vec![("x", "i64")]),
            main_fn(vec![], Some(field_access(struct_literal("Point", vec![("x", int_lit(1))]), "z"))),
        ];
        let krate = Crate { items, span: Span::root(0, 0) };
        assert!(matches!(eval_crate(&krate), Err(EvalError::NotImplemented(_))));
    }
}
