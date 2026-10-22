//! `forskscope-commands` — the app's own `compare` and `mcp` subcommands
//! (RFC-096), dispatched to from `forskscope`'s own `main` before
//! anything the app itself needs (GTK, the web view, settings, a data
//! folder) ever starts. Depends only on `forskscope-compare` and
//! `forskscope-report`, enforced by `cargo xtask crate-boundaries`
//! (RFC-096 §2) — that dependency list, not anything in this crate's own
//! code, is what keeps these subcommands unable to reach the code that
//! writes, even though the one executable they end up linked into
//! contains it.
//!
//! One public entry point per subcommand, each taking the arguments
//! *after* the subcommand name and returning the exit code the process
//! should exit with:
//!
//! ```text
//! forskscope compare <left> <right> [options]
//! forskscope mcp --root <dir> [--root <dir> …] [--allow-show]
//! ```
//!
//! Hand-written argument parsing, like the app's own (RFC-094 §1) — no
//! parser framework. Reads no settings, writes no session: every run
//! starts from nothing and ends with nothing kept (RFC-094's "stateless"
//! property).

pub mod compare;
pub mod mcp;
