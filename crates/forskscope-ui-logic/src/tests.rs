//! Tests for the view-model modules moved in from the old `forskscope-core`
//! (RFC-093 §1 step 5): `command`, `diff_decoration`, `line_map`.
//! `conflict_nav` was a fourth (F51, 0.199.0: deleted, along with its own
//! `conflict_nav_tests`, as dead view-model code — never wired into
//! `forskscope-ui`). Every other module in this crate keeps its tests
//! inline (`#[cfg(test)] mod tests` at the bottom of the file) - these
//! three keep the external `tests/*_tests.rs` file each already had, since
//! each spans more than one source file (e.g. `command_tests.rs` covers
//! both `command.rs` and `command/registry.rs`) and splitting it apart to
//! inline each half was out of scope for this move.

mod command_tests;
mod diff_decoration_tests;
mod line_map_tests;
