
// some_type_test.rs
type Foo = some Iterator<Item = i32>;

fn foo() -> Foo {
    (0..5).map(|i| i * 2)
}

fn main() {
    let v: Vec<i32> = foo().collect();
    println!("{:?}", v);
}
