# 로드맵·이슈·위키

## 1. 우선순위 로드맵
- [ ] Lexer 보강: 주석 토큰 + pull Cursor 전환 검토
- [ ] AST 확장: Let/Var/Call + 이항 연산
- [ ] Lowering 확장: 다중 stmt 파싱
- [x] MIR 스텁: AST → basic block CFG MIR (`toy/mir` 신설, HIR 대신 rustc mir 쪽으로 구체화)
- [ ] MIR opt: temp/copy 제거·copy propagation (naive MIR이 AST eval보다 느림)
- [x] Crate 분리: `toy/mir` = MIR 데이터만, `toy/ast-lowering` = AST → MIR lowering
- [x] HIR 스텁: `toy/hir` = AST → HIR (macro 제거·HirId·Semi·최상위 let→param) + `--hir`
- [x] THIR 스텁: `toy/thir` = HIR → THIR arena + 최소 typeck(반환타입 fixpoint) + `--thir`
- [ ] MIR를 THIR에서 내리기 (rustc 경로 HIR → THIR → MIR)
- [ ] typeck 확장: 불리언 타입·`Infer` 잔존 진단·구조체 반환 추론
- [ ] 위키 연동: 단계 완료마다 `[[rust-*]]` 정리
- [ ] 테스트 보강: crate별 케이스 추가

## 2. 알려진 이슈
- 시스템 cargo PATH 필요 (`~/.cargo/bin`, `.bashrc` 반영됨)
- 타 머신 target/ 권한 충돌 → `sudo chown -R` 또는 `CARGO_TARGET_DIR` 우회
- rustc 학습 기록은 위키 참조 (`Cursor::advance_token` 비교 포함)

## 3. 관련 위키
- [[rust-expand]]
- [[rust-tokenstream]]
- [[rust-some-trait-lowering]]
- [[rust-stable_hash]]
- [[rust-rtoy-mir]] (MIR 데이터 + lowering(`rtoy-ast-lowering`) · eval)
- [[rust-rtoy-hir]] · [[rust-rtoy-thir]] (desugar/arena + 최소 typeck)
