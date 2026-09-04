# 하이젠 & 매크로 확장 (Hygiene)

`Span`이 매크로 확장 체인을 추적하는 메커니즘. (`hygiene.rs`)

## `SyntaxContext` (`hygiene.rs:54`)

세션 단위 `SyntaxContextData`(`hygiene.rs:67`)로의 인덱스. `(ExpnId, Transparency)`
"marks"의 체인을 표현. 증분 컴파일 건전성을 위해 의도적으로 `Ord`/`PartialOrd` 미구현
(`hygiene.rs:59`). `outer_expn()` (`hygiene.rs:889`), `outer_expn_data()`
(`hygiene.rs:896`), `apply_mark`/`remove_mark` (`hygiene.rs:724`).

## `ExpnId` (`hygiene.rs:107`)

`{ krate: CrateNum, local_id: ExpnIndex }` — 전역 고유 매크로 호출 id.
`ExpnId::root()`는 확장되지 않은 AST. `LocalExpnId` (`hygiene.rs:122`)는 크레이트 로컬 부분.

## `ExpnData` (`hygiene.rs:969`)

확장별 메타데이터:

- `kind: ExpnKind`
- `parent: ExpnId`
- `call_site: Span` — 매크로 호출 span(재귀적으로 소스 호출 위치까지 추적 가능).
- `def_site: Span`
- `allow_internal_unstable`, `edition`, `macro_def_id`, `parent_module`,
  `local_inner_macros`, `diagnostic_opaque`
- `disambiguator` (`hygiene.rs:996`): 서로 다른 확장의 `Fingerprint` 충돌 방지.

## `ExpnHash(Fingerprint)` (`hygiene.rs:133`)

확장의 안정 해시.

## `Transparency` (`hygiene.rs:167`)

`Transparent` / `SemiOpaque` / `Opaque` — 확장 내 식별자 해석 제어.

## `HygieneData` (`hygiene.rs:339`)

세션 단위 전역 저장소: `local_expn_data`, `foreign_expn_data`,
`syntax_context_data`, `syntax_context_map`, disambiguator 맵. `SessionGlobals::new`
(`lib.rs:128`)에서 생성, `with_session_globals`로 접근. `walk_chain` (`hygiene.rs:474`)이
`call_site` span을 목표 컨텍스트까지 올라간다.

## 매크로 확장 소스와의 관계

`SourceMap`의 "파일"은 디스크 파일만이 아니다. 각 매크로 확장/디슈가링/doctest/인라인
어셈블리 버퍼도 고유 연속 `SourceFile` 범위를 받는다(가상 `FileName` 사용).
`Span`의 `ctxt`(`SyntaxContext`)가 확장 체인을 설명한다.

되돌리기:
- `Span::source_callsite`, `parent_callsite`, `find_ancestor_not_from_macro` — 매크로
  생성 span을 사용자가 쓴 호출 위치로.
- `original_sp(sp, enclosing_sp)` (`source_map.rs:28`): `expn_data.call_site` 체인을
  따라 원본 비매크로 span 복구.

여러 `SourceFile`이 섞일 수 있어 span이 파일을 건널 수 있으므로,
`is_valid_span` / `span_to_source`가 교차 파일 span을 감지·거부한다.
