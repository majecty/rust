# bench — 언어별 재귀 fib 비교

rtoy가 다른 언어 대비 어느 정도인지 보는 용도. AST 워킹 인터프리터라
fib(25)에서 CPython의 약 27배 느리다(원인: 이름 HashMap 변수 조회 + 호출마다 `FnItem` clone).

## 실행
```sh
ts_run toy/bench/fib-bench.ts 25 3      # 또는: node toy/bench/fib-bench.ts 25 3
```
- ARGS: `[n=25] [runs=3]` — 최소값(ms) 기준, 출력 값이 기대값과 다르면 `MISMATCH`로 표시.
- 컴파일 언어(c/rust/go)는 빌드 시간을 실행 시간에서 제외한다.
- lua/luajit 바이너리가 없으면 `$TMPDIR/luasrc`에 소스 빌드한다(root 불필요, 네트워크 필요).
- ruby/php/go가 없으면 `SKIP`으로 표시되고 나머지만 측정한다.

## 측정 결과 (2026-09-22, fib(25), 최소값)
| c | rust | luajit | lua | node | python | perl | rtoy |
|---|---|---|---|---|---|---|---|
| 1.7 | 1.7 | 2.8 | 6.1 | 17.4 | 24.4 | 34.0 | **649.8** |

컨테이너 부하에 따라 수치가 흔들린다(rtoy 배수는 27~50x 범위). 상대 비교용으로만 볼 것.

## rtoy 쪽 주의
- argv/stdin이 없어 `n`을 소스에 박는다(`{N}` 치환). 소스는 `$TMPDIR/fibbench/fib.rtoy.rs`.
- 파라미터 AST가 없어 인자는 선두 `let` 자리 관례를 쓴다: `fn fib() { let n = 0; if n < 2 { n } else { ... } }`.
- 드라이버 바이너리는 `$TMPDIR/rtoy-bench/debug/rtoy-driver`로 빌드해 직접 실행한다(`toy/run`은 매번 cargo를 거친다).
