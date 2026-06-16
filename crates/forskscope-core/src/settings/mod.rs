//! User settings model (RFC-009 §4, §6, §9, §10).
//!
//! `UserSettings` is the canonical settings record. It is persisted as JSON
//! via [`VersionedEnvelope`] (`SchemaName::Settings`, schema v1), read at
//! startup, and written on every deliberate user change.
//!
//! ## Design
//!
//! - All fields are plain Rust types — no Dioxus, no CSS, no GTK.
//! - `ThemeTokens` maps theme IDs to CSS variable names consumed by the
//!   Dioxus app; the core does not render CSS.
//! - `UserSettings::to_json` / `from_json` use `VersionedEnvelope` so the
//!   migration policy (`TooNew`, `ForwardMigration`) is automatic.
//! - Unknown JSON fields are silently ignored (forward-compat rule from
//!   RFC-009 §10).

pub mod display;
pub use display::{
    Density, FontFamilySetting, LocaleId, ThemeId, ThemeTokens,
    SETTINGS_SCHEMA_VERSION,
};

use std::fmt::Write as _;
use crate::diff::CompareProfile;
use crate::encoding::NewlinePolicy;
use crate::job::PerformanceLimits;
use crate::persist::{MigrationPolicy, SchemaName, VersionedEnvelope};

// ── Settings sections ─────────────────────────────────────────────────────────

/// Appearance settings (RFC-009 §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceSettings {
    pub theme:       ThemeId,
    pub density:     Density,
    pub font_family: FontFamilySetting,
    /// Point size for the diff font (6–50, default 14).
    pub font_size:   u8,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme:       ThemeId::Dark,
            density:     Density::Comfortable,
            font_family: FontFamilySetting::SystemMono,
            font_size:   14,
        }
    }
}

/// Diff view settings.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffSettings {
    /// The active compare profile (drives `DiffOptions`).
    pub compare_profile: CompareProfile,
    /// Show line numbers in diff panes.
    pub show_line_numbers: bool,
    /// Wrap long lines instead of horizontal scrolling.
    pub wrap_long_lines: bool,
}

impl Default for DiffSettings {
    fn default() -> Self {
        Self {
            compare_profile:   CompareProfile::default(),
            show_line_numbers: true,
            wrap_long_lines:   false,
        }
    }
}

/// File handling settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSettings {
    pub newline_policy:    NewlinePolicy,
    pub performance:       PerformanceLimits,
    /// Re-open the last session on startup.
    pub restore_session:   bool,
    /// Remember recently opened file pairs.
    pub recent_limit:      usize,
}

impl Default for FileSettings {
    fn default() -> Self {
        Self {
            newline_policy:  NewlinePolicy::Preserve,
            performance:     PerformanceLimits::default(),
            restore_session: true,
            recent_limit:    20,
        }
    }
}

/// Locale / language settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleSettings {
    pub locale: LocaleId,
}

impl Default for LocaleSettings {
    fn default() -> Self {
        Self { locale: LocaleId::english() }
    }
}

// ── Top-level UserSettings ────────────────────────────────────────────────────

/// The complete user settings record (RFC-009 §4).
///
/// Persisted via [`VersionedEnvelope`] as `SchemaName::Settings`.
/// Defaults are the "first-run" values and must never produce invalid state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserSettings {
    pub appearance: AppearanceSettings,
    pub diff:       DiffSettings,
    pub files:      FileSettings,
    pub locale:     LocaleSettings,
}

impl UserSettings {
    // ── Serialisation ─────────────────────────────────────────────────────

    /// Serialise to a `VersionedEnvelope` JSON string.
    pub fn to_json(&self) -> String {
        let payload = self.to_payload_json();
        VersionedEnvelope::new(SchemaName::Settings, SETTINGS_SCHEMA_VERSION, payload)
            .to_json()
    }

    /// Parse from envelope JSON. Returns `Err(String)` on any failure.
    pub fn from_json(json: &str) -> Result<ParsedSettings, String> {
        let envelope = VersionedEnvelope::parse(json)
            .map_err(|e| format!("settings envelope: {e}"))?;
        let migration = envelope.migration_policy(SETTINGS_SCHEMA_VERSION);
        if let MigrationPolicy::NewerSchema { file_version, .. } = migration {
            return Err(format!(
                "settings were written by a newer ForskScope (schema v{file_version})"
            ));
        }
        let settings = Self::from_payload_json(&envelope.payload_json)
            .unwrap_or_else(|_| {
                // RFC-009 §10: unknown/corrupt fields → fall back to defaults.
                UserSettings::default()
            });
        Ok(ParsedSettings { settings, migration })
    }

    // ── Internal JSON helpers ──────────────────────────────────────────────

    fn to_payload_json(&self) -> String {
        let a = &self.appearance;
        let d = &self.diff;
        let f = &self.files;
        let l = &self.locale;
        let mut s = String::new();
        let _ = writeln!(s, "{{");
        let _ = writeln!(s, "  \"appearance\": {{");
        let _ = writeln!(s, "    \"theme\": {:?},",       a.theme.as_str());
        let _ = writeln!(s, "    \"density\": {:?},",     a.density.as_str());
        let _ = writeln!(s, "    \"font_family\": {:?},", a.font_family.as_str());
        let _ = writeln!(s, "    \"font_size\": {}",      a.font_size);
        let _ = writeln!(s, "  }},");
        let _ = writeln!(s, "  \"diff\": {{");
        let _ = writeln!(s, "    \"compare_profile\": {:?},", d.compare_profile.name);
        let _ = writeln!(s, "    \"show_line_numbers\": {},", d.show_line_numbers);
        let _ = writeln!(s, "    \"wrap_long_lines\": {}",    d.wrap_long_lines);
        let _ = writeln!(s, "  }},");
        let _ = writeln!(s, "  \"files\": {{");
        let _ = writeln!(s, "    \"newline_policy\": {:?},",  newline_policy_str(f.newline_policy));
        let _ = writeln!(s, "    \"restore_session\": {},",   f.restore_session);
        let _ = writeln!(s, "    \"recent_limit\": {}",       f.recent_limit);
        let _ = writeln!(s, "  }},");
        let _ = writeln!(s, "  \"locale\": {:?}", l.locale.as_str());
        let _ = write!(s, "}}");
        s
    }

    fn from_payload_json(json: &str) -> Result<Self, ()> {
        let theme = extract_nested_str(json, "appearance", "theme")
            .and_then(|s| ThemeId::from_id(&s))
            .unwrap_or_default();
        let density = extract_nested_str(json, "appearance", "density")
            .and_then(|s| Density::from_id(&s))
            .unwrap_or_default();
        let font_family = extract_nested_str(json, "appearance", "font_family")
            .and_then(|s| FontFamilySetting::from_id(&s))
            .unwrap_or_default();
        let font_size = extract_nested_u64(json, "appearance", "font_size")
            .map(|n| (n.clamp(6, 50)) as u8)
            .unwrap_or(14);

        let profile_name = extract_nested_str(json, "diff", "compare_profile")
            .unwrap_or_else(|| "Default".into());
        let compare_profile = CompareProfile::all_presets()
            .into_iter()
            .find(|p| p.name == profile_name)
            .unwrap_or_default();
        let show_line_numbers = extract_nested_bool(json, "diff", "show_line_numbers")
            .unwrap_or(true);
        let wrap_long_lines = extract_nested_bool(json, "diff", "wrap_long_lines")
            .unwrap_or(false);

        let newline_policy = extract_nested_str(json, "files", "newline_policy")
            .as_deref()
            .and_then(newline_policy_from_str)
            .unwrap_or_default();
        let restore_session = extract_nested_bool(json, "files", "restore_session")
            .unwrap_or(true);
        let recent_limit = extract_nested_u64(json, "files", "recent_limit")
            .map(|n| n as usize)
            .unwrap_or(20);

        let locale_str = extract_str(json, "locale").unwrap_or_else(|| "en".into());

        Ok(UserSettings {
            appearance: AppearanceSettings { theme, density, font_family, font_size },
            diff: DiffSettings { compare_profile, show_line_numbers, wrap_long_lines },
            files: FileSettings {
                newline_policy,
                performance: PerformanceLimits::default(),
                restore_session,
                recent_limit,
            },
            locale: LocaleSettings { locale: LocaleId(locale_str) },
        })
    }
}

/// Result of [`UserSettings::from_json`].
pub struct ParsedSettings {
    pub settings:  UserSettings,
    pub migration: MigrationPolicy,
}

// ── Parse helpers ─────────────────────────────────────────────────────────────

fn extract_str(json: &str, field: &str) -> Option<String> {
    let key = format!("\"{}\":", field);
    let start = json.find(&key)? + key.len();
    let rest = json[start..].trim_start();
    if !rest.starts_with('"') { return None; }
    let inner = &rest[1..];
    let end = inner.find('"')?;
    Some(inner[..end].into())
}

fn extract_nested_str(json: &str, section: &str, field: &str) -> Option<String> {
    let sec_key = format!("\"{}\":", section);
    let sec_start = json.find(&sec_key)? + sec_key.len();
    let sec_rest = json[sec_start..].trim_start();
    if !sec_rest.starts_with('{') { return None; }
    let depth_end = find_close_brace(sec_rest)?;
    let section_json = &sec_rest[..depth_end + 1];
    extract_str(section_json, field)
}

fn extract_nested_u64(json: &str, section: &str, field: &str) -> Option<u64> {
    let sec_key = format!("\"{}\":", section);
    let sec_start = json.find(&sec_key)? + sec_key.len();
    let sec_rest = json[sec_start..].trim_start();
    if !sec_rest.starts_with('{') { return None; }
    let depth_end = find_close_brace(sec_rest)?;
    let section_json = &sec_rest[..depth_end + 1];
    let key = format!("\"{}\":", field);
    let start = section_json.find(&key)? + key.len();
    let rest = section_json[start..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn extract_nested_bool(json: &str, section: &str, field: &str) -> Option<bool> {
    let sec_key = format!("\"{}\":", section);
    let sec_start = json.find(&sec_key)? + sec_key.len();
    let sec_rest = json[sec_start..].trim_start();
    if !sec_rest.starts_with('{') { return None; }
    let depth_end = find_close_brace(sec_rest)?;
    let section_json = &sec_rest[..depth_end + 1];
    let key = format!("\"{}\":", field);
    let start = section_json.find(&key)? + key.len();
    let rest = section_json[start..].trim_start();
    if rest.starts_with("true")  { return Some(true);  }
    if rest.starts_with("false") { return Some(false); }
    None
}

fn find_close_brace(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        if ch == '{' { depth += 1; }
        if ch == '}' { depth -= 1; if depth == 0 { return Some(i); } }
    }
    None
}

fn newline_policy_str(p: NewlinePolicy) -> &'static str {
    match p {
        NewlinePolicy::Preserve  => "preserve",
        NewlinePolicy::ForceLf   => "lf",
        NewlinePolicy::ForceCrlf => "crlf",
    }
}

fn newline_policy_from_str(s: &str) -> Option<NewlinePolicy> {
    match s {
        "preserve" => Some(NewlinePolicy::Preserve),
        "lf"       => Some(NewlinePolicy::ForceLf),
        "crlf"     => Some(NewlinePolicy::ForceCrlf),
        _          => None,
    }
}
