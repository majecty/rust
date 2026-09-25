# HIR (desugar된 AST + 이름 해석)

## 1. 완료 범위
- crate: `toy/hir` (`rtoy-hir`) — HIR 데이터 + `lower(&Crate) -> Result<HirCrate, HirLowerError>` 한 파일
- 대응 rustc: `compiler/rustc_ast_lowering` (AST → HIR) + `compiler/rustc_hir/src/hir.rs`
- 실행: `./run --hir [--sample struct]` (resolve까지 마친 AST를 넣는다)

## 2. AST와 다른 점 (모두 `lower`가 만든다)
| 항목 | AST | HIR | rustc 대응 |
|---|---|---|---|
| 매크로 | `ExprKind::Macro` | 없음 (남아 있으면 `MacroNotExpanded`) | expand가 HIR 전에 끝난다 |
| 노드 id | 없음 | 전 노드 `HirId{owner, local}` | `hir::HirId` |
| 표현식 문장 | `StmtKind::Expr` (값 버림) | `StmtKind::Semi` | `hir::StmtKind::Semi` |
| fn 최상위 `let` | `Stmt` | `Body::params` (+ `Local::init` = 기본값) | toy 인자 관례(eval `bind_args`/MIR prologue)와 동일 |
| 이름 참조 | `Var.slot`/`Call.fn_index` | `ExprKind::Var{local}` / `Callee::User(FnId)` | `Res::Local` / `Res::Def` |
| 구조체 리터럴 | 이름 + 필드 | `Struct{struct_id, (순번, 값)}` | `ExprKind::Struct(&QPath, fields)` |
| 필드 접근 | `FieldAccess{field, slot}` | `Field{base, name}` (이름만) | `ExprKind::Field(base, Ident)` — 순번은 THIR이 정한다 |

- fn 본문의 블록/`if` 분기는 `ExprKind::Block{stmts, tail}` 하나로 표현한다 (rustc `hir::Block{stmts, expr}`).
- `if`의 else는 rustc처럼 `Option`으로 남긴다 (없으면 THIR이 `()`를 요구).

## 3. 덤프 (`--hir`, `samples/struct.rs`)
```
// hir — macro 없음 · Semi 명시 · fn 최상위 let = param · `#n`=HirId
struct Point { x: i64, y: i64 } // #0
fn main(p: ? = Point { 1, 2 }) { // #1
    p#0.x + p#0.y
}
```
- `#n` = `HirId.local`, 뒤 주석 `#0` = `OwnerId`(아이템 순번).
- `p: ? = ...` — toy에는 fn 파라미터 문법이 없어 타입이 항상 비어 있다(`?`). 기본값이 곧 toy의 "인자 기본값".

## 4. 미구현 · 편차
- rustc HIR의 attributes/arena(`owner`별 node map)/`visit` 없음. 트리를 그대로 소유한다(`Box`/`Vec`).
- `Param`에 기본값이 있는 건 toy 편차 (rustc `Param`에는 없다).
- HIR은 타입을 추론하지 않는다 — 선언된 타입 이름만 `TyKind`로 옮긴다(모르는 이름은 `Unknown`).
- resolve를 거치지 않은 AST도 slot을 이름으로 되찾아 lowering한다(테스트가 이 경로를 쓴다).
