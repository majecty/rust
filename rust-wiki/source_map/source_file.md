# `SourceFile`

`SourceMap` 안의 한 개의 **연속된 소스**를 표현하는 구조체. (`lib.rs:1926`)

## 주요 필드 (`lib.rs:1930`–`1959`)

- `name: FileName` — 파일/`<anon>`/`<macro expansion>` 등.
- `src: Option<Arc<String>>` — (정규화된) 소스 텍스트. 외부 크레이트는 `None`.
- `src_hash: SourceFileHash`, `checksum_hash: Option<SourceFileHash>` — 디버그 정보
  및 cargo freshness 검사용. (`lib.rs:1748`)
- `external_src: FreezeLock<ExternalSource>` — 외국 크레이트용 지연 로드 소스.
  (`lib.rs:1677`)
- `start_pos: BytePos` — 이 파일 바이트 0의 **절대** 위치(`SourceMap` 전체 기준).
  `register_source_file`가 할당.
- `normalized_source_len: RelativeBytePos` — 정규화(BOM/CRLF 제거) **후** 바이트 길이.
- `unnormalized_source_len: u32` — 원본 길이.
- `lines: FreezeLock<SourceFileLines>` — 줄 시작 오프셋.
- `multibyte_chars: Vec<MultiByteChar>` — UTF-8 멀티바이트 문자 위치·크기.
- `normalized_pos: Vec<NormalizedPos>` — 정규화 중 제거된 문자 기록.
- `stable_id: StableSourceFileId` — 세션 간 식별자. (`lib.rs:2131`)
- `cnum: CrateNum` — 소유 크레이트.

## 생성 & 정규화

- `SourceFile::new(...)` (`lib.rs:2159`): **비정규화** 소스를 해시
  (`SourceFileHash::new_in_memory`), `unnormalized_source_len` 캡처,
  `normalize_src` 실행 (`lib.rs:2551` → `remove_bom`, `normalize_newlines`,
  `lib.rs:2559`), `StableSourceFileId` 계산, `analyze_source_file`로
  `lines` + `multibyte_chars` 생성. `MAX_FILE_SIZE = u32::MAX - 1` (`lib.rs:2157`).
- `lines()` (`lib.rs:2265`)는 `&[RelativeBytePos]` 반환. `SourceFileLines::Lines`로
  저장되거나, 메타데이터 디코딩 후 `SourceFileLines::Diffs(SourceFileDiffs)`(컴팩트
  diff-list, `lib.rs:1884`)로 저장. `convert_diffs_to_lines_frozen`
  (`lib.rs:2209`)이 diff를 전체 lines 벡터로 늦게 펼친 후 `FreezeLock` 동결.

## 위치 변환 메서드 (`lib.rs:2361`–`2524`)

- `absolute_position(RelativeBytePos) -> BytePos` (`lib.rs:2371`): `pos + start_pos`.
- `relative_position(BytePos) -> RelativeBytePos` (`lib.rs:2376`): `pos - start_pos`.
- `end_position() -> BytePos` (`lib.rs:2381`): `absolute_position(normalized_source_len)`.
- `contains(byte_pos)` (`lib.rs:2412`): `start_pos ..= end_position()` (EOF 바로 다음
  위치도 파일에 속한 것으로 간주).
- `is_empty()` (`lib.rs:2417`): `normalized_source_len == 0`.
- `lookup_line(pos: RelativeBytePos) -> Option<usize>` (`lib.rs:2389`):
  `partition_point(|x| x <= &pos).checked_sub(1)` — 0-based 줄 인덱스, 빈/이전이면 None.
- `line_bounds(line_index) -> Range<BytePos>` (`lib.rs:2393`): 줄 바이트 범위
  (시작 포함, 끝 배타; 마지막 줄은 `end_position()`까지).
- `get_line(line_number) -> Option<Cow<str>>` (`lib.rs:2330`): 0-based 줄 텍스트.
- `bytepos_to_file_charpos(bpos)` (`lib.rs:2459`): 앞선 `multibyte_chars`의 여분 바이트
  합을 빼 바이트 오프셋을 *문자*(`CharPos`) 오프셋으로. `bpos >= mbc.pos + mbc.bytes`
  어서션으로 위치가 문자 중간에 걸치지 않음을 보장.
- `lookup_file_pos(pos) -> (usize /*1-based line*/, CharPos)` (`lib.rs:2483`).
- `lookup_file_pos_with_col_display(pos) -> (usize, CharPos, usize /*display col*>)`
  (`lib.rs:2503`): `char_width`(`lib.rs:2527`)로 display 열도 계산해 wide/control
  문자에서 터미널 밑줄 정렬이 맞게.
- `original_relative_byte_pos` / `normalized_byte_pos` (`lib.rs:2423`): LLVM 인라인
  어셈블리 바이트 오프셋과 정규화 형태 사이 변환(`normalized_pos` 사용).
