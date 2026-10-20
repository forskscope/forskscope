//! Core test suite (RFC-001 §10, RFC-002 §11).
//!
//! Tests validate the design specifications, not merely the code. Organized
//! into submodules under `src/tests/` per the project testing guidelines.

mod command_tests;
mod conflict_nav_tests;
mod diff_decoration_tests;
mod line_map_tests;
