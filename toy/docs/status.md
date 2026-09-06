# 현황 스냅샷 (2026-09-06)

## 1. 워크스페이스
- members: `toy/ast`, `toy/driver`, `toy/lexer`, `toy/span`, `toy/tokenstream-lowering` (5개)
- 상위: `~/code/rust` workspace `resolver = "2"`
- 실행 스크립트: `toy/run` (+ `run.ts`)

## 2. crate별 상태
- `rtoy-lexer`: char_indices 기반 tokenize (Ident/Int/Whitespace/Punct/Comment) + 테스트 1개
- `rtoy-ast`: Span/Crate/Item/ItemKind::Fn/Block/Stmt(Expr/Let)/LetStmt/Expr + `dummy_crate()` + 테스트 1개
- `rtoy-tokenstream-lowering`: tokens → Crate pull 파서 (`let`·빈몸·trivia) + 테스트 2개
- `rtoy-driver`: 파일 읽기 → lex → lowering → AST 출력 (`test.rs` 통과)

## 3. 커밋 히스토리
- `7a85bfa` TODO 갱신
- `6324147` ast 얼개
- `3e9399b` TODO 보강
- `fbf5a62` TODO 추가
