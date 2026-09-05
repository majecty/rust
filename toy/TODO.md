# rtoy 진행 상황

> rustc를 축소한 학습용 컴파일러. driver/lexer 분리 완료, 다음 단계는 parser.

## 현재 상태 (2025-09-05)
- 워크스페이스: `~/code/rust` members에 `toy/driver`, `toy/lexer` 등록됨
- `rtoy-lexer`: char_indices 기반 tokenize (Ident/Int/Whitespace/Punct) + 테스트 1개
- `rtoy-driver`: 파일 읽기 → lex → 토큰 10개 출력 (parse/lowering은 stub)
- 실행: `cd toy && ./run [file.rs]` (기본 입력 `toy/test.rs`)
- 커밋: `fbf5a62`(TODO 추가) ← `80b62ab`(driver/lexer 분리) ← `544912c`(toy 워크스페이스 편입)
- rustc 대응물 학습 기록은 위키 참조 (`Cursor::advance_token` 등 lexer 비교 분석 포함)

## rustc_lexer 대비 (2025-09-05 조사)
- rustc는 `Cursor::advance_token()` pull 방식, Token은 `{kind, len}`만 가짐(위치는 파서가 추적)
- 우리는 `Vec<Token>` push 방식 — 주석/리터럴/raw-ident 확장 시 pull 전환 검토
- 미구현: LineComment/BlockComment, 키워드 판별, 숫자 접미사·float, r#".."#, shebang, DocStyle(`///`)
- 참고: `compiler/rustc_lexer/src/lib.rs:59`(Token), `:533`(advance_token)

## Known issues
- 시스템 cargo 없음 → ./run 전에 `PATH=~/code/rust/build/aarch64-unknown-linux-gnu/stage0/bin:$PATH` 필요 (run.ts가 `cargo`를 PATH에서 찾기 때문)
- 다른 머신에서 target/ 권한 충돌 → `sudo chown -R` 또는 `CARGO_TARGET_DIR` 우회

## 다음 단계 (우선순위 순)
- [ ] rustc_lexer 대비 보강: 주석(LineComment/BlockComment)을 토큰으로, pull 방식 Cursor로 전환 검토
- [ ] AST 노드 정의 (`rustc_ast::token/ast.rs` 대비, 최소: Crate/Item/Fn/Block/Expr)
- [ ] parser crate 분리 (`toy/parser`) — token stream → AST, rustc_parse 대비
- [ ] ast_lowering 스텁 — AST → 간단 HIR (driver 내부 또는 `toy/lowering`)
- [ ] Span 타입 도입 (start/end → rustc_span 대비)
- [ ] 위키 연동: 완성된 단계마다 `[[rust-*]]` 페이지로 정리

## 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
