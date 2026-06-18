//! Re-exports the hunk decoration view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-024, RFC-035).
//! `DecorationIndex::from_set(dec_set)` provides O(1) row lookup by index
//! and side, replacing the inline `match hunk.kind` CSS logic in `hunk.rs`.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{DecorationIndex, DiffSide, RowDecoration};
