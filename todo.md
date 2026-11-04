# tpt-invariant — Project TODO

Maintainer: Phillip / TPT Solutions · License: MIT OR Apache-2.0 · Spec: `spec.txt` (v2.0)

**Locked decisions**
- Dual-licensed MIT OR Apache-2.0. Dependencies must NOT be Apache-2.0-only (MIT, MIT OR Apache-2.0, BSD, ISC, Unlicense are fine).
- `tpt-solver` (GitHub) work is tracked here: CDCL, QF_LIA, QF_BV.
- Contracts: `#[requires]`, `#[ensures]`, `result`, `old()`, loop `#[invariant]`.
- MVP Rust subset: ints, bool, `if` / `while` / `for` over ranges, function calls. No heap, arrays or ADTs.
- Overflow is a verification failure (debug semantics). `wrapping_*` / `checked_*` are modeled exactly.
- `check` default bound = 50. `--prove` runs Tier 1 at k=10, then k-induction with a 2s gate.
- Public release on GitHub + crates.io (all 3 crates).

---

## Phase 0 — Project setup & licensing
- [ ] Create GitHub repo `tpt-invariant` (public when ready)
- [x] Add `LICENSE-MIT` (copyright TPT Solutions)
- [x] Add `LICENSE-APACHE` (copyright TPT Solutions)
- [x] Add license section to README ("MIT OR Apache-2.0", contribution clause)
- [x] Create workspace root `Cargo.toml` (shared `[workspace.package]`: license, authors, edition, repository, rust-version)
- [x] Scaffold `crates/tpt-invariant-macros` (plus facade crate `crates/tpt-invariant`, which the `tpt_invariant::contracts` re-export needs)
- [x] Scaffold `crates/tpt-invariant-core` (`lib.rs`, `cfg.rs`, `symbolic.rs`, `solver.rs`)
- [x] Scaffold `crates/tpt-invariant-cli` (`main.rs`)
- [x] Set `license = "MIT OR Apache-2.0"` in every crate manifest
- [x] Add `deny.toml` (cargo-deny): allow-list MIT, BSD, ISC, Unlicense, `MIT OR Apache-2.0`; deny Apache-2.0-only
- [ ] Audit `tpt-solver` and its dependency tree for Apache-only licenses
- [x] Decide MSRV and pin in `rust-toolchain` / manifests (MSRV 1.85)
- [x] Add `rustfmt.toml`, clippy config, `.gitignore`
- [x] CI (GitHub Actions): fmt, clippy, test, cargo-deny — Windows, Linux, macOS (workflow written; not yet run on GitHub)
- [x] Add CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, issue templates
- [x] Write initial README (vision, install, usage placeholder)
- [ ] Reserve crate names on crates.io (`tpt-invariant-macros`, `-core`, `-cli`)
- [x] Resolve spec conflict: default bound 50 vs Tier 1 k=10 (documented in decisions above)

## Phase 1 — tpt-solver: CDCL SAT core
- [ ] Review the current tpt-solver architecture (QF_LRA) and plan the DPLL(T) refactor
- [ ] Define the theory-solver trait (assert, propagate, check, explain/conflict, backtrack)
- [ ] CNF representation and Tseitin transform for the boolean skeleton (`&&`, `||`, `!`, ite)
- [ ] CDCL core: unit propagation (2-watched literals)
- [ ] Conflict analysis (1-UIP) and clause learning
- [ ] Non-chronological backjumping
- [ ] Decision heuristic (VSIDS) and phase saving
- [ ] Restarts (Luby or glucose-style) and learned-clause deletion
- [ ] Incremental solving: push/pop or assumptions
- [ ] Integrate existing QF_LRA solver as a theory under DPLL(T)
- [ ] Unit tests + DIMACS benchmark suite
- [ ] Differential-test against a reference SAT solver
- [ ] Timeout / resource-limit support (wall clock + conflict budget)

## Phase 2 — tpt-solver: QF_LIA
- [ ] Integer variables and linear-constraint representation
- [ ] Integer reasoning on top of simplex (branch & bound)
- [ ] Cutting planes (Gomory) or equivalent
- [ ] Bounds propagation and GCD/divisibility tests
- [ ] Support for `div` / `mod` by constants
- [ ] Theory conflict explanations for DPLL(T)
- [ ] Model extraction (integer assignments)
- [ ] Tests incl. i32 / usize range constraints; no real-number rounding false positives
- [ ] Differential-test vs a reference solver (e.g. Z3 offline, for testing only)

## Phase 3 — tpt-solver: QF_BV
- [ ] Bitvector term representation (widths 8/16/32/64/128, signed + unsigned)
- [ ] Bit-blasting to CNF: add, sub, neg, comparisons
- [ ] Bit-blasting: mul, div, rem (signed + unsigned)
- [ ] Bit-blasting: shifts, rotates, bitwise and/or/xor/not
- [ ] Extract / concat / sign- and zero-extend
- [ ] Overflow-detection predicates (add/sub/mul overflow, signed + unsigned)
- [ ] Simplification / rewriting to shrink circuits
- [ ] Model extraction for bitvectors
- [ ] Tests against Rust integer semantics (wrapping, checked, shifts)
- [ ] Define the public solver API: assert / check-sat / get-model / timeout
- [ ] Publish a tpt-solver release with LIA + BV (version tag consumed by this project)

## Phase 4 — Macros crate (`tpt-invariant-macros`)
- [x] Define contract expression grammar (comparison, arithmetic, boolean ops, `result`, `old(..)`)
- [ ] Define the internal contract AST (shared type or serialized form; decide where it lives)
- [x] Parse `#[requires(..)]` with syn
- [x] Parse `#[ensures(..)]` with `result` binding
- [x] Support `old(expr)` in `ensures`
- [x] Parse loop `#[invariant(..)]` (decided: macro-form `invariant!(..)` inside loop body, expands to `()`)
- [x] Emit helpful compile errors with spans for malformed contracts
- [x] Ensure contracts are erased / zero-cost in normal builds (no runtime effect, original fn unchanged)
- [x] Re-export macros from `tpt_invariant::contracts`
- [x] Verify only syn / quote / proc-macro2 deps (license check passes)
- [x] trybuild tests for error messages; unit tests for parsing

## Phase 5 — Core: IR extraction & CFG (`cfg.rs`)
- [ ] Define IR types: blocks, statements, terminators, expressions, types (ints, bool)
- [ ] Locate verifiable functions in a source file / crate with syn (find fns carrying contracts)
- [ ] Walk crate sources (`src/**`, module tree) from the CLI
- [ ] Lower expressions: literals, vars, unary/binary ops, casts, method calls (`wrapping_*`, `checked_*`, `abs`, `min`, `max`)
- [ ] Lower `let` bindings, assignment, compound assignment
- [ ] Lower `if` / `else`, `if let` (rejected or limited, with clear diagnostic)
- [ ] Lower `while` loops
- [ ] Lower `for` over integer ranges (`a..b`, `a..=b`)
- [ ] Lower `loop` / `break` / `continue` / `return` / early `?`-less returns
- [ ] Lower `match` on integers/bools into branch chains
- [ ] Lower `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!`, `unreachable!`, `panic!`
- [ ] SSA conversion (phi nodes / versioned variables)
- [ ] Function-call handling: contract-based abstraction + optional inlining
- [ ] Unsupported-construct detection with clear "not supported in MVP" diagnostics
- [ ] CFG unit tests + DOT dump for debugging

## Phase 6 — Core: symbolic execution & VC generation (`symbolic.rs`)
- [ ] Symbolic expression type and typed symbolic store
- [ ] Path-condition accumulation and branch forking / state merging
- [ ] VC generation: `assert!` conditions
- [ ] VC generation: arithmetic overflow / underflow (debug semantics)
- [ ] VC generation: division / remainder by zero, `i32::MIN / -1`
- [ ] VC generation: shift-amount range
- [ ] VC generation: `ensures` at each return point (with `old()` snapshots)
- [ ] Assume `requires` at function entry
- [ ] Callee contracts: assert callee `requires`, assume callee `ensures`
- [ ] FFI / external calls as uninterpreted functions returning arbitrary values
- [ ] Non-linear arithmetic detection (var * var) -> bound or flag for Tier 3
- [ ] Source-span tracking on each VC for reporting
- [ ] Unit tests on small programs (known-valid and known-buggy)

## Phase 7 — Solver integration (`solver.rs`)
- [ ] Solver abstraction trait (so tpt-solver is swappable / mockable in tests)
- [ ] Translate symbolic exprs -> tpt-solver constraints (BV for exact overflow, LIA where sound & faster)
- [ ] Query: "Path ∧ ¬VC satisfiable?" -> UNSAT = proven, SAT = counterexample
- [ ] Counterexample extraction and mapping back to source variable names
- [ ] Per-query timeout and complexity gate plumbing
- [ ] Handle `unknown` / timeout results distinctly from SAT/UNSAT
- [ ] Pin tpt-solver dependency (git/crates.io version) and keep license audit green
- [ ] Integration tests using real tpt-solver
- [ ] Soundness tests: translation matches Rust integer semantics

## Phase 8 — The 3-tier Gate
- [ ] Tier 1: bounded loop unrolling (default 50; k=10 quick pass under `--prove`)
- [ ] Early return on counterexample found
- [ ] Tier 2: k-induction — base case (reuses bounded run)
- [ ] Tier 2: inductive step with havoc of loop-modified variables
- [ ] Use user `#[invariant]` as strengthening in the inductive step
- [ ] 2-second timeout and complexity gate for inductive step (configurable)
- [ ] Tier 3: graceful fallback with message ("Passed bounded verification up to k=N...")
- [ ] Result model: Proven (unbounded) / VerifiedBounded(k) / Counterexample / Unsupported
- [ ] `--strict`: gate trip -> failing result
- [ ] Nested loops handling policy
- [ ] Tests: loops proven unbounded, loops falling back, buggy loops

## Phase 9 — CLI (`tpt-invariant-cli`)
- [x] `cargo-invariant` binary so `cargo invariant check` works as a cargo subcommand
- [x] clap definitions: `check`, `--prove`, `--strict`, `--bound N`, `--timeout`
- [ ] Select package / file / function (`-p`, `--function`), workspace support
- [ ] Read `cargo metadata` to find sources
- [ ] Output formatting: colored pass / fail / bounded, counterexample ("Fails when x = -5") with source snippet
- [ ] Machine-readable output (`--format json`)
- [x] Exit codes: 0 ok, 1 counterexample, 2 strict gate trip / unsupported, other for internal errors (constants defined; clap usage errors also exit 2)
- [x] anyhow error handling; clear messages
- [ ] Verify `cargo install` works and measure binary size (~10MB target)
- [ ] CLI integration tests (assert_cmd)

## Phase 10 — Testing, quality & benchmarks
- [ ] Example corpus of contracted functions (max, abs, clamp, sum loop, gcd, binary search over ints)
- [ ] Golden-output tests for CLI
- [ ] Known-bug corpus: every counterexample must be reproducible when executed concretely
- [ ] Soundness fuzzing: random programs, compare symbolic results vs concrete execution
- [ ] Property tests for IR lowering
- [ ] Performance benchmarks (criterion) and install-size / compile-time measurements
- [ ] Comparison write-up vs Kani / Creusot / Prusti on the same examples
- [ ] Coverage report in CI

## Phase 11 — Docs & release
- [x] README with quickstart, flags, scope & boundaries (what it does NOT do)
- [ ] Docs on the Gated Unbounded philosophy and the tiers
- [ ] Contract language reference
- [ ] Rustdoc for all public items; docs.rs build passes
- [ ] `examples/` directory with runnable demos
- [x] CHANGELOG.md and versioning policy
- [ ] Final `cargo deny` + license audit before publishing
- [ ] Publish order: tpt-solver (if separate) -> macros -> core -> cli
- [ ] Tag v0.1.0, GitHub Release with binaries (optional) and announcement
- [ ] Website / landing page under TPT Solutions (optional)

## Phase 12 — Post-MVP backlog (V2+)
- [ ] MIR-based frontend for full fidelity (replace syn AST/CFG)
- [ ] Arrays / slices / `Vec` indexing and bounds checks (array theory)
- [ ] Structs, enums, `match` over ADTs
- [ ] Quantifiers (`forall` / `exists`) and ghost / pure functions
- [ ] Non-linear arithmetic support
- [ ] Floating-point reasoning
- [ ] Interpolation / IC3 / PDR for stronger unbounded proofs
- [ ] IDE integration (rust-analyzer diagnostics, VS Code extension)
- [ ] CI GitHub Action for `cargo invariant`
- [ ] Contract-aware `cargo test` runtime checks (optional)
