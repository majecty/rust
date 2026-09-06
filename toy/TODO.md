# rtoy 진입점

> rustc를 축소한 학습용 컴파일러. 상세는 아래 문서로 분리됨.

## 1. 바로 실행
- `cd toy && ./run [--lex|--ast] [file.rs]` (기본 입력 `toy/test.rs`)
- `--lex`: 토큰만 출력 · `--ast`: AST 출력 (기본값과 동일, 명시용)
- 파이프라인: 파일 읽기 → lex → lowering → AST `{:#?}` 출력

## 2. 문서 목차
- [현황 스냅샷](docs/status.md) — 워크스페이스·커밋·crate별 상태
- [Lexer](docs/lexer.md) — rustc_lexer 대비 분석
- [AST](docs/ast.md) — 노드 정의·Span
- [Lowering](docs/lowering.md) — tokenstream → AST 파서
- [로드맵](docs/roadmap.md) — 다음 단계·이슈·위키

## 3. 현재 상태 요약
- 워크스페이스 members에 toy 5개 crate 등록 (ast/driver/lexer/span/tokenstream-lowering)
- span: `rtoy-span::Span{start,end}` + `new/dummy/snippet` + 테스트 1개 (`snippet_roundtrip`)
- lexer: char_indices tokenize + `Comment`(`//`~개행전) + 테스트 1개 (`lexer→ast` 역전 해소)
- ast: Crate/Item/Fn/Block/Stmt(`Expr`/`Let`)/`LetStmt{name,ty,init}`/Expr + 테스트 1개
- lowering: `fn name() { stmt* tail? }` pull 파서 + 테스트 2개 (`let name[: ty] = int;`·빈몸 `{}`·`skip_trivia` 지원)
- driver: lex → lowering → AST 출력 동작 (`test.rs` `let x: u32 = 42;` 통과)
- Span: lexer→span→ast→lowering 배선 완료
  - `Token.span`, `Item.span`, `Block.span`, `Expr.span` 필드 유지
  - lowering: Item/Block/Expr span 생성 및 연결
  - `spans_cover_source` 테스트 통과 (총 5개 통과)
- 알려진 틈: Var/Call/이항연산 미지원·블록주석 미지원·타입은 식별자 1개만·`snippet` 범위검사 없음
- 상세 수치는 [현황](docs/status.md) 참조

## 4. 다음 할 일
- [x] Lexer: `//` 라인 주석 토큰 (`Comment`) ([상세](docs/lexer.md))
- [x] AST: Let 바인딩 (`LetStmt`) ([상세](docs/ast.md))
- [x] Lowering: 다중 stmt·빈몸 `{}`·`skip_trivia` ([상세](docs/lowering.md))
- [ ] Span 확장: `Ident{name,span}` → Item/Let 이름에 적용
- [ ] Span 확장: `Ty{kind,span}` → Let ty에 적용
- [ ] Span 확장: `Stmt{kind,span}` + `StmtKind` (rustc 패리티)
- [ ] Span 확장: `Crate/FnItem` 전체 span
- [ ] Lexer: 블록주석·pull Cursor 전환 ([상세](docs/lexer.md))
- [ ] AST: Var/Call 확장 ([상세](docs/ast.md))
- [ ] Lowering: 이항 연산 ([상세](docs/lowering.md))
- [ ] Span/docs: `snippet` 범위검사·`docs/status.md` 5-crate 동기화
- [ ] HIR 스텁: AST → 간단 HIR ([상세](docs/roadmap.md))
- [ ] 위키 정리: 단계마다 `[[rust-*]]` 페이지
- [ ] 테스트: crate별 파서 케이스 보강

## 5. 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
