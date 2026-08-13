// t.rs
fn foo(x: i32) -> impl Iterator<Item = i32> {
    (0..x).map(|i| i * 2)
}

fn main() {
    let v: Vec<i32> = foo(5).collect();
    println!("{:?}", v);
}
