// 함수 이름 중복 샘플 — resolve 단계에서 duplicate 에러로 실패한다.
// 실행: ./run samples/dup-fn.rs  또는  ./run --sample dup-fn
fn main() {
    let x: u32 = 1;
}

fn main() {
    2
}
