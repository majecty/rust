# `Span` / `SpanData`

컴파일러 전역에서 소스 위치를 가리키는 가벼운 핸들. (`span_encoding.rs:82`)

## 형태

- **`SpanData`** (`lib.rs:684`): `{ lo: BytePos, hi: BytePos, ctxt: SyntaxContext,
  parent: Option<LocalDefId> }` — 인턴되지 않은 raw 형태, `Send`/`Sync`.
  `is_dummy` ⇔ `lo==0 && hi==0`.
- **`Span`** (`span_encoding.rs:82`): 저장 공간 절약을 위해 **네 가지 컴팩트 인코딩**
  중 하나로 보관. `match_span_kind!` 매크로(`span_encoding.rs:204`)가 디코딩.

### 인코딩 종류 (`span_encoding.rs`)

1. `InlineCtxt { lo: u32, len: u16, ctxt: u16 }` — 가장 흔함, 완전 inline.
2. `InlineParent { lo, len|PARENT_TAG, parent: u16 }` — inline 위치 + parent `LocalDefId`.
3. `PartiallyInterned { index, ctxt: u16 }` — lo/hi 인턴, ctxt inline.
4. `Interned { index }` — lo/hi/ctxt 모두 인턴.

상수 `MAX_LEN`, `MAX_CTXT`, `PARENT_TAG`, `BASE_LEN_INTERNED_MARKER`,
`CTXT_INTERNED_MARKER` (`span_encoding.rs:236`)가 형식을 구분.
`Span::new` (`span_encoding.rs:248`)는 필요시 lo/hi를 바꾸고 가장 작은 형식을 선택.
`DUMMY_SP` (`span_encoding.rs:243`)는 전부 0.

## 인턴 (interning)

- **`SpanInterner`** (`span_encoding.rs:460`): `FxIndexSet<SpanData>`를 세션 전역
  락 뒤에 보관 (`with_span_interner`, `span_encoding.rs:473`).
- `Span::data()` / `data_untracked()` (`span_encoding.rs:285`)와
  `Span::ctxt()` / `parent()` (`span_encoding.rs:384`)가 모두 이 형식들을 통해 디코딩.

## 주요 연산

`Span`은 컴파일러의 작업용 말이다:

- `lo` / `hi` / `with_lo` / `with_hi`
- `from_expansion` (`span_encoding.rs:308`), `is_dummy`
- `contains` / `overlaps`
- `shrink_to_lo` / `shrink_to_hi`
- `source_callsite` / `parent_callsite`, `find_ancestor_*` (매크로 체인 순회)
- `edition`, `macro_backtrace`
- `to` / `between` / `until` (span 결합, `ctxt` 불일치는 `prepare_to_combine`로 처리)
- 하이젠 연산 `apply_mark` / `remove_mark` / `with_*_site_ctxt` (`lib.rs:750`)
