//! Core test suite (RFC-001 §10, RFC-002 §11).
//!
//! Tests validate the design specifications, not merely the code. Organized
//! into submodules under `src/tests/` per the project testing guidelines.

mod app_error_tests;
mod batch_tests;
mod cancel_tests;
mod command_tests;
mod compare_profile_tests;
mod conflict_nav_tests;
mod diff_decoration_tests;
mod diff_tests;
mod dir_cancel_tests;
mod dir_index_tests;
mod dir_tests;
mod document_tests;
mod edit_op_tests;
mod editability_tests;
mod encoding_tests;
mod error_tests;
mod external_state_tests;
mod external_tool_tests;
mod file_kind_tests;
mod file_size_tests;
mod ignore_tests;
mod job_tests;
mod line_map_tests;
mod merge_plan_tests;
mod merge_tests;
mod patch_tests;
mod persist_tests;
mod path_tests;
mod report_tests;
mod save_tests;
mod session_tests;
mod settings_tests;
mod three_way_tests;
mod transaction_log_tests;
mod vcs_tests;
mod watcher_tests;
mod xlsx_tests;
