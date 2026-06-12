//! Re-exports the scroll synchronisation view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-035).
//! `ScrollSyncState::from_scroll_top(px, row_height, total_rows)` converts
//! a raw `scrollTop` value into a `ScrollAnchor` shared by both panes;
//! `scroll_top_px()` derives the matching position for the other pane.
#[allow(unused_imports)]
pub use forskscope_ui_logic::ScrollSyncState;
