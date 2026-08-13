# Rust 컴파일러 테스트: `tests/ui/impl-trait/`

이 문서는 Rust 컴파일러 테스트 스위트의 `tests/ui/impl-trait/` 디렉터리에 있는 UI 테스트 파일들을 주제별로 정리한 것입니다. `impl Trait` 문법, 반환 위치/인자 위치/타입 별칭/트레이트 내 사용, 라이프타임, 자동 트레이트, 메서드 해상도, 정밀 캡처, CTFE 등 다양한 시나리오를 다루는 테스트를 한국어로 설명합니다.

## 목차

- [기본 문법과 위치](#basic)
- [반환 위치 impl Trait (RPIT)](#rpit)
- [트레이트 내 impl Trait (RPITIT)](#rpitit)
- [타입 별칭 impl Trait (TAIT)](#tait)
- [라이프타임과 멤버 제약](#lifetimes)
- [자동 트레이트와 누출](#auto)
- [메서드 해상도](#method)
- [정밀 캡처 (precise capturing)](#precise)
- [컴파일 타임 평가](#ctfe)
- [회귀 테스트 모음](#regressions)
- [기타](#misc)

## 기본 문법과 위치

impl Trait의 기본 문법, 허용/금지 위치, dyn 트레이트와의 상호작용 등

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `bare-trait-object-return.rs` | 포인터 없이 bare 트레이트 객체를 반환하는 경우의 회귀 (#18107)를 검증 |
| `basic-trait-impl.rs` | 단순 타입과 제네릭 타입에 대한 `impl Trait for Type` 기본 문법이 동작하는지 검증 |
| `dyn-impl-type-mismatch.rs` | `dyn Trait`과 `impl Trait` 사이의 타입 불일치를 검증 |
| `dyn-incompatible-trait-in-return-position-dyn-trait.rs` | 반환 위치 dyn 트레이트 객체에 dyn-incompatible 트레이트가 사용될 때 오류를 검증 |
| `dyn-incompatible-trait-in-return-position-impl-trait.rs` | 반환 위치 `impl Trait`에 dyn-incompatible 트레이트가 사용될 때 오류를 검증 |
| `dyn-trait-elided-two-inputs-assoc.rs` | 두 입력이 있는 RPIT에서 `dyn Bar`가 연관 타입을 통해 `+ 'static`으로 기본 처리됨을 회귀 (#62517)로 검증 |
| `dyn-trait-elided-two-inputs-param.rs` | 두 입력이 있는 RPIT에서 `dyn Object`가 매개변수를 통해 `+ 'static`으로 기본 처리됨을 회귀 (#62517)로 검증 |
| `dyn-trait-elided-two-inputs-ref-assoc.rs` | `&impl Foo<Item = dyn Bar>`에서 생략된 라이프타임에 대한 회귀 (#62517)를 검증 |
| `dyn-trait-elided-two-inputs-ref-param.rs` | `&impl Alpha<dyn Object>`에서 생략된 라이프타임에 대한 회귀 (#62517)를 검증 |
| `dyn-trait-return-should-be-impl-trait.rs` | bare `dyn Trait` 반환 시 `impl Trait` 또는 `Box<dyn Trait>` 사용을 제안하는 진단을 검증 |
| `extra-impl-in-trait-impl.rs` | 트레이트 구현에서 `impl impl Default for S` 같은 추가 `impl` 키워드에 대한 파서 오류를 검증 |
| `extra-item.rs` | 구현이 트레이트에 없는 추가 메서드를 포함할 때 오류를 검증 |
| `impl-trait-in-generic-param.rs` | 제네릭에서 `impl Trait`가 허용되지 않음을 검증하는 회귀 (#47715) |
| `impl-trait-in-macro.rs` | 매크로 내부에서 확장된 `impl Trait`와 타입 불일치 및 다중 트레이트 바운드를 검증 |
| `impl-trait-plus-priority.rs` | `impl`, `dyn`, bare 트레이트 타입에서 `+` 구문 분석 우선순위를 검증 |
| `impl_trait_projections.rs` | ADT 매개변수에서는 허용되지만 프로젝션과 익명 const에서는 허용되지 않는 `impl Trait` 위치를 검증 |
| `nested_impl_trait.rs` | 허용되고 금지된 중첩 `impl Trait` 위치를 검증 |
| `nesting.rs` | 배열 타입 매개변수에서 중첩된 익명 const를 가진 `impl Trait`를 검증 |
| `no-trait.rs` | `impl 'static`처럼 트레이트 없이 사용된 경우 오류를 검증 |
| `universal_in_adt_in_parameters.rs` | 함수 매개변수 내 ADT 내부 `impl Trait`가 여러 구체적 타입을 허용함을 검증 |
| `universal_in_impl_trait_in_parameters.rs` | 중첩 `impl Trait` 항목 타입을 가진 `impl Trait` 매개변수를 검증 |
| `universal_in_trait_defn_parameters.rs` | 트레이트 메서드 매개변수 정의에서 `impl Trait`가 허용됨을 검증 |
| `universal-mismatched-type.rs` | `impl Debug`를 `String`에 대입할 때 타입 불일치를 검증 |
| `universal_multiple_bounds.rs` | 다중 바운드(`Display + Clone`)를 가진 `impl Trait` 매개변수를 검증 |
| `universal-two-impl-traits.rs` | 두 개의 서로 다른 `impl Trait` 매개변수가 통합되지 않음을 검증 |
| `universal_wrong_bounds.rs` | 잘못된 바운드가 있는 `impl Trait` 매개변수에서 누락된 트레이트 오류를 검증 |
| `universal_wrong_hrtb.rs` | `impl Trait` 인자에서 잘못된 높은 순위 라이프타임 바운드를 검증 |
| `where-allowed-2.rs` | 반환 위치 ADT 내부 `impl Trait`와 에디션에 따른 추론을 검증 |
| `where-allowed.rs` | `impl Trait`가 허용되고 금지된 위치에 대한 종합 테스트 |
| `xcrate.rs` | 내부 함수에 접근하는 클로저를 반환하는 크로스 크레이트 RPIT을 검증 |
| `xcrate_simple.rs` | 클로저를 반환하는 간단한 크로스 크레이트 RPIT을 검증 |

## 반환 위치 impl Trait (RPIT)

반환 위치에서 impl Trait를 사용하는 경우와 중첩/재귀 반환 타입

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `bounds_regression.rs` | 연관 타입을 포함한 RPIT 바운드에 대한 회귀 테스트(`FakeFuture<Output = T::Return>`) |
| `closure-calling-parent-fn.rs` | 부모 함수에서 `impl Copy`를 반환하는 클로저와 관련된 회귀 (#54593)를 검증 |
| `cross-return-site-inference.rs` | 타입 불일치와 `?` 연산자가 있는 `impl Trait`의 반환 지점 간 추론을 검증 |
| `divergence.rs` | `panic!`로 인한 발산(divergence) 뒤에 `impl Trait` 반환 식이 오는 경우를 검증 |
| `dont-suggest-box-on-empty-else-arm.rs` | 빈 else 분기와 `impl Trait` 반환에서 `Box` 제안 대신 타입 불일치 오류가 발생함을 검증 |
| `feature-self-return-type.rs` | `Self` 또는 프로젝션이 RPIT 반환에 사용될 때 올바른 borrowck 오류를 검증 (#61949) |
| `impl_fn_associativity.rs` | 중첩 `impl Fn() -> impl Trait` 반환 타입의 결합성(associativity)을 검증 |
| `impl-fn-parsing-ambiguities.rs` | 중첩 `impl Fn` 반환 타입에서 `+`의 구문 분석 모호성을 검증 |
| `impl-fn-rpit-opaque-107883.rs` | `impl Fn` 반환 바운드 내부 불투명 타입에 대한 회귀 (#107883)를 검증 |
| `nested-return-type2.rs` | 연관 타입 바운드와 느긋한 추론이 있는 중첩 RPIT을 검증 (#107346) |
| `nested-return-type3.rs` | 연관 타입이 트레이트 타입 자체인 중첩 RPIT을 검증 |
| `nested-return-type4.rs` | 바운드에 나타나지 않는 라이프타임을 캡처하는 중첩 비동기 RPIT을 검증 |
| `nested-return-type.rs` | 제네릭 라이프타임을 가진 `impl Iterator<Item = impl Sized>` 중첩 반환을 검증 |
| `not_general_enough_regression_106630.rs` | 비동기 콜백에 대해 RPIT이 충분히 일반적이지 않았던 회귀 (#106630)를 검증 |
| `point-to-type-err-cause-on-impl-trait-return.rs` | 불일치한 `impl Trait` 타입에 대해 정확한 반환/분기를 가리키는 오류 메시지를 검증 |
| `question_mark.rs` | RPIT에서 `?` 연산자와 `impl Debug` 오류 타입을 사용하는 경우를 검증 |
| `recursive-auto-trait.rs` | `impl Send` 같은 자동 트레이트 바운드를 가진 재귀적 RPIT을 검증 |
| `recursive-bound-eval.rs` | 파서 메서드를 위한 중첩 의무 평가가 있는 재귀적 RPIT을 검증 |
| `recursive-coroutine-boxed.rs` | boxed self-call을 사용하는 재귀적 `impl Coroutine`을 검증 |
| `recursive-coroutine-indirect.rs` | boxing에 대한 오류를 포함한 간접 재귀 코루틴과 `impl Sized`를 검증 |
| `recursive-impl-trait-type-direct.rs` | 직접 재귀적 `impl Sized` 사용을 검증 |
| `recursive-impl-trait-type-indirect.rs` | 간접 재귀적 `impl Sized` 타입(Option, 튜플, 배열 등)이 금지됨을 검증 |
| `recursive-impl-trait-type-through-non-recursive.rs` | 다른 크레이트의 비재귀 래퍼를 통한 재귀적 `impl Sized`를 검증 |
| `return-never-type.rs` | 2024 에디션에서 `!`를 `impl Add<u32>` 함수로부터 반환하는 경우를 검증 |
| `return-position-impl-trait-minimal.rs` | 최소한의 RPIT 예제(`impl Debug`가 `&str` 반환)를 검증 |
| `rpit-assoc-pair-with-lifetime.rs` | 연관 타입 쌍과 라이프타임 생략 경고가 있는 RPIT을 검증 |
| `rpit-not-sized.rs` | 반환 위치의 `impl ?Sized`에 대한 오류를 검증 |
| `static-lifetime-return-position-impl-trait.rs` | const 제네릭 문자열과 반환 위치의 `impl Iterator`를 검증 |
| `suggest-calling-rpit-closure.rs` | `impl Fn() -> i32`를 반환하는 RPIT 클로저 호출을 제안하는 진단을 검증 |
| `unactionable_diagnostic.rs` | 제네릭 매개변수가 오래 살지 못할 수 있는 RPIT 반환에 대한 실용적인 라이프타임 도움말을 검증 |

## 트레이트 내 impl Trait (RPITIT)

트레이트 메서드와 연관 타입에서 impl Trait를 사용하는 경우

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `associated-impl-trait-type-generic-trait.rs` | 제네릭 트레이트에 대한 `impl Trait` 연관 타입 구현을 검증 |
| `associated-impl-trait-type-into-emplacable.rs` | 연관 타입의 `impl Sized`가 `Into`/`IntoEmplacable`과 함께 사용될 때 발생했던 회귀 (#146514)를 검증 |
| `associated-impl-trait-type-issue-114325.rs` | 비동기 RPITIT과 MIR drop tracking에서 "unexpected unsized tail" ICE가 발생했던 회귀 (#114325)를 검증 |
| `associated-impl-trait-type.rs` | 연관 타입에서 `impl Trait`를 사용하는 기본 동작(`impl_trait_in_assoc_type`)을 검증 |
| `associated-impl-trait-type-trivial.rs` | 연관 타입에 `impl Trait`를 사용하는 간단한 경우를 검증 |
| `associated-type-cycle.rs` | 연관 타입이 `<Self as Foo>::Bar` 같은 Self 프로젝션으로 정의될 때 순환이 감지되는지 검증 |
| `associated-type-fn-bound-issue-72207.rs` | 연관 타입 바운드에 전달된 클로저가 동작함을 확인하는 회귀 (#72207) 테스트 |
| `associated-type-undefine.rs` | 연관 타입에 사용된 `impl Trait`가 제약되지 않고 정의 사용에서 누락될 때 오류가 발생하는지 검증 |
| `defined-by-trait-resolution.rs` | 트레이트 쿼리가 RPIT의 유효한 정의 사용(defining use)임을 검증 |
| `ice-unexpected-param-type-whensubstituting-in-region-112823.rs` | GAT와 RPITIT에서 region 치환 시 "unexpected parameter Type(Repr)" ICE가 발생했던 회귀 (#112823)를 검증 |
| `impl-generic-mismatch.rs` | 트레이트 구현에서 `impl Trait`와 명시적 제네릭 매개변수 불일치를 검증 |
| `in-assoc-type.rs` | 다른 구현의 불투명 연관 타입에 대한 숨겨진 타입 등록을 검증 |
| `in-assoc-type-unconstrained.rs` | 연관 타입 내부에서 제약되지 않은 불투명 타입과 불일치한 제약을 검증 |
| `opaque-hidden-inferred-rpitit.rs` | 숨겨진 타입이 없는 RPITIT에서 `opaque_hidden_inferred_bound` 린트가 발생하지 않음을 검증 |
| `opaque-used-in-extraneous-argument.rs` | 불투명 `Fn` 타입에 불필요한 인자가 사용될 때 진단에서 ICE가 발생하던 회귀를 검증 |
| `projection-mismatch-in-impl-where-clause.rs` | `impl Trait` where 절에서 연관 타입을 프로젝션할 때 타입 불일치를 검증 |
| `projection.rs` | `impl Trait` 바운드 내부 프로젝션으로 MIR 검증이 실패하던 회귀를 검증 |
| `stashed-diag-issue-121504.rs` | impl trait 오류에서 비동기 메서드 시그니처가 있는 숨겨진 진단에 대한 회귀 (#121504)를 검증 |
| `stranded-opaque.rs` | 불투명 트레이트 바운드 내부의 고립된 연관 타입이 ICE를 일으키지 않음을 검증 |
| `trait_resolution.rs` | `impl Debug` 메서드에서 반환된 불투명 타입의 트레이트 해상도를 검증 |
| `trait_type.rs` | 잘못된 트레이트 메서드 시그니처에 대한 오류 진단을 검증 |
| `trait_upcasting_reference_mismatch.rs` | 트레이트 업캐스팅 시 `impl Sized` 참조 불일치로 인한 정보 없는 진단을 검증 |
| `trait_upcasting.rs` | 트레이트 객체 간 `Trait<Concrete>`와 `Trait<Opaque>`의 unsizing을 검증 |
| `type-alias-generic-param.rs` | 제네릭 매개변수를 가진 불투명 연관 타입의 정의 사용에 대한 회귀 (#59342)를 검증 |
| `type-arg-mismatch-due-to-impl-trait.rs` | 트레이트 구현 메서드의 `impl Trait`가 암시적 타입 매개변수를 도입해 불일치를 일으키는 경우를 검증 |
| `wf-check-hidden-type.rs` | 라이프타임 바운드가 있는 숨겨진 타입의 올바름(well-formedness) 검사에 대한 회귀 (#114728)를 검증 |
| `wf-eval-order.rs` | 프로젝션 및 불투명 타입과 함께 올바름 조건의 평가 순서를 검증 |

### `in-trait/`

트레이트 내 RPITIT 관련 테스트

| 파일 | 설명 |
|------|------|
| `alias-bounds-when-not-wf.rs` | next 솔버에서 WF가 아닌 별칭 바운드가 RPITIT에서 ICE 대신 적절한 오류를 생성하는지 검증 |
| `anonymize-binders-for-refine.rs` | 정제(refinement) 검사에서 높은 순위 RPITIT 바운드를 위해 바인더를 익명화하는지 검증 |
| `assumed-wf-bounds-in-impl.rs` | 비동기 트레이트 메서드의 올바른 타입이 합성 GAT 검사 시 가정됨을 검증 (#113796) |
| `async-and-ret-ref.rs` | 비동기 트레이트 메서드가 RPITIT에 대한 참조를 반환할 때 합성 연관 타입이 future보다 오래 살지 못함을 거부 (#117547) |
| `bad-item-bound-within-rpitit-2.rs` | RPITIT 항목 바운드에 선언되지 않은 라이프타임 사용을 감지 (#114146) |
| `bad-item-bound-within-rpitit.rs` | 트레이트보다 엄격한 라이프타임 바운드를 가진 GAT 구현을 거부하고 RPITIT 불일치를 경고 (#114145) |
| `bad-projection-from-opaque.rs` | 반환 타입의 프로젝션 경로 내부에 `impl Trait`를 사용하는 것을 거부 (#126725) |
| `box-coerce-span-in-default.rs` | 기본 RPITIT 본문의 match arm이 서로 다른 enum variant를 강제 변환할 때 span 오류 없이 동작함을 검증 |
| `cannot-capture-intersection.rs` | 바운드에 선언되지 않은 교차 라이프타임을 캡처하는 RPITIT 숨겨진 타입을 거부 |
| `check-wf-on-non-defaulted-rpitit.rs` | 기본값이 없는 RPITIT이 `Send`를 요구하는 래퍼 내부에 있을 때 올바름 검사가 잡아내는지 검증 |
| `cycle-effective-visibilities-during-dyn-compatibility-check.rs` | 트레이트의 RPITIT을 사용하는 고유 `impl dyn Trait`에 대해 dyn 호환성 오류가 보고됨을 검증 |
| `cycle-if-impl-doesnt-apply.rs` | 한 구현의 WF 검사가 다른 구현의 RPITIT 숨겨진 타입에 의존할 때 쿼리 순환을 피하는지 검증 |
| `deep-match.rs` | 구현의 반환 타입이 래핑된 RPITIT 구조와 일치하지 않을 때 거부 |
| `deep-match-works.rs` | 구현이 RPITIT과 동일한 구조로 구체적 타입을 반환할 때 허용 |
| `default-body.rs` | 기본 비동기 트레이트 본문이 컴파일되고 구현에서 그대로 사용될 수 있음을 검증 |
| `default-body-type-err-2.rs` | 기본 비동기 트레이트 본문에서 RPITIT를 포함한 타입 불일치를 감지 |
| `default-body-type-err.rs` | 기본 RPITIT 본문에서 `Deref` 프로젝션을 포함한 타입 불일치를 감지 |
| `default-body-with-rpit.rs` | RPITIT 반환 타입을 가진 기본 비동기 트레이트 본문이 컴파일됨을 검증 |
| `default-method-binder-shifting.rs` | 중첩 RPITIT에 대한 기본 메서드 프로젝션 가정 설치 시 바인더 이동이 올바른지 검증 |
| `default-method-constraint.rs` | 기본 RPITIT 메서드가 `Self`를 통해 자기 자신을 호출할 때 오류 없이 동작함을 검증 |
| `doesnt-satisfy.rs` | 숨겨진 타입이 트레이트 바운드를 만족하지 않는 RPITIT 구현을 거부 |
| `do-not-imply-from-trait-impl.rs` | 구현이 트레이트 시그니처가 허용하는 것보다 더 강한 RPITIT 숨겨진 타입을 노출하는 것을 거부 |
| `dont-consider-unconstrained-rpitits.rs` | 제약되지 않은 RPITIT가 제네릭 인자 불일치 제안에 고려되지 않음을 검증 |
| `dont-probe-missing-item-name-2.rs` | 누락된 제네릭 또는 연관 타입 오류 보고 시 RPITIT 이름을 조사하지 않음을 검증 |
| `dont-probe-missing-item-name-3.rs` | 모호한 연관 타입 오류 출력 시 RPITIT 이름을 조사하지 않음을 검증 |
| `dont-probe-missing-item-name-4.rs` | 연관 타입 타입 불일치 해결 시 RPITIT 이름을 조사하지 않음을 검증 |
| `dont-probe-missing-item-name.rs` | 누락된 연관 타입 오류 시 RPITIT 이름을 조사하지 않음을 검증 (#139873) |
| `dont-project-to-rpitit-with-no-value.rs` | 값이 없는 RPITIT에 프로젝션하지 않고 누락된 트레이트 항목을 보고 |
| `dump.rs` | 컴파일러 진단을 위해 불투명 RPITIT의 숨겨진 타입을 덤프 |
| `dyn-compatibility.rs` | RPITIT가 있는 트레이트를 트레이트 객체로 사용할 수 없음을 검증 |
| `dyn-compatibility-sized.rs` | RPITIT 메서드가 `Self: Sized`로 제한될 때 트레이트가 dyn 호환성을 유지함을 검증 |
| `early.rs` | 초기 묶인 라이프타임을 가진 RPITIT 및 비동기 트레이트 메서드가 컴파일됨을 검증 |
| `encode.rs` | RPITIT가 있는 트레이트가 라이브러리 크레이트로 인코딩될 수 있음을 검증 |
| `ensure-rpitits-are-created-before-freezing.rs` | 구현 self 타입이 잘못되더라도 def ID가 동결되기 전에 RPITIT가 생성됨을 검증 |
| `expeced-refree-to-map-to-reearlybound-ice-108580.rs` | RPITIT에서 free region을 초기 묶인 region으로 매핑할 때 ICE가 발생하던 회귀 (#108580)를 검증 |
| `false-positive-predicate-entailment-error.rs` | 현재 및 다음 솔버에서 RPITIT 바운드의 거짓 양성 predicate entailment 오류 회귀를 검증 (#158643) |
| `foreign-dyn-error.rs` | 외부 트레이트에 RPITIT가 있을 때 dyn 비호환성을 보고 |
| `foreign.rs` | 외부 RPITIT 트레이트를 구현하고 호출하며 지역 정제 경고를 확인 |
| `gat-outlives.rs` | 비동기 트레이트 메서드와 함께 사용된 GAT에 필요한 `Self: 'a` 바운드 누락을 감지 |
| `generics-mismatch.rs` | RPITIT 메서드에 추가 제네릭 매개변수를 추가한 구현을 거부 |
| `issue-102140.rs` | `Self: Sized` 바운드가 있는 `dyn Trait`의 RPITIT 메서드에 대한 회귀 (#102140) |
| `issue-102301.rs` | 제네릭 트레이트 매개변수와 재귀적 바운드를 가진 RPITIT에 대한 회귀 (#102301) |
| `issue-102571.rs` | unsized 프로젝션과 타입 불일치를 가진 중첩 RPITIT에 대한 회귀 (#102571) |
| `late-bound-in-object-assocty.rs` | object-safe 연관 타입 내부 RPITIT 바운드에서 늦게 묶인 라이프타임에 대한 회귀 (#132429) |
| `lifetime-in-associated-trait-bound.rs` | RPITIT 연관 트레이트 바운드가 `&self`의 익명 라이프타임을 참조할 수 있음을 검증 |
| `method-compatability-via-leakage-cycle.rs` | 재귀적 RPITIT에서 불투명 타입 누출을 통해 메서드 호환성을 검사할 때 발생하는 알려진 순환 (#139788) |
| `method-compatability-via-leakage.rs` | 덜 제약된 RPITIT을 가진 구현이 트레이트의 `Send` 바운드와 호환됨을 검증 |
| `method-signature-matches.rs` | RPITIT/비동기 메서드의 매개변수 수, 인자 타입, 라이프타임 불일치를 감지 |
| `mismatched-where-clauses.rs` | RPITIT 메서드에 더 엄격한 `where` 절을 추가한 구현을 거부 |
| `missing-lt-outlives-in-rpitit-114274.rs` | RPITIT 바운드에 선언되지 않은 라이프타임을 거부 (#114274) |
| `missing-static-bound-from-impl.rs` | 클로저 트레이트의 RPITIT을 트레이트 객체로 지울 때 `'static` 바운드 누락을 감지 |
| `nested-rpitit-bounds.rs` | `Deref`와 기본 본문을 가진 중첩 RPITIT 바운드가 컴파일됨을 검증 |
| `nested-rpitit.rs` | `Deref`와 구체적 정제 반환 타입을 가진 중첩 RPITIT가 컴파일됨을 검증 |
| `not-inferred-generic.rs` | RPITIT 메서드의 제약되지 않은 제네릭 매개변수가 호출 지점에서 추론될 수 없음을 보고 |
| `opaque-and-lifetime-mismatch.rs` | RPITIT을 래핑할 때 라이프타임 지정자 누락 및 잘못된 제네릭 인자를 감지 |
| `opaque-in-impl-is-opaque.rs` | RPITIT 반환 타입이 불투명한 상태로 유지되어 구체적 타입으로 강제 변환되지 않음을 검증 |
| `opaque-in-impl.rs` | 제네릭 및 구체적 self 타입에 대한 간단한 RPITIT 구현이 컴파일됨을 검증 |
| `opaque-variances.rs` | RPITIT 라이프타임이 NLL에서 불필요하게 캡처되지 않음을 검증 |
| `outlives-in-nested-rpit.rs` | 중첩 RPITIT에서 초기/늦게 묶인 라이프타임의 outlives를 올바르게 처리함을 검증 |
| `placeholder-implied-bounds.rs` | RPITIT에 대한 참조를 반환할 때 placeholder implied bounds로 인한 ICE를 피함을 검증 |
| `refine-captures.rs` | 구현이 트레이트보다 적은 라이프타임을 캡처할 때 RPITIT 정제 경고/오류를 검증 |
| `refine-cycle.rs` | 상호 재귀적 RPITIT 구현에 대한 정제 검사가 쿼리 순환을 일으키지 않음을 검증 |
| `refine-err.rs` | 정제 진단이 정의되지 않은 타입 매개변수 등 일반적인 해결 오류를 계속 표시함을 검증 |
| `refine-normalize.rs` | 정규화된 연관 타입 RPITIT 바운드가 거짓 정제로 표시되지 않음을 검증 |
| `refine-resolution-errors.rs` | RPITIT 정제 검사 중 해결 오류로 ICE가 발생하던 회귀 (#126670)를 검증 |
| `refine-return-type-notation.rs` | 반환 타입 표기(return-type notation)가 RPITIT 정제 진단과 올바르게 상호작용함을 검증 |
| `refine.rs` | 명시적 옵트인 없이 반환 타입을 정제하는 RPITIT 구현을 거부 |
| `return-dont-satisfy-bounds.rs` | RPITIT 구현의 반환 타입이 원래 트레이트의 제네릭 바운드를 만족하지 않을 때 거부 |
| `return-type-notation.rs` | 반환 타입 표기가 중첩 RPITIT 바운드에서 사용될 수 있음을 검증 |
| `reveal.rs` | RPITIT 뒤의 구체적 타입이 알려진 래퍼를 통해 강제 변환될 때 드러남을 검증 |
| `rpitit-cycle-in-generics-of.rs` | 연관 항목을 참조하는 RPITIT의 제네릭 계산 시 쿼리 순환을 피함을 검증 |
| `rpitit-duplicate-associated-fn.rs` | 중복된 RPITIT 트레이트 메서드 정의 및 구현을 거부 (#140796) |
| `rpitit-duplicate-associated-fn-with-nested.rs` | 중첩 불투명 바운드를 가진 중복 RPITIT 메서드를 거부 (#143560) |
| `rpitit-hidden-types-self-implied-wf.rs` | RPITIT 숨겨진 타입의 unsound self-imposed 라이프타임 바운드를 감지 |
| `rpitit-hidden-types-self-implied-wf-via-param.rs` | 구현이 더 엄격한 라이프타임 바운드를 추가해 unsound RPITIT 숨겨진 타입을 초래하는 경우를 감지 |
| `rpitit-shadowed-by-missing-adt.rs` | RPITIT 바운드에 누락된 ADT가 있을 때 해결 오류를 처리 (#113903) |
| `shorthand-projection-in-rpitit-bound.rs` | RPITIT 바운드 내부 축약 프로젝션 표기가 올바르게 해결됨을 검증 |
| `sibling-function-constraint.rs` | 형제 함수가 다른 함수의 RPITIT 숨겨진 타입을 제약할 수 없음을 검증 |
| `signature-mismatch.rs` | 비동기 RPITIT 트레이트 시그니처와 구현 간 라이프타임 캡처 호환성을 검증 |
| `sized-rpits-dont-need-pointer-like.rs` | `Self: Sized`로 제한된 RPITIT 메서드가 트레이트를 dyn 비호환으로 만들지 않음을 검증 |
| `span-bug-issue-121457.rs` | 라이프타임 불일치를 가진 GAT를 참조하는 RPITIT의 span 버그에 대한 회귀 (#121457) |
| `specialization-broken.rs` | RPITIT 메서드의 기본 특수화에 대한 오류 처리를 검증 |
| `specialization-substs-remap.rs` | 특수화가 RPITIT에 대해 substs를 올바르게 재매핑함을 검증 |
| `success.rs` | 간단한 RPITIT 구현이 컴파일되고 정제되는 기본 양성 테스트 |
| `suggest-missing-item.rs` | 누락된 비동기 및 RPITIT 트레이트 항목에 대한 rustfix 제안을 검증 |
| `synthetic-hir-has-parent.rs` | RPITIT의 HIR 부모를 순회할 때 패닉이 발생하지 않음을 검증 |
| `trait-more-generics-than-impl.rs` | RPITIT 트레이트 메서드보다 적은 제네릭 매개변수를 가진 구현을 거부 |
| `unconstrained-lt.rs` | RPITIT 구현에서 제약되지 않은 라이프타임 매개변수를 감지 |
| `variance.rs` | 다른 캡처 목록을 가진 RPITIT 불투명 타입의 변성을 덤프 및 검증 |
| `variances-of-gat.rs` | RPITIT 자체가 제네릭 연관 타입일 때의 변성 처리를 검증 |
| `wf-bounds.rs` | RPITIT 바운드의 잘못된 중첩 타입이 거부됨을 검증 (#101663) |
| `where-clause.rs` | 트레이트 제네릭 매개변수에 `where` 절이 있는 RPITIT가 컴파일됨을 검증 |

## 타입 별칭 impl Trait (TAIT)

type alias impl Trait(TAIT) 정의, 정의 사용, 비공개 사용 등

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `async_scope_creep.rs` | TAIT/RPIT 비동기 스코프 크리프와 `define_opaque` 및 future 별칭 동작을 검증 |
| `bound-normalization-fail.rs` | `impl Trait` 반환 바운드 안에서 `T::Assoc`를 정규화하지 못하는 경우의 실패를 검증 (#60414) |
| `bound-normalization-pass.rs` | `impl Trait` 및 TAIT 바운드 안에서 `T::Assoc`와 프로젝션을 정규화하는 동작을 검증 (#60414) |
| `deduce-signature-from-supertrait.rs` | TAIT를 통해 슈퍼트레이트 바운드에서 클로저 시그니처를 추론하는 동작을 검증 |
| `define-via-const.rs` | 클로저 타입을 가진 const 항목으로 TAIT를 정의하는 경우를 검증 |
| `define-via-extern.rs` | `extern` 함수 선언에서 TAIT를 정의하려 할 때 오류가 발생함을 검증 |
| `eagerly-reveal-in-local-body.rs` | 함수 본문 내 지역 TAIT가 필드 접근을 위해 즉시 드러나는지 검증 |
| `equal-hidden-lifetimes.rs` | 불투명 타입이 숨겨진 라이프타임을 검사할 때 동일한 region을 고려하는지 검증 |
| `erased-regions-in-hidden-ty.rs` | 불투명 타입의 숨겨진 타입에 `ReErased`가 포함되는 경우를 검증 |
| `failed-to-resolve-instance-ice-105488.rs` | 재귀적 `impl MyFnOnce`와 함께 인스턴스 해결 실패로 ICE가 발생했던 회귀 (#105488)를 검증 |
| `failed-to-resolve-instance-ice-123145.rs` | 재귀적 `impl Handler`와 함께 인스턴스 해결 실패로 ICE가 발생했던 회귀 (#123145)를 검증 |
| `fallback_inference.rs` | 불투명 타입이 `PhantomData` 내부에 숨겨져 있을 때 타입 추론 폴백을 검증 |
| `fallback.rs` | `Option::map_or`을 사용하는 `impl Iterator` 반환에 대한 타입 추론 폴백을 검증 |
| `hidden-type-is-opaque-2.rs` | 함수 제네릭 매개변수를 통해 클로저에서 불투명 타입의 숨겨진 타입을 추론할 수 없음을 검증 |
| `hidden-type-is-opaque.rs` | 불투명 타입의 숨겨진 타입이 클로저 본문을 통해 올바르게 흐르는지 검증 |
| `lazy_subtyping_of_opaques.rs` | 숨겨진 타입을 제약하지 않고 불투명 타입을 포함한 하위 타입 추론을 검증 |
| `multiple-defining-usages-in-body.rs` | 하나의 본문에서 여러 정의 사용으로부터 충돌하는 구체적 타입이 오류로 처리되는지 검증 |
| `name-mentioning-macro.rs` | 트레이트 바운드에 타입 이름을 언급하는 매크로에서 확장된 불투명 타입을 검증 |
| `nested-return-type2-tait2.rs` | 연관 타입 바운드를 만족하지 못하는 TAIT 변형 중첩 반환 타입을 검증 |
| `nested-return-type2-tait3.rs` | 내부에 `impl Send`를 사용하는 TAIT 변형 중첩 반환 타입을 검증 |
| `nested-return-type2-tait.rs` | 바깥쪽 `impl Trait`에 TAIT 연관 바운드가 있는 중첩 반환 타입의 TAIT 변형을 검증 |
| `nested-return-type3-tait2.rs` | 내부 TAIT가 트레이트 타입과 동일한 중첩 반환 타입의 TAIT 변형을 검증 |
| `nested-return-type3-tait3.rs` | TAIT 내부에 중첩 `impl Send`가 있는 TAIT 변형을 검증 |
| `nested-return-type3-tait.rs` | 바깥쪽 `impl Trait`와 내부 TAIT가 있는 중첩 반환 타입의 TAIT 변형을 검증 |
| `no-anonymize-regions.rs` | 정의 사용의 구조적 동등성에서 익명화/비익명화 region 처리에 대한 회귀 (#139587)를 검증 |
| `normalize-opaque-with-bound-vars.rs` | 코드 생성 중 escaping bound vars를 가진 불투명 타입 정규화에 대한 회귀를 검증 |
| `private_unused.rs` | 사용되지 않는 비공개 타입 매개변수를 가진 불투명 타입이 경고하지 않음을 검증 |
| `recursive-ice-101862.rs` | non-universal region 치환이 있는 불투명 타입에서 ICE가 발생했던 회귀 (#101852)를 검증 |
| `recursive-in-exhaustiveness.rs` | 소모성 검사에서 재귀적 불투명 정의가 스택 오버플로우를 일으키지 않음을 검증 |
| `recursive-type-alias-impl-trait-declaration.rs` | `PartialEq<(Foo, i32)>`를 가진 재귀적 TAIT 선언을 검증 |
| `recursive-type-alias-impl-trait-declaration-too-subtle-2.rs` | 구현이 튜플 내부에 불투명 타입을 사용하는 재귀적 TAIT를 검증 |
| `recursive-type-alias-impl-trait-declaration-too-subtle.rs` | 미묘한 재귀적 TAIT 오류와 트레이트 메서드 호환성을 검증 |
| `reveal-during-codegen.rs` | 코드 생성 중 `Option<impl Sized>`로 불투명 타입이 드러나는지 검증 |
| `two_tait_defining_each_other2.rs` | 숨겨진 불투명 타입으로 서로를 정의하는 두 TAIT를 검증 |
| `two_tait_defining_each_other3.rs` | 조건부 및 구체적 반환과 함께 서로를 정의하는 두 TAIT를 검증 |
| `two_tait_defining_each_other.rs` | 혼합된 반환 및 인자 사용으로 서로를 정의하는 두 TAIT를 검증 |
| `type-alias-impl-trait-in-fn-body.rs` | 함수 본문 내부 TAIT 선언이 불투명하게 사용될 수 없음을 검증 |
| `unpin-for-future.rs` | TAIT를 사용한 컴파일러 생성 비동기 future에 `Unpin`을 구현할 수 없음을 검증 |
| `unsize_adt.rs` | 불투명 배열 `[impl Sized; N]`를 `[Concrete]`로 unsizing하는 ADT를 검증 |
| `unsize-cast-validation-rpit.rs` | RPIT 배열을 가진 unsizing 캐스트가 MIR에서 검증되는지 검증 |
| `unsized_coercion2.rs` | 불투명 타입의 unsizing 강제가 트레이트 객체로 제약되는 대신 발생하는지 검증(다음/이전 솔버) |
| `unsized_coercion3.rs` | `dyn Send` 트레이트 바운드 오류와 함께 불투명 타입 unsizing 강제를 검증 |
| `unsized_coercion4.rs` | `Box<u32>`로의 명시적 캐스트와 불투명 타입 unsizing 강제를 검증 |
| `unsized_coercion5.rs` | `Box<dyn Trait + Send>`로의 명시적 캐스트와 불투명 타입 unsizing 강제를 검증 |
| `unsized_coercion.rs` | 불투명 타입 unsizing 강제가 `Box<dyn Trait>`로 제약되는 대신 발생하는지 검증 |
| `unsize_slice.rs` | 불투명 슬라이스 `[impl Sized; N]`를 `[Concrete]`로 unsizing하는 경우를 검증 |
| `upvar_captures.rs` | ADT에 라이프타임 지정자가 누락되었을 때 클로저 upvar 캡처에 대한 회귀 (#123255)를 검증 |

### `non-defining-uses/`

TAIT 비공개 사용(non-defining use) 테스트

| 파일 | 설명 |
|------|------|
| `ambiguous-ops.rs` | 불투명 타입에 대한 비호출 연산(산술, 역참조, 인덱스)이 항목 바운드에 포함되지 않아도 현재 솔버 오류/다음 솔버 허용과 함께 지원됨을 검증 |
| `as-projection-term.rs` | 불투명 타입을 클로저 시그니처의 프로젝션 항으로 정규화할 때 추론 변수 숨겨진 타입이 잘못된 정의 사용으로 처리되지 않음을 검증 |
| `avoid-inference-constraints-from-blanket-2.rs` | 포괄 구현이 불투명 타입 인자를 불완전하게 제약해 예상치 못한 타입 불일치를 일으키지 않음을 검증하는 회귀 테스트 |
| `avoid-inference-constraints-from-blanket-3.rs` | 포괄 구현의 중첩 where 바운드가 불투명 타입에 적용되지 않을 때 불만족 트레이트 바운드로 제약하지 않음을 검증하는 회귀 테스트 |
| `avoid-inference-constraints-from-blanket.rs` | 비공개 사용에서 포괄 구현이 불투명 타입을 포괄 타입으로 제약하지 않음을 검증하는 회귀 테스트 |
| `call-expr-incorrect-choice-diagnostics.rs` | 기대한 함수형 트레이트를 구현하지 않는 불투명 타입을 호출할 때 생성되는 진단을 검증 |
| `deref-constrains-self-ty.rs` | 아직 정의되지 않은 불투명 타입이 대부분 경직되게 처리되더라도 자동 역참조가 불투명 타입을 제약할 수 있음을 보임 |
| `double-wrap-with-defining-use.rs` | 불투명 타입이 정의 사용으로 두 번 래핑될 때 ICE(#140545)가 발생하던 회귀를 검증 |
| `function-call-on-infer.rs` | 비공개 사용에서 불투명 `Fn`, `FnMut`, `FnOnce` 타입을 호출하는 동작을 검증하는 회귀 테스트 |
| `ice-issue-146191.rs` | 중첩 `impl Future`와 `impl Trait` 바운드를 가진 재귀적 비동기 블록에서 ICE(#146191)가 발생하던 회귀를 검증 |
| `impl-deref-function-call.rs` | 정의 범위 내부에서 `impl Deref`를 통해 함수형 불투명 타입에 대한 호출이 동작함을 검증하는 회귀 테스트 |
| `multiple-opaques-ambig.rs` | 여러 불투명 타입이 서로 다른 항목 바운드를 산출할 때 모호한 바운드가 적용되지 않음을 검증하는 회귀 테스트 |
| `multiple-opaques-ok.rs` | 하나 이상의 불투명 타입에 대한 하위 통합 시에도 적용 가능한 항목 바운드를 사용함을 검증하는 회귀 테스트 |
| `no-rigid-alias.rs` | 트레이트 바운드만 있는 불투명 타입이 연관 타입의 경직된 별칭으로 잘못 정규화되지 않음을 검증 |
| `recursive-call.rs` | 불투명 `self` 타입을 반환하는 재귀적 메서드 호출이 비공개 사용 오류를 일으키지 않음을 검증하는 회귀 테스트 |
| `shex_compat-regression-test.rs` | pretty-printer 메서드에서 불투명 클로저 타입의 재귀적 클로저 호출에 대한 회귀 테스트 |
| `use-blanket-impl.rs` | 항목 바운드와 포괄 구현이 함께 불투명 이터레이터의 요소 타입을 조급히 추론함을 검증하는 회귀 테스트 |
| `use-item-bound-over-blanket-impl.rs` | 불투명 타입의 연관 타입을 정규화할 때 항목 바운드가 포괄 구현보다 우선함을 검증하는 회귀 테스트 |
| `use-item-bound.rs` | 메서드 해상도 전에 항목 바운드가 불투명 타입의 연관 타입을 조급히 추론함을 검증하는 회귀 테스트 |

## 라이프타임과 멤버 제약

impl Trait의 라이프타임 캡처, HRTB, 멤버 제약, 변성 등

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `bivariant-lifetime-liveness.rs` | 불투명 타입에 캡처되지 않은 라이프타임이 살아 있어야 할 필요가 없음을 확인하는 회귀 (#116794) 테스트 |
| `captured-invalid-lifetime.rs` | 불투명 타입에 중복되거나 섀도잉된 라이프타임 `'a`가 ICE를 일으키던 회귀를 검증 |
| `capture-lifetime-not-in-hir.rs` | HIR에 없는 불변 라이프타임을 캡처하는 불투명 타입의 변성 덤프를 검증 |
| `defining-use-captured-non-universal-region.rs` | 캡처된 non-universal region의 정의 사용에서 제네릭 라이프타임 인자를 다루는 회귀 (#110726)를 검증 |
| `defining-use-uncaptured-non-universal-region-2.rs` | 캡처되지 않은 라이프타임이 재귀적 `impl Iterator` 반환에 있는 회귀 (#110623)를 검증 |
| `defining-use-uncaptured-non-universal-region-3.rs` | 불투명 타입에 캡처되지 않은 const 제네릭 라이프타임 매개변수를 다루는 동작을 검증 |
| `defining-use-uncaptured-non-universal-region.rs` | 재귀적 `impl Sized`에서 캡처되지 않은 non-universal region에 대한 회귀 (#111906)를 검증 |
| `does-not-live-long-enough.rs` | 로컬 참조를 캡처하는 `impl Iterator` 반환 시 라이프타임 오류가 보고되는지 검증 |
| `fresh-lifetime-from-bare-trait-obj-114664.rs` | RPIT의 bare 트레이트 객체에서 새 라이프타임이 생성되는 회귀 (#114664)를 검증 |
| `future-no-bound-vars-ice-112347.rs` | 높은 순위 바운드가 있는 TAIT에서 "future has no bound vars" ICE가 발생했던 회귀 (#112347)를 검증 |
| `generic-with-implicit-hrtb-without-dyn.rs` | 2015/2021 에디션에서 bare 트레이트 객체의 RPIT에 암시적 HRTB가 동작하는지 검증 |
| `higher-ranked-lifetime-capture-deduplication.rs` | 바깥쪽 RPIT에서 높은 순위 라이프타임을 캡처하는 중첩 `impl Trait`에 대한 오류를 검증 |
| `impl-fn-hrtb-bounds-2.rs` | 에디션 간 중첩 `impl Fn` 반환에서 높은 순위 라이프타임 캡처를 검증 |
| `impl-fn-hrtb-bounds.rs` | 중첩 `impl Fn` 반환에서 높은 순위 라이프타임 캡처 오류를 검증 |
| `impl-fn-predefined-lifetimes.rs` | 미리 정의된 라이프타임 `'a`와 `'_`를 가진 중첩 `impl Fn` 반환을 검증 |
| `implicit-capture-late.rs` | `dyn for<'a> Deref<Target = impl ?Sized>` 반환에서 늦게 묶인(late-bound) 라이프타임을 암시적으로 캡처하는 동작을 검증 |
| `lifetime-ambiguity-regression.rs` | 중첩 `impl Trait` 이터레이터의 숨겨진 타입에서 라이프타임 모호성을 검증 |
| `lifetimes.rs` | RPIT, HRTB, 클로저 캡처, region 바운드에 대한 종합 라이프타임 테스트 |
| `mapping-duplicated-lifetimes-issue-114597.rs` | 비동기 RPIT에서 중복된 라이프타임 매핑에 대한 회귀 (#114597)를 검증 |
| `must_outlive_least_region_or_bound.rs` | 불투명 타입이 필요한 바운드만큼 오래 살지 못하는 라이프타임을 캡처할 때 오류를 검증 |
| `needs_least_region_or_bound.rs` | 여러 라이프타임을 가진 불투명 타입 반환 시 최소 region이나 명시적 바운드를 찾는지 검증 |
| `nested-hkl-lifetime.rs` | 중첩 `impl Fn` 파서 컴비네이터에서 높은 순위 라이프타임 처리를 검증 |
| `nested-rpit-hrtb-2.rs` | 높은 순위 라이프타임을 참조하는 중첩 RPIT을 검증 |
| `nested-rpit-hrtb.rs` | 중첩 RPIT과 높은 순위 트레이트 바운드 간 상호작용을 종합적으로 검증 |
| `nested-rpit-with-anonymous-lifetimes.rs` | 이터레이터에서 익명 라이프타임 참조를 가진 중첩 RPIT을 검증 |
| `opt-out-bound-not-satisfied.rs` | 비Sized 트레이트에 `?` opt-out 바운드가 있는 RPIT에 대한 회귀 (#135730)를 검증 |
| `printing-binder.rs` | 불투명 타입 진단에서 높은 순위 바인더 출력을 검증 |
| `region-escape-via-bound-contravariant-closure.rs` | 반공변 클로저가 RPIT 바운드를 통해 region escape를 허용하는지 검증 |
| `region-escape-via-bound-contravariant.rs` | 반공변 참조가 RPIT 바운드를 통해 region escape를 허용하는지 검증 |
| `region-escape-via-bound.rs` | 불변 셀이 바깥 region이 escape하더라도 region escape를 거부하는지 검증 (#46541) |
| `static-return-lifetime-infered.rs` | `impl Iterator` 캡처에 대한 정적 라이프타임 요구 추론 오류를 검증 |
| `type_parameters_captured.rs` | RPIT에서 타입 매개변수가 캡처되고 `'static`으로 가정되지 않음을 검증 |
| `universal_hrtb_anon.rs` | 익명 참조를 가진 `impl Fn` 매개변수에서 보편적 HRTB를 검증 |
| `universal_hrtb_named.rs` | 명명된 참조를 가진 `impl Fn` 매개변수에서 보편적 HRTB를 검증 |
| `variance.rs` | 초기/늦은 바인딩 및 캡처/비캡처 경우의 불투명 타입 라이프타임 변성을 검증 |

## 자동 트레이트와 누출

자동 트레이트(Send, Sync 등) 추론 및 누출(leakage) 검사

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `auto-trait-selection-freeze.rs` | 불투명 타입에서 `Freeze` 자동 트레이트 선택 실패가 이후 코드 경로에 영향을 미치는지 검증 |
| `auto-trait-selection.rs` | 불투명 타입에서 `Send` 자동 트레이트 선택 실패가 이후 코드 경로에 영향을 미치는지 검증 |
| `negative-reasoning.rs` | 불투명 타입이 다른 트레이트를 구현하지 않는다고 가정할 수 없음을 검증 |

## 메서드 해상도

불투명(opaque) 타입에서의 메서드 탐색 및 해상도

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `autoderef.rs` | `use<'_>`를 가진 불투명 타입에 대한 자동 역참조가 현재/다음 솔버에서 동작하는지 검증 |
| `call_method_ambiguous.rs` | 재귀 호출과 `use<'_>` 라이프타임 캡처가 있는 RPIT에서 메서드 해상도를 검증 |
| `call_method_on_inherent_impl_on_rigid_type.rs` | 경직된 타입 뒤의 불투명 참조에서 메서드를 찾을 수 없음을 검증(현재/다음 솔버) |
| `call_method_on_inherent_impl_ref-err.rs` | 임포트 없이 `&impl Debug`에 대한 트레이트 구현 메서드 조회가 실패함을 검증 |
| `call_method_on_inherent_impl_ref-ok.rs` | 임포트 시 `&impl Debug`에 대한 트레이트 구현 메서드 조회가 성공함을 검증 |
| `call_method_on_inherent_impl.rs` | `impl Debug`에 대한 포괄 트레이트 구현 메서드 조회가 성공함을 검증 |
| `call_method_without_import.rs` | 불투명 타입이 트레이트를 임포트한 경우에만 해당 트레이트 메서드를 인식함을 검증 |
| `no-method-suggested-traits.rs` | 메서드를 찾을 수 없을 때 제안된 트레이트 임포트 진단을 검증 |
| `recursive-parent-trait-method-call.rs` | `FutureExt::boxed`를 사용한 재귀적 비동기 future의 메서드 해상도를 검증 |

## 정밀 캡처 (precise capturing)

use<...>를 이용한 정밀 라이프타임/타입/const 캡처

### `precise-capturing/`

정밀 캡처 use<...> 테스트

| 파일 | 설명 |
|------|------|
| `apit.rs` | 인자 위치 `impl Trait`에서는 `use<...>` 정밀 캡처 문법이 허용되지 않음을 검증 |
| `bad-lifetimes.rs` | elided `'_`, `'static`, 선언되지 않은 이름 등 `use<...>`의 잘못된 라이프타임 매개변수를 거부 |
| `bad-params.rs` | 누락된 타입/const 매개변수, `Self` 별칭, 함수, 지역 변수 등 `use<...>`의 잘못된 매개변수를 거부 |
| `bound-modifiers.rs` | `?`, `async`, `const`, `for<>` 같은 바운드 수식자를 `use<...>`에 적용할 수 없음을 검증 |
| `capture-parent-arg.rs` | 부모 라이프타임을 `use<...>`로 캡처하는 동작과 필요한 라이프타임 캡처 실패 시 오류를 검증 |
| `capturing-implicit.rs` | 명시적 `use<>`가 있어도 중첩 반환 위치 `impl Trait`에서 암시적 라이프타임이 여전히 캡처됨을 검증 |
| `duplicated-use.rs` | 동일한 바운드 목록에 `use<...>` 정밀 캡처를 중복 사용하는 것을 거부 |
| `dyn-use.rs` | `dyn` 트레이트 객체 바운드에서 `use<...>`를 사용할 수 없음을 검증 |
| `elided.rs` | 생략된 라이프타임 `'_`가 `use<...>` 내부에서 사용될 수 있음을 검증 |
| `external-macro.rs` | 외부 매크로에서 생성된 반환 위치 `impl Trait` 코드가 Rust 2024 호출 지점에서 동작함을 검증 |
| `foreign-2021.rs` | Rust 2021 크레이트의 반환 위치 `impl Trait`가 Rust 2024에서 의도보다 많은 라이프타임을 캡처할 수 있음을 검증 |
| `forgot-to-capture-const.rs` | 범위 내 const 매개변수가 `use<...>`에 언급되어야 함을 검증 |
| `forgot-to-capture-lifetime.rs` | 숨겨진 타입 또는 바운드 목록이 캡처하는 라이프타임 매개변수가 `use<...>`에 언급되어야 함을 검증 |
| `forgot-to-capture-type.rs` | 타입 매개변수와 트레이트 `Self` 타입이 `use<...>`에 언급되어야 함을 검증 |
| `hidden-type-suggestion.rs` | 숨겨진 타입이 라이프타임을 캡처할 때 누락된 라이프타임이나 `use<...>` 바운드 추가를 제안하는 진단을 검증 |
| `higher-ranked.rs` | 중첩 반환 위치 `impl Trait`의 높은 순위 라이프타임 캡처를 `use<>`로 건너뛸 수 있음을 검증 |
| `illegal-positions.rs` | `use<...>`가 반환 위치 `impl Trait`에서만 허용되고 트레이트 바운드, 제네릭 바운드, `dyn` 타입에서는 허용되지 않음을 검증 |
| `migration-note.rs` | Rust 2024 라이프타임 캡처 규칙으로 인한 대여 오류에 대한 마이그레이션 린트 설명을 검증 |
| `ordering.rs` | `use<...>`가 중복 매개변수를 거부하고 라이프타임을 비라이프타임 매개변수 앞에 나열하도록 요구함을 검증 |
| `outlives.rs` | 정밀 캡처를 통해 불투명 타입이 outlives 요구를 만족하면서 라이프타임 매개변수를 건너뛸 수 있음을 검증 |
| `overcaptures-2024-but-fine.rs` | Rust 2024에서 호출 지점에서 라이프타임을 단축할 수 있어 일부 라이프타임 과잉 캡처가 허용됨을 검증 |
| `overcaptures-2024-machine-applicable.rs` | 단순한 라이프타임 과잉 캡처 경우에 기계 적용 가능한 rustfix 제안을 생성함을 검증 |
| `overcaptures-2024.rs` | Rust 2024에서 반환 위치 `impl Trait`이 라이프타임을 과잉 캡처할 때 `impl_trait_overcaptures` 린트가 경고함을 검증 |
| `parenthesized.rs` | 괄호로 묶인 `use<...>` 바운드를 거부 |
| `redundant-machine-applicable.rs` | 기계 적용 가능한 rustfix가 중복 `use<...>`를 올바른 인접 `+` 연결자와 함께 제거함을 검증 |
| `redundant.rs` | 이미 범위 내 모든 매개변수를 캡처하는 중복 `use<...>` 바운드에 린트를 적용 |
| `rpitit-captures-more-method-lifetimes.rs` | RPITIT 구현이 트레이트 정의보다 많은 메서드 라이프타임을 캡처할 수 없음을 검증 |
| `rpitit-impl-captures-too-much.rs` | `'_`를 통해 트레이트 정의보다 많은 라이프타임을 캡처하는 RPITIT 구현을 거부 |
| `rpitit-outlives-2.rs` | 캡처되지 않은 RPITIT 인자가 `'static`에 대한 outlives 계산에서 건너뛰어짐을 검증 |
| `rpitit-outlives.rs` | 캡처되지 않은 RPITIT 인자가 불투명 추론 시 멤버 제약 region 수집에서 건너뛰어짐을 검증 |
| `rpitit.rs` | RPITIT이 트레이트 선택에 영향을 주는 트레이트 헤더 라이프타임을 계속 캡처해야 함을 검증 |
| `self-capture.rs` | 트레이트 메서드 반환 타입 내 `use<...>`에서 `Self`를 캡처할 수 있음을 검증 |
| `unexpected-token.rs` | `use<...>` 내부 예상치 못한 토큰이 치명 오류가 아닌 유용한 파스 오류를 생성함을 검증 |

## 컴파일 타임 평가

const 문맥에서 impl Trait가 동작하는 방식

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `closure-in-impl-trait-arg.rs` | 익명 const 내부의 클로저가 `impl Trait` 인자로 사용되는 경우를 검증 |
| `closure-in-impl-trait.rs` | 익명 const 내부의 클로저가 `impl Trait` 반환 타입으로 사용되는 경우를 검증 |
| `closure-in-type.rs` | 익명 const 내부의 참조를 포함한 클로저가 `impl Trait` 타입으로 사용되는 경우를 검증 |
| `ice-148622-opaque-as-const-generics.rs` | 불투명 타입을 const 제네릭 매개변수로 사용할 때 ICE가 발생했던 회귀 (#148622)를 검증 |
| `inside-item-nested-in-anon-const.rs` | 타입 별칭의 익명 const 내부 `impl Trait`가 TAIT/ATPIT로 처리되지 않음을 검증 (#139055) |
| `normalize-tait-in-const.rs` | const 문맥에서 TAIT를 정규화하는 동작에 대한 회귀 (#103507)를 검증 |
| `opaque-cast-field-access-in-future.rs` | 비동기 블록 내부 불투명 캐스트를 통한 필드 접근 시 타입 어노테이션이 필요함을 검증 |
| `struct-field-fragment-in-name.rs` | 익명 const 트레이트 인자 내부 구조체 필드 조각과 `impl Trait`를 검증 |

## 회귀 테스트 모음

특정 이슈 번호에 대한 회귀 테스트 모음

### `issues/`

이슈 번호별 회귀 테스트

| 파일 | 설명 |
|------|------|
| `fuzzer-ice-issue-112201.rs` | 죽은 코드의 재귀적 불투명 타입 정의 사용이 ICE를 일으키던 문제의 회귀 (#112201) |
| `infinite-impl-trait-issue-38064.rs` | 상호 재귀적 `impl Trait` 반환 타입이 무한 크기 타입 대신 정중히 거부됨을 검증 (#38064) |
| `issue-100075-2.rs` | 제네릭 재귀적 `impl Sized` 함수가 불투명 반환 타입을 해결할 수 없음을 검증 |
| `issue-100075.rs` | `&'static T`/`Option`를 가진 재귀적 `impl Marker` 함수가 불투명 반환 타입을 해결할 수 없음을 검증 |
| `issue-100187.rs` | 라이프타임 바운드 트레이트 프로젝션을 가진 중첩 `impl Sized` 검사가 통과함을 검증 |
| `issue-102605.rs` | 잘못된 `main` 반환 타입과 비동기 함수 결과 불일치가 적절한 오류를 생성함을 검증 |
| `issue-103181-1.rs` | 중첩 비동기 `impl Future`에서 누락된 `HttpBody::Error` 연관 타입으로 ICE가 발생하던 문제의 회귀 |
| `issue-103181-2.rs` | 비동기 함수에서 `Send` future 출력을 정규화할 때 해결되지 않은 식별자 오류를 보고 |
| `issue-103599.rs` | `impl T` 반환을 가진 재귀적 `wrap(wrap(x))`가 불투명 타입을 해결할 수 없음을 검증 |
| `issue-104815.rs` | 라이프타임과 boxed 참조를 가진 `impl Resolver` 및 `impl Iterator` 검사가 통과함을 검증 |
| `issue-105826.rs` | 라이프타임이 있는 중첩 구조체를 가진 가변 `impl Write` 반환 검사가 통과함을 검증 |
| `issue-108591.rs` | 가변 참조와 `impl Sized`에서 불투명 타입 별칭 및 라이프타임 바운드 검사가 통과함을 검증 |
| `issue-108592.rs` | 라이프타임 및 `static` 클로저 캡처를 가진 불투명 타입 별칭 검사가 통과함을 검증 |
| `issue-21659-show-relevant-trait-impls-3.rs` | 구조체에서 메서드를 찾을 수 없을 때 관련 트레이트 구현을 표시하는 진단을 검증 (#21659) |
| `issue-35668.rs` | `impl Iterator` 본문 내부의 잘못된 곱셈이 타입 오류를 생성함을 검증 |
| `issue-36792.rs` | 자기 자신을 반환하는 재귀 함수의 `impl Copy`가 성공적으로 실행됨을 검증 |
| `issue-42479.rs` | `self` 필드 참조를 반복하는 `impl Iterator` 메서드 검사가 통과함을 검증 |
| `issue-46959.rs` | `deny(non_camel_case_types)`와 함께 간단한 `impl Iterator` map 검사가 통과함을 검증 |
| `issue-49376.rs` | 자기 참조적 `impl Trait`과 연산자 트레이트 바운드의 중첩이 스택 오버플로우를 일으키지 않음을 검증 |
| `issue-49556.rs` | 라이프타임이 있는 `impl Iterator` 반환의 연결된 iterator map 검사가 통과함을 검증 |
| `issue-49579.rs` | 클로저를 사용한 `impl Iterator` 피보나치 이터레이터 검사가 통과함을 검증 |
| `issue-49685.rs` | drop 정리로 `impl Trait` 이터레이터 반환이 드러날 때 ICE가 발생하던 문제의 회귀 |
| `issue-51185.rs` | `impl Into<for<'a> fn(&'a ())>`를 통한 변환이 성공적으로 실행됨을 검증 |
| `issue-52128.rs` | 라이프타임이 있는 `BTreeMap` 범위를 반복하는 중첩 `impl Iterator` 반환 검사가 통과함을 검증 |
| `issue-53457.rs` | `Clone` 클로저를 캡처하는 TAIT 타입 별칭 검사가 통과함을 검증 |
| `issue-54600.rs` | `Option` 변수 바인딩의 타입에 `impl Trait`를 사용할 수 없음을 검증 |
| `issue-54840.rs` | 참조 변수 바인딩의 타입에 `impl Trait`를 사용할 수 없음을 검증 |
| `issue-54895.rs` | 바깥쪽 `impl Trait`에서 높은 순위 라이프타임을 캡처하는 중첩 `impl Trait`를 거부 |
| `issue-54966.rs` | 알 수 없는 `Oper<impl FnMut()>` 반환이 ICE를 일으키던 문제의 회귀 |
| `issue-55608-captures-empty-region.rs` | 숨겨진 빈 region을 캡처하는 `impl Trait`가 ICE를 일으키던 문제의 회귀 |
| `issue-55872-1.rs` | 구체적 TAIT 연관 타입에서 사용되지 않은 타입 매개변수가 트레이트 바운드 오류를 일으킴을 검증 |
| `issue-55872-2.rs` | `Send` 바운드가 있는 TAIT 연관 타입의 비동기 블록에서 사용되지 않은 타입 매개변수 오류를 검증 |
| `issue-55872-3.rs` | `Copy` 바운드가 있는 TAIT 연관 타입의 비동기 블록에서 오류를 검증 |
| `issue-55872.rs` | TAIT 연관 타입의 클로저가 사용되지 않은 타입 매개변수를 누출함을 검증 |
| `issue-56445.rs` | 라이프타임 매개변수가 있는 구조체의 배열 내 `const fn size()` 검사가 통과함을 검증 |
| `issue-57464-unexpected-regions.rs` | `ReScope` region을 가진 불투명 타입을 생성하는 클로저가 적절히 검사되고 `Send`임을 검증 |
| `issue-57979-deeply-nested-impl-trait-in-assoc-proj.rs` | 연관 타입 프로젝션 깊이 중첩된 `impl Trait`가 허용되지 않음을 검증 |
| `issue-57979-impl-trait-in-path.rs` | 연관 타입 프로젝션의 경로 구성 요소 내부에 `impl Trait`가 허용되지 않음을 검증 |
| `issue-57979-nested-impl-trait-in-assoc-proj.rs` | 연관 타입 프로젝션 내부에 중첩된 `impl Trait`가 허용되지 않음을 검증 |
| `issue-58504.rs` | 코루틴 변수 바인딩의 배열 타입에 `impl Trait`를 사용할 수 없음을 검증 |
| `issue-58956.rs` | const 타입 또는 const 내부 변수 바인딩의 타입에 `impl Trait`를 사용할 수 없음을 검증 |
| `issue-62742.rs` | 일치하지 않는 raw 트레이트 구현을 가진 타입 별칭이 미충족 트레이트 바운드를 보고함을 검증 |
| `issue-65581.rs` | 중첩 `impl Trait` 바운드와 재귀적 연관 트레이트 프로젝션이 통과함을 검증 |
| `issue-67830.rs` | 사용자 정의 트레이트에서 바깥쪽 `impl Trait`의 높은 순위 라이프타임을 캡처하는 중첩 `impl Trait`를 거부 |
| `issue-68532.rs` | `Self`를 참조하는 impl 내부 const 제네릭 배열 크기 검사가 통과함을 검증 |
| `issue-70877.rs` | 다른 불투명 타입으로 숨겨진 불투명 타입과 제약되지 않은 TAIT 항목이 오류를 생성함을 검증 |
| `issue-70971.rs` | 변수 바인딩의 튜플 타입 내부에 `impl Trait`가 허용되지 않음을 검증 |
| `issue-72911.rs` | 해결되지 않은 모듈 경로를 가진 `impl Iterator` 함수가 ICE를 일으키던 문제의 회귀 |
| `issue-74282.rs` | 익명 구조체의 TAIT 클로저 타입 불일치가 적절한 오류를 반환함을 검증 |
| `issue-77987.rs` | 재귀적 `impl Foo<TAIT>` 반환이 ICE를 일으키던 문제의 회귀 |
| `issue-78722-2.rs` | 익명 const 외부에 선언된 불투명 타입의 숨겨진 타입을 등록할 수 없음을 검증 |
| `issue-78722.rs` | const 제네릭 배열 길이 내부 TAIT에 잘못된 비동기 반환 타입이 future 출력 불일치를 생성함을 검증 |
| `issue-79099.rs` | const 제네릭 배열 길이 바인딩에서 `impl Future`가 에디션별로 적절한 오류를 생성함을 검증 |
| `issue-82139.rs` | `impl Trait` 연관 타입이 함수 본문 내부의 해결되지 않은 값 이름을 숨김을 검증 |
| `issue-83919.rs` | 비동기 메서드의 중첩 `impl Future` 연관 타입이 본문이 future가 아닐 때 ICE를 일으키던 문제의 회귀 |
| `issue-83929-impl-trait-in-generic-default.rs` | 제네릭 매개변수 기본값에 `impl Trait`를 사용할 수 없음을 검증 |
| `issue-84073.rs` | 복잡한 클로저/트레이트 설정에서 `_`를 `Option<_>`에 할당할 때 오버플로우를 검증 |
| `issue-84919.rs` | 변수 바인딩의 타입에 `impl Trait`를 사용할 수 없음을 검증 |
| `issue-86201.rs` | `Fn` 트레이트에 대한 TAIT 타입 별칭이 연관 출력을 통해 호출될 수 있음을 검증 |
| `issue-86642.rs` | 정적 변수의 타입에 `impl Trait`를 사용할 수 없음을 검증 |
| `issue-86719.rs` | 연관 타입의 잘못된 `impl ;`과 누락된 트레이트 멤버에 대해 적절한 오류를 생성함을 검증 |
| `issue-86800.rs` | 함수 반환 및 비동기 메서드의 TAIT future 별칭이 불투명 타입을 제약하지 못함을 검증 |
| `issue-87295.rs` | 구조체 변수 바인딩의 타입에 `impl Trait`를 사용할 수 없음을 검증 |
| `issue-87340.rs` | 제네릭 impl의 `impl Trait` 연관 타입이 타입 매개변수가 제약되지 않았다고 오류를 생성함을 검증 |
| `issue-87450.rs` | 깊이 재귀적인 `impl Fn()` 반환이 불투명 타입을 해결할 수 없음을 검증 |
| `issue-88236-2.rs` | 바깥쪽 `impl Trait`의 높은 순위 라이프타임을 캡처하는 중첩 `impl Trait` 변형이 올바르게 오류를 생성함을 검증(이전 스택 오버플로우) |
| `issue-88236.rs` | 바깥쪽 `impl Trait`의 높은 순위 라이프타임을 캡처하는 중첩 `impl Trait`가 올바르게 오류를 생성함을 검증(이전 스택 오버플로우) |
| `issue-89312.rs` | 라이프타임 바운드 TAIT 별칭이 클로저 인자로 사용될 때 검사가 통과함을 검증 |
| `issue-92305.rs` | `impl Iterator<Item = Vec>` 반환 내부의 누락된 `Vec` 제네릭을 보고 |
| `issue-93788.rs` | 연관 타입을 가진 HRTB 트레이트가 `while let` 루프에서 통과함을 검증 |
| `issue-99073-2.rs` | `impl Display` 제네릭 함수에 명시적 타입 인자를 전달하는 것을 거부 |
| `issue-99073.rs` | 제네릭 매개변수가 `&F`로 대체될 때 `impl Fn` 반환을 가진 재귀적 `fix` 콤비네이터 오류를 검증 |
| `issue-99348-impl-compatibility.rs` | 연관 타입 제약의 TAIT이 impl 호환성 타입 불일치를 일으킴을 검증 |
| `issue-99642-2.rs` | `Box<dyn Iterator>`를 통한 `impl Iterator<Item = impl Sized>`의 TAIT 변형 검사가 통과함을 검증 |
| `issue-99642.rs` | `Box<dyn Iterator>`를 통한 중첩 `impl Iterator<Item = impl Sized>` 검사가 통과함을 검증 |
| `issue-99914.rs` | 비동기 함수를 반환하는 클로저의 `impl Trait` 반환 타입 추론이 타입 불일치를 발생시킴을 검증 |
| `type-error-post-normalization-test.rs` | TAIT 정의 사용 불일치가 MIR `PostAnalysisNormalize` 패스에서 `{type_error}`를 주입하지 않음을 검증 |

## 기타

기타 주제 및 진단 메시지 테스트

### (루트 레벨 테스트)

| 파일 | 설명 |
|------|------|
| `deprecated_annotation.rs` | 트레이트와 타입이 모두 deprecated된 불투명 타입을 반환하는 경우를 검증 |
| `different_where_bounds.rs` | 동일한 where 바운드에 대한 매개변수 환경 정규화 캐시 일관성을 검증 |
| `equality2.rs` | `impl Trait` 및 연관 타입 누출과 함께 동등성 및 특수화를 검증 |
| `equality-in-canonical-query.rs` | 정규 트레이트 쿼리에서 RPIT 동등성에 대한 회귀 (#116877)를 검증 |
| `equality-rpass.rs` | `impl Trait` 및 특수화에 대한 런패스 동등성 테스트 |
| `equality.rs` | `impl Trait` 및 특수화에 대한 타입 동등성 검사 |
| `example-calendar.rs` | 중첩된 `impl Trait` 이터레이터를 사용하는 큰 통합 예제(달력 포맷팅) |
| `example-st.rs` | `impl FnMut(&mut State) -> Result<U, Error>`를 사용하는 상태 모나드 유사 예제 |
| `impl-subtyper2.rs` | `Option<impl Iterator>`와 `None` 간 하위 타입 관계를 검증 |
| `impl-subtyper.rs` | 연결된 이터레이터 어댑터 간 `impl Iterator` 하위 타입 관계를 검증 |

### `Table of Contents`

Table of Contents

| 파일 | 설명 |
|------|------|

### ``apit/``

`apit/`

| 파일 | 설명 |
|------|------|
| `arg-position-impl-trait-too-long.rs` | 번역 필요 |
| `impl-generic-mismatch-ab.rs` | 번역 필요 |

### ``auto-trait-leakage/``

`auto-trait-leakage/`

| 파일 | 설명 |
|------|------|
| `auto-trait-coherence.rs` | 번역 필요 |
| `auto-trait-contains-err.rs` | 번역 필요 |
| `auto-trait-leak.rs` | 번역 필요 |
| `auto-trait-leak-rpass.rs` | 번역 필요 |
| `auto-trait-leak2.rs` | 번역 필요 |
| `avoid-query-cycle-via-item-bound.rs` | 번역 필요 |

### ``diagnostics/``

`diagnostics/`

| 파일 | 설명 |
|------|------|
| `fully-qualified-path-impl-trait.rs` | 번역 필요 |
| `highlight-difference-between-expected-trait-and-found-trait.rs` | 번역 필요 |

### ``explicit-generic-args-with-impl-trait/``

`explicit-generic-args-with-impl-trait/`

| 파일 | 설명 |
|------|------|
| `const-args.rs` | 번역 필요 |
| `explicit-generic-args-for-impl.rs` | 번역 필요 |
| `explicit-generic-args.rs` | 번역 필요 |
| `issue-87718.rs` | 번역 필요 |
| `not-enough-args.rs` | 번역 필요 |

### ``in-bindings/``

`in-bindings/`

| 파일 | 설명 |
|------|------|
| `bad-nesting.rs` | 번역 필요 |
| `dont-make-def-id.rs` | 번역 필요 |
| `escaping-bound-var.rs` | 번역 필요 |
| `implicit-sized.rs` | 번역 필요 |
| `lifetime-equality.rs` | 번역 필요 |
| `lifetime-failure.rs` | 번역 필요 |
| `nesting-lifetime-failure.rs` | 번역 필요 |
| `nesting.rs` | 번역 필요 |
| `region-lifetimes.rs` | 번역 필요 |
| `simple.rs` | 번역 필요 |
| `trait-failure.rs` | 번역 필요 |

### ``in-ctfe/``

`in-ctfe/`

| 파일 | 설명 |
|------|------|
| `array-len-size-of.rs` | 번역 필요 |
| `array-len.rs` | 번역 필요 |
| `enum-discr.rs` | 번역 필요 |
| `fully_monomorphic_const_eval.rs` | 번역 필요 |
| `match-arm-exhaustive.rs` | 번역 필요 |

### ``method/``

`method/`

| 파일 | 설명 |
|------|------|
| `broken-deref-chain.rs` | 번역 필요 |
| `method-resolution.rs` | 번역 필요 |
| `method-resolution2.rs` | 번역 필요 |
| `method-resolution3.rs` | 번역 필요 |
| `method-resolution4.rs` | 번역 필요 |
| `method-resolution5-deref-no-constrain.rs` | 번역 필요 |
| `method-resolution5-deref.rs` | 번역 필요 |
| `would-constrain-opaque.rs` | 번역 필요 |

### ``member-constraints/``

`member-constraints/`

| 파일 | 설명 |
|------|------|
| `apply_member_constraint-no-req-eq.rs` | 번역 필요 |
| `incomplete-constraint.rs` | 번역 필요 |
| `min-choice-reject-ambiguous.rs` | 번역 필요 |
| `min-choice.rs` | 번역 필요 |
| `nested-impl-trait-fail.rs` | 번역 필요 |
| `nested-impl-trait-pass.rs` | 번역 필요 |
| `placeholders_lift_to_static.rs` | 번역 필요 |
| `reject-choice-due-to-prev-constraint.rs` | 번역 필요 |

### ``multiple-lifetimes/``

`multiple-lifetimes/`

| 파일 | 설명 |
|------|------|
| `error-handling-2.rs` | 번역 필요 |
| `error-handling.rs` | 번역 필요 |
| `inverse-bounds.rs` | 번역 필요 |
| `multiple-lifetimes.rs` | 번역 필요 |
| `ordinary-bounds-pick-original-elided.rs` | 번역 필요 |
| `ordinary-bounds-pick-original-type-alias-impl-trait.rs` | 번역 필요 |
| `ordinary-bounds-pick-original.rs` | 번역 필요 |
| `ordinary-bounds-pick-other.rs` | 번역 필요 |
| `ordinary-bounds-unrelated.rs` | 번역 필요 |
| `ordinary-bounds-unsuited.rs` | 번역 필요 |

### ``rpit/``

`rpit/`

| 파일 | 설명 |
|------|------|
| `const_check_false_cycle.rs` | 번역 필요 |
| `dyn-in-nested-rpit.rs` | 번역 필요 |
| `early_bound.rs` | 번역 필요 |
| `equal-lifetime-params-ok.rs` | 번역 필요 |
| `inherits-lifetime.rs` | 번역 필요 |
| `non-defining-use-lifetimes.rs` | 번역 필요 |
| `non-defining-use.rs` | 번역 필요 |
| `precise-capture-155151.rs` | 번역 필요 |
| `unit-impl-default-36379.rs` | 번역 필요 |

### ``transmute/``

`transmute/`

| 파일 | 설명 |
|------|------|
| `in-defining-scope.rs` | 번역 필요 |
| `outside-of-defining-scope.rs` | 번역 필요 |
