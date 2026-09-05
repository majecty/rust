# 로드맵·이슈·위키

## 1. 우선순위 로드맵
- [ ] Lexer 보강: 주석 토큰 + pull Cursor 전환 검토
- [ ] AST 확장: Let/Var/Call + 이항 연산
- [ ] Lowering 확장: 다중 stmt 파싱
- [ ] HIR 스텁: AST → 간단 HIR (`toy/hir` 신설 또는 driver 내부)
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
