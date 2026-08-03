//! RFC-076 patch 4: proves `persist`/`load`'s Store-independent halves
//! actually go through `SettingsRepository` — not just that the
//! lower-level core parsers work (handoff §6) — and that `persist`'s merge
//! preserves fields the UI does not own.

use std::fs;
use std::path::PathBuf;

use forskscope_core::persist::v2::PersistenceLoad;
use forskscope_core::persist::v2::settings::{PersistedSettingsV2, SettingsRepository};

use super::{build_save_payload, load_settings, persist_settings};
use crate::state::AppSettings;

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-ui-settings-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir.join("settings.json")
}

#[test]
fn build_save_payload_overlays_ui_fields_but_preserves_the_rest() {
    let base = PersistedSettingsV2 {
        density: forskscope_core::settings::display::Density::Spacious,
        show_line_numbers: false,
        ..PersistedSettingsV2::default()
    };
    let mut edited = AppSettings::from_v2(&base);
    edited.context_lines = 9;

    let merged = build_save_payload(&edited, &base);

    assert_eq!(merged.context_lines, 9, "UI-editable field must apply");
    assert_eq!(
        merged.density,
        forskscope_core::settings::display::Density::Spacious,
        "non-UI-owned field must survive a save the UI has no control over"
    );
    assert!(
        !merged.show_line_numbers,
        "non-UI-owned field must survive a save the UI has no control over"
    );
}

#[test]
fn persist_settings_writes_through_the_real_repository() {
    let path = temp_path("persist");
    let repo = SettingsRepository::new(path);
    let payload = PersistedSettingsV2 {
        context_lines: 11,
        ..PersistedSettingsV2::default()
    };

    persist_settings(&payload, &repo);

    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value.context_lines, 11),
        other => panic!("expected Current after persist_settings, got {other:?}"),
    }
}

#[test]
fn load_settings_resolves_missing_file_as_fresh_defaults() {
    let path = temp_path("load-missing");
    let _ = fs::remove_file(&path);
    let repo = SettingsRepository::new(path);

    let (settings, resolution) = load_settings(&repo);

    assert!(!resolution.write_disabled);
    assert_eq!(
        settings.diff_font_size,
        PersistedSettingsV2::default().diff_font_size
    );
}

#[test]
fn load_settings_round_trips_through_persist_settings() {
    let path = temp_path("round-trip");
    let repo = SettingsRepository::new(path);
    let payload = PersistedSettingsV2 {
        context_lines: 8,
        explorer_compact: true,
        ..PersistedSettingsV2::default()
    };
    persist_settings(&payload, &repo);

    let (settings, _resolution) = load_settings(&repo);

    assert_eq!(settings.context_lines, 8);
    assert!(settings.explorer_compact);
}
