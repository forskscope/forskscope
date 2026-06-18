//! Re-exports the deep-compare filter view-model from `forskscope-ui-logic`
//! so `deep_compare.rs` can migrate from its local `DeepFilter` enum
//! (RFC-020 §5a, RFC-037).
#[allow(unused_imports)]
pub use forskscope_ui_logic::{DeepCompareSummary, DeepFilter, apply_filter};
