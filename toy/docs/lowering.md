# Lowering — tokenstream → AST

## 1. 현재 파서
- 위치: `toy/tokenstream-lowering`
- 방식: pull 파서, 입력 tokens → Crate
- 문법: `fn name() { int }` 단일 형태만
- 테스트 1개 통과 중

## 2. 확장 대상
- 블록 내 다중 stmt
- Let/Var/Call 표현
- 이항 연산자
- 에러 복구 (현재 panic 수준)
- rustc 대응: `rustc_expand` / tokenstream 변환 대비 정리
