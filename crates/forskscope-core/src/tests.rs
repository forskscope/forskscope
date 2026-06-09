//! Core test suite (RFC-001 §10, RFC-002 §11).
//!
//! Tests validate the design specifications, not merely the code. Organized
//! into submodules under `src/tests/` per the project testing guidelines.

mod diff_tests;
mod dir_tests;
mod document_tests;
mod encoding_tests;
mod ignore_tests;
mod merge_tests;
mod patch_tests;
mod save_tests;
