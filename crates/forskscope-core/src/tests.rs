//! Core test suite (RFC-001 §10, RFC-002 §11).
//!
//! Tests validate the design specifications, not merely the code. Organized
//! into submodules under `src/tests/` per the project testing guidelines.

mod batch_tests;
mod command_tests;
mod compare_prep_tests;
mod conflict_nav_tests;
mod diff_decoration_tests;
mod edit_op_tests;
mod env_passthrough_tests;
mod external_tool_tests;
mod line_map_tests;
mod merge_plan_tests;
mod merge_tests;
mod patch_tests;
mod persist_v2_repository_tests;
mod persist_v2_runtime_tests;
mod persist_v2_schema_enum_wire_format_tests;
mod persist_v2_session_tests;
mod persist_v2_settings_tests;
mod platform_tests;
mod report_tests;
mod save_target_tests;
mod save_tests;
mod support;
mod three_way_tests;
mod transaction_log_tests;
mod vcs_tests;
mod watcher_tests;
