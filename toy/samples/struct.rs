// struct 샘플 — 정의 + 리터럴 + 필드 접근.
// 실행: ./run samples/struct.rs  또는  ./run --sample struct
struct Point {
    x: i64,
    y: i64,
}

fn main() {
    let p = Point { x: 1, y: 2 };
    p.x + p.y
}
