//! rtoy eval — 미니 AST 인터프리터.
//! fn main() { let x = 1 + 2; x + 3 } → 6

use rtoy_ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    UndefinedVar(String),
    NotAFunction(String),
    ArgCountMismatch { expected: usize, got: usize },
    EmptyMain,
    NotImplemented(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::UndefinedVar(n) => write!(f, "undefined variable: {}", n),
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

/// 환경: 함수 정의 + 구조체 정의 저장소.
struct Env {
    fns: HashMap<String, FnItem>,
    structs: HashMap<String, StructItem>,
}

/// 값 타입.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Struct { name: String, fields: HashMap<String, Value> },
    Unit,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Unit => write!(f, "()"),
            Value::Struct { name, fields } => {
                let mut keys: Vec<&String> = fields.keys().collect();
                keys.sort();
                let body: Vec<String> = keys.iter().map(|k| format!("{k}: {}", fields[*k])).collect();
                write!(f, "{name} {{ {} }}", body.join(", "))
            }
        }
    }
}

/// Crate를 실행하여 main의 반환값을 계산.
pub fn eval_crate(krate: &Crate) -> Result<Value, EvalError> {
    let mut env = Env { fns: HashMap::new(), structs: HashMap::new() };

    // 모든 아이템을 env에 등록
    for item in &krate.items {
        match &item.kind {
            ItemKind::Fn(f) => {
                env.fns.insert(item.name.name.clone(), f.clone());
            }
            ItemKind::Struct(s) => {
                env.structs.insert(item.name.name.clone(), s.clone());
            }
            ItemKind::MacroDef(_) => {} // 매크로 정의는 무시
            ItemKind::Macro { .. } => {} // 미전개 매크로는 무시
        }
    }

    // main 실행 (body를 복제하여 borrow 충돌 회피)
    let main_body = env.fns.get("main")
        .ok_or(EvalError::NotAFunction("main".into()))?
        .body.clone();
    eval_block(&main_body, &mut env, &mut HashMap::new())
}

/// 블록 실행.
fn eval_block(block: &Block, env: &mut Env, var_env: &mut HashMap<String, Value>) -> Result<Value, EvalError> {
    for stmt in &block.stmts {
        match &stmt.kind {
            StmtKind::Let(let_stmt) => {
                let val = if let Some(init) = &let_stmt.init {
                    eval_expr(init, env, var_env)?
                } else {
                    Value::Int(0)
                };
                var_env.insert(let_stmt.name.name.clone(), val);
            }
            StmtKind::Expr(expr) => {
                eval_expr(expr, env, var_env)?;
            }
        }
    }

    if let Some(tail) = &block.tail {
        eval_expr(tail, env, var_env)
    } else {
        Ok(Value::Unit)
    }
}

/// 식 계산.
fn eval_expr(expr: &Expr, env: &mut Env, var_env: &mut HashMap<String, Value>) -> Result<Value, EvalError> {
    match &expr.kind {
        ExprKind::Int(n) => Ok(Value::Int(*n)),

        ExprKind::Var(ident) => {
            var_env.get(&ident.name)
                .cloned()
                .ok_or_else(|| EvalError::UndefinedVar(ident.name.clone()))
        }

        ExprKind::Binary { op, lhs, rhs } => {
            let l = eval_expr(lhs, env, var_env)?;
            let r = eval_expr(rhs, env, var_env)?;
            match (op, &l, &r) {
                (BinOp::Add, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (BinOp::Sub, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (BinOp::Mul, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (BinOp::Div, Value::Int(a), Value::Int(b)) => {
                    if *b == 0 { return Err(EvalError::NotImplemented("division by zero".into())); }
                    Ok(Value::Int(a / b))
                }
                (BinOp::Mod, Value::Int(a), Value::Int(b)) => {
                    if *b == 0 { return Err(EvalError::NotImplemented("mod by zero".into())); }
                    Ok(Value::Int(a % b))
                }
                _ => Err(EvalError::NotImplemented(format!("binary {:?} on non-int", op))),
            }
        }

        ExprKind::StructLiteral { name, fields } => {
            let _ = env.structs.get(&name.name)
                .ok_or_else(|| EvalError::NotImplemented(format!("unknown struct {}", name.name)))?;
            let mut field_map = HashMap::new();
            for f in fields {
                let val = eval_expr(&f.value, env, var_env)?;
                field_map.insert(f.name.name.clone(), val);
            }
            Ok(Value::Struct { name: name.name.clone(), fields: field_map })
        }

        ExprKind::FieldAccess { base, field } => {
            let val = eval_expr(base, env, var_env)?;
            match val {
                Value::Struct { name: _, fields } => {
                    fields.get(&field.name)
                        .cloned()
                        .ok_or_else(|| EvalError::NotImplemented(format!("unknown field {}", field.name)))
                }
                _ => Err(EvalError::NotImplemented("field access on non-struct".into())),
            }
        }

        ExprKind::Call { callee, args } => {
            // 내장 함수 처리
            if callee.name == "print" {
                let vals: Result<Vec<_>, _> = args.iter().map(|a| eval_expr(a, env, var_env)).collect();
                let vals = vals?;
                for v in &vals {
                    match v {
                        Value::Int(n) => print!("{} ", n),
                        Value::Struct { name, .. } => print!("{} {{...}} ", name),
                        Value::Unit => print!("() "),
                    }
                }
                println!();
                return Ok(Value::Unit);
            }

            // 사용자 정의 함수
            let f = env.fns.get(&callee.name)
                .ok_or_else(|| EvalError::NotAFunction(callee.name.clone()))?
                .clone();

            if args.len() != f.body.stmts.iter().filter(|s| matches!(s.kind, StmtKind::Let(_))).count() {
                // 인자 수 = let 문 수 (간이 카운트)
                // 더 정확하게는 파라미터 수가 필요하지만 현재 AST에 파라미터 없음
            }

            let mut local_vars = HashMap::new();
            let mut arg_idx = 0;
            for stmt in &f.body.stmts {
                if let StmtKind::Let(let_stmt) = &stmt.kind {
                    let val = if arg_idx < args.len() {
                        eval_expr(&args[arg_idx], env, var_env)?
                    } else if let Some(init) = &let_stmt.init {
                        eval_expr(init, env, &mut local_vars)?
                    } else {
                        Value::Int(0)
                    };
                    local_vars.insert(let_stmt.name.name.clone(), val);
                    arg_idx += 1;
                }
            }

            // 본문 실행 (let 문은 위에서 처리했으므로 expr만)
            for stmt in &f.body.stmts {
                if let StmtKind::Expr(expr) = &stmt.kind {
                    eval_expr(expr, env, &mut local_vars)?;
                }
            }

            if let Some(tail) = &f.body.tail {
                eval_expr(tail, env, &mut local_vars)
            } else {
                Ok(Value::Unit)
            }
        }

        ExprKind::Macro { .. } => Err(EvalError::NotImplemented("macro call (not expanded)".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtoy_span::Span;

    fn int_lit(n: i64) -> Expr {
        Expr { kind: ExprKind::Int(n), span: Span::root(0, 0) }
    }

    fn var(name: &str) -> Expr {
        Expr { kind: ExprKind::Var(Ident { name: name.into(), span: Span::root(0, 0) }), span: Span::root(0, 0) }
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
                span: Span::root(0, 0),
            }),
            span: Span::root(0, 0),
        }
    }

    #[test]
    fn eval_int_literal() {
        let krate = Crate { items: vec![main_fn(vec![], Some(int_lit(42)))], span: Span::root(0, 0) };
        assert_eq!(eval_crate(&krate).unwrap(), Value::Int(42));
    }

    #[test]
    fn eval_add() {
        let krate = Crate { items: vec![main_fn(vec![], Some(add(int_lit(1), int_lit(2))))], span: Span::root(0, 0) };
        assert_eq!(eval_crate(&krate).unwrap(), Value::Int(3));
    }

    #[test]
    fn eval_let_and_var() {
        let krate = Crate {
            items: vec![main_fn(
                vec![let_stmt("x", int_lit(10))],
                Some(add(var("x"), int_lit(5))),
            )],
            span: Span::root(0, 0),
        };
        assert_eq!(eval_crate(&krate).unwrap(), Value::Int(15));
    }

    #[test]
    fn eval_nested_let() {
        let krate = Crate {
            items: vec![main_fn(
                vec![
                    let_stmt("a", int_lit(3)),
                    let_stmt("b", add(var("a"), int_lit(7))),
                ],
                Some(var("b")),
            )],
            span: Span::root(0, 0),
        };
        assert_eq!(eval_crate(&krate).unwrap(), Value::Int(10));
    }

    #[test]
    fn eval_sub_mul() {
        let krate = Crate {
            items: vec![main_fn(
                vec![],
                Some(Expr {
                    kind: ExprKind::Binary {
                        op: BinOp::Mul,
                        lhs: Box::new(int_lit(3)),
                        rhs: Box::new(add(int_lit(1), int_lit(2))),
                    },
                    span: Span::root(0, 0),
                }),
            )],
            span: Span::root(0, 0),
        };
        assert_eq!(eval_crate(&krate).unwrap(), Value::Int(9));
    }

    #[test]
    fn eval_undefined_var() {
        let krate = Crate { items: vec![main_fn(vec![], Some(var("x")))], span: Span::root(0, 0) };
        assert!(matches!(eval_crate(&krate), Err(EvalError::UndefinedVar(_))));
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
                    span: s,
                }).collect(),
            },
            span: s,
        }
    }

    fn field_access(base: Expr, field: &str) -> Expr {
        let s = Span::root(0, 0);
        Expr {
            kind: ExprKind::FieldAccess {
                base: Box::new(base),
                field: Ident { name: field.into(), span: s },
            },
            span: s,
        }
    }

    #[test]
    fn eval_struct_literal() {
        let krate = Crate {
            items: vec![
                struct_def("Point", vec![("x", "i64"), ("y", "i64")]),
                main_fn(vec![], Some(struct_literal("Point", vec![
                    ("x", int_lit(1)),
                    ("y", int_lit(2)),
                ]))),
            ],
            span: Span::root(0, 0),
        };
        let val = eval_crate(&krate).unwrap();
        match val {
            Value::Struct { name, fields } => {
                assert_eq!(name, "Point");
                assert_eq!(fields["x"], Value::Int(1));
                assert_eq!(fields["y"], Value::Int(2));
            }
            _ => panic!("expected struct value"),
        }
    }

    #[test]
    fn eval_field_access() {
        let krate = Crate {
            items: vec![
                struct_def("Point", vec![("x", "i64"), ("y", "i64")]),
                main_fn(vec![], Some(field_access(
                    struct_literal("Point", vec![
                        ("x", int_lit(10)),
                        ("y", int_lit(20)),
                    ]),
                    "x",
                ))),
            ],
            span: Span::root(0, 0),
        };
        assert_eq!(eval_crate(&krate).unwrap(), Value::Int(10));
    }
}
