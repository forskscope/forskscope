//! RFC-076 settings schema v2: routing, migration, and validation tests.
//!
//! Covers the acceptance criteria from the RFC-076 handoff's §"Core schema
//! tests": current-envelope round-trip, exact field-for-field legacy v0 and
//! core v1 migration, future/corrupt/unrecognized-legacy handling, unknown
//! payload fields tolerated, and range/index normalization.

use crate::diff::{CaseSensitivity, DiffAlgorithm, InlineMode, NewlineCompareMode, WhitespaceMode};
use crate::encoding::NewlinePolicy;
use crate::job::PerformanceLimits;
use crate::persist::v2::PersistenceLoad;
use crate::persist::v2::settings::{PersistedDiffProfileV2, PersistedSettingsV2, load_settings_v2};
use crate::settings::display::{
    Density, DiffFontFamilySetting, FontFamilySetting, LocaleId, ThemeId,
};

const SETTINGS_V0_FIXTURE: &str = include_str!("fixtures/persistence/settings-v0.json");
const SETTINGS_V1_ENVELOPE_FIXTURE: &str =
    include_str!("fixtures/persistence/settings-v1-envelope.json");
const SETTINGS_V2_FIXTURE: &str = include_str!("fixtures/persistence/settings-v2.json");

fn sample_v2() -> PersistedSettingsV2 {
    PersistedSettingsV2 {
        theme: ThemeId::Night,
        language: LocaleId::english(),
        diff_font_size: 15,
        diff_font_family: DiffFontFamilySetting::Consolas,
        appearance_font_size: 12,
        appearance_font_family: FontFamilySetting::SystemSans,
        density: Density::Spacious,
        context_lines: 4,
        last_left_dir: Some("/tmp/fixtures/left".into()),
        last_right_dir: None,
        profiles: vec![PersistedDiffProfileV2 {
            name: "Exact (default)".into(),
            whitespace: WhitespaceMode::Significant,
            newlines: NewlineCompareMode::Significant,
            case: CaseSensitivity::Sensitive,
            inline_mode: InlineMode::Lazy,
            algorithm: DiffAlgorithm::Myers,
            built_in: true,
        }],
        active_profile: 0,
        ignore_extensions: "o, tmp".into(),
        ignore_dirs: "target".into(),
        explorer_compact: true,
        enable_binary_comparison: false,
        remember_explorer_dirs: true,
        show_line_numbers: true,
        wrap_long_lines: false,
        newline_policy: NewlinePolicy::Preserve,
        restore_session: true,
        recent_limit: 20,
        performance: PerformanceLimits::default(),
    }
}

fn envelope_for(schema_version: u32, payload: &serde_json::Value) -> String {
    serde_json::json!({
        "schema_name": "settings",
        "schema_version": schema_version,
        "app_version": "0.165.1",
        "created_unix": 1_700_000_000u64,
        "updated_unix": 1_700_000_000u64,
        "payload": payload,
    })
    .to_string()
}

// ── Current (v2) ─────────────────────────────────────────────────────────

#[test]
fn current_v2_envelope_round_trips() {
    let payload = serde_json::to_value(sample_v2()).unwrap();
    let raw = envelope_for(2, &payload);
    match load_settings_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value, sample_v2()),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn current_v2_tolerates_unknown_payload_fields() {
    let mut payload = serde_json::to_value(sample_v2()).unwrap();
    payload
        .as_object_mut()
        .unwrap()
        .insert("some_future_field".into(), serde_json::json!("unused"));
    let raw = envelope_for(2, &payload);
    match load_settings_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value, sample_v2()),
        other => panic!("expected Current despite unknown field, got {other:?}"),
    }
}

/// Pins the v2 wire *format*, not just round-trip self-consistency (review
/// 035 C2). `SETTINGS_V2_FIXTURE` is a literal JSON file, independent of how
/// `PersistedSettingsV2` currently serializes itself — a struct round-trip
/// test cannot catch a variant rename because both directions would agree on
/// the new name. This test can: it fails the moment any enum variant's wire
/// representation changes, since every enum reachable from the payload
/// appears in this fixture at least once.
#[test]
fn current_v2_golden_fixture_parses_to_the_exact_expected_struct() {
    let expected = PersistedSettingsV2 {
        theme: ThemeId::Night,
        language: LocaleId::japanese(),
        diff_font_size: 15,
        diff_font_family: DiffFontFamilySetting::Consolas,
        appearance_font_size: 12,
        appearance_font_family: FontFamilySetting::SystemSans,
        density: Density::Spacious,
        context_lines: 4,
        last_left_dir: Some("/tmp/fixtures/left".into()),
        last_right_dir: None,
        profiles: vec![
            PersistedDiffProfileV2 {
                name: "Exact (default)".into(),
                whitespace: WhitespaceMode::Significant,
                newlines: NewlineCompareMode::Significant,
                case: CaseSensitivity::Sensitive,
                inline_mode: InlineMode::Lazy,
                algorithm: DiffAlgorithm::Myers,
                built_in: true,
            },
            PersistedDiffProfileV2 {
                name: "Trailing whitespace, insensitive, no inline".into(),
                whitespace: WhitespaceMode::IgnoreTrailing,
                newlines: NewlineCompareMode::IgnoreDifference,
                case: CaseSensitivity::Insensitive,
                inline_mode: InlineMode::None,
                algorithm: DiffAlgorithm::Patience,
                built_in: false,
            },
            PersistedDiffProfileV2 {
                name: "Ignore all whitespace, eager inline, lcs".into(),
                whitespace: WhitespaceMode::IgnoreAll,
                newlines: NewlineCompareMode::Significant,
                case: CaseSensitivity::Sensitive,
                inline_mode: InlineMode::EagerForSmallHunks,
                algorithm: DiffAlgorithm::Lcs,
                built_in: false,
            },
            PersistedDiffProfileV2 {
                name: "Ignore blank lines, histogram".into(),
                whitespace: WhitespaceMode::IgnoreBlankLines,
                newlines: NewlineCompareMode::Significant,
                case: CaseSensitivity::Sensitive,
                inline_mode: InlineMode::Lazy,
                algorithm: DiffAlgorithm::Histogram,
                built_in: false,
            },
        ],
        active_profile: 0,
        ignore_extensions: "o, tmp".into(),
        ignore_dirs: "target".into(),
        explorer_compact: true,
        enable_binary_comparison: false,
        remember_explorer_dirs: true,
        show_line_numbers: true,
        wrap_long_lines: false,
        newline_policy: NewlinePolicy::ForceLf,
        restore_session: true,
        recent_limit: 20,
        performance: PerformanceLimits::default(),
    };
    match load_settings_v2(SETTINGS_V2_FIXTURE) {
        PersistenceLoad::Current { value } => assert_eq!(value, expected),
        other => panic!("expected Current, got {other:?}"),
    }
}

// ── Legacy v0 ────────────────────────────────────────────────────────────

#[test]
fn legacy_v0_fixture_migrates_every_field_exactly() {
    match load_settings_v2(SETTINGS_V0_FIXTURE) {
        PersistenceLoad::MigratedLegacy {
            value,
            source_backup_required,
        } => {
            assert!(source_backup_required);
            assert_eq!(value.theme, ThemeId::Light);
            assert_eq!(value.language, LocaleId::japanese());
            assert_eq!(value.diff_font_size, 16);
            assert_eq!(value.diff_font_family, DiffFontFamilySetting::Consolas);
            assert_eq!(value.context_lines, 5);
            assert_eq!(value.last_left_dir, Some("/tmp/fixtures/left".into()));
            assert_eq!(value.last_right_dir, Some("/tmp/fixtures/right".into()));
            assert_eq!(value.profiles.len(), 5);
            assert_eq!(value.profiles[4].name, "My Custom Profile");
            assert_eq!(value.profiles[4].whitespace, WhitespaceMode::IgnoreAll);
            assert_eq!(value.profiles[4].case, CaseSensitivity::Insensitive);
            assert_eq!(value.profiles[4].algorithm, DiffAlgorithm::Patience);
            assert!(!value.profiles[4].built_in);
            assert_eq!(value.active_profile, 4);
            assert_eq!(value.ignore_extensions, "o, class, tmp");
            assert_eq!(value.ignore_dirs, "target, node_modules");
            assert!(value.explorer_compact);
            assert!(value.enable_binary_comparison);
            assert!(!value.remember_explorer_dirs);
            // Core-only fields not present in v0 fall back to core defaults.
            assert_eq!(value.density, Density::default());
            assert_eq!(value.performance, PerformanceLimits::default());
        }
        other => panic!("expected MigratedLegacy, got {other:?}"),
    }
}

#[test]
fn unrecognized_legacy_shape_is_corrupt_not_defaults() {
    let raw = r#"{"totally":"unrelated","shape":true}"#;
    match load_settings_v2(raw) {
        PersistenceLoad::Corrupt { .. } => {}
        other => panic!("expected Corrupt for an unrecognized no-schema_name shape, got {other:?}"),
    }
}

// ── Core v1 ──────────────────────────────────────────────────────────────

#[test]
fn core_v1_envelope_fixture_migrates_every_represented_field() {
    match load_settings_v2(SETTINGS_V1_ENVELOPE_FIXTURE) {
        PersistenceLoad::MigratedVersion { value, from } => {
            assert_eq!(from, 1);
            assert_eq!(value.theme, ThemeId::Light);
            assert_eq!(value.appearance_font_size, 18);
            assert_eq!(value.appearance_font_family, FontFamilySetting::SystemSerif);
            assert_eq!(value.density, Density::Compact);
            assert_eq!(value.language, LocaleId::japanese());
            assert!(!value.show_line_numbers);
            assert!(value.wrap_long_lines);
            assert_eq!(value.newline_policy, NewlinePolicy::ForceLf);
            assert!(!value.restore_session);
            assert_eq!(value.recent_limit, 7);
            // The one selected core profile becomes profiles[0]...
            assert_eq!(value.profiles[0].name, "Code Review");
            assert_eq!(value.profiles[0].algorithm, DiffAlgorithm::Histogram);
            assert!(value.profiles[0].built_in);
            assert_eq!(value.active_profile, 0);
            // ...and canonical UI built-ins are appended, not replaced.
            assert!(value.profiles.iter().any(|p| p.name == "Exact (default)"));
            assert!(value.profiles.iter().any(|p| p.name == "Ignore whitespace"));
            // v1 has no explorer/ignore-pattern concept: core/UI defaults apply.
            assert_eq!(value.ignore_extensions, "");
            assert!(!value.explorer_compact);
        }
        other => panic!("expected MigratedVersion{{from: 1}}, got {other:?}"),
    }
}

// ── Future / corrupt ─────────────────────────────────────────────────────

#[test]
fn future_schema_version_is_preserved_not_defaulted() {
    let payload = serde_json::to_value(sample_v2()).unwrap();
    let raw = envelope_for(3, &payload);
    match load_settings_v2(&raw) {
        PersistenceLoad::FutureVersion { schema, version } => {
            assert_eq!(schema, "settings");
            assert_eq!(version, 3);
        }
        other => panic!("expected FutureVersion, got {other:?}"),
    }
}

#[test]
fn wrong_schema_name_is_corrupt() {
    let payload = serde_json::to_value(sample_v2()).unwrap();
    let raw = serde_json::json!({
        "schema_name": "session",
        "schema_version": 2,
        "app_version": "0.165.1",
        "created_unix": 0,
        "updated_unix": 0,
        "payload": payload,
    })
    .to_string();
    match load_settings_v2(&raw) {
        PersistenceLoad::Corrupt { .. } => {}
        other => panic!("expected Corrupt for a schema-name mismatch, got {other:?}"),
    }
}

#[test]
fn malformed_json_is_corrupt() {
    match load_settings_v2("{not valid json") {
        PersistenceLoad::Corrupt { .. } => {}
        other => panic!("expected Corrupt for malformed JSON, got {other:?}"),
    }
}

#[test]
fn malformed_payload_under_valid_envelope_is_corrupt() {
    let raw = serde_json::json!({
        "schema_name": "settings",
        "schema_version": 2,
        "app_version": "0.165.1",
        "created_unix": 0,
        "updated_unix": 0,
        "payload": "not an object",
    })
    .to_string();
    match load_settings_v2(&raw) {
        PersistenceLoad::Corrupt { .. } => {}
        other => panic!("expected Corrupt for a malformed payload, got {other:?}"),
    }
}

// ── Normalization ─────────────────────────────────────────────────────────

#[test]
fn out_of_range_active_profile_normalizes_to_zero() {
    let mut invalid = sample_v2();
    invalid.active_profile = 99;
    let payload = serde_json::to_value(&invalid).unwrap();
    let raw = envelope_for(2, &payload);
    match load_settings_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value.active_profile, 0),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn empty_profile_list_restores_builtin_defaults() {
    let mut invalid = sample_v2();
    invalid.profiles = vec![];
    invalid.active_profile = 0;
    let payload = serde_json::to_value(&invalid).unwrap();
    let raw = envelope_for(2, &payload);
    match load_settings_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value.profiles.len(), 4),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn out_of_range_appearance_font_size_clamps() {
    let mut invalid = sample_v2();
    invalid.appearance_font_size = 200;
    let payload = serde_json::to_value(&invalid).unwrap();
    let raw = envelope_for(2, &payload);
    match load_settings_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value.appearance_font_size, 50),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn out_of_range_diff_font_size_clamps() {
    let mut too_large = sample_v2();
    too_large.diff_font_size = 9999;
    let raw = envelope_for(2, &serde_json::to_value(&too_large).unwrap());
    match load_settings_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value.diff_font_size, 50),
        other => panic!("expected Current, got {other:?}"),
    }

    let mut too_small = sample_v2();
    too_small.diff_font_size = 0;
    let raw = envelope_for(2, &serde_json::to_value(&too_small).unwrap());
    match load_settings_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value.diff_font_size, 6),
        other => panic!("expected Current, got {other:?}"),
    }
}
