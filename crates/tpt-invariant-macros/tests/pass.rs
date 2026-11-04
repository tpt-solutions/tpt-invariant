//! Positive usage: contracts compile and leave the function's behaviour unchanged.

use tpt_invariant_macros::{ensures, invariant, requires};

#[requires(x > 0 && y > 0)]
#[ensures(result >= x && result >= y)]
fn max_pos(x: i32, y: i32) -> i32 {
    if x > y { x } else { y }
}

#[ensures(result == old(x) + 1)]
fn bump(x: u32) -> u32 {
    x + 1
}

#[requires(n >= 0)]
#[ensures(result == n)]
fn count(n: i64) -> i64 {
    let mut i = 0;
    while i < n {
        invariant!(i <= n);
        i += 1;
    }
    i
}

#[test]
fn contracts_are_transparent() {
    assert_eq!(max_pos(3, 7), 7);
    assert_eq!(max_pos(9, 2), 9);
    assert_eq!(bump(41), 42);
    assert_eq!(count(5), 5);
}
