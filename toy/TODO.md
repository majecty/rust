# rtoy 진입점

> rustc를 축소한 학습용 컴파일러. 상세는 아래 문서로 분리됨.

## 1. 바로 실행
- `cd toy && ./run [--lex|--ast|--trace] [file.rs]` (기본 입력 `toy/test.rs`)
- `--lex`: 토큰만 출력 · `--ast`: AST 출력 (기본값과 동일, 명시용)
- `--trace`: lex→lowering(before)→expand(after)→final 한 줄 요약 출력
- `./run --sample <이름>` (`ok/dup-fn/twice`, `-h`에 목록)
- 파이프라인: 읽기 → lex → lowering → expand → resolve → AST 출력

## 2. 문서 목차
- [현황 스냅샷](docs/status.md) — 워크스페이스·커밋·crate별 상태
- [Lexer](docs/lexer.md) — rustc_lexer 대비 분석
- [AST](docs/ast.md) — 노드 정의·Span
- [Lowering](docs/lowering.md) — tokenstream → AST 파서
- [로드맵](docs/roadmap.md) — 다음 단계·이슈·위키

## 3. 현재 상태 요약
- 워크스페이스 members에 toy 7개 crate 등록 (ast/driver/expand/lexer/span/tokenstream-lowering/resolve)
- span: `Span{lo,hi,ctxt,parent}` + `SyntaxContext/ExpnId/ExpnData` + `root/copied_arg/fresh_child/chain/same_var` + 테스트 5개
- lexer: char_indices tokenize + `Comment`(`//`~개행전) + 테스트 1개 (`lexer→ast` 역전 해소)
- ast: Crate/Item/Fn/Block/Stmt(`Expr`/`Let`)/`LetStmt{name,ty,init}`/Expr(`Int`/`Var`/`Call`/`Macro`)/이항연산 + 테스트 1개
- expand: `lift/expand(twice/my_let)` + `expand_expr/expand_crate(twice!(x)->x+x)` + 테스트 3개
- samples: `twice.rs` (`let y=1; twice!(y)` → tail `Binary(Add)`) · driver `--trace` 한 줄 요약
- lowering: `fn name() { stmt* tail? }` pull 파서 + 테스트 4개 (`let`·빈몸·`skip_trivia`·이항연산 우선순위·Var/Call)
- resolve: 중복 fn 이름 검사 + 테스트 2개
- driver: lex → lowering → expand → resolve → AST 출력 (`--sample twice` 통과, trace시 최종 덤프 생략)
- 테스트 총 16개 통과 (span 5/expand 3/ast 1/lexer 1/lowering 4/resolve 2) · 커밋 `60f46d5` `994c6af` `84eebf4`
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
- [ ] Expand: `my_let!` AST 전개 + 위생 테스트
- [ ] Span/docs: `docs/status.md` 7-crate 동기화
- [ ] HIR 스텁: AST → 간단 HIR ([상세](docs/roadmap.md))
- [ ] 위키 정리: 단계마다 `[[rust-*]]` 페이지
- [ ] 테스트: crate별 파서 케이스 보강

## 5. 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
