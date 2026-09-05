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
- 워크스페이스 members에 toy 4개 crate 등록
- lexer: char_indices tokenize 얼개 + 테스트 1개
- ast: Crate/Item/Fn/Block/Stmt/Expr 얼개 + 테스트 1개
- lowering: `fn name() { int }` pull 파서 + 테스트 1개
- driver: lex → lowering → AST 출력 동작
- Span 최소형 도입 완료
- 상세 수치는 [현황](docs/status.md) 참조

## 4. 다음 할 일
- [ ] Lexer: 주석 토큰·pull Cursor 전환 ([상세](docs/lexer.md))
- [ ] AST: Let/Var/Call 확장 ([상세](docs/ast.md))
- [ ] Lowering: 다중 stmt·이항 연산 ([상세](docs/lowering.md))
- [ ] HIR 스텁: AST → 간단 HIR ([상세](docs/roadmap.md))
- [ ] 위키 정리: 단계마다 `[[rust-*]]` 페이지
- [ ] 테스트: crate별 파서 케이스 보강

## 5. 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
