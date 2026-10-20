//! UI-layer settings types: theme, language, diff profiles, app settings (RFC-009).
//!
//! RFC-076 (patch 4): this is a non-serializing view adapter, not an
//! independent disk format. The canonical, persisted type is
//! [`forskscope_desktop::persist::schema::settings::PersistedSettings`]; see
//! [`AppSettings::from_v2`] and [`AppSettings::merge_into_v2`] for the
//! boundary. `Theme`/`Lang`/`DiffFontFamily`/`DiffAlgorithmSetting`/
//! `DiffProfile` are UI-only projections of the richer core types
//! (`ThemeId`/`LocaleId`/`DiffFontFamilySetting`/`DiffAlgorithm`/
//! `PersistedDiffProfile`) — never serialized on their own.

use std::path::PathBuf;

pub use forskscope_compare::DiffAlgorithm;
use forskscope_compare::DiffOptions;
use forskscope_compare::diff::InlineMode;
use forskscope_compare::{CaseSensitivity, NewlineCompareMode, WhitespaceMode};
use forskscope_desktop::persist::schema::settings::{PersistedDiffProfile, PersistedSettings};
pub use forskscope_desktop::settings::display::ColourVision;
use forskscope_desktop::settings::display::DiffFontFamilySetting;
use forskscope_desktop::settings::{LocaleId, ThemeId};

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

/// RFC-088 §2: the root class the red–green-safe palette combines with the
/// theme class to override — `""` for `Standard`, so the root carries no
/// extra class and only the theme's own tokens apply. `ColourVision` is a
/// core type (re-exported above), so this is a free function rather than an
/// inherent method (the orphan rule).
pub fn colour_vision_css_class(cv: ColourVision) -> &'static str {
    match cv {
        ColourVision::Standard => "",
        ColourVision::RedGreenSafe => "cv-rgsafe",
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
///
/// F168: every field here is exactly what [`PersistedDiffProfile`] holds —
/// `from_v2`/`to_v2` is a lossless round trip for any value either side can
/// produce, including ones the dialog itself never writes (`whitespace:
/// IgnoreTrailing`/`IgnoreBlankLines`, `algorithm: Lcs`), so a profile a
/// future dialog control or a hand-edited `settings.json` sets is carried
/// through unchanged rather than silently narrowed on the next save.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffProfile {
    pub name: String,
    pub whitespace: WhitespaceMode,
    pub ignore_case: bool,
    pub algorithm: DiffAlgorithmSetting,
    /// Newline-style handling. The Settings dialog can set `IgnoreDifference`
    /// (F159(a)); built-in profiles stay `Significant`.
    pub newlines: NewlineCompareMode,
    /// Whether character-level highlighting starts on, automatically turns
    /// on for small replace hunks, or is unavailable (F168 §2.2). The dialog
    /// offers all three; `to_diff_options` passes this straight through, and
    /// a new tab under this profile seeds `char_mode` from it (`EagerForSmallHunks`
    /// starts with highlighting already on — `state/compare.rs`).
    pub inline_mode: InlineMode,
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

/// F168: carries `Lcs` so a profile already set to it (by a future dialog
/// control, or a hand-edited `settings.json`) round-trips exactly — the
/// dialog's own algorithm `<select>` still only ever writes `Myers`,
/// `Patience` or `Histogram` (handoff 076 §2.2: "do not add LCS to the
/// algorithm list").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffAlgorithmSetting {
    #[default]
    Myers,
    Patience,
    Histogram,
    Lcs,
}

impl DiffProfile {
    pub fn to_diff_options(&self) -> DiffOptions {
        let algo = match self.algorithm {
            DiffAlgorithmSetting::Myers => DiffAlgorithm::Myers,
            DiffAlgorithmSetting::Patience => DiffAlgorithm::Patience,
            DiffAlgorithmSetting::Histogram => DiffAlgorithm::Histogram,
            DiffAlgorithmSetting::Lcs => DiffAlgorithm::Lcs,
        };
        DiffOptions {
            // `DiffOptions.ignore_whitespace` is a single bool the engine
            // treats as "strip every whitespace character" — see
            // `engine.rs`'s `line_key`. It cannot tell `IgnoreTrailing` or
            // `IgnoreBlankLines` apart from `IgnoreAll`; all three collapse
            // to the same comparison today. `self.whitespace` still carries
            // the distinct value losslessly (ignore_whitespace below is
            // derived, not stored), so a later engine change can make the
            // distinction real without another migration.
            ignore_whitespace: self.whitespace != WhitespaceMode::Significant,
            ignore_case: self.ignore_case,
            algorithm: algo,
            ignore_newlines: self.newlines == NewlineCompareMode::IgnoreDifference,
            inline_mode: self.inline_mode,
            ..DiffOptions::default()
        }
    }

    /// Projects a canonical v2 profile into the UI's view. F168: every field
    /// round-trips exactly now — `whitespace`, `inline_mode` and `algorithm`
    /// used to narrow (see git history for the account of each loss); none
    /// of them do anymore. The invariant
    /// `to_v2(from_v2(p)) == p for every p` is asserted over every
    /// combination of enum values in the test module, not samples.
    pub fn from_v2(p: &PersistedDiffProfile) -> Self {
        Self {
            name: p.name.clone(),
            whitespace: p.whitespace,
            ignore_case: p.case == CaseSensitivity::Insensitive,
            algorithm: match p.algorithm {
                DiffAlgorithm::Myers => DiffAlgorithmSetting::Myers,
                DiffAlgorithm::Patience => DiffAlgorithmSetting::Patience,
                DiffAlgorithm::Histogram => DiffAlgorithmSetting::Histogram,
                DiffAlgorithm::Lcs => DiffAlgorithmSetting::Lcs,
            },
            newlines: p.newlines,
            inline_mode: p.inline_mode,
            built_in: p.built_in,
        }
    }

    /// Inverse of [`Self::from_v2`]. Matches
    /// `forskscope_desktop::persist::schema::settings::ui_builtin_profiles`'s exact
    /// mapping so a round trip through the UI never changes a built-in
    /// profile's canonical shape.
    pub fn to_v2(&self) -> PersistedDiffProfile {
        PersistedDiffProfile {
            name: self.name.clone(),
            whitespace: self.whitespace,
            newlines: self.newlines,
            case: if self.ignore_case {
                CaseSensitivity::Insensitive
            } else {
                CaseSensitivity::Sensitive
            },
            inline_mode: self.inline_mode,
            algorithm: match self.algorithm {
                DiffAlgorithmSetting::Myers => DiffAlgorithm::Myers,
                DiffAlgorithmSetting::Patience => DiffAlgorithm::Patience,
                DiffAlgorithmSetting::Histogram => DiffAlgorithm::Histogram,
                DiffAlgorithmSetting::Lcs => DiffAlgorithm::Lcs,
            },
            built_in: self.built_in,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub theme: Theme,
    /// RFC-088: a second, independent axis beside `theme` — it combines
    /// with every theme rather than adding new ones.
    pub colour_vision: ColourVision,
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
    /// When `true`, hidden entries (dotfiles; the Windows hidden attribute) are
    /// excluded from the Explorer tree **and** from every comparison that
    /// reads this directory tree — recursive compare, Deep Compare, RFC-080
    /// tier 1 (F149). Default `false`: hidden entries are shown.
    pub hide_dotfiles: bool,
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
    pub fn ignore_rules(&self) -> forskscope_compare::IgnoreRules {
        let mut rules = forskscope_compare::IgnoreRules::from_settings(
            &self.ignore_extensions,
            &self.ignore_dirs,
        );
        rules.hide_dotfiles = self.hide_dotfiles;
        rules
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
            colour_vision: v2.colour_vision,
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
            hide_dotfiles: v2.hide_dotfiles,
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
            colour_vision: self.colour_vision,
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
            hide_dotfiles: self.hide_dotfiles,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// F159(a): the mode a profile carries must reach the diff, or the
    /// Settings checkbox would save a setting that changes nothing. Falsify by
    /// hardcoding `ignore_newlines: false` in `to_diff_options`.
    #[test]
    fn a_profiles_newline_mode_reaches_the_diff_options() {
        let profile = DiffProfile {
            name: "Ignore newlines".into(),
            whitespace: WhitespaceMode::Significant,
            ignore_case: false,
            algorithm: DiffAlgorithmSetting::Myers,
            newlines: NewlineCompareMode::IgnoreDifference,
            inline_mode: InlineMode::Lazy,
            built_in: false,
        };
        assert!(profile.to_diff_options().ignore_newlines);
    }

    // ── F168 §2.1: lossless round trip ─────────────────────────────────────

    const ALL_WHITESPACE: [WhitespaceMode; 4] = [
        WhitespaceMode::Significant,
        WhitespaceMode::IgnoreTrailing,
        WhitespaceMode::IgnoreAll,
        WhitespaceMode::IgnoreBlankLines,
    ];
    const ALL_NEWLINES: [NewlineCompareMode; 2] = [
        NewlineCompareMode::Significant,
        NewlineCompareMode::IgnoreDifference,
    ];
    const ALL_CASE: [CaseSensitivity; 2] =
        [CaseSensitivity::Sensitive, CaseSensitivity::Insensitive];
    const ALL_INLINE: [InlineMode; 3] = [
        InlineMode::None,
        InlineMode::Lazy,
        InlineMode::EagerForSmallHunks,
    ];
    const ALL_ALGORITHMS: [DiffAlgorithm; 4] = [
        DiffAlgorithm::Myers,
        DiffAlgorithm::Patience,
        DiffAlgorithm::Histogram,
        DiffAlgorithm::Lcs,
    ];

    fn every_persisted_profile() -> Vec<PersistedDiffProfile> {
        let mut out = Vec::new();
        for &whitespace in &ALL_WHITESPACE {
            for &newlines in &ALL_NEWLINES {
                for &case in &ALL_CASE {
                    for &inline_mode in &ALL_INLINE {
                        for &algorithm in &ALL_ALGORITHMS {
                            for &built_in in &[false, true] {
                                out.push(PersistedDiffProfile {
                                    name: "p".to_string(),
                                    whitespace,
                                    newlines,
                                    case,
                                    inline_mode,
                                    algorithm,
                                    built_in,
                                });
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// F168: every value `PersistedDiffProfile` can hold survives a trip
    /// through the UI's `DiffProfile`, not just the ones the dialog itself
    /// writes — 4×2×2×3×4×2 = 384 combinations, not samples. Falsify by
    /// reintroducing any one of the three old collapses (e.g. `from_v2`
    /// folding `WhitespaceMode::IgnoreTrailing` to `IgnoreAll`, or `to_v2`
    /// hardcoding `inline_mode: InlineMode::Lazy`, or `DiffAlgorithm::Lcs`
    /// folding to `Myers`): this fails on the matching combination.
    #[test]
    fn every_persisted_profile_round_trips_through_the_ui_type_exactly() {
        for p in every_persisted_profile() {
            let round_tripped = DiffProfile::from_v2(&p).to_v2();
            assert_eq!(round_tripped, p, "did not round-trip: {p:?}");
        }
    }

    #[test]
    fn to_diff_options_carries_inline_mode_through() {
        for &inline_mode in &ALL_INLINE {
            let profile = DiffProfile {
                name: "p".into(),
                whitespace: WhitespaceMode::Significant,
                ignore_case: false,
                algorithm: DiffAlgorithmSetting::Myers,
                newlines: NewlineCompareMode::Significant,
                inline_mode,
                built_in: false,
            };
            assert_eq!(profile.to_diff_options().inline_mode, inline_mode);
        }
    }
}
