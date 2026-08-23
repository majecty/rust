
// some_arg_test.rs
fn foo(x: some Iterator<Item = i32>) {
    for v in x {
        println!("some arg {v}");
    }
}

fn main() {
    foo((0..5).map(|i| i * 2));
}
