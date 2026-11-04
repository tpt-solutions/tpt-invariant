use tpt_invariant_macros::requires;

#[requires(result > 0)]
fn f(x: i32) -> i32 {
    x
}

fn main() {}
