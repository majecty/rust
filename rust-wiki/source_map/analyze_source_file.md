# `analyze_source_file`

원시 소스를 `SourceFile::lines`와 `SourceFile::multibyte_chars`로 분해하는 모듈.
(`analyze_source_file.rs`)

## `analyze_source_file(src)` (`analyze_source_file.rs:11`)

`(Vec<RelativeBytePos>, Vec<MultiByteChar>)` 반환:

- 소스 스캔으로 줄 시작 위치(상대 바이트 위치, **`RelativeBytePos(0)`으로 시작**)와
  멀티바이트 문자 목록 생성.
- 런타임 분기 SIMD 고속 경로 (`cfg_select!` 사용):
  - **x86/x86_64**: `analyze_source_file_sse2` (`analyze_source_file.rs:59`) — 16바이트
    청크 SSE2 처리. `_mm_cmplt_epi8`로 멀티바이트 바이트 탐지, `_mm_cmpeq_epi8`로 `\n`
    탐지. 멀티바이트 바이트 등장 시 청크마다 제네릭 디코더로 폴백.
  - **loongarch64**: `lsx` intrinsic 변형 (`analyze_source_file.rs:163`).
  - **기타 타깃**: 제네릭 경로.
- `analyze_source_file_generic` (`analyze_source_file.rs:259`): 바이트 단위 스캔.
  `\n`면 `pos+1`을 다음 줄 시작으로 기록, `>= 128` 바이트면 UTF-8 문자 디코딩해
  `MultiByteChar { pos, bytes }`(bytes 2..=4) 기록. `scan_len`을 넘어 읽은 만큼 반환(멀티바이트 spillover).
- 디스패치 후, 끝이 `src.len()`과 같은 가짜 마지막 줄 시작을 제거 (`analyze_source_file.rs:21`).

이 함수가 위 모든 줄/열 계산에 쓰이는 `lines`와 `multibyte_chars`를 채운다.
