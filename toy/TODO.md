# rtoy 진입점

> rustc를 축소한 학습용 컴파일러. 상세는 아래 문서로 분리됨.

## 1. 바로 실행
- `cd toy && ./run [--lex|--ast|--trace] [file.rs]` (기본 입력 `toy/test.rs`)
- `--lex`: 토큰만 출력 · `--ast`: AST만 출력 (기본은 main을 eval 실행)
- `--trace`: lex→lowering(before)→expand(after)→final 한 줄 요약 출력
- `./run --sample <이름>` (`ok/dup-fn/dup-fn-macro/twice/struct`, `-h`에 목록)
- 파이프라인: 읽기 → lex → lowering → expand → resolve → eval (main 실행, `value:` 출력)

## 2. 문서 목차
- [현황 스냅샷](docs/status.md) — 워크스페이스·커밋·crate별 상태
- [Lexer](docs/lexer.md) — rustc_lexer 대비 분석
- [AST](docs/ast.md) — 노드 정의·Span
- [Lowering](docs/lowering.md) — tokenstream → AST 파서
- [로드맵](docs/roadmap.md) — 다음 단계·이슈·위키

## 3. 현재 상태 요약
- 워크스페이스 members에 toy 8개 crate 등록 (ast/driver/eval/expand/lexer/span/tokenstream-lowering/resolve)
- span: `Span{lo,hi,ctxt,parent}` + `SyntaxContext/ExpnId/ExpnData` + `root/copied_arg/fresh_child/chain/same_var` + 테스트 5개
- lexer: char_indices tokenize + `Comment`(`//`~개행전) + 테스트 1개 (`lexer→ast` 역전 해소)
- ast: Crate/Item/Fn/Block/Stmt(`Expr`/`Let`)/`LetStmt`/Expr(`Int`/`Var{name,slot}`/`Call`/`Macro`/`StructLiteral`/`FieldAccess`/`If`)/이항연산(`+ - * / %`·`<`)/`MacroDef`/`StructItem`/`FieldInit` + 테스트 1개
- expand: `lift/expand(twice/my_let)` + `expand_expr/expand_item/expand_crate(twice!/def_fn!)` + `macro_rules!` 패턴매칭/전개 + 테스트 11개
- eval: 단일 byte array 메모리 인터프리터 — `Memory(Vec<u8>)` + `StructLayout{size, slots}` + `Value::Struct` offset 핸들, **resolve가 채운 slot 우선**(미해결은 이름 fallback) + **변수 프레임 `Frame=Vec<Value>`**(resolve가 채운 `Var/LetStmt.slot`만 사용, 이름표 없음) + `FnItem.locals`(프레임 크기) + `Rc<FnItem>` 공유 + `if`/`<`/재귀 호출 + 테스트 19개
- samples: `twice.rs` · `dup-fn-macro.rs` (`def_fn!(foo)+fn foo` 중복 재현) · `struct.rs` (정의/리터럴/필드) · driver `--trace` 한 줄 요약
- lowering: `fn name() { stmt* tail? }` + `if cond { } else { }` + `<` 비교 + 아이템 매크로 `def_fn!(foo)` + `struct`/리터럴/`p.x` 파서 + `if` 조건 no-struct-literal 제한 + 테스트 9개
- resolve: 중복 fn 검사(미전개 Macro 제외) + **미정의 변수 에러**(`ResolveKind::UndefinedVar`, driver E0425) + **필드 참조 ident→slot**(`FieldAccess/FieldInit.slot`) + **지역변수 slot 배정 + `FnItem.locals`**(shadowing은 같은 slot 재사용) + 테스트 9개
- driver: lex → lowering → expand → resolve → eval (기본 `value:` 출력, `--ast`면 AST 덤프)
- bench: `toy/bench/fib-bench.ts` — python/node/perl/lua/luajit/c/rust/rtoy fib 비교(미설치·빌드 실패는 SKIP, 컴파일 시간 제외, rtoy도 release) · min/median/max + warmup 1회 + `--md`/`--json=<path>`. fib(25) rtoy 22ms ≈ CPython의 1.3배
- 테스트 총 55개 통과 (span 5/lexer 1/ast 1/lowering 9/resolve 9/expand 11/eval 19) · struct + slot 변형 + `if`/재귀(fib 실행 가능) + 프레임 섀도잉/재호출
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
- [ ] Eval 성능 3: 호출 프레임 `Vec<Value>` 재사용(호출 깊이별 스택 + base offset) — 호출당 할당 1회 제거
- [ ] Eval 성능 4: callee를 fn index로 resolve가 심기 — 호출당 `HashMap<String,_>` 해시 제거
- [x] Resolve 엄격화: 미정의 변수는 resolve 에러(`ResolveKind`, driver E0425) · eval은 resolve된 slot만 사용(이름 fallback 제거) · resolve 미경유 AST는 `EvalError::UnresolvedLocal`로 거부 — eval 테스트도 실제 resolve를 거치게 함
- [ ] Resolve: 선언 전 사용(`x; let x = 1;`)이 조용히 `()`가 되는 구멍 — 선언 순서 추적 또는 uninit 센티넬
- [x] Lowering: `if` 조건 no-struct-literal 제한 (rustc 패리티) — `if x { .. }`가 struct literal로 오파싱되던 버그 수정 + 테스트 1개
- [ ] Lowering: `fn` 파라미터 AST 도입해 `let` 자리 관례 제거
- [ ] Resolve 진단: 매크로 생성 이름의 `expanded from def_fn! here` note + snippet이 호출문 전체를 보여줄지 결정
- [x] Span/docs: `docs/status.md` 8-crate 현행화 (2026-09-22)
- [ ] Driver: `--ast`가 resolve 이후(변형된) 트리임을 표기하고 `--ast-raw` 분리 고민
- [ ] Resolve: 제어흐름(`if`·재귀) 도입 시 지역변수 타입 추적을 블록/분기 인식으로 확장
- [ ] HIR 스텁: AST → 간단 HIR ([상세](docs/roadmap.md))
- [ ] Eval: 구조체 중첩 필드 (layout 안에 하위 layout offset)
- [x] Bench: `toy/bench/fib-bench.ts` — 재귀 fib로 python/node/perl/lua/luajit/c/rust/rtoy 비교 (ruby/php/go는 미설치면 SKIP)
- [x] Bench: 통계 보강 — min/median/max + warmup 1회 + 검증값 첫 run 고정 + `--md`/`--json=<path>` (rtoy도 runs 3으로 통일)
- [ ] 위키 정리: 단계마다 `[[rust-*]]` 페이지 ([[rust-rtoy-eval]]은 갱신 완료)
- [ ] 테스트: crate별 파서 케이스 보강

## 5. 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
