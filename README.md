# tpt-invariant

A lightweight, zero-FFI, native Rust verifier for logical correctness:
arithmetic, bounds, state transitions and user-defined contracts.

> **Status: pre-alpha.** The contract macros and the `cargo invariant` CLI
> skeleton exist. The verification engine and the `tpt-solver` backend are
> still in development. `cargo invariant check` currently exits with an
> internal-error message and does not verify anything.

## Vision

- **Fast to install.** `cargo install tpt-invariant-cli` gives you a small
  binary. There is no C++ or Z3 dependency.
- **Rust-native.** Rust's borrow checker and type system already guarantee
  memory and thread safety, so tpt-invariant only checks logical correctness.
- **Gated unbounded.** Bounded checking is fast but incomplete. Unbounded
  checking is complete but can be slow. The tool runs a bounded pass first, then
  tries k-induction under a timeout gate. If the gate trips, it reports a
  bounded result instead of hanging.

## Contracts

```rust
use tpt_invariant::contracts::{ensures, invariant, requires};

#[requires(x > 0 && y > 0)]
#[ensures(result >= x && result >= y)]
fn max(x: i32, y: i32) -> i32 {
    if x > y { x } else { y }
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
```

- `#[requires(e)]`: precondition over the parameters.
- `#[ensures(e)]`: postcondition. It may use `result` and `old(param)`.
- `invariant!(e)`: loop invariant, used inside a loop body. It expands to `()`.

Contracts have no runtime effect. Malformed contracts produce compile errors
that point at the offending part of the expression.

## CLI (planned interface)

```text
cargo invariant check                    # bounded, up to 50
cargo invariant check --prove            # bounded k=10, then k-induction with a 2s gate
cargo invariant check --prove --strict   # fail if the unbounded gate trips
cargo invariant check --bound 100        # custom bound
```

Exit codes: `0` verified, `1` counterexample, `2` strict-gate trip or
unsupported construct, `3` internal error.

## Scope and boundaries

tpt-invariant does **not** handle:

- memory safety or undefined behaviour (the Rust compiler does this);
- concurrency or thread safety (the compiler and tools such as Loom do this);
- non-linear arithmetic between variables (`x * y`). These are bounded or
  reported as unsupported;
- FFI and system calls. These are treated as black boxes that return any valid
  value.

## Workspace layout

| Crate | Purpose |
|-------|---------|
| `tpt-invariant` | User-facing facade; re-exports the macros as `tpt_invariant::contracts` |
| `tpt-invariant-macros` | Proc macros `#[requires]`, `#[ensures]`, `invariant!` |
| `tpt-invariant-core` | Verification engine (IR, symbolic execution, solver gate). In progress |
| `tpt-invariant-cli` | `cargo-invariant` binary |

## Building and testing

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all -- --check
cargo deny check licenses
```

The minimum supported Rust version is **1.85**.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

Dependencies must be licensed under MIT, BSD, ISC, Unlicense, or
`MIT OR Apache-2.0`. Apache-2.0-only dependencies are not allowed; `cargo deny`
enforces this.

Copyright (c) 2026 TPT Solutions.
