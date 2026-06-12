//! UserSettings model and JSON persistence tests (RFC-009 §4, §6, §10).

use crate::diff::CompareProfile;
use crate::encoding::NewlinePolicy;
use crate::persist::MigrationPolicy;
use crate::settings::{
    AppearanceSettings, Density, FontFamilySetting, LocaleId, LocaleSettings,
    ThemeId, UserSettings, SETTINGS_SCHEMA_VERSION,
};

// ── Defaults ──────────────────────────────────────────────────────────────────

#[test]
fn default_settings_are_valid_first_run_state() {
    let s = UserSettings::default();
    assert_eq!(s.appearance.theme, ThemeId::Dark);
    assert_eq!(s.appearance.font_size, 14);
    assert_eq!(s.appearance.density, Density::Comfortable);
    assert!(s.diff.show_line_numbers);
    assert!(!s.diff.wrap_long_lines);
    assert_eq!(s.diff.compare_profile, CompareProfile::default());
    assert_eq!(s.files.newline_policy, NewlinePolicy::Preserve);
    assert!(s.files.restore_session);
    assert_eq!(s.files.recent_limit, 20);
    assert_eq!(s.locale.locale.as_str(), "en");
}

// ── ThemeId ───────────────────────────────────────────────────────────────────

#[test]
fn all_theme_ids_round_trip_through_as_str() {
    for theme in ThemeId::all() {
        let s = theme.as_str();
        assert_eq!(ThemeId::from_id(s), Some(*theme), "{s} must round-trip");
    }
}

#[test]
fn theme_id_from_str_unknown_returns_none() {
    assert_eq!(ThemeId::from_id("solarized"), None);
}

#[test]
fn theme_css_var_names_returns_twelve_entries_per_theme() {
    use crate::settings::ThemeTokens;
    for theme in ThemeId::all() {
        assert_eq!(ThemeTokens::css_var_names(*theme).len(), 12,
            "must have 12 CSS variables per theme");
    }
}

#[test]
fn css_var_names_all_start_with_fsk_prefix() {
    use crate::settings::ThemeTokens;
    for (var, _) in ThemeTokens::css_var_names(ThemeId::Dark) {
        assert!(var.starts_with("--fsk-"), "CSS var {var} must have --fsk- prefix");
    }
}

// ── Density ───────────────────────────────────────────────────────────────────

#[test]
fn all_density_values_round_trip() {
    for d in [Density::Comfortable, Density::Compact, Density::Spacious] {
        assert_eq!(Density::from_id(d.as_str()), Some(d));
    }
}

// ── FontFamilySetting ────────────────────────────────────────────────────────

#[test]
fn all_font_family_settings_round_trip() {
    for f in [
        FontFamilySetting::SystemMono,
        FontFamilySetting::SystemSans,
        FontFamilySetting::SystemSerif,
    ] {
        assert_eq!(FontFamilySetting::from_id(f.as_str()), Some(f));
    }
}

// ── JSON round-trip ───────────────────────────────────────────────────────────

#[test]
fn default_settings_round_trip_through_json() {
    let original = UserSettings::default();
    let json = original.to_json();
    let parsed = UserSettings::from_json(&json).unwrap();
    assert_eq!(parsed.migration, MigrationPolicy::CompatibleRead);
    let s = parsed.settings;
    assert_eq!(s.appearance.theme,       original.appearance.theme);
    assert_eq!(s.appearance.font_size,   original.appearance.font_size);
    assert_eq!(s.appearance.density,     original.appearance.density);
    assert_eq!(s.diff.show_line_numbers, original.diff.show_line_numbers);
    assert_eq!(s.diff.wrap_long_lines,   original.diff.wrap_long_lines);
    assert_eq!(s.files.newline_policy,   original.files.newline_policy);
    assert_eq!(s.files.restore_session,  original.files.restore_session);
    assert_eq!(s.files.recent_limit,     original.files.recent_limit);
    assert_eq!(s.locale.locale.as_str(), original.locale.locale.as_str());
}

#[test]
fn non_default_settings_round_trip() {
    let mut original = UserSettings::default();
    original.appearance.theme       = ThemeId::Light;
    original.appearance.font_size   = 18;
    original.appearance.density     = Density::Compact;
    original.diff.show_line_numbers = false;
    original.diff.wrap_long_lines   = true;
    original.diff.compare_profile   = CompareProfile::code_review();
    original.files.newline_policy   = NewlinePolicy::ForceLf;
    original.files.restore_session  = false;
    original.files.recent_limit     = 10;
    original.locale.locale          = LocaleId::japanese();

    let json = original.to_json();
    let parsed = UserSettings::from_json(&json).unwrap().settings;

    assert_eq!(parsed.appearance.theme,       ThemeId::Light);
    assert_eq!(parsed.appearance.font_size,   18);
    assert_eq!(parsed.appearance.density,     Density::Compact);
    assert!(!parsed.diff.show_line_numbers);
    assert!(parsed.diff.wrap_long_lines);
    assert_eq!(parsed.diff.compare_profile.name, "Code Review");
    assert_eq!(parsed.files.newline_policy,   NewlinePolicy::ForceLf);
    assert!(!parsed.files.restore_session);
    assert_eq!(parsed.files.recent_limit,     10);
    assert_eq!(parsed.locale.locale.as_str(), "ja");
}

#[test]
fn json_contains_schema_version() {
    let json = UserSettings::default().to_json();
    assert!(json.contains(&format!("\"schema_version\": {SETTINGS_SCHEMA_VERSION}")));
}

#[test]
fn newer_schema_version_returns_error() {
    let json = UserSettings::default().to_json();
    let bumped = json.replace(
        &format!("\"schema_version\": {SETTINGS_SCHEMA_VERSION}"),
        &format!("\"schema_version\": {}", SETTINGS_SCHEMA_VERSION + 5),
    );
    assert!(UserSettings::from_json(&bumped).is_err(),
        "settings from a newer app must not silently load");
}

#[test]
fn corrupt_json_falls_back_to_defaults_not_error() {
    // RFC-009 §10: unknown/corrupt fields → fall back to defaults.
    // (Envelope still needs to be valid; the payload can be mangled.)
    let original = UserSettings::default();
    let json = original.to_json();
    // Corrupt the payload by injecting garbage theme value.
    let corrupted = json.replace("\"dark\"", "\"undefined_theme\"");
    let result = UserSettings::from_json(&corrupted);
    assert!(result.is_ok(), "corrupt payload must fall back to defaults, not error");
    let s = result.unwrap().settings;
    // Theme fell back to default because "undefined_theme" is unknown.
    assert_eq!(s.appearance.theme, ThemeId::Dark);
}

// ── LocaleId ─────────────────────────────────────────────────────────────────

#[test]
fn locale_id_helpers() {
    assert_eq!(LocaleId::english().as_str(), "en");
    assert_eq!(LocaleId::japanese().as_str(), "ja");
}

// ── AppearanceSettings font_size bounds ───────────────────────────────────────

#[test]
fn font_size_is_clamped_on_load() {
    // Inject an out-of-range font_size into the JSON payload.
    let mut s = UserSettings::default();
    s.appearance.font_size = 14;
    let json = s.to_json();
    // Replace the font_size with an extreme value.
    let too_big = json.replace("\"font_size\": 14", "\"font_size\": 999");
    let loaded = UserSettings::from_json(&too_big).unwrap().settings;
    assert!(loaded.appearance.font_size <= 50,
        "font_size must be clamped to ≤ 50 on load");
}

#[test]
fn settings_schema_version_is_one() {
    // Stable value — changing it requires a migration plan.
    assert_eq!(SETTINGS_SCHEMA_VERSION, 1);
}
