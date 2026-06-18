//! Re-exports the command palette view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-019 §"Command palette", Slice 7).
//! `build_palette(registry, ctx, query)` returns a sorted `Vec<PaletteRow>`
//! with availability evaluated and shortcut hints resolved.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{PaletteRow, build_palette, palette_enabled_count};
