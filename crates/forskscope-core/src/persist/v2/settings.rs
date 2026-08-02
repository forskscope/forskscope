//! Settings schema v2 (RFC-076 §"Settings schema v2").
//!
//! [`PersistedSettingsV2`] is a superset of the fields represented by either
//! the shipping UI's plain-JSON `AppSettings` (schema v0) or the existing,
//! never-shipped-to-users core `UserSettings` (schema v1) — so converging
//! them cannot silently discard a field either source actually uses. The
//! diff-pane font (UI-owned, five families) and the appearance font
//! (core-owned, three families) stay distinct fields per RFC-076: `CourierNew`
//! and `Consolas` are exact choices that must not normalize to `SystemMono`.

mod legacy;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{PersistenceError, PersistenceLoad};
use crate::diff::{CaseSensitivity, DiffAlgorithm, InlineMode, NewlineCompareMode, WhitespaceMode};
use crate::encoding::NewlinePolicy;
use crate::job::PerformanceLimits;
use crate::settings::UserSettings;
use crate::settings::display::{
    Density, DiffFontFamilySetting, FontFamilySetting, LocaleId, ThemeId,
};
use legacy::{
    LegacyAppSettingsV0, LegacyDiffAlgorithmV0, LegacyDiffFontFamilyV0, LegacyLangV0, LegacyThemeV0,
};

pub const SETTINGS_SCHEMA_NAME: &str = "settings";
pub const SETTINGS_SCHEMA_VERSION_V2: u32 = 2;

const FONT_SIZE_MIN: u8 = 6;
const FONT_SIZE_MAX: u8 = 50;
const CONTEXT_LINES_MAX: usize = 20;

// ── Canonical v2 payload ────────────────────────────────────────────────────

/// The canonical, converged settings payload. See the module doc for why
/// some fields look duplicated — they represent genuinely distinct settings
/// that the UI and core each already track separately.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedSettingsV2 {
    pub theme: ThemeId,
    pub language: LocaleId,

    /// Diff-pane font size (UI-owned).
    pub diff_font_size: u32,
    /// Diff-pane font family (UI-owned; five choices, not normalized).
    #[serde(default)]
    pub diff_font_family: DiffFontFamilySetting,

    /// Appearance (application chrome) font size (core-owned).
    #[serde(default = "default_appearance_font_size")]
    pub appearance_font_size: u8,
    /// Appearance font family (core-owned; three choices).
    #[serde(default)]
    pub appearance_font_family: FontFamilySetting,
    #[serde(default)]
    pub density: Density,

    pub context_lines: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_left_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_right_dir: Option<PathBuf>,

    pub profiles: Vec<PersistedDiffProfileV2>,
    pub active_profile: usize,

    #[serde(default)]
    pub ignore_extensions: String,
    #[serde(default)]
    pub ignore_dirs: String,
    #[serde(default)]
    pub explorer_compact: bool,
    #[serde(default)]
    pub enable_binary_comparison: bool,
    #[serde(default = "default_true")]
    pub remember_explorer_dirs: bool,

    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default)]
    pub wrap_long_lines: bool,
    #[serde(default)]
    pub newline_policy: NewlinePolicy,

    #[serde(default = "default_true")]
    pub restore_session: bool,
    #[serde(default = "default_recent_limit")]
    pub recent_limit: usize,
    #[serde(default)]
    pub performance: PerformanceLimits,
}

/// One named compare preset in v2 shape. Modeled on the core's richer
/// [`crate::diff::CompareProfile`] rather than the UI's two-boolean v0 shape,
/// since v2 is the converged canonical form; v0 profiles migrate up into it
/// (see [`migrate_from_v0`]), never the reverse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedDiffProfileV2 {
    pub name: String,
    #[serde(default)]
    pub whitespace: WhitespaceMode,
    #[serde(default)]
    pub newlines: NewlineCompareMode,
    #[serde(default)]
    pub case: CaseSensitivity,
    #[serde(default)]
    pub inline_mode: InlineMode,
    #[serde(default)]
    pub algorithm: DiffAlgorithm,
    /// Built-in profiles ship with the app and cannot be deleted.
    #[serde(default)]
    pub built_in: bool,
}

fn default_true() -> bool {
    true
}
fn default_recent_limit() -> usize {
    20
}
fn default_appearance_font_size() -> u8 {
    14
}

// ── Routing ──────────────────────────────────────────────────────────────

/// Load and route a raw settings file. Pure: no file I/O, no side effects.
pub fn load_settings_v2(raw: &str) -> PersistenceLoad<PersistedSettingsV2> {
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => {
            return PersistenceLoad::Corrupt {
                detail: PersistenceError::MalformedJson,
            };
        }
    };
    let Some(obj) = value.as_object() else {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedJson,
        };
    };

    let Some(name_value) = obj.get("schema_name") else {
        return match serde_json::from_value::<LegacyAppSettingsV0>(value.clone()) {
            Ok(v0) => PersistenceLoad::MigratedLegacy {
                value: migrate_from_v0(v0),
                source_backup_required: true,
            },
            Err(_) => PersistenceLoad::Corrupt {
                detail: PersistenceError::UnrecognizedLegacyShape,
            },
        };
    };
    let Some(name) = name_value.as_str() else {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedJson,
        };
    };
    if name != SETTINGS_SCHEMA_NAME {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::SchemaNameMismatch {
                found: name.to_string(),
            },
        };
    }
    let Some(version) = obj.get("schema_version").and_then(|v| v.as_u64()) else {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedVersion,
        };
    };
    let version = version as u32;
    if version > SETTINGS_SCHEMA_VERSION_V2 {
        return PersistenceLoad::FutureVersion {
            schema: name.to_string(),
            version,
        };
    }
    let Some(payload) = obj.get("payload") else {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedPayload,
        };
    };
    match version {
        SETTINGS_SCHEMA_VERSION_V2 => {
            match serde_json::from_value::<PersistedSettingsV2>(payload.clone()) {
                Ok(v2) => PersistenceLoad::Current {
                    value: normalize(v2),
                },
                Err(_) => PersistenceLoad::Corrupt {
                    detail: PersistenceError::MalformedPayload,
                },
            }
        }
        1 => match UserSettings::from_payload_json(&payload.to_string()) {
            Ok(v1) => PersistenceLoad::MigratedVersion {
                value: migrate_from_v1(v1),
                from: 1,
            },
            Err(_) => PersistenceLoad::Corrupt {
                detail: PersistenceError::MalformedPayload,
            },
        },
        _ => PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedVersion,
        },
    }
}

// ── Migration ────────────────────────────────────────────────────────────

fn migrate_from_v0(v0: LegacyAppSettingsV0) -> PersistedSettingsV2 {
    let profiles = v0
        .profiles
        .iter()
        .map(|p| PersistedDiffProfileV2 {
            name: p.name.clone(),
            whitespace: if p.ignore_whitespace {
                WhitespaceMode::IgnoreAll
            } else {
                WhitespaceMode::Significant
            },
            newlines: NewlineCompareMode::Significant,
            case: if p.ignore_case {
                CaseSensitivity::Insensitive
            } else {
                CaseSensitivity::Sensitive
            },
            inline_mode: InlineMode::Lazy,
            algorithm: match p.algorithm {
                LegacyDiffAlgorithmV0::Myers => DiffAlgorithm::Myers,
                LegacyDiffAlgorithmV0::Patience => DiffAlgorithm::Patience,
                LegacyDiffAlgorithmV0::Histogram => DiffAlgorithm::Histogram,
            },
            built_in: p.built_in,
        })
        .collect();

    normalize(PersistedSettingsV2 {
        theme: match v0.theme {
            LegacyThemeV0::Dark => ThemeId::Dark,
            LegacyThemeV0::Light => ThemeId::Light,
            LegacyThemeV0::Night => ThemeId::Night,
        },
        language: match v0.language {
            LegacyLangV0::En => LocaleId::english(),
            LegacyLangV0::Ja => LocaleId::japanese(),
        },
        diff_font_size: v0.diff_font_size,
        diff_font_family: match v0.diff_font_family {
            LegacyDiffFontFamilyV0::Monospace => DiffFontFamilySetting::SystemMono,
            LegacyDiffFontFamilyV0::SansSerif => DiffFontFamilySetting::SystemSans,
            LegacyDiffFontFamilyV0::Serif => DiffFontFamilySetting::SystemSerif,
            LegacyDiffFontFamilyV0::CourierNew => DiffFontFamilySetting::CourierNew,
            LegacyDiffFontFamilyV0::Consolas => DiffFontFamilySetting::Consolas,
        },
        // v0 has no appearance-font or density concept; core defaults apply.
        appearance_font_size: default_appearance_font_size(),
        appearance_font_family: FontFamilySetting::default(),
        density: Density::default(),
        context_lines: v0.context_lines,
        last_left_dir: v0.last_left_dir,
        last_right_dir: v0.last_right_dir,
        profiles,
        active_profile: v0.active_profile,
        ignore_extensions: v0.ignore_extensions,
        ignore_dirs: v0.ignore_dirs,
        explorer_compact: v0.explorer_compact,
        enable_binary_comparison: v0.enable_binary_comparison,
        remember_explorer_dirs: v0.remember_explorer_dirs,
        // v0 has no display/file-policy concept for these; core defaults apply.
        show_line_numbers: true,
        wrap_long_lines: false,
        newline_policy: NewlinePolicy::default(),
        restore_session: true,
        recent_limit: default_recent_limit(),
        performance: PerformanceLimits::default(),
    })
}

fn migrate_from_v1(v1: UserSettings) -> PersistedSettingsV2 {
    let selected = PersistedDiffProfileV2 {
        name: v1.diff.compare_profile.name.clone(),
        whitespace: v1.diff.compare_profile.whitespace,
        newlines: v1.diff.compare_profile.newlines,
        case: v1.diff.compare_profile.case,
        inline_mode: v1.diff.compare_profile.inline_mode,
        algorithm: v1.diff.compare_profile.algorithm,
        built_in: is_core_preset_name(&v1.diff.compare_profile.name),
    };
    let mut profiles = vec![selected];
    for builtin in ui_builtin_profiles() {
        if !profiles.iter().any(|p| p.name == builtin.name) {
            profiles.push(builtin);
        }
    }

    normalize(PersistedSettingsV2 {
        theme: v1.appearance.theme,
        language: v1.locale.locale.clone(),
        // v1 has no diff-pane font concept distinct from appearance; UI
        // defaults apply and the user's next UI-driven save fills it in.
        diff_font_size: default_appearance_font_size() as u32,
        diff_font_family: DiffFontFamilySetting::default(),
        appearance_font_size: v1.appearance.font_size,
        appearance_font_family: v1.appearance.font_family,
        density: v1.appearance.density,
        // v1 has no context-lines, explorer, or ignore-pattern concept; the
        // UI's own default (matches `legacy::legacy_default_context_lines`).
        context_lines: 3,
        last_left_dir: None,
        last_right_dir: None,
        profiles,
        active_profile: 0,
        ignore_extensions: String::new(),
        ignore_dirs: String::new(),
        explorer_compact: false,
        enable_binary_comparison: false,
        remember_explorer_dirs: true,
        show_line_numbers: v1.diff.show_line_numbers,
        wrap_long_lines: v1.diff.wrap_long_lines,
        newline_policy: v1.files.newline_policy,
        restore_session: v1.files.restore_session,
        recent_limit: v1.files.recent_limit,
        performance: v1.files.performance,
    })
}

/// The UI's four shipping built-in profiles, expressed in v2 shape. Canonical
/// for v2 because these are the profiles users have actually seen; core's
/// richer [`crate::diff::CompareProfile::all_presets`] is a different,
/// UI-unreached preset set (see the review request for the full rationale).
fn ui_builtin_profiles() -> Vec<PersistedDiffProfileV2> {
    let base = |name: &str, whitespace, case, algorithm| PersistedDiffProfileV2 {
        name: name.to_string(),
        whitespace,
        newlines: NewlineCompareMode::Significant,
        case,
        inline_mode: InlineMode::Lazy,
        algorithm,
        built_in: true,
    };
    vec![
        base(
            "Exact (default)",
            WhitespaceMode::Significant,
            CaseSensitivity::Sensitive,
            DiffAlgorithm::Myers,
        ),
        base(
            "Ignore whitespace",
            WhitespaceMode::IgnoreAll,
            CaseSensitivity::Sensitive,
            DiffAlgorithm::Myers,
        ),
        base(
            "Ignore case",
            WhitespaceMode::Significant,
            CaseSensitivity::Insensitive,
            DiffAlgorithm::Myers,
        ),
        base(
            "Histogram",
            WhitespaceMode::Significant,
            CaseSensitivity::Sensitive,
            DiffAlgorithm::Histogram,
        ),
    ]
}

fn is_core_preset_name(name: &str) -> bool {
    matches!(
        name,
        "Default" | "Code Review" | "Loose Text" | "Large File Safe"
    )
}

/// Normalizes invalid indexes/ranges without dropping otherwise valid fields
/// (RFC-076 §"Validation").
fn normalize(mut v2: PersistedSettingsV2) -> PersistedSettingsV2 {
    v2.appearance_font_size = v2.appearance_font_size.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX);
    v2.context_lines = v2.context_lines.min(CONTEXT_LINES_MAX);
    if v2.profiles.is_empty() {
        v2.profiles = ui_builtin_profiles();
    }
    if v2.active_profile >= v2.profiles.len() {
        v2.active_profile = 0;
    }
    v2
}
