# 조회 & 변환 (Lookups)

`SourceMap`의 공개 API가 바이트 위치를 사람이 읽는 위치로 바꾸는 과정.

## 핵심 경로: `lookup_char_pos`

`SourceMap::lookup_char_pos(pos)` (`source_map.rs:415`) → `Loc { file, line, col, col_display }`

1. `lookup_source_file(pos)` (`source_map.rs:409`)로 `Arc<SourceFile>` 획득.
2. `SourceFile::lookup_file_pos_with_col_display(pos)` (`lib.rs:2503`) 호출:
   - `relative_position`으로 상대 오프셋.
   - `lookup_line`로 0-based 줄 인덱스 (`lib.rs:2389`).
   - `bytepos_to_file_charpos`로 `CharPos` (`lib.rs:2459`).
   - `char_width`로 display 열 계산 (`lib.rs:2527`).

## 바이트 오프셋 조회

- `lookup_byte_offset(bpos)` (`source_map.rs:1040`) →
  `SourceFileAndBytePos { sf, pos: bpos - sf.start_pos }`.
- `lookup_source_file_idx(pos)` (`source_map.rs:1050`):
  `partition_point(|x| x.start_pos <= pos) - 1`로 파일 인덱스 (MonotonicVec 안정성 의존).

## 줄/열 정보

- `lookup_line(pos)` (`source_map.rs:422`) → `SourceFileAndLine { sf, line }` (0-based).
  빈 파일이면 `Err(sf)`.
- `span_to_location_info(sp)` (`source_map.rs:465`) →
  `(Option<Arc<SourceFile>>, lo_line, lo_col+1, hi_line, hi_col+1)` (1-based).
- `span_to_lines(sp)` (`source_map.rs:527`) →
  `FileLines { file, lines: Vec<LineInfo> }`로 각 줄의 `LineInfo { line_index, start_col, end_col }`.

## 문자열 / 스니펫

- `span_to_string` / `span_to_short_string` / `span_to_diagnostic_string`
  (`source_map.rs:432`): `file:line:col` 형식.
- `span_to_snippet(sp)` (`source_map.rs:611`): 소스 텍스트 `String`.
- `span_to_prev_source` / `span_to_next_source` (`source_map.rs:636`).
- `span_to_source(extract_source)` (`source_map.rs:565`): 위 모든 스니펫 메서드의
  기반. `lo`/`hi` 해석 → 소스 존재 확인(로컬 `src` 또는 `external_src`) → 경계 검사 →
  클로저 호출. 에러 종류: `DistinctSources`, `MalformedForSourcemap`,
  `SourceNotAvailable`, `IllFormedSpan`.

## 유효성 & span 조작

- `is_valid_span(sp)` (`source_map.rs:506`): 양끝이 같은 `SourceFile`에 있는지.
  교차 파일이면 `SpanLinesError::DistinctSources`.
- 진단용 span 확장/축소 헬퍼 다수 (`source_map.rs:642`–`973`):
  `span_extend_to_prev_char`, `span_extend_while_whitespace`, `span_extend_to_line`,
  `span_until_char`, `span_through_char`, `guess_head_span`, `start_point`,
  `end_point`, `next_point`, `find_width_of_character_at_span`(멀티바이트 경계),
  `span_wrapped_by_angle_or_parentheses` 등.

## 외부 소스 지연 로드

- `ensure_source_file_source_present(sf)` (`source_map.rs:1058`): imported
  `SourceFile`의 소스를 해시 일치 시 지연 로드(로컬 파일 역매핑 시도 포함).
- `is_imported(sp)` (`source_map.rs:1085`): span이 외부 크레이트 소스인지.
