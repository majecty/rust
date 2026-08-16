
// any_test.rs
fn foo(x: any Iterator<Item = i32>) {
    for v in x {
        println!("any {v}");
    }
}

fn main() {
    foo((0..5).map(|i| i * 2));
}
