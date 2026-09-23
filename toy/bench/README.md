# bench — 언어별 재귀 fib 비교

rtoy가 다른 언어 대비 어느 정도인지 보는 용도. AST 워킹 인터프리터라
fib(25)에서 CPython과 비슷하거나 약간 빠르다(0.85~1.0배, 원인: AST 노드 순회 + 값마다 move).

## 실행
```sh
ts_run toy/bench/fib-bench.ts 25 3      # 또는: node toy/bench/fib-bench.ts 25 3
```
- ARGS: `[n=25] [runs=3] [--md] [--json=<path>]` — warmup 1회를 버리고 runs회 측정해 **min/median/max**(ms)를 낸다.
  검증값(기대값 대조)은 첫 측정 run에서 고정하며, 다르면 `MISMATCH`로 표시.
- `--md`면 markdown 표를, `--json=<path>`면 결과 JSON을 추가로 출력한다.
- 컴파일 언어(c/rust/go)는 빌드 시간을 실행 시간에서 제외한다.
- lua/luajit 바이너리가 없으면 `$TMPDIR/luasrc`에 소스 빌드한다(root 불필요, 네트워크 필요).
- ruby/php/go가 없으면 `SKIP`으로 표시되고 나머지만 측정한다.
- **rtoy도 release로 빌드한다** — c는 `-O2`, rust는 `-O`, go는 기본 최적화라 조건을 맞춤.
  (debug로 빌드하면 같은 코드가 648ms로 4배 넘게 느리게 나온다.)

## 측정 결과 (fib(25), min, release 프로필)
`--md`로 이 표를 재생성한다. min은 노이즈에 강하지만 median/max와 함께 볼 것.
| c | rust | luajit | lua | rtoy | python | node | perl |
|---|---|---|---|---|---|---|---|
| 1.2 | 2.2 | 2.7 | 5.7 | **14.8** | 16.8 | 27.7 | 33.7 |

node는 실행마다 편차가 크다(측정 중 18.9~33.3ms). rtoy는 14.0~14.8ms로 안정적이다.

컨테이너 부하에 따라 수치가 흔들린다(rtoy 배수는 1.2~2x 범위). 상대 비교용으로만 볼 것.

### 참고: rtoy 개선 이력 (fib(25), 같은 소스)
| 단계 | debug | release |
|---|---|---|
| 초기 (debug 빌드로 측정) | 648 ms | 139 ms |
| release 빌드로 측정 조건 통일 | 648 ms | 160 ms |
| eval: locals HashMap → 선형 스캔 `Vec`, `FnItem` clone → `Rc` | 171 ms | 26 ms |
| eval: slot `Vec<Value>` 프레임 (성능 2) | - | 22 ms |
| eval: 호출 프레임 arena 재사용 `Frame{base,size}` (성능 3) | - | 19 ms |
| eval: `Call.fn_index`로 함수 테이블 조회 (성능 4) | - | 15 ms |

초기(2026-09-22) 27배 수치는 debug 빌드 결과였고, 위 표는 최적화 후 재측정 값이다.

## rtoy 쪽 주의
- argv/stdin이 없어 `n`을 소스에 박는다(`{N}` 치환). 소스는 `$TMPDIR/fibbench/fib.rtoy.rs`.
- 파라미터 AST가 없어 인자는 선두 `let` 자리 관례를 쓴다: `fn fib() { let n = 0; if n < 2 { n } else { ... } }`.
- 드라이버 바이너리는 `$TMPDIR/rtoy-bench/release/rtoy-driver`로 빌드해 직접 실행한다(`toy/run`은 매번 cargo를 거친다).
