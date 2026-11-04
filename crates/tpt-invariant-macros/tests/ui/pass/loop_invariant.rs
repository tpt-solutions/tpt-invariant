use tpt_invariant_macros::{ensures, invariant, requires};

#[requires(n >= 0)]
#[ensures(result == n)]
fn walk(n: u8) -> u8 {
    let mut i = 0;
    while i < n {
        invariant!(i <= n && !(i > n));
        i += 1;
    }
    i
}

fn main() {
    assert_eq!(walk(3), 3);
}
