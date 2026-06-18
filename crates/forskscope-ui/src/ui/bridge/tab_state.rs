//! Re-exports the tab-state → CommandContext bridge from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-003, RFC-019).
//! `TabStateSnapshot` is populated from `TabSnapshot` fields;
//! `context_from_snapshot` maps it to a `CommandContext` for `build_toolbar`.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{TabStateSnapshot, context_from_snapshot};
