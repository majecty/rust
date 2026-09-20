use my_macro::make_x;
use my_macro_common::{opaque_make_y, opaque_make_y_with_val}; // Opaque 매크로 임포트

fn main() {
    println!("=== 1. Transparent (proc macro call-site) ===");
    transparent_demo();

    println!();
    println!("=== 2. SemiOpaque (macro-rules) ===");
    semi_opaque_demo();

    println!();
    println!("=== 3. Opaque (macro_export) ===");
    opaque_demo();

    println!();
    println!("=== 4. SemiOpaque vs Opaque 동작 차이 ===");
    difference_demo();

    println!();
    println!("=== 5. $crate ctxt 확인 ===");
    crate_test_demo();

    println!();
    println!("=== 6. 두 매크로 충돌 방지 ===");
    conflict_demo();

    println!();
    println!("=== 7. 아이템 이름 해석 ===");
    item_demo();
}

// --- Transparent (proc macro) ---
fn transparent_demo() {
    let x = 10;
    make_x!();          // let x = 1이 삽입됨 (Transparent)
    println!("x = {}", x);  // 1 출력 — shadow
}

// --- SemiOpaque (macro-rules) ---
macro_rules! define_local {
    () => {
        let x = 1;       // 정의부 기준으로 이름 해석
    };
}

fn semi_opaque_demo() {
    let x = 10;
    define_local!();
    println!("x = {}", x);  // 10 출력 — 매크로의 x는 정의부 전용
}

// --- Opaque (macro_export) ---
fn opaque_demo() {
    let y = 10;
    opaque_make_y!();      // Opaque: 정의부 ctxt 사용
    println!("y = {}", y);  // 10 출력 — shadowing 안 됨 (정의부 기준)
}

// --- SemiOpaque vs Opaque 동작 차이 ---
// SemiOpaque: 정의부 ctxt 사용
macro_rules! semi_value {
    () => { println!("semi_value: {}", semi_def_value); };
}
const semi_def_value: i32 = 10;  // 정의부에 가까운 이름

// Opaque: 호출부 ctxt 사용
macro_rules! opaque_value {
    () => { println!("opaque_value: {}", opaque_use_value); };
}

fn difference_demo() {
    const opaque_use_value: i32 = 20;  // 호출부에 가까운 이름
    semi_value!();    // 10 출력 — 정의부 semi_def_value 참조
    opaque_value!();  // 20 출력 — 호출부 opaque_use_value 참조
}

// --- $crate로 ctxt 확인 ---
// Opaque: $crate 사용 가능 (정의부 크레이트 참조)
macro_rules! opaque_crate {
    () => { println!("opaque $crate path: {}", stringify!($crate)); };  // OK
}

// SemiOpaque: $crate 사용 불가 (컴파일 에러)
// macro_rules! semi_crate {
//     () => { println!("semi $crate: {}", $crate); };  // ERROR!
// }

fn crate_test_demo() {
    opaque_crate!();  // OK — $crate가 my_macro_common crate로 치환
    // semi_crate!();  // ERROR: $crate는 SemiOpaque에서 사용 불가
}

// --- 충돌 방지 ---
macro_rules! make_x_a {
    () => { let x = 1; };
}
macro_rules! make_x_b {
    () => { let x = 2; };
}

fn conflict_demo() {
    make_x_a!();
    make_x_b!();
    println!("충돌 없이 두 매크로 모두 실행됨");
}

// --- 아이템 이름 해석 ---
macro_rules! define_item {
    () => {
        fn helper() -> i32 { 42 }
    };
}

fn item_demo() {
    define_item!();
    let result = helper();
    println!("helper() = {}", result);  // 42
}
