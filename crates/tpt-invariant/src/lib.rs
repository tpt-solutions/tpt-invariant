//! User-facing entry point for `tpt-invariant`.
//!
//! Import the contract macros from [`contracts`]:
//!
//! ```ignore
//! use tpt_invariant::contracts::{ensures, invariant, requires};
//!
//! #[requires(x > 0 && y > 0)]
//! #[ensures(result >= x && result >= y)]
//! fn max(x: i32, y: i32) -> i32 {
//!     if x > y { x } else { y }
//! }
//! ```
//!
//! The contracts have no runtime effect. Verification is run with
//! `cargo invariant check` (see the `tpt-invariant-cli` crate).

/// Contract macros: `#[requires]`, `#[ensures]` and `invariant!`.
pub mod contracts {
    pub use tpt_invariant_macros::{ensures, invariant, requires};
}
