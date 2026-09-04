# 아키텍처 개요

`rustc_span`의 소스 위치 시스템은 몇 개의 핵심 타입으로 계층을 이룬다.

## 구성 요소 계층

```
SourceMap
 ├─ files: RwLock<SourceMapFiles>
 │    ├─ source_files: MonotonicVec<Arc<SourceFile>>   (정렬된 전체 소스 파일 목록)
 │    └─ stable_id_to_source_file: UnhashMap<StableSourceFileId, Arc<SourceFile>>
 ├─ file_loader: Box<dyn FileLoader + Sync + Send>      (std::fs 또는 커스텀)
 ├─ path_mapping: FilePathMapping                      (--remap-path-prefix)
 ├─ working_dir: RealFileName
 └─ hash_kind / checksum_hash_kind: SourceFileHashAlgorithm

SourceFile   (SourceMap 내 한 개의 연속된 소스 범위)
 ├─ name: FileName
 ├─ src: Option<Arc<String>>          (정규화된 소스; 외부 크레이트는 None)
 ├─ start_pos: BytePos                 (SourceMap 전체에서의 절대 위치)
 ├─ normalized_source_len: RelativeBytePos
 ├─ lines: FreezeLock<SourceFileLines> (줄 시작 오프셋)
 ├─ multibyte_chars: Vec<MultiByteChar>
 ├─ normalized_pos: Vec<NormalizedPos>
 ├─ stable_id: StableSourceFileId
 └─ cnum: CrateNum

Span = (BytePos lo, BytePos hi, SyntaxContext ctxt, Option<LocalDefId> parent)
       ▲ 인턴됨 (SpanInterner, span_encoding.rs)
```

## 역할 분담

- **`SourceMap`**: 크레이트 내 모든 소스를 소유하고, 바이트 위치를 파일/줄/열로
  변환하는 공개 API를 제공. (`source_map.rs:185`)
- **`SourceFile`**: 한 소스 조각을 표현. `start_pos`(절대)와
  `normalized_source_len`(상대)로 범위를 정의하고, 줄/멀티바이트 정보를 보관.
  (`lib.rs:1926`)
- **`Span`**: 컴파일러 전역에서 위치를 가리키는 가벼운 핸들. 하이젠 정보(`ctxt`)를
  함께 들고 매크로 확장 체인을 추적. (`span_encoding.rs:82`)
- **위치 프리미티브** `BytePos`(절대) / `RelativeBytePos`(파일 상대) / `CharPos`(문자
  오프셋)가 변환 경계를 정의. (`lib.rs:2697`, `lib.rs:2701`, `lib.rs:2709`)
- **`HygieneData`**: 매크로 확장 메타데이터(`ExpnId`, `ExpnData`, `SyntaxContext`)를
  세션 단위로 저장. (`hygiene.rs:339`)
- **`analyze_source_file`**: 원시 소스를 `lines` + `multibyte_chars`로 분해.
  (`analyze_source_file.rs:11`)
- **`CachingSourceMapView`**: 반복적인 위치 변환 시 마지막 줄을 캐싱해 속도 향상.
  (`caching_source_map_view.rs:12`)

## 바이트 위치 → 줄/열 변환 단계

1. `Span`이 `SourceMap` 상의 절대 `BytePos`(`lo`, `hi`)를 저장.
2. `SourceMap::lookup_source_file_idx(pos)`가 `start_pos` 기준 이분 탐색으로
   소유 `SourceFile`을 찾음. (`source_map.rs:1050`)
3. `SourceFile::relative_position(pos)`가 `start_pos`를 빼 `RelativeBytePos`로.
   (`lib.rs:2376`)
4. `SourceFile::lookup_line(rel)`이 `lines` 이분 탐색으로 0-based 줄 인덱스.
   (`lib.rs:2389`)
5. `SourceFile::bytepos_to_file_charpos(rel)`이 앞선 `multibyte_chars`의 여분
   바이트를 빼 바이트 오프셋을 `CharPos`(문자 오프셋)로. (`lib.rs:2459`)
6. `lookup_file_pos_with_col_display`가 이를 `(line, col, col_display)`로 합침.
   `col_display`는 wide/control 문자 정렬을 위해 `char_width`로 재계산.

`CachingSourceMapView`는 연속된 위치가 같은 캐시 줄에 떨어지면 2~5단계를 건너뛴다.

## 매크로 확장 소스

`SourceMap`의 "파일"은 디스크 파일만이 아니다. 각 매크로 확장/디슈가링/doctest/
인라인 어셈블리 버퍼도 고유한 연속 `SourceFile` 범위를 받는다(가상 `FileName` 사용).
`Span`의 `ctxt`(`SyntaxContext`)가 확장 체인을 설명하며,
`Span::source_callsite`, `parent_callsite`, `find_ancestor_not_from_macro`,
`original_sp`가 `ExpnData::call_site` 체인을 타고 사용자가 쓴 호출 위치로 되돌아간다.
여러 `SourceFile`이 섞일 수 있어 span이 파일을 건널 수 있으므로,
`is_valid_span` / `span_to_source`가 교차 파일 span을 감지·거부한다.

## 크로스 크레이트 재사용

`SourceFile`은 컴팩트한 `SourceFileDiffs` + `multibyte_chars` + `normalized_pos` +
해시 형태로 크레이트 메타데이터에 직렬화되고, `new_imported_source_file`로 다시
복원된다(실제 텍스트는 `ensure_source_file_source_present`가 진단용으로 필요할 때
지연 로드될 때까지 `None`).
