# 현황 스냅샷 (2026-09-22)

## 1. 워크스페이스
- members 12개: `toy/ast`, `toy/ast-lowering`, `toy/driver`, `toy/eval`, `toy/expand`, `toy/hir`, `toy/lexer`, `toy/mir`, `toy/resolve`, `toy/span`, `toy/thir`, `toy/tokenstream-lowering`
- 상위: `~/code/rust` workspace `resolver = "2"`
- 실행 스크립트: `toy/run` (+ `run.ts`)
- 파이프라인: 읽기 → lex → lowering → expand → resolve → eval (기본 `value:` 출력)
  - HIR/THIR 덤프: resolve → `--hir`(desugar+HirId) → `--thir`(arena+타입검사)
  - MIR 경로: resolve → `toy/ast-lowering`(AST → MIR) → `--mir` 덤프 또는 `--mir-eval`(MIR 인터프리터)

## 2. crate별 상태
(검증: `cd toy && cargo test -p <crate> --lib`, 총 78개 통과)

- `rtoy-lexer`: char_indices tokenize (Ident/Int/Punct/Whitespace/Comment) · 테스트 1
- `rtoy-span`: `Span{lo,hi,ctxt,parent}` + `SyntaxContext/ExpnId/ExpnData` + `root/new/chain/same_var` · 테스트 5
- `rtoy-ast`: Crate/Item/Fn/Block/Stmt(`Expr`/`Let`)/Expr(`Int`/`Var`/`Call`/`Macro`/`StructLiteral`/`FieldAccess`/`If`)/`MacroDef`/`StructItem` · 테스트 1
- `rtoy-tokenstream-lowering`: tokens → Crate (`fn`/`struct`/`let`/이항연산/`if`+`<`/매크로 호출, `if` 조건 no-struct-literal 제한) · 테스트 9
- `rtoy-expand`: `macro_rules!` 패턴매칭·전개 + `twice!`/`def_fn!` · 테스트 11
- `rtoy-resolve`: 중복 fn 검사(미전개 Macro 제외) + 필드 참조 `ident → slot` 제자리 변형(`&mut Crate`) + 지역변수 slot·`Call.fn_index` 배정 · 테스트 10
- `rtoy-hir`: HIR 데이터 + AST → HIR lowering(`lower`) — macro 제거(미전개면 `MacroNotExpanded`) · 전 노드 `HirId{owner,local}` · expr stmt → `StmtKind::Semi` · fn 최상위 `let` → `Body::params`(초기화식=기본값, toy 인자 관례) · 참조를 `FnId`/`StructId`/`LocalId`로 해석 · 필드는 이름만 남기고 순번은 THIR이 정한다 · `dump()`(desugar된 소스) · 테스트 5
- `rtoy-thir`: THIR 데이터 + HIR → THIR lowering + **toy 첫 typeck** — arena `exprs`/`blocks`/`stmts`/`params`, `Ty::{I64,Unit,Struct,Infer}`, 반환 타입 fixpoint(재귀 호출), `ThirLowerError`(MismatchedTypes/NotAStruct/UnknownField/WrongArgCount), `dump()`(arena+확정 타입) · 테스트 9
- `rtoy-mir`: MIR 데이터만 (rustc_middle/mir 부분집합) — `Body{locals, arg_locals, entry, blocks}`, `BasicBlock{stmts, terminator}`, `Place{local, proj}`/`Rvalue`/`Operand`, `TerminatorKind{Goto/SwitchInt/Call/Return}`, `StructDef`, `dump()`(rustc `-Zunpretty=mir` 흉내)·`stmt_str`/`term_str`(eval 트레이스 공용). `BinOp`만 `rtoy-ast` 것을 그대로 쓴다 · 테스트 1
- `rtoy-ast-lowering`: AST(resolve 후) → MIR lowering — struct/fn 표 수집, `if`→`SwitchInt`, 최상위 `let`→prologue 블록, `MirLowerError`(UnknownFn/UnknownStruct/UnknownField/UnknownLocal/Unsupported) · 테스트 9
- `rtoy-eval`: 단일 byte array 메모리 인터프리터 — `Memory(Vec<u8>)` + `StructLayout{size, slots}`, `Value::Struct`는 offset 핸들, 필드는 byte offset으로 읽고 씀. resolve가 채운 slot 우선(미해결만 이름 fallback), 프레임은 arena `Vec<Value>` 위 `Frame{base,size}` 창(호출마다 재사용·할당 없음), callee는 `Call.fn_index`로 직접 조회, `Rc<FnItem>` 공유(호출마다 AST clone 안 함), `if`/`<`/재귀 지원 · 테스트 19 (같은 arena/`Frame`/`Value`/`Memory`를 MIR 인터프리터 `eval_mir`와 공유)
- `rtoy-driver`: `--lex`/`--ast`/`--mir`/`--mir-eval`/`--trace` + eval 배선(`Runtime::format_value`) — MIR lowering은 `rtoy-ast-lowering`
- `toy/bench/fib-bench.ts`: 언어별 재귀 fib 비교(rtoy 포함, 컴파일 시간 제외, lua/luajit는 없으면 소스 빌드) + rtoy-mir(MIR eval) 컬럼

## 3. 최근 커밋
- `736cced7bca` toy: struct item, literal, field access + eval
- `5184c3ee50c` toy: macro_rules! expansion + eval crate
- `2f0ed1e6619` toy: expand def_fn! item macro with dup-fn sample
- `a5bbd2c0170` toy: split driver into args/pipeline/diagnostics/trace modules

## 4. 알려진 틈
- 블록주석 미지원, 타입은 식별자 1개만, `snippet` 범위검사 없음
- eval: 구조체 중첩 필드 미지원, 인자를 `let` 순서로 유추, MIR도 같은 관례(prologue 블록)
- MIR: 타입 추론 없음(리터럴 초기화만 Struct), unwind/StorageLive 없음, **mir-opt 패스 없음 → naive MIR이 AST eval보다 느림(fib(25) 37ms vs 16ms)**
- MIR: `Rvalue::BinaryOp`가 `rtoy_ast::BinOp`을 그대로 써서 `rtoy-mir`가 `rtoy-ast`에 묶여 있다 (IR 전용 enum 분리는 미완)
- resolve 후 AST가 변형된 상태라 `--ast`는 post-resolve 트리(`slot: Some(n)` 노출)
- THIR: `Infer`가 남아도 오류로 모으지 않는다(진단 없음), 불리언/참조/메서드 호출 없음 — MIR는 아직 THIR을 거치지 않는다
- 전개 생성 토큰의 `parent/ctxt` 배선(`chain` 역추적) 미완
