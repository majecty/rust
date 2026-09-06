# rtoy 진입점

> rustc를 축소한 학습용 컴파일러. 상세는 아래 문서로 분리됨.

## 1. 바로 실행
- `cd toy && ./run [file.rs]` (기본 입력 `toy/test.rs`)
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
- lexer: char_indices tokenize 얼개 + 테스트 1개 (Span 수입원 `rtoy-span`, `lexer→ast` 역전 해소)
- ast: Crate/Item/Fn/Block/Stmt/Expr 얼개 + 테스트 1개 (`Span`은 `rtoy-span` 재수출)
- lowering: `fn name() { int }` pull 파서 + 테스트 2개 (직접 `rtoy-span` 의존)
- driver: lex → lowering → AST 출력 동작
- Span: lexer→span→ast→lowering 배선 완료
  - `Token.span`, `Item.span`, `Block.span`, `Expr.span` 필드 유지
  - lowering: Item/Block/Expr span 생성 및 연결
  - `spans_cover_source` 테스트 통과 (총 5개 통과)
- 알려진 틈: 빈몸 `{}` 파서 실패·`snippet` 범위검사 없음·`test.rs`는 현 문법 밖이라 driver `items: []`
- 상세 수치는 [현황](docs/status.md) 참조

## 4. 다음 할 일
- [ ] Lexer: 주석 토큰·pull Cursor 전환 ([상세](docs/lexer.md))
- [ ] AST: Let/Var/Call 확장 ([상세](docs/ast.md))
- [ ] Lowering: 다중 stmt·이항 연산·빈몸 `{}` 복구 ([상세](docs/lowering.md))
- [ ] Span/docs: `snippet` 범위검사·`docs/status.md` 5-crate 동기화
- [ ] HIR 스텁: AST → 간단 HIR ([상세](docs/roadmap.md))
- [ ] 위키 정리: 단계마다 `[[rust-*]]` 페이지
- [ ] 테스트: crate별 파서 케이스 보강

## 5. 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
