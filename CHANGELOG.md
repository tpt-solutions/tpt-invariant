# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The project uses [Semantic Versioning](https://semver.org/). Before 1.0.0, minor
versions may contain breaking changes.

## [Unreleased]

### Added
- CFG IR (`tpt-invariant-core::cfg`) and `syn` lowering of free functions for the
  MVP subset (`tpt-invariant-core::lower`), with a DOT dump and span-accurate
  "not supported in the MVP subset" errors.
- Workspace with four crates: `tpt-invariant` (facade), `tpt-invariant-macros`,
  `tpt-invariant-core` (skeleton), and `tpt-invariant-cli` (`cargo-invariant`).
- Contract macros `#[requires]`, `#[ensures]` and `invariant!`, with a syntactic
  grammar check and span-accurate compile errors. The contracts are erased at
  build time.
- `cargo invariant check` argument parsing (`--prove`, `--strict`, `--bound`,
  `--timeout`, `-p`, `--function`, `--format`) with the documented exit codes.
  The check itself reports "not implemented yet".
- Licensing: `MIT OR Apache-2.0`, `deny.toml` license policy, MSRV 1.85.
- CI workflow (fmt, clippy, test, cargo-deny on Linux, Windows and macOS; MSRV check).
