# THIR (타입이 붙은 HIR = toy 첫 typeck)

## 1. 완료 범위
- crate: `toy/thir` (`rtoy-thir`) — THIR 데이터 + `lower_crate(&HirCrate) -> Result<ThirCrate, Vec<ThirLowerError>>`
- 대응 rustc: `compiler/rustc_middle/src/thir.rs` (데이터) + `compiler/rustc_hir_analysis/src/thir/` (lowering/typeck)
- 실행: `./run --thir [--sample struct]`
- **toy에서 타입검사가 처음 들어오는 단계다.** rustc는 typeck 결과를 THIR에 채우지만, 여기서는 lowering이 직접 추론한다(최소형).

## 2. HIR과 다른 점
| 항목 | HIR | THIR | rustc 대응 |
|---|---|---|---|
| 표현식 | 트리 | arena `exprs: Vec<Expr>` + `ExprId` | `Thir.exprs: IndexVec<ExprId, Expr>` |
| 타입 | 선언 이름만 | 모든 식이 `Ty` 확정 (`Infer` = 아직 모름) | `Expr.ty` (typeck 결과) |
| 블록/문장 | `ExprKind::Block` | `blocks`/`stmts` + `ExprKind::Block{block}` | `thir::Block{stmts, expr}` |
| `let` | `StmtKind::Let(Local)` | `StmtKind::Let{local, ty, initializer: Option}` | `StmtKind::Let` |
| 필드 접근 | 이름만 | `struct_id` + 필드 순번까지 확정 | `Field{lhs, variant_index, name: FieldIdx}` |
| 파라미터 | `Body::params` | `Param{local, ty, default}` | `Param{ty, ...}` (`default`만 toy 편차) |

## 3. 타입 규칙 (최소형)
| 식 | 규칙 | 위반 시 |
|---|---|---|
| 정수 리터럴 | `i64` | — |
| `let`/파라미터 | 선언 타입, 없으면 초기화식 타입, 둘 다 없으면 `Infer` | 선언 ≠ 초기화 → `MismatchedTypes` |
| 이항 연산 | 양쪽 `i64` → `i64` | `binary op lhs/rhs` |
| `if` | 조건은 `i64`, else 없으면 then은 `()`이고 결과도 `()` | `if condition` / `if without else` |
| 두 분기 | 타입 통일 (한 쪽이 `Infer`면 다른 쪽을 따른다) | `if branches` |
| 호출 | 인자 개수·타입을 callee 파라미터와 맞춤, 결과는 callee 반환 타입 | `WrongArgCount` / `call argument` |
| `print` 내장 | 인자 `i64`, 결과 `()` | `print argument` |
| `p.x` | 기반이 `Struct`일 때만 필드 타입 | `NotAStruct` / `UnknownField` |
| 구조체 리터럴 | `Struct(id)`, 각 필드 값을 선언 타입과 맞춤 | `struct field` |

- 반환 타입은 fixpoint(`fns.len()+1`회 재-lowering)로 정한다 — `fib`가 `fib`를 부르므로 한 바퀴로는 모자란다.
  rustc는 typeck를 먼저 돌려 이 문제를 없앤다 (toy는 이 자리를 fixpoint로 대신한다).
- `Ty::Infer`는 오류가 아니라 "모름"이다. 남아 있으면 덤프에 `_`로 보이고 검사를 건너뛴다.

## 4. 덤프 (`--thir`, `samples/struct.rs`)
```
fn main -> i64 { // fn_id=0
    params: #0 p: Point = e2
    body: value=e8 ty=i64
    b0: stmts=[] expr=e7
    e0: i64 = Literal(1)
    e1: i64 = Literal(2)
    e2: Point = Adt(Point { f0:e0, f1:e1 })
    e3: Point = VarRef(#0 p)
    e4: i64 = Field(e3 .x 순번=0 struct=Point)
    e7: i64 = Binary(+ e4 e6)
    e8: i64 = Block(b0)
}
```
- 자식이 먼저 arena에 들어가고 부모 id가 뒤에 온다 (rustc THIR과 같은 순서).
- 타입 오류 예 (`fn main() { let x = 1; x.y }`):
  `error: thir lowering failed: expected struct in field access, found `i64` at [23..26]`

## 5. 미구현
- 불리언/참조/제네릭/트레이트/메서드 호출 없음 (`if` 조건은 `i64`, `print` 인자도 `i64`).
- `Infer`로 남은 식을 모아 보고하지 않는다 (rustc `E0282`류 진단 없음).
- MIR은 아직 THIR을 거치지 않는다(현재 `ast-lowering`이 AST → MIR). rustc 경로는 HIR → THIR → MIR이다.
