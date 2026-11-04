//! The `tpt-invariant` verification engine.
//!
//! Pipeline (see `spec.txt`, section 3):
//!
//! 1. Contract ingestion: `tpt-invariant-macros` (syntax only).
//! 2. IR extraction and CFG generation: [`cfg`].
//! 3. Symbolic execution and VC generation: [`symbolic`].
//! 4. Solver integration: [`solver`].
//!
//! Only the module skeletons exist yet. Each module documents what remains.

pub mod cfg;
pub mod solver;
pub mod symbolic;
