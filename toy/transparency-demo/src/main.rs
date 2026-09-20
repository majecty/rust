use my_macro::make_x;

fn main() {
    println!("=== 1. Transparent (proc macro call-site) ===");
    transparent_demo();

    println!();
    println!("=== 2. SemiOpaque (macro-rules) ===");
    semi_opaque_demo();

    println!();
    println!("=== 3. 두 매크로 충돌 방지 ===");
    conflict_demo();

    println!();
    println!("=== 4. 아이템 이름 해석 ===");
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
