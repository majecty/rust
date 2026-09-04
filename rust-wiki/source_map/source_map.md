# `SourceMap`

`SourceMap`은 한 크레이트 컴파일 중 사용된 모든 소스 코드를 추적한다. 정수 바이트
위치를 원본 소스 위치로 매핑한다. (`source_map.rs:185`)

## 필드

- `files: RwLock<SourceMapFiles>` — 소유한 소스 파일 목록.
- `file_loader: IntoDynSyncSend<Box<dyn FileLoader + Sync + Send>>`
- `path_mapping: FilePathMapping` — `--remap-path-prefix` 경로 재매핑.
- `working_dir: RealFileName`
- `hash_kind: SourceFileHashAlgorithm` / `checksum_hash_kind: Option<…>`

### `SourceMapFiles` (`source_map.rs:171`)

```rust
struct SourceMapFiles {
    source_files: monotonic::MonotonicVec<Arc<SourceFile>>,
    stable_id_to_source_file: UnhashMap<StableSourceFileId, Arc<SourceFile>>,
}
```

`MonotonicVec`(`source_map.rs:45`)은 push만 가능하고 in-place 수정이 불가능해,
인덱스가 `SourceMap` 수명 내내 안정적이다 (이분 탐색에 필수).

## 생성자

- `SourceMap::new(path_mapping)` (`source_map.rs:207`): `RealFileLoader` + Md5 해시.
- `SourceMap::with_inputs(SourceMapInputs { file_loader, path_mapping, hash_kind,
  checksum_hash_kind })` (`source_map.rs:216`): 범용 생성자. 작업 디렉터리를
  `path_mapping.to_real_filename`로 재매핑해 기록.

## 파일 로딩 & 등록

### `FileLoader` trait (`source_map.rs:81`)

```rust
pub trait FileLoader {
    fn file_exists(&self, path: &Path) -> bool;
    fn read_file(&self, path: &Path) -> io::Result<String>;          // 정규화되어 String 반환
    fn read_binary_file(&self, path: &Path) -> io::Result<Arc<[u8]>>; // 비정규화, Arc로
    fn current_directory(&self) -> io::Result<PathBuf>;
}
```

- `RealFileLoader` (`source_map.rs:99`): `std::fs` 기반. `read_file`은
  `SourceFile::MAX_FILE_SIZE` 초과 파일 거부.
- `load_file(path)` (`source_map.rs:246`): 읽어 `FileName::Real(...)`(재매핑)로
  `new_source_file` 호출.
- `load_binary_file(path)` (`source_map.rs:256`): 정규화 없는 바이트 로드
  (예: `include_bytes!`). dep-info에 남기 위해 `SourceFile`도 등록하고,
  파일 범위의 `Span`과 함께 `(Arc<[u8]>, Span)` 반환.
- `new_source_file(filename, src)` (`source_map.rs:316`) →
  `try_new_source_file` (`source_map.rs:326`): `StableSourceFileId`를 계산하고,
  **동일 stable id의 파일이 이미 있으면 그대로 반환(idempotent)**. 없으면
  `SourceFile::new`로 만들고 `register_source_file` 호출.
- `register_source_file` (`source_map.rs:291`): `start_pos` 할당. 새 파일의
  `start_pos`는 `last_file.end_position() + 1`. **+1 간격**은 빈 파일이나 파일
  사이 위치도 구분되게 한다. `u32` 오버플로우 시 `OffsetOverflowError`.
- `new_imported_source_file(...)` (`source_map.rs:355`): 실제 소스 텍스트가 없는
  외부 크레이트용 `SourceFile` 생성 (`src: None`, `external_src: Foreign`).
  매개변수(줄 오프셋, 멀티바이트 문자, 정규화 위치, 해시)는 디코딩된 크레이트
  메타데이터에서 온다.

## 조회 & 변환 공개 API

- `lookup_source_file_idx(pos)` (`source_map.rs:1050`):
  `partition_point(|x| x.start_pos <= pos) - 1`. `pos`를 담은 파일 인덱스.
- `lookup_source_file(pos)` (`source_map.rs:409`): `BytePos` → `Arc<SourceFile>`.
- `lookup_byte_offset(bpos)` (`source_map.rs:1040`):
  `SourceFileAndBytePos { sf, pos: bpos - sf.start_pos }`.
- `lookup_char_pos(pos)` (`source_map.rs:415`): `Loc { file, line, col, col_display }`
  — 핵심 위치→위치 변환. `sf.lookup_file_pos_with_col_display(pos)` 호출.
- `lookup_line(pos)` (`source_map.rs:422`): `SourceFileAndLine { sf, line }`
  (0-based) 또는 빈 파일이면 `Err(sf)`.
- `is_valid_span(sp)` (`source_map.rs:506`): 양끝이 같은 `SourceFile`에 속하는지
  확인. 아니면 `SpanLinesError::DistinctSources`. (참고: `is_valid_position`이라는
  이름의 메서드는 없음.)
- `span_to_location_info(sp)` (`source_map.rs:465`):
  `(Option<Arc<SourceFile>>, lo_line, lo_col+1, hi_line, hi_col+1)` — 1-based.
- `span_to_string` / `span_to_short_string` / `span_to_diagnostic_string`
  (`source_map.rs:432`): `file:line:col` 형식. `RemapPathScopeComponents`가
  경로 재매핑 여부 결정(DIAGNOSTICS 등).
- `span_to_lines(sp)` (`source_map.rs:527`): `FileLines { file, lines: Vec<LineInfo> }`
  (`LineInfo { line_index, start_col, end_col }`, 0-based 열 범위). `is_valid_span`로 검증.
- `span_to_source(extract_source)` (`source_map.rs:565`): 모든 스니펫 메서드의
  기반. `lo`/`hi` 해석, 소스 존재 확인(로컬 `src` 또는 `external_src`), 경계 검사 후
  클로저 호출. 에러: `DistinctSources`, `MalformedForSourcemap`, `SourceNotAvailable`,
  `IllFormedSpan`.
- `span_to_snippet(sp)` (`source_map.rs:611`): span의 소스 텍스트 `String`.
- `span_to_prev_source` / `span_to_next_source` (`source_map.rs:636`) 등.
- **span 조작 헬퍼** (진단용으로 span을 소스 텍스트 기준으로 늘리기/줄이기):
  `span_extend_to_prev_char`, `span_extend_while_whitespace`, `span_extend_to_line`,
  `span_until_char`, `span_through_char`, `guess_head_span`, `start_point`,
  `end_point`, `next_point`, `find_width_of_character_at_span`(멀티바이트 경계 처리),
  `span_wrapped_by_angle_or_parentheses` 등 (`source_map.rs:642`–`973`).
- `ensure_source_file_source_present(sf)` (`source_map.rs:1058`): imported
  `SourceFile`의 외부 소스를 지연 로드(해시 일치 시, 로컬 파일 역매핑 시도 포함).
- `get_source_file`, `source_file_by_stable_id` (`source_map.rs:284`),
  `files()` (`source_map.rs:280`), `is_imported`, `stmt_span` /
  `mac_call_stmt_semi_span` (매크로 인지 statement span), `doctest_offset_line`.

## `FilePathMapping` (`source_map.rs:1120`)

- `Vec<(PathBuf, PathBuf)>` 재매핑 테이블 + `filename_remapping_scopes:
  RemapPathScopeComponents` (bitflags: `MACRO`, `DIAGNOSTICS`, `DEBUGINFO`,
  `COVERAGE`, `DOCUMENTATION`, `OBJECT` 별칭 — `lib.rs:236`).
- `map_prefix` / `remap_path_prefix` (`source_map.rs:1141`): *가장 마지막으로
  일치하는* `--remap-path-prefix` 규칙 적용(뒤에 올수록 우선).
- `to_real_filename(working_dir, local_path)` (`source_map.rs:1191`): 항상 존재하는
  로컬 경로와 재매핑된 `maybe_remapped` 경로를 모두 담은 `RealFileName` 생성.
- `reverse_map_prefix_heuristically` (`source_map.rs:1263`): 의존성 소스를 로컬에서
  찾기 위한 최선의 역매핑.

`original_sp(sp, enclosing_sp)` (`source_map.rs:28`): 확장 체인을 따라 원본
비매크로 span을 복구하는 모듈 프리 헬퍼.
