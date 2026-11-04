# Contributing to tpt-invariant

Thanks for your interest in tpt-invariant.

## Before you start

- For anything beyond a small fix, open an issue first so we can agree on the
  approach. The roadmap is in `todo.md` and the design is in `spec.txt`.
- The project's scope is deliberate. Memory safety, concurrency and FFI are out
  of scope (see `spec.txt`, section 6).

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo deny check licenses
```

- The minimum supported Rust version is 1.85. Do not use newer language or
  library features without raising it in `Cargo.toml` and `clippy.toml`.
- Compile-error tests live in `crates/tpt-invariant-macros/tests/ui`. Their
  `.stderr` files are compiler-version sensitive. If you update them, say which
  toolchain you used.
- New dependencies must be licensed under MIT, BSD, ISC, Unlicense or
  `MIT OR Apache-2.0`. Run `cargo deny check licenses` before opening a PR.

## Pull requests

- Keep each PR focused on one change.
- Add or update tests for behaviour changes.
- Describe what changed and why, and link the relevant `todo.md` item.

## License

By contributing, you agree that your contributions are licensed under the same
terms as the project, `MIT OR Apache-2.0`, without additional conditions.
