//! Re-exports the explorer status-badge view-model from `forskscope-ui-logic`
//! so UI components have a single stable import path (RFC-020 §5a, RFC-054).
//! `RowStatusKind` and `StatusRow` replace the local `DigestState` enum in
//! `dir_pane.rs`; components can migrate imports at their own pace.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{RowStatusKind, StatusRow};
