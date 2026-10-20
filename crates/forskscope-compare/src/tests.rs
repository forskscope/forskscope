//! Comparison-engine test suite (RFC-001 §10, RFC-002 §11, RFC-093).
//!
//! Tests validate the design specifications, not merely the code. Organized
//! into submodules under `src/tests/` per the project testing guidelines.

mod app_error_tests;
mod cancel_tests;
mod compare_profile_tests;
mod diff_tests;
mod dir_cancel_tests;
mod dir_ignore_tests;
mod dir_tests;
mod dir_unreadable_tests;
mod document_tests;
mod editability_tests;
mod encoding_tests;
mod error_tests;
mod external_state_tests;
mod file_kind_tests;
mod file_size_tests;
mod ignore_tests;
mod job_tests;
mod path_tests;
mod support;
#[cfg(feature = "spreadsheet")]
mod xlsx_tests;
