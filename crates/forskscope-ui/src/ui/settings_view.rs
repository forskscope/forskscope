//! Re-exports the settings form view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-009, Slice 5).
//! `theme_choices()`, `density_choices()`, `font_family_choices()`, and
//! `profile_presets()` provide the picker options for each settings field.
//! `validate_font_size`, `clamp_font_size`, `validate_context_lines`
//! validate numeric inputs.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{
    ProfileChoice, SelectChoice,
    clamp_font_size, density_choices, find_active, font_family_choices,
    profile_presets, theme_choices, validate_context_lines, validate_font_size,
};
