# AST 노드 정의

## 1. 완료 범위
- Span 최소형: `rtoy-ast::Span{start, end}` (rustc_span 대비)
- 노드: Crate / Item / ItemKind::Fn / Block / Stmt / Expr
- 헬퍼: `dummy_crate()` + 테스트 1개
- 대비: `rustc_ast::token/ast.rs` 최소 부분집합

## 2. 미구현 (확장 대상)
- Let 바인딩
- 변수 참조 (Var)
- 함수 호출 (Call)
- 이항 연산
- 다중 stmt 블록 (현재 단일 패스)
