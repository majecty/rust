# Source Map 위키 (`rustc_span`)

Rust 컴파일러에서 소스 코드 위치(source location)를 추적하는 시스템 정리.
대상 코드: `compiler/rustc_span/` (주로 `source_map.rs`, `lib.rs`, `hygiene.rs`,
`span_encoding.rs`, `analyze_source_file.rs`, `caching_source_map_view.rs`).

## 핵심 아이디어

크레이트를 컴파일하며 파싱되는 모든 소스 조각(실제 파일, 인메모리 문자열, 매크로
확장 버퍼, doctest, 인라인 어셈블리)은 **하나의 세션 단위 `SourceMap`** 안에서
연속된 정수 바이트 위치(byte position) 범위를 차지한다.

- `Span` = 절대 `BytePos` 두 개(`lo`, `hi`) + 하이젠 파일(hygiene) 메타데이터.
- `SourceMap`(과 그 안의 `SourceFile`들)이 나중에 그 위치를 파일/줄/열/스니펫
  정보로 변환한다.

## 데이터 흐름

```
Parser / Lexer
   │  read bytes (FileLoader) + normalize (BOM/CRLF)        [lib.rs normalize_src]
   ▼
SourceMap::load_file / new_source_file
   │  - StableSourceFileId 계산, 해시
   │  - analyze_source_file() → lines + multibyte_chars      [analyze_source_file.rs]
   │  - start_pos = prev.end + 1 할당                         [register_source_file]
   ▼
SourceMap.files: MonotonicVec<Arc<SourceFile>>   (각각이 BytePos 범위를 커버)
   │
   │  파싱이 Span{ lo: BytePos, hi: BytePos, ctxt: SyntaxContext } 방출
   ▼
Span  (SpanInterner로 인턴됨; ctxt는 HygieneData의 ExpnId/ExpnData에 연결)
   │
   │  이후 진단(diagnostics) 단계에서:
   ▼
SourceMap::lookup_char_pos / span_to_snippet / span_to_lines / lookup_byte_offset
   │  BytePos ─► SourceFile (start_pos 기준 이분 탐색)
   │  RelativeBytePos ─► lookup_line (lines) + bytepos_to_file_charpos (multibyte_chars)
   ▼
Loc / FileLines / snippet  →  에러 메시지, 디버그 정보, 증분 해시
   (CachingSourceMapView가 마지막 줄을 캐싱해 속도 향상)
```

## 문서 목록

| 문서 | 내용 |
|------|------|
| [architecture.md](architecture.md) | 전체 구성 요소와 관계도 |
| [source_map.md](source_map.md) | `SourceMap`, `SourceMapFiles`, `FileLoader`, `FilePathMapping` |
| [source_file.md](source_file.md) | `SourceFile` 구조체와 필드/메서드, 정규화 |
| [span.md](span.md) | `Span` / `SpanData` 인코딩, 인턴, 주요 연산 |
| [positions.md](positions.md) | `BytePos` / `RelativeBytePos` / `CharPos` / `Pos` trait |
| [lookups.md](lookups.md) | 바이트 위치 → 줄/열 변환, 조회 API |
| [hygiene.md](hygiene.md) | `SyntaxContext`, `ExpnId`, `ExpnData`, 매크로 확장 |
| [analyze_source_file.md](analyze_source_file.md) | 소스를 lines/multibyte_chars로 파싱 |
| [caching_source_map_view.md](caching_source_map_view.md) | `CachingSourceMapView` 캐싱 |
| [supporting_types.md](supporting_types.md) | `FileName`, 해시, `Loc`, 에러 타입 등 |
