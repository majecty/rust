# `Iterator`를 못 찾는 컴파일 에러 — pre-interned symbol 인덱스 불일치

## 요약

`someany/test.rs`를 수정 후 stage1 rustc로 컴파일하면
`E0405: cannot find trait Iterator` / `E0425: cannot find type Vec` 가 난다.
원인은 **컴파일러의 pre-interned symbol(`symbols!` 매크로 목록의 `sym::*`/`kw::*`)을
메타데이터에 "문자열이 아니라 interning 인덱스"로 저장**하는 데 있고,
symbol 목록 변경(키워드 `any` 추가)으로 인덱스가 +1 밀려서
**기존에 빌드된 stdlib rmeta의 심볼이 전부 엉뚱한 이름으로 복원**되기 때문이다.
stdlib를 새 컴파일러로 재빌드하면 해결된다.

---

## 1. 증상

수정 후(현재 워킹트리) stage1 rustc로 `someany/test.rs`를 컴파일:

```
$ rustup run stage1 rustc ./someany/test.rs
error[E0405]: cannot find trait `Iterator` in this scope
 --> ./someany/test.rs:3:24
  |
3 | fn foo(x: i32) -> impl Iterator<Item = i32> {
  |                        ^^^^^^^^ not found in this scope

error[E0425]: cannot find type `Vec` in this scope
 --> ./someany/test.rs:8:12
  |
8 |     let v: Vec<i32> = foo(5).collect();
  |            ^^^ not found in this scope
```

단순 프로그램도 깨진다:

```
$ printf 'fn main() { let v: Vec<i32> = vec![]; }' | rustup run stage1 rustc -
error[E0425]: cannot find type `Vec` in this scope
```

`Iterator`/`Vec`처럼 **pre-interned symbol인 타입만** 안 풀리고
`std::prelude::v1`, `std::iter` 같은 (비 pre-interned) 모듈 경로는 정상적으로 풀린다.

---

## 2. 원인 — 심볼은 인덱스로 저장된다

### 2.1 pre-interned symbol이란

`compiler/rustc_span/src/symbol.rs`의 `symbols!` 매크로에 선언된 심볼
(`Iterator`, `Vec`, `Item`, `Option`, …)은 컴파일러가 기동하면서 고정 인덱스로
pre-intern 된다. `Iterator`도 여기 있다 (`symbol.rs:252`).

### 2.2 메타데이터 직렬화: pre-interned면 인덱스만 저장

`rustc_metadata/src/rmeta/encoder.rs:486-511`:

```rust
fn encode_symbol_or_byte_symbol(&mut self, index: u32, emit_str_or_byte_str: ...) {
    // if symbol/byte symbol is predefined, emit tag and symbol index
    if Symbol::is_predefined(index) {
        self.opaque.emit_u8(SYMBOL_PREDEFINED);
        self.opaque.emit_u32(index);          // ← 문자열이 아니라 인덱스만
    } else {
        // otherwise write it as string ...
        ...
    }
}
```

### 2.3 역직렬화: 인덱스를 그대로 `Symbol::new`로 복원

`rustc_metadata/src/rmeta/decoder.rs:380-400`:

```rust
match tag {
    SYMBOL_STR      => read_and_intern_str_or_byte_str_this(self), // 문자열 → 이번 컴파일러 기준 인터닝
    SYMBOL_OFFSET   => ...,
    SYMBOL_PREDEFINED => new_from_index(self.read_u32()),          // ← 그냥 Symbol::new(index)
    _ => unreachable!(),
}
```

즉 **rlib/rmeta를 만든 컴파일러와 읽는 컴파일러의 symbol 테이블이 다르면
인덱스가 그대로 다른 문자열을 가리킨다.** 비-pre-interned 이름(소문자 모듈명 등)은
문자열로 저장되므로 테이블이 달라도 문제가 없다.

---

## 3. 이번 변경이 테이블을 어떻게 바꿨나

`compiler/rustc_span/src/symbol.rs`의 현재 변경:

1. `Keywords`에 `Any: "any"` 추가 (weak keyword, `symbol.rs:129`) → 이후 키워드/모든 심볼 인덱스 **+1**
2. `Symbols`에서 `any` 제거 (`// any,`) → 이후 심볼 인덱스 **-1**

`Iterator`/`Vec`/`Option` 등은 목록상 `any`보다 앞에 있어서 (1)만 적용, **순수 +1 시프트**.
`Any` 키워드 추가가 유일한 원인이다.

---

## 4. 결정적 증거 — 복원된 이름이 정확히 "한 칸 앞 심볼"이다

### 4.1 `Iterator` → `ItemContext`, `Item` → `IrTyKind` (ICE 덤프)

`std::collections::HashMap`을 쓰는 프로그램이 ICE:

```
thread 'rustc' panicked at compiler/rustc_metadata/src/rmeta/decoder/cstore_impl.rs:230:1:
DefId(2:10856 ~ core[e0fb]::iter::traits::iterator::ItemContext::IrTyKind) does not have a "trait_def"
```

경로가 `core::iter::traits::iterator::ItemContext::IrTyKind`로 깨졌다.
실제 core에는 `core::iter::traits::iterator::Iterator::Item`이 있는데
`Iterator`(구 인덱스)가 **`ItemContext`**로, `Item`이 **`IrTyKind`**로 복원됐다.
symbol 목록에서 두 심볼 모두 **바로 앞 항목**이다:

```
symbol.rs:249  IrTyKind
symbol.rs:250  Item          ← (구 인덱스) → 현재 컴파일러에서 "IrTyKind"로 복원
symbol.rs:251  ItemContext   ← (구 인덱스) → 현재 컴파일러에서 "Item"이 아니라 ... 
symbol.rs:252  Iterator      ← (구 인덱스) → 현재 컴파일러에서 "ItemContext"로 복원
```

`+1 시프트`가 정확히 맞아떨어진다. 모듈명 `iter`/`traits`/`iterator`(소문자, 비 pre-interned)는
정상적으로 복원된 것도 같은 증거다.

### 4.2 `Option` → `Ord` 트레이트로 오해석

```
$ cat c.rs
fn main() { let o: Option<i32> = Some(1); }

warning: trait objects without an explicit `dyn` are deprecated
help: if this is a dyn-compatible trait, use `dyn` ...
error[E0107]: trait takes 0 generic arguments but 1 generic argument was supplied
note: trait defined here, with 0 generic parameters
  --> library/core/src/cmp.rs:994:17
994 | pub const trait Ord: ...
error[E0038]: the trait `Option` is not dyn compatible
```

소스의 `Option`이 `core::cmp::Ord`의 def로 해석됐다. 목록상
`Option(267)` → `Ord(268)` 이므로, `Ord`의 def(구 인덱스)가 새 컴파일러에서
`Symbol::new(구인덱스)` = "Option"으로 복원된 것. 역시 +1 시프트.

### 4.3 정상인 것들 — 문자열로 저장되는 이름

- `fn main() {}` → 정상
- `std::prelude::v1`(경로 자체), `std::iter`, `core::iter::traits::iterator` 모듈 → 정상 (전부 비 pre-interned)
- `Option`을 제외한 소문자 모듈명/메서드 등 → 정상

"pre-interned만 깨지고, 문자열 저장되는 건 안 깨진다"는 2장의 메커니즘과 정확히 일치.

---

## 5. 빌드 타임라인 (왜 어긋났나)

```
library/std rmeta   : Aug 13 20:42  ← 이 시점의 symbol 테이블로 작성 (Any 키워드 없음)
stage1 rustc 바이너리: Aug 15 23:26  ← Any 키워드가 들어간 테이블로 빌드
```

stage1 rustc만 재빌드했고 stdlib은 `./x.py build --stage 1 --keep-stage-std 1` 때문에
재사용됐다. 새 컴파일러가 옛 stdlib rmeta를 읽으면서 pre-interned 인덱스가 전부 +1
어긋나 깨진 이름으로 복원된 것.

---

## 6. 검증 — stdlib 재빌드 후 완전 해결

```
$ ./x.py build --stage 1 --keep-stage-std 0   # stdlib도 새 컴파일러로 재빌드
   Compiling core v0.0.0 ...
   Compiling std v0.0.0 ...
   Build completed successfully in 0:05:21
```

재빌드 후 동일 입력 검증:

| 테스트                                   | 수정 전                            | 수정 후                    |
|------------------------------------------|------------------------------------|----------------------------|
| `test.rs` (impl Iterator)                | E0405 Iterator                     | **컴파일 성공 (exit 0)**   |
| `Vec` prelude                            | E0425 Vec                          | **성공**                   |
| `HashMap`                                | ICE `ItemContext::IrTyKind`        | **성공**                   |
| `Option<i32>`                            | `Ord` 트레이트로 오해석 + E0038    | **정상 (`core::option::Option`)** |
| `usize + u64`                            | ICE "expected associated item"     | **정상 타입 에러 E0308**   |

---

## 7. 재발 방지

`satest.mts`에 `--build-std` 옵션을 추가했다. stdlib까지 재빌드한다:

```
node satest.mts --build-std full   # = ./x.py build --stage 1 (keep-stage-std 없음)
```

symbol 목록(`symbols!`)을 바꾸면 **반드시** stdlib까지 재빌드해야 한다.
기존 `--build`(`--keep-stage-std 1`)는 stdlib 재사용이라 이 버그를 다시 만들 수 있다.
