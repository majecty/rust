# `CachingSourceMapView`

`CachingSourceMapView<'sm>` (`caching_source_map_view.rs:12`)는 `&SourceMap`을 감싸고
마지막으로 조회한 단일 `(file, line_bounds, line_number)`를 캐싱한다.

## 동기

span을 해시할 때(예: 증분 컴파일 / stable hashing) 가까이 있는 많은 위치를 반복
변환하므로, 마지막 줄 조회를 캐싱해 매번 `SourceMap` 전체를 검색하는 비용을 없앤다.

## 메서드

- `new` (`caching_source_map_view.rs:32`): `files()[0]`로 캐시 시드.
- `byte_pos_to_line_and_col(pos)` (`caching_source_map_view.rs:60`): 캐시 히트(위치가
  `line_bounds` 안)면 즉시 반환. 아니면 `file_for_position` →
  `SourceMap::lookup_source_file_idx`로 올바른 파일 찾아 재계산 후
  `(Arc<SourceFile>, 1-based line, RelativeBytePos col)` 반환.
- `span_data_to_lines_and_cols(span_data)` (`caching_source_map_view.rs:81`): 한 span에
  대한 최적화 경로. `lo`를 캐시하고, `hi`가 다른 줄에 있을 때만 계산. `(&SourceFile,
  lo_line, lo_col, hi_line, hi_col)` 반환(열은 `pos - line_bounds.start`).
- `file_contains` (`caching_source_map_view.rs:162`): `SourceFile::contains`를 감싸되
  **빈 파일은 아무것도 안 담는다**(인덱싱할 줄이 없음)고 취급.

`line_bounds`는 배타 상한 `Range<BytePos>`. 줄 끝이 개행 없이 끝난 파일의 마지막 문자
바로 다음에 떨어지는 span 끝 같은 edge case는 드문 캐시 미스가 되지만 정확히 해석된다.
