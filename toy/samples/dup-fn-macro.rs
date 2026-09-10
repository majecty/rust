// 매크로 생성 함수 vs 직접 정의 — resolve에서 duplicate 에러로 실패한다.
// 실행: ./run samples/dup-fn-macro.rs
def_fn!(foo)
fn foo() {
    1
}
