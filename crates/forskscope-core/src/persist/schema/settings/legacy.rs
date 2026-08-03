//! Legacy v0 DTO: an exact, frozen mirror of the shipping UI's plain-JSON
//! settings shape (`forskscope_ui::state::settings::AppSettings`).
//!
//! `forskscope-core` cannot depend on `forskscope-ui`, so this is a private
//! copy, field-for-field including serde attributes. It must change only if
//! the shipping v0 format itself changes, never to track v2 evolution.

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct LegacyAppSettingsV0 {
    pub(super) theme: LegacyThemeV0,
    pub(super) language: LegacyLangV0,
    pub(super) diff_font_size: u32,
    #[serde(default)]
    pub(super) diff_font_family: LegacyDiffFontFamilyV0,
    #[serde(default = "legacy_default_context_lines")]
    pub(super) context_lines: usize,
    #[serde(default)]
    pub(super) last_left_dir: Option<PathBuf>,
    #[serde(default)]
    pub(super) last_right_dir: Option<PathBuf>,
    #[serde(default = "legacy_default_profiles")]
    pub(super) profiles: Vec<LegacyDiffProfileV0>,
    #[serde(default)]
    pub(super) active_profile: usize,
    #[serde(default)]
    pub(super) ignore_extensions: String,
    #[serde(default)]
    pub(super) ignore_dirs: String,
    #[serde(default)]
    pub(super) explorer_compact: bool,
    #[serde(default)]
    pub(super) enable_binary_comparison: bool,
    #[serde(default = "legacy_default_true")]
    pub(super) remember_explorer_dirs: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum LegacyThemeV0 {
    Dark,
    Light,
    Night,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum LegacyLangV0 {
    En,
    Ja,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum LegacyDiffFontFamilyV0 {
    #[default]
    Monospace,
    SansSerif,
    Serif,
    CourierNew,
    Consolas,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum LegacyDiffAlgorithmV0 {
    #[default]
    Myers,
    Patience,
    Histogram,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LegacyDiffProfileV0 {
    pub(super) name: String,
    pub(super) ignore_whitespace: bool,
    pub(super) ignore_case: bool,
    pub(super) algorithm: LegacyDiffAlgorithmV0,
    #[serde(default)]
    pub(super) built_in: bool,
}

fn legacy_default_true() -> bool {
    true
}

pub(super) fn legacy_default_context_lines() -> usize {
    3
}

/// Mirrors `forskscope_ui::state::settings::default_profiles()` exactly —
/// the same four built-ins, in the same order, with the same field values.
fn legacy_default_profiles() -> Vec<LegacyDiffProfileV0> {
    let p = |name: &str, ignore_whitespace, ignore_case, algorithm| LegacyDiffProfileV0 {
        name: name.to_string(),
        ignore_whitespace,
        ignore_case,
        algorithm,
        built_in: true,
    };
    vec![
        p(
            "Exact (default)",
            false,
            false,
            LegacyDiffAlgorithmV0::Myers,
        ),
        p(
            "Ignore whitespace",
            true,
            false,
            LegacyDiffAlgorithmV0::Myers,
        ),
        p("Ignore case", false, true, LegacyDiffAlgorithmV0::Myers),
        p("Histogram", false, false, LegacyDiffAlgorithmV0::Histogram),
    ]
}
