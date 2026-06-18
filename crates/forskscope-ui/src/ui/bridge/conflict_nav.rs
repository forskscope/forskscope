//! Re-exports the conflict navigator rail view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-034, Slice 6).
//! `ConflictNavView::from_navigator(nav, can_save)` produces the complete
//! rail snapshot: rows with glyphs/CSS, progress text, can_save, prev/next IDs.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{ConflictNavView, ConflictRailRow};
