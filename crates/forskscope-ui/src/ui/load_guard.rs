//! Re-exports the large-file load guard view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-013 §"Large file prompt").
//! `guard_for_sizes(left_bytes, right_bytes)` returns a [`LoadGuard`] telling
//! `open_compare` / `DiffWorkspace` whether to proceed, warn, or ask the user.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{LoadGuard, guard_for_sizes, guard_for_sizes_with_limits};
