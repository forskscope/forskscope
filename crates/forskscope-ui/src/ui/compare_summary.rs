//! Re-exports the diff summary view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-006).
//! `CompareStatusSummary` drives the status bar; `DiffNavigationState`
//! drives the prev/next hunk toolbar buttons and their aria labels.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{CompareStatusSummary, DiffNavigationState};
