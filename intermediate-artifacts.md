# 테스트 코드 컴파일 결과를 중간 산출물로 직접 눈으로 확인하는 법

이 문서는 rustc 저장소에서 직접 빌드한(stage1) 컴파일러로 테스트 코드를 컴파일하고,
컴파일 파이프라인의 각 단계에서 만들어지는 **중간 산출물(AST / HIR / THIR / MIR / LLVM IR / 어셈블리 / 오브젝트)** 을
하나씩 꺼내서 눈으로 확인하는 방법을 정리한 것입니다.

> 모든 명령은 이 저장소의 stage1 컴파일러(`rustc 1.99.0-dev`, `aarch64-unknown-linux-gnu`)로
> 실제 실행해 검증했습니다. (2026-08-02 기준)

---

## 0. 전제: stage1 컴파일러 준비

stage1 rustc와 std를 만들고, `rustup` 커스텀 툴체인으로 등록합니다.

```bash
# 1) 컴파일러 + std 빌드 및 stage1 sysroot 조립
./x.py build --stage 1 library/std

# 2) rustup 툴체인 등록 (LD_LIBRARY_PATH 걱정 없이 바로 사용 가능)
rustup toolchain link stage1 build/aarch64-unknown-linux-gnu/stage1
```

이 문서에서는 아래 두 가지를 줄여 씁니다.

| 약칭 | 실제 값 |
|------|---------|
| `$R` | `rustup run stage1 rustc` (혹은 `build/aarch64-unknown-linux-gnu/stage1/bin/rustc`) |
| `$LLVM` | `build/aarch64-unknown-linux-gnu/stage1/lib/rustlib/aarch64-unknown-linux-gnu/bin` (opt, llc, llvm-dis, llvm-objdump, llvm-nm 등) |

직접 바이너리를 실행할 때는 `rustup run` 대신 `LD_LIBRARY_PATH=build/host/stage1/lib`를 걸어야 합니다.
`rustup run stage1`은 이걸 자동 처리하므로 아래 예시는 전부 `rustup run stage1`을 기준으로 합니다.

### (선택) doctest를 돌리려면 rustdoc이 필요

`cargo +stage1 test`에서 doctest를 실행하려면 커스텀 툴체인 안에 `rustdoc`이 있어야 합니다.
이미 빌드된 rustdoc 바이너리를 복사해 넣으면 됩니다.

```bash
cp build/host/stage1-tools-bin/rustdoc_tool_binary \
   build/aarch64-unknown-linux-gnu/stage1/bin/rustdoc
```

복사하지 않았다면 `cargo +stage1 test --lib`처럼 doctest를 제외하고 실행하면 됩니다.

---

## 1. 예시 테스트 소스

작은 프로젝트나 UI 테스트 파일을 그대로 사용합니다. 아래는 이 문서 전체에서 쓰는 예시입니다.

```rust
// t.rs
fn foo(x: i32) -> impl Iterator<Item = i32> {
    (0..x).map(|i| i * 2)
}

fn main() {
    let v: Vec<i32> = foo(5).collect();
    println!("{:?}", v);
}
```

또한 `tests/ui/` 아래 테스트 파일(`tests/ui/impl-trait/basic-trait-impl.rs` 같은 것)을
그대로 인자로 넘겨도 됩니다. `//@ ...`, `//~` 같은 지시어/주석은 일반 주석일 뿐이라
직접 컴파일하는 데 지장이 없습니다(7장 참고).

---

## 2. 프론트엔드 산출물: AST / HIR / THIR

컴파일 후반부로 갈수록 산출물이 "구체 타입/정의 위치"를 더 많이 담게 됩니다.

| 보고 싶은 것 | 명령 | 핵심 관찰점 |
|--------------|------|-------------|
| AST (트리 전체) | `$R -Zunpretty=ast-tree t.rs` | `TyKind::ImplTrait` 등 원래 문법 구조 |
| 매크로 확장 결과 | `$R -Zunpretty=expanded t.rs` | 매크로로 생성된 코드 |
| HIR (간결) | `$R -Zunpretty=hir t.rs` | `impl Trait`이 주석으로 표시된 반환 타입 |
| HIR (타입 주석) | `$R -Zunpretty=hir,typed t.rs` | 캐스트가 붙은 typed HIR |
| HIR (전체 트리) | `$R -Zunpretty=hir-tree t.rs` | `OpaqueDef`/`OpaqueTy` 노드 확인 |
| THIR (전체 트리) | `$R -Zunpretty=thir-tree t.rs` | 표현식별 타입, `Alias(Opaque ...)` 확인 |
| 최종 MIR (텍스트) | `$R -Zunpretty=mir t.rs` | 숨은 타입(hidden type)이 확정된 MIR |
| 최종 MIR (CFG dot) | `$R -Zunpretty=mir-cfg t.rs` | basic block 그래프(graphviz dot) |

`-Zunpretty`에 넣을 수 있는 값 전체 목록:

```
normal, expanded, expanded,identified, expanded,hygiene,
ast-tree, ast-tree,expanded,
hir, hir,identified, hir,typed, hir-tree,
thir-tree, thir-flat,
mir, stable-mir, mir-cfg
```

`impl Trait`(RPIT) 예시를 `-Zunpretty=hir`로 보면 반환 타입이 `/*impl Trait*/`로 남아 있습니다.

```
fn foo(x: i32) -> /*impl Trait*/ { Range { start: 0, end: x }.map(|i| i * 2) }
```

`-Zunpretty=hir-tree`에서는 opaque 타입 정의가 보입니다.

```
kind: OpaqueDef(
    OpaqueTy {
        hir_id: HirId(DefId(0:3 ~ t[909c]::foo).24),
        def_id: DefId(0:4 ~ t[909c]::foo::{opaque#0}),
        ...
```

`-Zunpretty=mir`에서는 숨은 타입이 구체적으로 드러납니다.

```
fn foo(_1: i32) -> Map<std::ops::Range<i32>, {closure@t.rs:2:16: 2:19}> {
    ...
    _0 = <std::ops::Range<i32> as Iterator>::map::<i32, {closure@t.rs:2:16: 2:19}>(
             move _2, const ZeroSized: {closure@t.rs:2:16: 2:19}) -> [return: bb1, unwind continue];
```

---

## 3. 타입 검사 / 단형화(monomorphization) 정보

| 명령 | 내용 |
|------|------|
| `$R -Zverbose -Zunpretty=thir-tree t.rs` | 타입을 축약 없이 (`'{erased}`, `Alias(Opaque ...)` 등) 출력 |
| `$R -Zprint-type-sizes t.rs -o /tmp/out.o` | 최종 레이아웃이 잡힌 타입들의 크기/정렬/필드 패딩 출력 |
| `$R -Zprint-mono-items t.rs -o /tmp/out.o` | 단형화된 인스턴스 목록 (`MONO_ITEM fn ...` 형태) |
| `$R -Zdump-mono-stats=<경로> t.rs` | 단형화 비용 통계를 `<경로>.mono_items.md`(markdown)로 저장 |
| `$R -Znext-solver=globally ...` | 새 trait solver를 전역 적용해 비교 (impl Trait 관련 차이 확인용) |

> `-o /tmp/out.o` 처럼 **실제 출력 경로**를 지정하세요. `-o /dev/null`로 두면
> `couldn't create a temp dir: Permission denied ... at path "/dev/..."` 오류가 납니다
> (덤프 자체는 만들어지지만 잡음이 생깁니다).

---

## 4. MIR pass별 덤프

`-Zdump-mir`는 최종 MIR이 아니라 **각 최적화/분석 pass가 실행될 때마다** MIR을
`mir_dump/` 디렉터리에 파일로 찍습니다.

```bash
# 전부 덤프 (파일이 수백 개 생김)
$R -Zdump-mir=all t.rs -o /tmp/out.o

# 함수명 + pass 조합으로 필터링
$R -Zdump-mir="foo & SimplifyCfg" t.rs -o /tmp/out.o

# 덤프 디렉터리 지정
$R -Zdump-mir=all -Zdump-mir-dir=/tmp/md t.rs -o /tmp/out.o
```

생성되는 파일 이름 형태:

```
t.foo.1-1-000.built.after.mir
t.foo.1-1-006.SimplifyCfg-initial.before.mir
t.foo.1-1-006.SimplifyCfg-initial.after.mir
t.foo-{closure#0}.2-1-004.SimplifyCfg-post-analysis.after.mir
```

- `1-1-000`은 `(promoted 번호)-(mir 최적화 레벨)-(pass 실행 순서)` 정도로 보면 됩니다.
- 같은 pass에 대해 `before.mir`(pass 적용 전)와 `after.mir`(적용 후)가 쌍으로 생겨
  diff로 pass 효과를 바로 비교할 수 있습니다.
- 필터 표현식: `함수명 & Pass이름`, `함수명 | 다른함수` 형태. 패턴은 부분 일치입니다.

관련 옵션들:

| 옵션 | 효과 |
|------|------|
| `-Zdump-mir-graphviz` | 각 pass마다 `.dot`(CFG 그래프)도 함께 생성 |
| `-Zdump-mir-dataflow` | 데이터플로우 결과가 담긴 `.dot` 생성 (borrowck 등 분석 보기용) |
| `-Zdump-mir-exclude-alloc_bytes` | raw allocation 바이트 생략 (테스트 표준 출력용) |
| `-Zdump-mir-exclude-pass-number` | 파일명에서 pass 번호 생략 |

`.dot`는 graphviz(`dot`)가 설치돼 있으면 PNG/SVG로 렌더링해 볼 수 있습니다
(이 머신에는 `dot` 미설치 상태).

---

## 5. 백엔드 산출물: MIR / LLVM IR / 어셈블리 / 오브젝트

`--emit`으로 백엔드 각 단계의 산출물을 직접 파일로 뽑습니다.

```bash
$R --emit=mir      t.rs -o t.mir     # 최적화까지 거친 최종 MIR (.mir)
$R --emit=llvm-ir  t.rs -o t.ll      # LLVM IR (.ll)
$R --emit=asm      t.rs -o t.s       # 어셈블리 (.s)
$R --emit=obj      t.rs -o t.o       # 오브젝트 파일 (.o)
$R t.rs -o t && ./t                  # (링크 후) 실행
```

`.ll` / `.s` / `.o`는 LLVM 툴체인으로 더 파고들 수 있습니다. 툴 위치는 `$LLVM`입니다.

```bash
# LLVM IR 최적화 (함수 수가 확 줄어듦: 62 -> 8)
$LLVM/opt -O2 -S t.ll -o t.opt.ll
grep -c '^define' t.ll t.opt.ll

# LLVM IR -> 어셈블리
$LLVM/llc t.ll -o t.s

# LLVM IR -> 사람이 읽기 좋은 형태
$LLVM/llvm-dis t.ll

# 오브젝트 디스어셈블 / 심볼 목록
$LLVM/llvm-objdump -d t.o
$LLVM/llvm-nm t.o
```

심볼 이름은 v0 mangling(`_R...`)이라 잘 안 보이는데, 해당 함수가 인라인되어 있으면
`llvm-objdump -d t.o | grep 3foo` 같은 부분 매치로 위치를 찾으면 됩니다.
(`3foo`는 심볼 안의 crate/함수명 `foo`를 뜻하는 문자수 접두사입니다.)

---

## 6. cargo로 테스트 코드를 빌드할 때

`cargo +stage1`으로 테스트 프로젝트를 빌드/실행하면서 산출물을 뽑습니다.

```bash
# 테스트만 실행 (doctest 제외)
cargo +stage1 test --lib

# RUSTFLAGS로 모든 테스트 타깃의 MIR을 pass별로 덤프
RUSTFLAGS="-Zdump-mir=all -Zdump-mir-dir=md" cargo +stage1 test --lib

# 마지막 rustc 호출에만 추가 인자 전달 -> .ll / .s / .o 생성
cargo +stage1 rustc --lib -- --emit=llvm-ir
cargo +stage1 rustc --lib -- --emit=asm
cargo +stage1 rustc --lib -- --emit=obj
```

생성 위치:

- `--emit=*` 산출물: `target/debug/deps/crate-<hash>.ll` 등
- `-Zdump-mir-dir=md` 덤프: crate 루트의 `md/` (rustc 실행 cwd가 crate 루트)

주의:

- **incremental 캐시에 걸리면** 소스가 바뀌지 않는 한 rustc가 다시 실행되지 않아 덤프가
  안 생깁니다. `touch src/lib.rs`로 강제 리빌드하거나 캐시를 지우세요.
- 도구체인에 rustdoc이 없으면 `cargo +stage1 test`의 doctest 단계에서
  `'rustdoc' is not installed for the custom toolchain 'stage1'` 오류가 납니다.
  `--lib`를 붙이거나 0장의 rustdoc 복사 단계를 거치면 됩니다.

---

## 7. `tests/ui/` 테스트를 직접 확인하는 법

UI 테스트 파일을 그대로 stage1 rustc에 넘겨 중간 산출물을 뽑는 방법입니다.
`//@ run-pass`, `//@ compile-flags`, `//~` 같은 지시어는 전부 주석이므로 문제없습니다.

```bash
# HIR 확인
rustup run stage1 rustc -Zunpretty=hir \
  tests/ui/impl-trait/basic-trait-impl.rs

# MIR pass별 덤프
rustup run stage1 rustc -Zdump-mir=all -Zdump-mir-dir=/tmp/md \
  tests/ui/impl-trait/basic-trait-impl.rs -o /tmp/out.o

# LLVM IR / asm
rustup run stage1 rustc --emit=llvm-ir \
  tests/ui/impl-trait/basic-trait-impl.rs -o /tmp/t.ll
```

특징:

- **컴파일 실패가 예정된 테스트**(`//@ check-fail` / `//~ error`)는 컴파일 에러 메시지가
  그대로 출력되는데, 이것 자체가 산출물입니다. expected stderr(`.stderr`)와 대조해 보면 됩니다.
- `fn main`을 갖지 않는 라이브러리 성격의 테스트는 `--emit=llvm-ir`/`-Zdump-mir`까지는
  문제없지만, 링크가 필요한 `-o` 실행 파일 생성은 실패할 수 있습니다. 이때는 `--emit=obj`로 두세요.

정식 테스트 하네스로는:

```bash
./x.py test tests/ui/impl-trait/basic-trait-impl.rs        # 1개 실행
./x.py test tests/ui/impl-trait/                           # 디렉터리 전체
./x.py test tests/ui/impl-trait/basic-trait-impl.rs --bless # expected 출력 갱신
```

하네스를 통한 rustc 호출에 추가 플래그를 넣고 싶으면 테스트 파일에 지시어를 추가합니다.

```
//@ compile-flags: -Zunpretty=hir
```

`compile-flags`를 바꾼 뒤 `--bless`로 `.stderr`를 갱신하면 하네스 기준으로도
산출물을 저장해 둘 수 있습니다.

---

## 8. impl Trait(RPIT) 관찰 체크리스트

위 도구를 가지고 `fn foo() -> impl Iterator<...>` 흐름을 따라가 볼 때 보이는 지점입니다.

| 단계 | 명령 | 찾을 것 |
|------|------|---------|
| 파싱 | `-Zunpretty=ast-tree` | `TyKind::ImplTrait` |
| HIR lowering | `-Zunpretty=hir-tree` | `OpaqueDef(OpaqueTy { def_id: ...::foo::{opaque#0}, ... })` |
| 타입 검사 | `-Zunpretty=thir-tree` / `-Zverbose` | `Alias(Opaque { def_id: ...::foo::{opaque#0} })` |
| MIR | `-Zunpretty=mir` | 숨은 타입 확정: `Map<Range<i32>, {closure@...}>` |
| pass별 | `-Zdump-mir=all` | opaque 타입을 다루는 pass 전후 비교 |
| 새 solver | `-Znext-solver=globally` | 오래된/새 solver의 타입 판정 차이 확인 |

`DefId(0:4 ~ t[909c]::foo::{opaque#0})`의 `0:4`는 (crate 번호: DefId 인덱스)입니다.
같은 opaque 정의가 AST 이후 모든 단계에서 같은 `def_id`로 이어지는지 따라가는 게
impl Trait 이해에 가장 유용합니다.

---

## 9. 요약: 한 줄 명령 모음

```bash
# AST / HIR / THIR
rustup run stage1 rustc -Zunpretty=ast-tree   t.rs
rustup run stage1 rustc -Zunpretty=hir-tree   t.rs
rustup run stage1 rustc -Zunpretty=thir-tree  t.rs
rustup run stage1 rustc -Zunpretty=hir        t.rs
rustup run stage1 rustc -Zunpretty=mir        t.rs
rustup run stage1 rustc -Zunpretty=mir-cfg    t.rs

# MIR pass별 덤프
rustup run stage1 rustc -Zdump-mir=all -Zdump-mir-dir=/tmp/md t.rs -o /tmp/out.o
rustup run stage1 rustc -Zdump-mir="foo & SimplifyCfg"        t.rs -o /tmp/out.o

# 백엔드 산출물
rustup run stage1 rustc --emit=mir     t.rs -o t.mir
rustup run stage1 rustc --emit=llvm-ir t.rs -o t.ll
rustup run stage1 rustc --emit=asm     t.rs -o t.s
rustup run stage1 rustc --emit=obj     t.rs -o t.o

# LLVM 툴 분석
LLVM=build/aarch64-unknown-linux-gnu/stage1/lib/rustlib/aarch64-unknown-linux-gnu/bin
$LLVM/opt -O2 -S t.ll -o t.opt.ll
$LLVM/llc t.ll -o t.s
$LLVM/llvm-objdump -d t.o
$LLVM/llvm-nm t.o

# cargo 프로젝트
RUSTFLAGS="-Zdump-mir=all -Zdump-mir-dir=md" cargo +stage1 test --lib
cargo +stage1 rustc --lib -- --emit=llvm-ir
cargo +stage1 rustc --lib -- --emit=asm

# UI 테스트
rustup run stage1 rustc -Zunpretty=hir tests/ui/impl-trait/basic-trait-impl.rs
./x.py test tests/ui/impl-trait/basic-trait-impl.rs
```

---

## 10. 자주 겪는 문제

| 증상 | 원인 / 해결 |
|------|-------------|
| `can't find crate for std` | stage1 sysroot 미조립. `./x.py build --stage 1 library/std` 실행 |
| `only metadata stub found for rlib dependency std` | 빌드 중간 상태의 std rlib을 쓴 경우. 위 명령으로 sysroot 다시 조립 |
| `Permission denied ... at path "/dev/..."` | `-o /dev/null` 사용. 실제 경로로 대체 |
| `'rustdoc' is not installed for the custom toolchain` | 0장의 rustdoc 복사 단계 실행, 또는 `--lib`로 doctest 제외 |
| `-Zdump-mir` 후 덤프 파일이 안 보임 | incremental 캐시 재사용. `touch src/lib.rs` 후 재실행 |
| UI 테스트 직접 컴파일 시 링크 오류 | `main` 없는 라이브러리 성격 테스트. `--emit=obj` 또는 `-Zunpretty=*`/`-Zdump-mir`까지만 사용 |
