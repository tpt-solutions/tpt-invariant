use tpt_invariant_macros::requires;

#[requires(x > 0)]
struct Point {
    x: i32,
}

fn main() {}
