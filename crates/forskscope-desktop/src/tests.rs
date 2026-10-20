//! Desktop test suite (RFC-001 §10, RFC-002 §11).
//!
//! Tests validate the design specifications, not merely the code. Organized
//! into submodules under `src/tests/` per the project testing guidelines.

mod env_passthrough_tests;
mod external_tool_tests;
mod persist_v2_repository_tests;
mod persist_v2_runtime_tests;
mod persist_v2_schema_enum_wire_format_tests;
mod persist_v2_session_tests;
mod persist_v2_settings_tests;
mod platform_tests;
mod support;
mod vcs_tests;
mod watcher_tests;
