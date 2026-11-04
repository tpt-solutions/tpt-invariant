//! Solver abstraction and `tpt-solver` integration (todo.md, Phase 7).
//!
//! Planned: a trait so the backend can be swapped or mocked, translation of
//! symbolic expressions to BV / LIA constraints, and `unknown` / timeout
//! handling.
//!
//! Blocked: the `tpt-solver` repository is not available in this workspace,
//! so the backend cannot be pinned or audited yet.
