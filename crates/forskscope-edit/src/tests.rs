//! Edit test suite (RFC-001 §10, RFC-002 §11).
//!
//! Tests validate the design specifications, not merely the code. Organized
//! into submodules under `src/tests/` per the project testing guidelines.

mod batch_tests;
mod compare_prep_tests;
mod dir_tests;
mod edit_op_tests;
mod merge_plan_tests;
mod merge_tests;
mod save_target_tests;
mod save_tests;
mod support;
mod three_way_tests;
mod transaction_log_tests;
