# 현황 스냅샷 (2026-09-22)

## 1. 워크스페이스
- members 8개: `toy/ast`, `toy/driver`, `toy/eval`, `toy/expand`, `toy/lexer`, `toy/resolve`, `toy/span`, `toy/tokenstream-lowering`
- 상위: `~/code/rust` workspace `resolver = "2"`
- 실행 스크립트: `toy/run` (+ `run.ts`)
- 파이프라인: 읽기 → lex → lowering → expand → resolve → eval (기본 `value:` 출력)

## 2. crate별 상태
(검증: `cd toy && cargo test -p <crate> --lib`, 총 47개 통과)

- `rtoy-lexer`: char_indices tokenize (Ident/Int/Punct/Whitespace/Comment) · 테스트 1
- `rtoy-span`: `Span{lo,hi,ctxt,parent}` + `SyntaxContext/ExpnId/ExpnData` + `root/new/chain/same_var` · 테스트 5
- `rtoy-ast`: Crate/Item/Fn/Block/Stmt(`Expr`/`Let`)/Expr(`Int`/`Var`/`Call`/`Macro`/`StructLiteral`/`FieldAccess`/`If`)/`MacroDef`/`StructItem` · 테스트 1
- `rtoy-tokenstream-lowering`: tokens → Crate (`fn`/`struct`/`let`/이항연산/`if`+`<`/매크로 호출) · 테스트 8
- `rtoy-expand`: `macro_rules!` 패턴매칭·전개 + `twice!`/`def_fn!` · 테스트 11
- `rtoy-resolve`: 중복 fn 검사(미전개 Macro 제외) + 필드 참조 `ident → slot` 제자리 변형(`&mut Crate`) · 테스트 5
- `rtoy-eval`: 단일 byte array 메모리 인터프리터 — `Memory(Vec<u8>)` + `StructLayout{size, slots}`, `Value::Struct`는 offset 핸들, 필드는 byte offset으로 읽고 씀. resolve가 채운 slot 우선(미해결만 이름 fallback), `if`/`<`/재귀 지원 · 테스트 16
- `rtoy-driver`: `--lex`/`--ast`/`--trace` + eval 배선(`Runtime::format_value`)

## 3. 최근 커밋
- `736cced7bca` toy: struct item, literal, field access + eval
- `5184c3ee50c` toy: macro_rules! expansion + eval crate
- `2f0ed1e6619` toy: expand def_fn! item macro with dup-fn sample
- `a5bbd2c0170` toy: split driver into args/pipeline/diagnostics/trace modules

## 4. 알려진 틈
- 블록주석 미지원, 타입은 식별자 1개만, `snippet` 범위검사 없음
- eval: 구조체 중첩 필드 미지원, 인자를 `let` 순서로 유추, 변수 조회가 이름 HashMap이라 느림(fib(25) 650ms vs CPython 24ms)
- resolve 후 AST가 변형된 상태라 `--ast`는 post-resolve 트리(`slot: Some(n)` 노출)
- 전개 생성 토큰의 `parent/ctxt` 배선(`chain` 역추적) 미완
