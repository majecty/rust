# rtoy 진행 상황

> rustc를 축소한 학습용 컴파일러. driver/lexer 분리 완료, 다음 단계는 parser.

## 현재 상태 (2025-09-05)
- 워크스페이스: `~/code/rust` members에 `toy/driver`, `toy/lexer` 등록됨
- `rtoy-lexer`: char_indices 기반 tokenize (Ident/Int/Whitespace/Punct) + 테스트 1개
- `rtoy-driver`: 파일 읽기 → lex → 토큰 10개 출력 (parse/lowering은 stub)
- 실행: `cd toy && ./run [file.rs]` (기본 입력 `toy/test.rs`)
- 커밋: `80b62ab` Split toy into rtoy-driver and rtoy-lexer crates

## Known issues
- 시스템 cargo 없음 → `PATH=~/code/rust/build/aarch64-unknown-linux-gnu/stage0/bin:$PATH`
- 다른 머신에서 target/ 권한 충돌 → `sudo chown -R` 또는 `CARGO_TARGET_DIR` 우회

## 다음 단계
- [ ] AST 노드 정의 (`rustc_ast::AstToken` 대비, 최소: Crate/Item/Fn/Block/Expr)
- [ ] parser crate 분리 (`toy/parser`) — token stream → AST, rustc_parse 대비
- [ ] ast_lowering 스텁 — AST → 간단 HIR (driver 내부 또는 `toy/lowering`)
- [ ] Span 타입 도입 (start/end → rustc_span 대비)
- [ ] 위키 연동: 완성된 단계마다 `[[rust-*]]` 페이지로 정리

## 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
