# rtoy 진행 상황

> rustc를 축소한 학습용 컴파일러. driver/lexer/ast/lowering 얼개 완료, 다음 단계는 표현식 확장.

## 현재 상태 (2026-03-05)
- 워크스페이스: `~/code/rust` members에 `toy/ast`, `toy/driver`, `toy/lexer`, `toy/tokenstream-lowering` 등록됨
- `rtoy-lexer`: char_indices 기반 tokenize (Ident/Int/Whitespace/Punct) + 테스트 1개
- `rtoy-ast`: Span/Crate/Item/ItemKind::Fn/Block/Stmt/Expr 얼개 + `dummy_crate()` + 테스트 1개
- `rtoy-tokenstream-lowering`: tokens → Crate pull 파서 (`fn name() { int }`) + 테스트 1개
- `rtoy-driver`: 파일 읽기 → lex → lowering 호출 → AST 출력 (`{:#?}` debug print)
- 실행: `cd toy && ./run [file.rs]` (기본 입력 `toy/test.rs`)
- 커밋: `7a85bfa`(TODO 갱신) ← `6324147`(ast 얼개) ← `3e9399b`(TODO 보강) ← `fbf5a62`(TODO 추가)
- rustc 대응물 학습 기록은 위키 참조 (`Cursor::advance_token` 등 lexer 비교 분석 포함)

## rustc_lexer 대비 (2025-09-05 조사)
- rustc는 `Cursor::advance_token()` pull 방식, Token은 `{kind, len}`만 가짐(위치는 파서가 추적)
- 우리는 `Vec<Token>` push 방식 — 주석/리터럴/raw-ident 확장 시 pull 전환 검토
- 미구현: LineComment/BlockComment, 키워드 판별, 숫자 접미사·float, r#".."#, shebang, DocStyle(`///`)
- 참고: `compiler/rustc_lexer/src/lib.rs:59`(Token), `:533`(advance_token)

## Known issues
- 시스템 cargo 없음 → `./run` 전에 `~/.cargo/bin` PATH 필요 (이제 설치됨, `.bashrc` 반영)
- 다른 머신에서 target/ 권한 충돌 → `sudo chown -R` 또는 `CARGO_TARGET_DIR` 우회

## 다음 단계 (우선순위 순)
- [ ] rustc_lexer 대비 보강: 주석(LineComment/BlockComment)을 토큰으로, pull 방식 Cursor로 전환 검토
- [x] AST 노드 정의 (`rustc_ast::token/ast.rs` 대비, 최소: Crate/Item/Fn/Block/Expr) — 얼개만, Let/Var/Call은 TODO
- [x] tokenstream-lowering crate (`toy/tokenstream-lowering`) — `fn name() { int }` pull 방식 파서
- [x] Span 타입 도입 (start/end → rustc_span 대비) — `rtoy-ast::Span{start,end}` 최소형
- [ ] 확장: Let/Var/Call 표현, 블록 내 다중 stmt, 이항 연산
- [ ] ast_lowering 스텁 — AST → 간단 HIR (driver 내부 또는 `toy/hir`)
- [ ] 위키 연동: 완성된 단계마다 `[[rust-*]]` 페이지로 정리

## 관련 위키
- [[rust-expand]] · [[rust-tokenstream]] · [[rust-some-trait-lowering]] · [[rust-stable_hash]]
