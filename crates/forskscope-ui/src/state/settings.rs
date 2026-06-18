//! UI-layer settings types: theme, language, diff profiles, app settings (RFC-009).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use forskscope_core::DiffAlgorithm;
use forskscope_core::DiffOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme { Dark, Light, Night }

impl Theme {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Dark  => "theme-dark",
            Self::Light => "theme-light",
            Self::Night => "theme-night",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lang { En, Ja }

// Re-export for UI use without depending on the core type directly.

/// A named preset for diff options — stored in settings, applied when
/// opening new comparisons (RFC-009 compare profiles).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffProfile {
    pub name: String,
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    pub algorithm: DiffAlgorithmSetting,
    /// Built-in profiles ship with the app and cannot be deleted.
    #[serde(default)]
    pub built_in: bool,
}

/// Serialisable wrapper around `DiffAlgorithm` for profile persistence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DiffAlgorithmSetting { #[default] Myers, Patience, Histogram }

impl DiffProfile {
    pub fn to_diff_options(&self) -> DiffOptions {
        let algo = match self.algorithm {
            DiffAlgorithmSetting::Myers     => DiffAlgorithm::Myers,
            DiffAlgorithmSetting::Patience  => DiffAlgorithm::Patience,
            DiffAlgorithmSetting::Histogram => DiffAlgorithm::Histogram,
        };
        DiffOptions {
            ignore_whitespace: self.ignore_whitespace,
            ignore_case:       self.ignore_case,
            algorithm:         algo,
            ..DiffOptions::default()
        }
    }
}

fn default_profiles() -> Vec<DiffProfile> {
    vec![
        DiffProfile { name: "Exact (default)".into(),   ignore_whitespace: false, ignore_case: false, algorithm: DiffAlgorithmSetting::Myers,     built_in: true },
        DiffProfile { name: "Ignore whitespace".into(), ignore_whitespace: true,  ignore_case: false, algorithm: DiffAlgorithmSetting::Myers,     built_in: true },
        DiffProfile { name: "Ignore case".into(),       ignore_whitespace: false, ignore_case: true,  algorithm: DiffAlgorithmSetting::Myers,     built_in: true },
        DiffProfile { name: "Histogram".into(),         ignore_whitespace: false, ignore_case: false, algorithm: DiffAlgorithmSetting::Histogram, built_in: true },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub language: Lang,
    pub diff_font_size: u32,
    #[serde(default = "default_ctx")]
    pub context_lines: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_left_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_right_dir: Option<PathBuf>,
    #[serde(default = "default_profiles")]
    pub profiles: Vec<DiffProfile>,
    #[serde(default)]
    pub active_profile: usize,
    /// Comma-separated file extensions to ignore (e.g. `"o, class, tmp"`).
    #[serde(default)]
    pub ignore_extensions: String,
    /// Comma-separated directory-name patterns to ignore (e.g. `"target, node_modules, *.cache"`).
    #[serde(default)]
    pub ignore_dirs: String,
    /// When `true`, the Explorer shows each pane independently (no spacer rows),
    /// breaking cross-pane alignment. Default `false` (aligned mode) (RFC-068).
    #[serde(default)]
    pub explorer_compact: bool,
    /// When `false` (default), binary files cannot be compared and appear
    /// as non-actionable in the Explorer (RFC-066).
    #[serde(default)]
    pub enable_binary_comparison: bool,
}

fn default_ctx() -> usize { 3 }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark, language: Lang::En, diff_font_size: 14,
            context_lines: 3, last_left_dir: None, last_right_dir: None,
            profiles: default_profiles(), active_profile: 0,
            ignore_extensions: String::new(), ignore_dirs: String::new(),
            enable_binary_comparison: false,
            explorer_compact: false,
        }
    }
}

impl AppSettings {
    /// Build an [`IgnoreRules`] snapshot from the current settings.
    pub fn ignore_rules(&self) -> forskscope_core::IgnoreRules {
        forskscope_core::IgnoreRules::from_settings(&self.ignore_extensions, &self.ignore_dirs)
    }
}

/// Specification for a batch file-copy operation (deep compare "Copy all").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCopySpec {
    pub items: Vec<(PathBuf, PathBuf)>,   // (src, dst)
    pub label: String,
}
