//! UI-layer settings types: theme, language, diff profiles, app settings (RFC-009).
//!
//! RFC-076 (patch 4): this is a non-serializing view adapter, not an
//! independent disk format. The canonical, persisted type is
//! [`forskscope_core::persist::schema::settings::PersistedSettings`]; see
//! [`AppSettings::from_v2`] and [`AppSettings::merge_into_v2`] for the
//! boundary. `Theme`/`Lang`/`DiffFontFamily`/`DiffAlgorithmSetting`/
//! `DiffProfile` are UI-only projections of the richer core types
//! (`ThemeId`/`LocaleId`/`DiffFontFamilySetting`/`DiffAlgorithm`/
//! `PersistedDiffProfile`) — never serialized on their own.

use std::path::PathBuf;

pub use forskscope_core::DiffAlgorithm;
use forskscope_core::DiffOptions;
use forskscope_core::persist::schema::settings::{PersistedDiffProfile, PersistedSettings};
use forskscope_core::settings::display::DiffFontFamilySetting;
use forskscope_core::settings::{LocaleId, ThemeId};
use forskscope_core::{CaseSensitivity, NewlineCompareMode, WhitespaceMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    Night,
}

impl Theme {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Dark => "theme-dark",
            Self::Light => "theme-light",
            Self::Night => "theme-night",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ja,
}

// Re-export for UI use without depending on the core type directly.

/// A named preset for diff options — stored in settings, applied when
/// opening new comparisons (RFC-009 compare profiles).
#[derive(Debug, Clone, PartialEq)]
pub struct DiffProfile {
    pub name: String,
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    pub algorithm: DiffAlgorithmSetting,
    /// Built-in profiles ship with the app and cannot be deleted.
    pub built_in: bool,
}

/// Font family preset for the diff panes (RFC-070).
/// Presets use safe cross-platform CSS font stacks; no font enumeration needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffFontFamily {
    /// `ui-monospace, monospace` — system default monospace (default).
    #[default]
    Monospace,
    /// `system-ui, sans-serif` — system default proportional.
    SansSerif,
    /// `Georgia, serif` — system default serif.
    Serif,
    /// `Courier New, Courier, monospace` — classic fixed-pitch.
    CourierNew,
    /// `Consolas, Menlo, monospace` — developer-oriented fixed-pitch.
    Consolas,
}

impl DiffFontFamily {
    /// CSS `font-family` value for this preset.
    pub fn css_value(self) -> &'static str {
        match self {
            Self::Monospace => "ui-monospace, monospace",
            Self::SansSerif => "system-ui, sans-serif",
            Self::Serif => "Georgia, serif",
            Self::CourierNew => "Courier New, Courier, monospace",
            Self::Consolas => "Consolas, Menlo, monospace",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffAlgorithmSetting {
    #[default]
    Myers,
    Patience,
    Histogram,
}

impl DiffProfile {
    pub fn to_diff_options(&self) -> DiffOptions {
        let algo = match self.algorithm {
            DiffAlgorithmSetting::Myers => DiffAlgorithm::Myers,
            DiffAlgorithmSetting::Patience => DiffAlgorithm::Patience,
            DiffAlgorithmSetting::Histogram => DiffAlgorithm::Histogram,
        };
        DiffOptions {
            ignore_whitespace: self.ignore_whitespace,
            ignore_case: self.ignore_case,
            algorithm: algo,
            ..DiffOptions::default()
        }
    }

    /// Projects a canonical v2 profile into the UI's two-bool view. Lossy
    /// only for `whitespace`/`newlines`/`inline_mode` values the shipping
    /// Settings dialog has never been able to produce (`IgnoreTrailing`,
    /// `IgnoreBlankLines`, non-`Significant` newlines, non-`Lazy` inline
    /// mode) — every profile the UI itself has ever written round-trips
    /// exactly, since it only ever writes the values [`Self::to_v2`]
    /// produces.
    pub fn from_v2(p: &PersistedDiffProfile) -> Self {
        Self {
            name: p.name.clone(),
            ignore_whitespace: p.whitespace != WhitespaceMode::Significant,
            ignore_case: p.case == CaseSensitivity::Insensitive,
            algorithm: match p.algorithm {
                DiffAlgorithm::Myers => DiffAlgorithmSetting::Myers,
                DiffAlgorithm::Patience => DiffAlgorithmSetting::Patience,
                DiffAlgorithm::Histogram => DiffAlgorithmSetting::Histogram,
                // The UI has never offered Lcs; fold to the closest default
                // rather than losing the profile entirely.
                DiffAlgorithm::Lcs => DiffAlgorithmSetting::Myers,
            },
            built_in: p.built_in,
        }
    }

    /// Inverse of [`Self::from_v2`]. Matches
    /// `forskscope_core::persist::schema::settings::ui_builtin_profiles`'s exact
    /// mapping so a round trip through the UI never changes a built-in
    /// profile's canonical shape.
    pub fn to_v2(&self) -> PersistedDiffProfile {
        PersistedDiffProfile {
            name: self.name.clone(),
            whitespace: if self.ignore_whitespace {
                WhitespaceMode::IgnoreAll
            } else {
                WhitespaceMode::Significant
            },
            newlines: NewlineCompareMode::Significant,
            case: if self.ignore_case {
                CaseSensitivity::Insensitive
            } else {
                CaseSensitivity::Sensitive
            },
            inline_mode: forskscope_core::diff::InlineMode::Lazy,
            algorithm: match self.algorithm {
                DiffAlgorithmSetting::Myers => DiffAlgorithm::Myers,
                DiffAlgorithmSetting::Patience => DiffAlgorithm::Patience,
                DiffAlgorithmSetting::Histogram => DiffAlgorithm::Histogram,
            },
            built_in: self.built_in,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub theme: Theme,
    pub language: Lang,
    pub diff_font_size: u32,
    /// Font family used in the diff panes (RFC-070). Default: Monospace.
    pub diff_font_family: DiffFontFamily,
    pub context_lines: usize,
    pub last_left_dir: Option<PathBuf>,
    pub last_right_dir: Option<PathBuf>,
    pub profiles: Vec<DiffProfile>,
    pub active_profile: usize,
    /// Comma-separated file extensions to ignore (e.g. `"o, class, tmp"`).
    pub ignore_extensions: String,
    /// Comma-separated directory-name patterns to ignore (e.g. `"target, node_modules, *.cache"`).
    pub ignore_dirs: String,
    /// When `true`, the Explorer shows each pane independently (no spacer rows),
    /// breaking cross-pane alignment. Default `false` (aligned mode) (RFC-068).
    pub explorer_compact: bool,
    /// When `false` (default), binary files cannot be compared and appear
    /// as non-actionable in the Explorer (RFC-066).
    pub enable_binary_comparison: bool,
    /// When `true` (default), the Explorer remembers the last directory shown in
    /// each pane and restores them on the next launch. When `false`, the Explorer
    /// always starts at the user's home directory.
    pub remember_explorer_dirs: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self::from_v2(&PersistedSettings::default())
    }
}

impl AppSettings {
    /// Build an [`IgnoreRules`] snapshot from the current settings.
    pub fn ignore_rules(&self) -> forskscope_core::IgnoreRules {
        forskscope_core::IgnoreRules::from_settings(&self.ignore_extensions, &self.ignore_dirs)
    }

    /// Projects the UI-editable subset of the canonical v2 settings
    /// (RFC-076) into this view type.
    pub fn from_v2(v2: &PersistedSettings) -> Self {
        Self {
            theme: match v2.theme {
                ThemeId::Dark => Theme::Dark,
                ThemeId::Light => Theme::Light,
                ThemeId::Night => Theme::Night,
            },
            language: if v2.language == LocaleId::japanese() {
                Lang::Ja
            } else {
                Lang::En
            },
            diff_font_size: v2.diff_font_size,
            diff_font_family: match v2.diff_font_family {
                DiffFontFamilySetting::SystemMono => DiffFontFamily::Monospace,
                DiffFontFamilySetting::SystemSans => DiffFontFamily::SansSerif,
                DiffFontFamilySetting::SystemSerif => DiffFontFamily::Serif,
                DiffFontFamilySetting::CourierNew => DiffFontFamily::CourierNew,
                DiffFontFamilySetting::Consolas => DiffFontFamily::Consolas,
            },
            context_lines: v2.context_lines,
            last_left_dir: v2.last_left_dir.clone(),
            last_right_dir: v2.last_right_dir.clone(),
            profiles: v2.profiles.iter().map(DiffProfile::from_v2).collect(),
            active_profile: v2.active_profile,
            ignore_extensions: v2.ignore_extensions.clone(),
            ignore_dirs: v2.ignore_dirs.clone(),
            explorer_compact: v2.explorer_compact,
            enable_binary_comparison: v2.enable_binary_comparison,
            remember_explorer_dirs: v2.remember_explorer_dirs,
        }
    }

    /// Merges this view's UI-editable fields onto `base` (the last-resolved
    /// canonical value), leaving every non-UI-owned field (appearance font,
    /// density, `show_line_numbers`, ...) untouched via struct-update syntax
    /// — a field neither this function nor [`Self::from_v2`] mentions is
    /// automatically preserved rather than silently reset, including any
    /// added to `PersistedSettings` after this code was written. This is
    /// what makes `persist()` safe to call after every settings-dialog
    /// change without resetting fields the UI has no control over.
    pub fn merge_into_v2(&self, base: &PersistedSettings) -> PersistedSettings {
        PersistedSettings {
            theme: match self.theme {
                Theme::Dark => ThemeId::Dark,
                Theme::Light => ThemeId::Light,
                Theme::Night => ThemeId::Night,
            },
            language: match self.language {
                Lang::En => LocaleId::english(),
                Lang::Ja => LocaleId::japanese(),
            },
            diff_font_size: self.diff_font_size,
            diff_font_family: match self.diff_font_family {
                DiffFontFamily::Monospace => DiffFontFamilySetting::SystemMono,
                DiffFontFamily::SansSerif => DiffFontFamilySetting::SystemSans,
                DiffFontFamily::Serif => DiffFontFamilySetting::SystemSerif,
                DiffFontFamily::CourierNew => DiffFontFamilySetting::CourierNew,
                DiffFontFamily::Consolas => DiffFontFamilySetting::Consolas,
            },
            context_lines: self.context_lines,
            last_left_dir: self.last_left_dir.clone(),
            last_right_dir: self.last_right_dir.clone(),
            profiles: self.profiles.iter().map(DiffProfile::to_v2).collect(),
            active_profile: self.active_profile,
            ignore_extensions: self.ignore_extensions.clone(),
            ignore_dirs: self.ignore_dirs.clone(),
            explorer_compact: self.explorer_compact,
            enable_binary_comparison: self.enable_binary_comparison,
            remember_explorer_dirs: self.remember_explorer_dirs,
            ..base.clone()
        }
    }
}

/// Specification for a batch file-copy operation (deep compare "Copy all").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCopySpec {
    pub items: Vec<(PathBuf, PathBuf)>, // (src, dst)
    pub label: String,
}
