//! `cargo invariant`: command-line front end for tpt-invariant.
//!
//! Exit codes:
//! - `0`: verified (or bounded-verified) with no counterexample
//! - `1`: counterexample found
//! - `2`: strict-mode gate tripped, or the code is unsupported
//! - `3`: internal error (including "engine not implemented yet")

use std::ffi::OsString;
use std::process::ExitCode;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

/// Exit code: verified, no counterexample.
pub const EXIT_OK: u8 = 0;
/// Exit code: a counterexample was found.
pub const EXIT_COUNTEREXAMPLE: u8 = 1;
/// Exit code: strict gate tripped or unsupported construct.
pub const EXIT_GATE_OR_UNSUPPORTED: u8 = 2;
/// Exit code: internal error.
pub const EXIT_INTERNAL: u8 = 3;

/// Default bounded-verification depth (`spec.txt`, section 5).
pub const DEFAULT_BOUND: u32 = 50;
/// Depth of the quick pass run under `--prove`.
pub const PROVE_QUICK_BOUND: u32 = 10;
/// Default wall-clock gate for the inductive step, in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 2;

#[derive(Parser, Debug)]
#[command(
    name = "cargo-invariant",
    bin_name = "cargo invariant",
    version,
    about = "Gated-unbounded logical verifier for Rust contracts"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Verify the contracts in the selected crate.
    Check(CheckArgs),
}

#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
struct CheckArgs {
    /// Run the bounded tier at k=10, then attempt k-induction under a timeout gate.
    #[arg(long)]
    prove: bool,

    /// Fail (exit 2) if the unbounded gate trips instead of degrading to a bounded result.
    #[arg(long, requires = "prove")]
    strict: bool,

    /// Bounded-verification depth.
    #[arg(long, value_name = "N", default_value_t = DEFAULT_BOUND, value_parser = clap::value_parser!(u32).range(1..))]
    bound: u32,

    /// Wall-clock gate for the inductive step, in seconds.
    #[arg(long, value_name = "SECS", default_value_t = DEFAULT_TIMEOUT_SECS, value_parser = clap::value_parser!(u64).range(1..))]
    timeout: u64,

    /// Package to check (workspace members are checked when omitted).
    #[arg(short = 'p', long, value_name = "SPEC")]
    package: Option<String>,

    /// Only check the function with this name.
    #[arg(long, value_name = "NAME")]
    function: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    Text,
    Json,
}

/// Removes the `invariant` word that `cargo` passes when it runs a
/// `cargo-invariant` binary as `cargo invariant ...`.
fn normalize_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut args: Vec<OsString> = args.into_iter().collect();
    if args.get(1).is_some_and(|a| a == "invariant") {
        args.remove(1);
    }
    args
}

fn run_check(args: &CheckArgs) -> Result<u8> {
    // The verification engine is not wired up yet (todo.md, Phases 5-8).
    // Report that explicitly rather than printing a false "verified".
    let _ = args;
    bail!("the verification engine is not implemented yet");
}

fn main() -> ExitCode {
    let cli = Cli::parse_from(normalize_args(std::env::args_os()));
    let result = match &cli.command {
        Command::Check(args) => run_check(args),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(EXIT_INTERNAL)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn strips_cargo_subcommand_name() {
        let args = normalize_args(["cargo-invariant", "invariant", "check"].map(OsString::from));
        assert_eq!(args, ["cargo-invariant", "check"].map(OsString::from));
        let args = normalize_args(["cargo-invariant", "check"].map(OsString::from));
        assert_eq!(args, ["cargo-invariant", "check"].map(OsString::from));
    }

    #[test]
    fn defaults_match_spec() {
        let cli = Cli::parse_from(["cargo-invariant", "check"]);
        let Command::Check(args) = cli.command;
        assert_eq!(args.bound, 50);
        assert_eq!(args.timeout, 2);
        assert!(!args.prove && !args.strict);
        assert_eq!(args.format, Format::Text);
    }

    #[test]
    fn strict_requires_prove() {
        assert!(Cli::try_parse_from(["cargo-invariant", "check", "--strict"]).is_err());
        assert!(Cli::try_parse_from(["cargo-invariant", "check", "--prove", "--strict"]).is_ok());
    }

    #[test]
    fn bound_must_be_positive() {
        assert!(Cli::try_parse_from(["cargo-invariant", "check", "--bound", "0"]).is_err());
        assert!(Cli::try_parse_from(["cargo-invariant", "check", "--bound", "100"]).is_ok());
    }
}
