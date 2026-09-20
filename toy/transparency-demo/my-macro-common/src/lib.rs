// Opaque 매크로: macro_rules! + macro_export
// 정의부 ctxt를 사용하므로 호출부 이름과 충돌하지 않음
#[macro_export]
macro_rules! opaque_make_y {
    () => { let y = 1; };
}

#[macro_export]
macro_rules! opaque_make_y_with_val {
    ($val:expr) => { let y = $val; };  // 정의부 ctxt, 완전히 분리
}