# 위치 프리미티브 (Position Primitives)

바이트/문자 오프셋을 표현하는 작은 타입들. 변환 경계를 정의한다. (`lib.rs`)

## `Pos` trait (`lib.rs:2631`)

```rust
pub trait Pos: Copy + PartialEq + ... {
    fn from_usize(n: usize) -> Self;
    fn to_usize(self) -> usize;
    fn from_u32(n: u32) -> Self;
    fn to_u32(self) -> u32;
}
```

`impl_pos!` 매크로(`lib.rs:2638`)로 구현되며 `Add`/`Sub`도 추가.

## 구체 타입

| 타입 | 정의 | 의미 |
|------|------|------|
| `BytePos(pub u32)` | `lib.rs:2697` | `SourceMap` 전체에서의 **절대** 오프셋 |
| `RelativeBytePos(pub u32)` | `lib.rs:2701` | `SourceFile` 시작 기준 **상대** 오프셋 |
| `CharPos(pub usize)` | `lib.rs:2709` | 줄 안에서의 **문자**(바이트 아님) 오프셋 |

- `BytePos` / `RelativeBytePos`는 `u32`로 `Encodable`/`Decodable`. (`lib.rs:2712`)

## 변환 경계

- 절대 ↔ 상대 변환은 `SourceFile` 경계에서 일어난다:
  - `SourceFile::absolute_position(RelativeBytePos) -> BytePos` (`lib.rs:2371`)
  - `SourceFile::relative_position(BytePos) -> RelativeBytePos` (`lib.rs:2376`)
- 상대 바이트 오프셋 → 문자 오프셋은 `multibyte_chars`로 환산:
  - `SourceFile::bytepos_to_file_charpos(bpos)` (`lib.rs:2459`)

```
BytePos (절대, SourceMap 상)
   │  SourceFile::relative_position
   ▼
RelativeBytePos (파일 상대 바이트)
   │  SourceFile::bytepos_to_file_charpos
   ▼
CharPos (줄 안 문자 오프셋)
```
