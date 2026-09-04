# 부속 타입 (Supporting Types)

`lib.rs` 등에 정의된 보조 타입들.

## 파일 / 해시 / 줄 메타데이터

- **`FileName`** (`lib.rs:507`): `Real(RealFileName)` + 가상 변형
  `CfgSpec`/`Anon`/`MacroExpansion`/`ProcMacroSourceCode`/`CliCrateAttr`/`Custom`/
  `DocTest`/`InlineAsm`. 각각 `Hash64`(또는 DocTest용 path/offset)를 든다.
  표시는 `FileNameDisplay` (`lib.rs:526`), `RemapPathScopeComponents` (`lib.rs:236`)로 제어.
- **`RealFileName`** (`lib.rs:304`): `local`(원본)과 `maybe_remapped`
  `InnerRealFileName`(name + working_directory + embeddable_name) + 활성 `scopes`를
  모두 보관. `path(scope)`, `embeddable_name(scope)`, `local_path()`, `empty()`,
  `from_virtual_path`, `update_for_crate_metadata`. 완전 재매핑 시 remapped 부분만
  해시하는 `Hash` impl (`lib.rs:326`).
- **`MultiByteChar`** (`lib.rs:1661`): `{ pos: RelativeBytePos, bytes: u8 }`.
- **`NormalizedPos`** (`lib.rs:1670`): `{ pos: RelativeBytePos, diff: u32 }` — 정규화 중
  `pos`에서부터 제거된 바이트 수.
- **`SourceFileLines`** (`lib.rs:1884`): `Lines(Vec<RelativeBytePos>)` |
  `Diffs(SourceFileDiffs)`. `SourceFileDiffs` (`lib.rs:1907`)는 메타데이터에 쓰이는
  컴팩트 diff-list.
- **`SourceFileHash`** (`lib.rs:1748`): `{ kind: SourceFileHashAlgorithm, value: [u8;32] }`.
  `new_in_memory`/`new` (Md5/Sha1/Sha256/Blake3, `lib.rs:1713`), `matches`, `hash_bytes`.
- **`StableSourceFileId(Hash128)`** (`lib.rs:2131`): `(FileName, Optional<StableCrateId>)`
  해시. 증분 세션 간 안정. 로컬 크레이트 파일은 `StableCrateId` 미확정 상태라 `None`으로 해시.
- **`OffsetOverflowError`** (`lib.rs:1709`): `MAX_FILE_SIZE` 초과 시 반환.
- `ExternalSource` / `ExternalSourceKind` (`lib.rs:1677`): 외국 크레이트용 지연 로드 소스.

## 위치 결과 타입

- **`Loc`** (`lib.rs:2742`): `{ file: Arc<SourceFile>, line, col: CharPos, col_display }`.
- **`SourceFileAndLine`** (`lib.rs:2755`), **`SourceFileAndBytePos`** (`lib.rs:2761`).
- **`LineInfo`** (`lib.rs:2767`): `{ line_index, start_col, end_col }`.
- **`FileLines`** (`lib.rs:2778`): `{ file, lines: Vec<LineInfo> }` (`span_to_lines` 결과).

## 에러 타입 (`lib.rs:2789`)

- `SpanLinesError`, `SpanSnippetError`, `DistinctSources`, `MalformedSourceMapPositions`.
