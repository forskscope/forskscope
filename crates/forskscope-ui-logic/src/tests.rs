//! Tests for the view-model modules moved in from `forskscope-core`
//! (RFC-093 §1 step 5): `command`, `conflict_nav`, `diff_decoration`,
//! `line_map`. Every other module in this crate keeps its tests inline
//! (`#[cfg(test)] mod tests` at the bottom of the file) - these four keep
//! the external `tests/*_tests.rs` file each already had, since each spans
//! more than one source file (e.g. `command_tests.rs` covers both
//! `command.rs` and `command/registry.rs`) and splitting it apart to
//! inline each half was out of scope for this move.

mod command_tests;
mod conflict_nav_tests;
mod diff_decoration_tests;
mod line_map_tests;
