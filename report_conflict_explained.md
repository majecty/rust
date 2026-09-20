# report_conflict 설명

## 1. 한 줄 요약
같은 스코프에 같은 이름이 두 번 정의됐을 때 두 번째 정의에 "이름이 중복됐다" 에러(E0252/E0254/E0255/E0259/E0260/E0428)와 수정 제안을 붙여 보고하는 함수.

위치: rustc `resolver` (`report_conflict(ident, ns, old/new_binding)`), 중복 1회만 보고(`name_already_seen`).

## 2. 배경지식 (10줄)
- `Namespace`: 이름 공간 3종(Value, Type, Macro) — 같은 철자도 서랍이 다르면 공존한다.
- `Decl(binding)`: 이름 정의 묶음(대상 `Res`, 위치 `span`, 종류 `DeclKind`, 소속 `parent_module`).
- `Res/DefKind`: 이름이 가리키는 실체와 종류(모듈, trait, 타입, extern crate 등).
- `Span`: 코드 위치, `dummy`는 가짜 위치, `guess_head_span`은 이름 토큰만 잘라 표시한다.
- `Import/use`: `use`로 가져온 이름, `nested(a::{b,c})`와 `glob(*)`와 `MacroUse`와 `MacroExport` 변형이 있다.
- `ModuleKind/container`: 충돌이 난 그릇 설명(block 또는 정의 종류), `TyCtxt` 직접 조회 대신 매칭으로 구해 질의 순환을 피한다.
- `extern prelude/from_item`: 외부 크레이트 미리보기와 item이 직접 만든 이름인지 구분, 제거 제안 가드에 쓴다.
- `에러 코드`: 원인별 번호(E0428 정의끼리, E0252 use끼리, E0259 extern crate끼리, E0254와 E0260과 E0255 섞임).
- `진단 생성`: `NameDefinedMultipleTime` 본체와 `Reimported`와 `Redefined`와 `Import`와 `Definition` 라벨을 `create_err`로 묶어 `emit`한다.
- `중복 억제와 제안`: `name_already_seen`으로 한 번만 보고하고, `opt_def_id`가 같으면 제거, 다르면 이름 변경 제안을 붙인다.

## 3. 비유에서 설계자 수준까지

### 1. Namespace, 이름 서랍 나누기
장난감 상자와 색연필 통에 같은 스티커를 붙여도 헷갈리지 않는 것과 같다. 값 상자, 타입 상자, 매크로 상자가 따로 있으면 같은 철자도 따로 놀 수 있다.

코드는 Namespace를 ValueNS, TypeNS, MacroNS 열거형으로 나눈다. report_conflict의 ns 인자가 충돌 도메인을 정하고 descr와 old_kind 판별에 쓰인다.

서랍을 나눈 데에는 조회 비용과 순환 참조를 피하려는 이유가 있다. TyCtxt def_kind_descr 대신 ModuleKind 매칭으로 container를 구해 resolver 안에서 resolver를 다시 부르는 질의 사이클을 막는다.

코드에서는 ns: Namespace 인자로 들어와 ns.descr()로 본문에 박히고, match (ns, old_binding.res()) 분기로 old_kind이 value, macro, extern crate, module, trait, type 중 하나로 정해진다.

### 2. Decl, 이름표 묶음
식당 대기표에 이름, 테이블 번호, 받은 시간이 적히듯, 이름표에도 식별자, 가리키는 대상, 코드 위치가 적혀 있다.

코드는 Decl 구조체로 묶는다. res는 해소된 정의, span은 위치, kind는 DeclKind로 직접 정의인지 Import인지 구분하고, parent_module으로 소속 스코프를 안다.

수명 Decl<'ra>는 아레나 보관용이고, is_import와 is_import_user_facing을 나눈 점이 뜻밖의 함정이다. 겉보기엔 import라도 매크로용은 사용자에게 보여주면 안 되기 때문이다.

코드에서는 old_binding: Decl<'ra>와 new_binding: Decl<'ra> 쌍으로 들어와 binding.res(), binding.span, binding.kind 안의 DeclKind::Import { import }로 풀어 쓰고 분기를 탄다.

### 3. Res와 DefKind, 진짜 주인 가리기
같은 이름표라도 진짜 주인이 같으면 한 장 버리면 되고, 주인이 다르면 진짜 다툼이다. 우유 두 팩과 우유와 콜라를 구분하는 것과 같다.

코드는 Res 열거형으로 주인을 적는다. DefKind::Mod면 모듈, DefKind::Trait면 트레잇, 그 외는 타입 계열로 보고, extern crate는 따로 센다.

주인 구분을 TypeNS 안에서 더 쪼개는 이유는 메시지 때문이다. extern crate, module, trait, type을 다르게 불러야 사용자가 어디가 겹쳤는지 바로 안다.

코드에서는 old_binding.res()와 new_binding.res()로 꺼내고, res.opt_def_id() 비교로 duplicate 여부를 정하고, match (ns, old_binding.res())로 old_kind 문자열을 고른다.

### 4. Span, 위치와 밑줄 자르기
다툼이 나면 손가락으로 정확히 가리켜야 한다. 줄 전체가 아니라 이름 석 자만 짚는 것이 좋다.

코드는 Span으로 위치를 적는다. guess_head_span으로 use 줄 전체가 아니라 이름 토큰만 잘라내고, dummy span은 컴파일러가 만든 가짜 위치라 밑줄을 믿으면 안 된다.

순서도 위치로 정한다. old span 앞쪽이 new보다 뒤에 있으면 둘을 바꿔 항상 늦게 온 정의에 밑줄을 긋는다. 그래야 호출 순서와 무관하게 결과가 같다.

코드에서는 old_binding.span.lo()와 new_binding.span.lo()를 비교해 재귀 호출로 정규화하고, guess_head_span(new_binding.span)을 span으로 쓰고 old span과 다를 때만 옛 정의 라벨을 붙인다.

### 5. Import와 ImportKind, 가져온 표의 종류
직접 지은 집과 빌려온 집은 다르게 다뤄야 한다. 빌려온 집이 두 채 겹치면 계약서 한 장 버리면 되지만, 지은 집끼리 겹치면 새로 지어야 한다.

코드는 DeclKind::Import { import } 안에 ImportKind를 둔다. nested는 a::{b, c} 조각, glob은 별표 전체, MacroUse와 MacroExport는 매크로용이다.

가져온 표인지 겉으로 보이는 표인지 나눈 이유는 제안 안전 때문이다. 매크로용 import에 지우라는 가위를 대면 사용자가 고칠 수 없는 코드를 건드리게 된다.

코드에서는 (&new_binding.kind, &old_binding.kind) 매칭으로 Import를 꺼내고, can_suggest에서 MacroUse와 MacroExport를 제외하고, import.is_nested()와 import.is_glob()으로 조각 제거와 통째 제거와 제외를 가른다.

### 6. Container와 ModuleKind, 다툼이 난 그릇
같은 싸움이라도 교실 안과 운동장 안은 설명이 달라진다. 어디에서 났는지 적어야 찾아간다.

코드는 parent_module에서 꺼낸 ModuleKind로 그릇을 적는다. Block이면 block, Def면 kind.descr(def_id)로 모듈이나 함수나 구조체 같은 설명을 붙인다.

굳이 TyCtxt def_kind_descr를 쓰지 않는 이유는 순환 때문이다. 이름 해결 중에 종류 조회를 다시 부르면 질의가 서로를 기다리며 멈춘다.

코드에서는 old_binding.parent_module.unwrap().expect_local().kind 매칭으로 container를 만들고 NameDefinedMultipleTime { container } 필드로 넣어준다.

### 7. extern prelude와 from_item, 바깥 이름 가드
가게 밖 진열대와 가게 안 계산대는 규칙이 다르다. 밖에서 들어온 이름은 함부로 버리라고 하면 안 된다.

코드는 extern_prelude 맵으로 바깥에서 온 이름인지 본다. introduced_by_item()이면 item이 직접 들여온 이름이라 믿을 수 있고, 아니면 매크로나 전역 설정이 만든 이름일 수 있다.

제거 제안은 진짜 중복에 바깥 이름 가드까지 통과해야 나간다. extern crate끼리 겹치거나 from_item일 때만 한 장 버리기를 권한다.

코드에서는 self.extern_prelude.get(&ident)로 entry를 보고 is_none_or로 from_item을 계산한 뒤, duplicate와 has_dummy_span과 합쳐 should_remove_import를 만든다.

### 8. 에러 코드, 원인별 번호표
병원에서 감기는 내과, 골절은 정형외과로 보내듯 번호만 봐도 창구를 안다. E0428은 직접 정의끼리, E0252는 use끼리 겹친 쪽이다.

코드는 세 겹 매칭으로 번호를 고른다. extern crate끼리면 E0259, 한쪽만 extern crate면 import끼리인지 보고 E0254와 E0260을 가르고, 나머지는 사용자 노출 import 여부에 따라 E0428과 E0252와 E0255로 나눈다.

번호를 세분한 이유는 검색과 호환 때문이다. 옛 코드 번호를 함부로 바꾸면 문서와 테스트와 사용자 기억이 깨진다.

코드에서는 let code = match (old.is_extern_crate(), new.is_extern_crate())와 안쪽 is_import와 is_import_user_facing 매칭으로 정하고 with_code(code)로 붙인다.

### 9. 진단 생성, 문장과 라벨 조립
판결문은 주문, 이유, 증거 첨부로 이뤄진다. 밑줄 위치, 이름, 서랍 설명, 그릇 설명, 새 정의 라벨, 옛 정의 라벨이 한 세트다.

코드는 NameDefinedMultipleTime 구조체에 span, name, descr, container, label, old_binding_label을 채운다. 새 정의는 Reimported와 Redefined로, 옛 정의는 Import와 Definition으로 나눈다.

라벨을 나눈 이유는 문구 재사용 때문이다. fluent 메시지를 갈아끼우기 쉽게 하고 span 처리와 속성 처리를 독립시킨다.

코드에서는 dcx().create_err(...)로 err을 만들고, import 제안이 있으면 subdiagnostic을 붙인 뒤 err.emit()으로 쏘고 name_already_seen에 적는다.

### 10. 중복 억제와 제안, 한 번만 말하고 골라 권하기
같은 얘기를 세 번 하면 시끄럽다. 한 번만 말하고, 버릴지 이름을 바꿀지 골라 권해야 한다.

코드는 duplicate가 참이고 span이 진짜이고 바깥 가드를 통과할 때만 제거 쪽으로 간다. nested면 조각 제거, 단일 use면 use_span_with_attributes 통째 제거, 대상이 다르면 rename이다.

속성 있는 import가 끼면 속성 없는 쪽을 지우게 고른다. 속성을 날리면 사용자가 붙인 이유가 사라지기 때문이다.

코드에서는 import.use_span_with_attributes로 ToolOnlyRemoveUnnecessaryImport를, 조각은 add_suggestion_for_duplicate_nested_use로, 별명은 add_suggestion_for_rename_of_use(name, import, span)로 연결한다.

## 4. 조합해서 다시 보기
이 함수는 두 자리표의 시간순을 정리해 늦게 온 정의에 에러를 붙이고, 네임스페이스와 정의 종류와 import 여부에 따라 에러 코드와 메시지 라벨을 고른 뒤, 두 정의가 같은 대상을 가리키는 진짜 중복이면 불필요한 `use` 제거 제안을, 다른 대상이면 이름 변경 제안을 달아 1회만 보고하는, Rust 이름 해결기의 중복 이름 심판 함수이다.

흔한 착각: 무조건 새 `use`를 지우라고 하지 않음 — `dummy span`, `glob`, 매크로 import, `extern prelude`가 아닌 경우는 제안을 자제함.
