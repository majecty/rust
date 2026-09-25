# rtoy 진입점

> rustc를 축소한 학습용 컴파일러. 상세는 아래 문서로 분리됨.

## 1. 바로 실행
- `cd toy && ./run [--lex|--ast|--hir|--thir|--trace] [file.rs]` (기본 입력 `toy/test.rs`)
- `--lex`: 토큰만 출력 · `--ast`: AST만 · `--hir`: HIR(desugar+이름해석) · `--thir`: THIR(arena+타입검사) (기본은 main을 eval 실행)
- `--trace`: lex→lowering(before)→expand(after)→final 한 줄 요약 출력
- `./run --sample <이름>` (`ok/dup-fn/dup-fn-macro/twice/struct`, `-h`에 목록)
- 파이프라인: 읽기 → lex → lowering → expand → resolve → eval (main 실행, `value:` 출력)
  - 덤프 경로: `--ast`(resolve 후 AST) · `--hir`(desugar+HirId) · `--thir`(arena+타입검사)

## 2. 문서 목차
- [현황 스냅샷](docs/status.md) — 워크스페이스·커밋·crate별 상태
- [Lexer](docs/lexer.md) — rustc_lexer 대비 분석
- [AST](docs/ast.md) — 노드 정의·Span
- [Lowering](docs/lowering.md) — tokenstream → AST 파서
- [HIR](docs/hir.md) — desugar·HirId·최상위 let→param (rustc_ast_lowering 대응)
- [THIR](docs/thir.md) — arena·타입확정·최소 typeck (rustc thir 대응)
- [로드맵](docs/roadmap.md) — 다음 단계·이슈·위키

## 3. 현재 상태 요약
- 워크스페이스 members에 toy 12개 crate 등록 (ast/ast-lowering/driver/eval/expand/hir/lexer/mir/resolve/span/thir/tokenstream-lowering)
- span: `Span{lo,hi,ctxt,parent}` + `SyntaxContext/ExpnId/ExpnData` + `root/copied_arg/fresh_child/chain/same_var` + 테스트 5개
- lexer: char_indices tokenize + `Comment`(`//`~개행전) + 테스트 1개 (`lexer→ast` 역전 해소)
- ast: Crate/Item/Fn/Block/Stmt(`Expr`/`Let`)/`LetStmt`/Expr(`Int`/`Var{name,slot}`/`Call`/`Macro`/`StructLiteral`/`FieldAccess`/`If`)/이항연산(`+ - * / %`·`<`)/`MacroDef`/`StructItem`/`FieldInit` + 테스트 1개
- expand: `lift/expand(twice/my_let)` + `expand_expr/expand_item/expand_crate(twice!/def_fn!)` + `macro_rules!` 패턴매칭/전개 + 테스트 11개
- eval: 단일 byte array 메모리 인터프리터 — `Memory(Vec<u8>)` + `StructLayout{size, slots}` + `Value::Struct` offset 핸들, **resolve가 채운 slot 우선**(미해결은 이름 fallback) + **프레임은 arena `Vec<Value>` 위 `Frame{base,size}` 창**(resolve가 채운 `Var/LetStmt.slot`만 사용, 호출마다 창 재사용·할당 없음) + `FnItem.locals`(프레임 크기) + `Rc<FnItem>` 공유 + **`Call.fn_index`로 함수 테이블 직접 조회**(미해결은 이름 fallback) + `if`/`<`/재귀 호출 + 테스트 19개
- samples: `twice.rs` · `dup-fn-macro.rs` (`def_fn!(foo)+fn foo` 중복 재현) · `struct.rs` (정의/리터럴/필드) · driver `--trace` 한 줄 요약
- lowering: `fn name() { stmt* tail? }` + `if cond { } else { }` + `<` 비교 + 아이템 매크로 `def_fn!(foo)` + `struct`/리터럴/`p.x` 파서 + `if` 조건 no-struct-literal 제한 + 테스트 9개
- resolve: 중복 fn 검사(미전개 Macro 제외) + **미정의 변수 에러**(`ResolveKind::UndefinedVar`, driver E0425) + **필드 참조 ident→slot**(`FieldAccess/FieldInit.slot`) + **지역변수 slot 배정 + `FnItem.locals`**(shadowing은 같은 slot 재사용) + **`Call.fn_index` 함수 테이블 번호**(eval 이름 해시 제거) + 테스트 10개
- hir: `toy/hir` — HIR 데이터 + AST → HIR lowering: macro 제거(미전개면 `HirLowerError::MacroNotExpanded`) · 전 노드 `HirId{owner,local}` · expr stmt → `StmtKind::Semi` · **fn 최상위 `let` → `Body::params`**(초기화식=기본값) · 참조를 `FnId`/`StructId`/`LocalId`로 해석 · 필드는 이름만(순번은 THIR) · `dump()`(desugar된 소스) + 테스트 5개
- thir: `toy/thir` — THIR 데이터(arena `exprs`/`blocks`/`stmts`/`params`) + HIR → THIR lowering + **toy 첫 typeck**(`Ty::{I64,Unit,Struct,Infer}`, 반환타입 fixpoint, `ThirLowerError` 4종) + `dump()`(arena+타입) + 테스트 9개
- mir: `toy/mir` — MIR 데이터(rustc `rustc_middle/mir` 부분집합): `Body{locals, arg_locals, entry, blocks}` + `BasicBlock{stmts, terminator}` + `Place{local, proj}`/`Rvalue`/`Operand` + `TerminatorKind{Goto/SwitchInt/Call/Return}` + `dump()`/`stmt_str`/`term_str` + 테스트 1개
- ast-lowering: `toy/ast-lowering` — AST(resolve 후) → MIR lowering(struct/fn 표, `if`→`SwitchInt`, 최상위 `let`→prologue 블록, `MirLowerError`) + 테스트 9개
- mir-eval: `rtoy-eval::eval_mir` — MIR CFG를 선형 스캔(AST eval의 arena/`Frame`/`Value`/`Memory` 재사용) + `--mir-eval` + AST eval과 교차검증(samples/fib/print/에러샘플 동일). **최적화 패스가 없어 naive MIR은 AST eval보다 느림** (fib(25) MIR 29ms vs AST 13~16ms, §5 로그) — mir-opt가 다음 단계
- mir-steps(웹 스테퍼): `eval_mir_traced(mir, budget)` — `budget` 스텝만 실행하고 실행 경로(`StepEvent`)·현재 위치(`Cursor`=fn/bb/지역변수)를 남긴다 (기존 `eval_mir`는 budget 없이 이 함수의 얇은 래퍼, 기록 오버헤드 0). driver `--mir-steps=N`이 `== trace ==/== cursor ==/== value ==` 마커로 출력
- web: `toy/web/serve.ts` + `index.html` — 소스→HIR/THIR/MIR 단계 선택 뷰 + **한 줄씩 실행**. `POST /api/compile?steps=N`이 드라이버 `--mir-steps=N` 출력을 파싱해 trace/cursor/value를 JSON으로 반환, 페이지가 현재 MIR 줄을 하이라이트하고 실행 경로 클릭=그 스텝까지 재생. **매 요청 처음부터 재실행하는 리플레이 방식**이라 큰 프로그램엔 느림 (2단계=콜스택 명시화)
- driver: lex → lowering → expand → resolve → {hir/thir 덤프 | eval} (기본 `value:` 출력) · `--ast`/`--hir`/`--thir`/`--mir`/`--mir-eval`/`--mir-steps=N` — MIR lowering은 `rtoy-ast-lowering`
- bench: `toy/bench/fib-bench.ts` — python/node/perl/lua/luajit/c/rust/rtoy/rtoy-mir fib 비교(미설치·빌드 실패는 SKIP, 컴파일 시간 제외, rtoy도 release) · min/median/max + warmup 1회 + `--md`/`--json=<path>`. fib(25) rtoy 15ms ≈ CPython의 0.9배, rtoy-mir 29ms (§5 성능 로그)
- 테스트 총 78개 통과 (span 5/lexer 1/ast 1/lowering 9/resolve 10/expand 11/hir 5/thir 9/mir 1/ast-lowering 9/eval 19) · struct + slot 변형 + `if`/재귀(fib 실행 가능) + 프레임 섀도잉/재호출
- Span: lexer→span→ast→lowering 배선 완료
  - `Token.span`, `Item.span`, `Block.span`, `Expr.span` 필드 유지
  - lowering: Item/Block/Expr span 생성 및 연결
  - `spans_cover_source` 테스트 통과 (총 9개 통과)
- 알려진 틈: 블록주석 미지원·타입은 식별자 1개만·`snippet` 범위검사 없음
- 상세 수치는 [현황](docs/status.md) 참조

## 4. 다음 할 일
- [x] Lexer: `//` 라인 주석 토큰 (`Comment`) ([상세](docs/lexer.md))
- [x] AST: Let 바인딩 (`LetStmt`) ([상세](docs/ast.md))
- [x] Lowering: 다중 stmt·빈몸 `{}`·`skip_trivia` ([상세](docs/lowering.md))
- [x] Span 확장: `Ident{name,span}` → Item/Let 이름에 적용
- [x] Span 확장: `Ty{kind,span}` → Let ty에 적용
- [x] Span 확장: `Stmt{kind,span}` + `StmtKind` (rustc 패리티)
- [x] Span 확장: `Crate/FnItem` 전체 span
- [ ] Lexer: 블록주석·pull Cursor 전환 ([상세](docs/lexer.md))
- [x] AST: Var/Call 확장 ([상세](docs/ast.md))
- [x] Lowering: 이항 연산 ([상세](docs/lowering.md))
- [x] Span: `Span{lo,hi,ctxt,parent}` 전면 개편 (호환심 제거, `root/new`만)
- [x] Expand: `twice!(x)->x+x` + `Macro` 노드 + driver 배선 + `samples/twice.rs`
- [x] Driver: `--trace` 단계별 한 줄 요약 (lex/before/after/final)
- [ ] Span 배선: 전개 생성토큰에 `parent/ctxt` 연결 (`chain` 역추적·`same_var` 위생)
- [x] Expand: `def_fn!(foo)->fn foo(){}` 아이템 매크로 + `expand_item` (중복 에러문구 고민용)
- [x] Expand: `macro_rules!` 미니 구현 — `$x:expr` 패턴매칭 + 템플릿전개 + `MacroDef`/`TokenTree`/`Vis` AST 노드 + 테스트 7개
- [ ] Expand: `$(...)*` 반복 패턴 지원
- [ ] Expand: `$x:ident`, `$x:ty` 등 fragment 종류 확장
- [x] Eval: 미니 인터프리터 — `fn main()` 실행 + `let`/`+`/`-`/`*`/`/`/`%` 지원 + 함수 호출 + 테스트 9개
- [x] AST: struct 아이템·리터럴·필드 접근 노드 (`StructItem`/`StructLiteral`/`FieldAccess`/`FieldInit`)
- [x] Lowering: `struct Name { field: Ty }` + `Name { f: v }` + `p.x` 파싱
- [x] Eval: struct 값·필드 접근 평가 (단일 byte array + `StructLayout` offset)
- [x] Resolve: 필드 참조 `ident → slot` 제자리 변형 (`FieldAccess/FieldInit.slot`, 미해결은 eval fallback)
- [x] Driver: eval 배선 (기본 `value:` 출력, `--ast`는 AST)
- [x] Samples: `struct.rs`
- [ ] Expand: `my_let!` AST 전개 + 위생 테스트
- [x] Eval: 재귀 호출 + `if`/`else` + `<` (인자 바인딩은 선두 `let` 자리 관례)
- [x] Eval 성능 1: 변수 프레임 `HashMap` → 선형 스캔 `Vec<(String,Value)>` + `Rc<FnItem>` 공유(호출마다 AST clone 제거) — fib(25) release 160ms → 26ms
- [x] Eval 성능 2: resolve가 지역변수 slot을 채워 `Vec<Value>` 인덱스 접근(이름 비교 제거) — `Frame=Vec<Value>` + `FnItem.locals`, fib(30) 215→176ms(-18%) · fib(25) rtoy 26.7→22ms
- [x] Eval 성능 3: 호출 프레임 `Vec<Value>` 재사용(호출 깊이별 스택 + base offset) — 호출당 할당 제거, fib(25) release 22.0→19.0ms (1.33→1.12x CPython)
- [x] Eval 성능 4: callee를 fn index로 resolve가 심기 — 호출당 `HashMap<String,_>` 해시 제거, fib(25) release 19.0→14.8ms (1.12→0.88x CPython)
- [x] Resolve 엄격화: 미정의 변수는 resolve 에러(`ResolveKind`, driver E0425) · eval은 resolve된 slot만 사용(이름 fallback 제거) · resolve 미경유 AST는 `EvalError::UnresolvedLocal`로 거부 — eval 테스트도 실제 resolve를 거치게 함
- [ ] Resolve: 선언 전 사용(`x; let x = 1;`)이 조용히 `()`가 되는 구멍 — 선언 순서 추적 또는 uninit 센티넬
- [x] Lowering: `if` 조건 no-struct-literal 제한 (rustc 패리티) — `if x { .. }`가 struct literal로 오파싱되던 버그 수정 + 테스트 1개
- [ ] Lowering: `fn` 파라미터 AST 도입해 `let` 자리 관례 제거
- [ ] Resolve 진단: 매크로 생성 이름의 `expanded from def_fn! here` note + snippet이 호출문 전체를 보여줄지 결정
- [x] Span/docs: `docs/status.md` 9-crate 현행화
- [ ] Driver: `--ast`가 resolve 이후(변형된) 트리임을 표기하고 `--ast-raw` 분리 고민
- [ ] Resolve: 제어흐름(`if`·재귀) 도입 시 지역변수 타입 추적을 블록/분기 인식으로 확장
- [x] Crate 분리: `toy/mir`(MIR 데이터) + `toy/ast-lowering`(AST → MIR lowering) — basic block CFG + `--mir` 덤프, mir 1개·ast-lowering 9개 테스트
- [x] MIR eval: `rtoy-eval::eval_mir` — CFG 선형 스캔(arena/`Frame` 재사용) + `--mir-eval` + AST eval과 교차검증(samples/fib/print/에러샘플)
- [ ] MIR opt: 불필요 temp/copy 제거·copy propagation·상수 폴딩 — (1) lowering 순수식 temp 제거까지 완료(fib(25) MIR eval 38.0→29.1ms), 상세는 §5 성능 로그
- [x] MIR 스텝 실행: `eval_mir_traced(budget)` + driver `--mir-steps=N` (실행 경로·현재 위치·지역변수 출력)
- [x] 웹 스테퍼(1단계·리플레이): `?steps=N` API + 페이지 스텝 버튼·MIR 줄 하이라이트·실행 경로 클릭 재생·지역변수
- [ ] 웹 스테퍼 2단계: `MirVm::exec` 재귀를 명시적 콜스택으로 바꿔 세션 상태 유지(재실행 없이 step/continue/중단점, 스텝 O(1))
- [ ] 웹: 소스 줄 하이라이트 (`Stmt.span.lo` → 줄 번호 매핑; 덤프에 span 주석 추가)
- [ ] MIR: `--mir` 덤프를 샘플별 스냅샷 테스트로 고정
- [x] HIR: `toy/hir` — AST → HIR lowering (macro 제거·HirId·Semi·최상위 let→param·id 해석) + `--hir` 덤프 + 테스트 5개
- [x] THIR: `toy/thir` — HIR → THIR arena + 최소 typeck(반환타입 fixpoint, 선언/초기화·인자 불일치 진단) + `--thir` 덤프 + 테스트 9개
- [ ] MIR: AST 대신 THIR에서 내리기 (rustc 경로: HIR → THIR → MIR)
- [ ] THIR: `Infer`가 남은 식 진단(E0282류) · 불리언 타입 도입해 `if` 조건을 bool로
- [ ] HIR/THIR: `--trace` 단계 요약에 hir/thir 한 줄 추가
- [ ] Eval: 구조체 중첩 필드 (layout 안에 하위 layout offset)
- [x] Bench: `toy/bench/fib-bench.ts` — 재귀 fib로 python/node/perl/lua/luajit/c/rust/rtoy 비교 (ruby/php/go는 미설치면 SKIP)
- [x] Bench: 통계 보강 — min/median/max + warmup 1회 + 검증값 첫 run 고정 + `--md`/`--json=<path>` (rtoy도 runs 3으로 통일)
- [ ] 위키 정리: 단계마다 `[[rust-*]]` 페이지 ([[rust-rtoy-eval]]은 갱신 완료)
- [ ] 테스트: crate별 파서 케이스 보강

## 5. MIR 성능 로그
> 측정: `ts_run /tmp/pi/mir-perf.ts 25` (release · fib(25) · min/median · runs 5 · AST min은 ±10% 노이즈). 고칠 때마다 한 줄 추가.

| 단계 | 변경 | AST eval (min) | MIR eval (min/median) | MIR/AST | MIR 규모 (fib 1콜) |
|---|---|---|---|---|---|
| 0 | baseline: naive MIR | 13.9 ms | 38.0 / 38.4 ms | 2.73x | stmt 14 · term 7 · locals 13 |
| 0b | 버그: trace-off(cursor 안 뽑을 때)에도 매 스텝 `stmt_str`/`term_str`로 String 생성 — 주석은 "오버헤드 0"인데 실제로는 스텝의 대부분 | 14.5 ms | 303 / 320 ms | 19x | `begin_step`이 text를 지연 생성(trace 켜졌을 때만)으로 수정 (`eval/src/mir.rs`) |
| 1 | lowering: 순수식(Int 리터럴·Var·FieldAccess)은 temp 없이 Operand 직접 — `_3 = copy _1; _2 = copy _3 < copy _4` → `_2 = copy _1 < const 2` | 13.2 ms | 29.1 / 29.2 ms | 1.9~2.2x | stmt 8 · term 7 · locals 7 (MIR eval -23%) |
| 2 | (예정) `mir/src/opt.rs` copy propagation + dead temp 제거 | | | | |
| 3 | (예정) VM 프리디코드 (CFG → 평탄 Instr) | | | | |

해석: 0b는 이 벤치 자체가 무효였던 케이스(수치가 아니라 버그). 1은 stmt를 절반으로 줄였지만 MIR eval은 23%만 줄였다 — 남은 2배 차이는 **step 수**(15 vs AST 노드 방문 ~7)와 step당 비용 둘 다이므로, 2(step 수)·3(step당 비용)을 함께 봐야 한다.

## 6. 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
