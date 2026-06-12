//! Re-exports the toolbar view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-019 §5).
//! `build_toolbar(registry, ctx)` produces the fully-evaluated
//! `Vec<ToolbarSection>` for the diff workspace toolbar.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{
    ToolbarItem, ToolbarSection, build_toolbar, enabled_count, find_item,
};
