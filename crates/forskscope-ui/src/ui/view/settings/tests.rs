//! RFC-076 patch 4: proves `persist`/`load`'s Store-independent halves
//! actually go through `SettingsRepository` — not just that the
//! lower-level core parsers work (handoff §6) — and that `persist`'s merge
//! preserves fields the UI does not own.

use std::fs;
use std::path::PathBuf;

use forskscope_core::persist::schema::PersistenceLoad;
use forskscope_core::persist::schema::settings::runtime::{
    MigrationCommitOutcome, SettingsRuntimeOutcome, SettingsRuntimeResolution,
};
use forskscope_core::persist::schema::settings::{PersistedSettings, SettingsRepository};

use super::{build_save_payload, load_settings, persist_settings, recovery_modal, reset_settings};
use crate::state::{AppSettings, Modal};

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-ui-settings-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir.join("settings.json")
}

#[test]
fn build_save_payload_overlays_ui_fields_but_preserves_the_rest() {
    let base = PersistedSettings {
        density: forskscope_core::settings::display::Density::Spacious,
        show_line_numbers: false,
        ..PersistedSettings::default()
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
    let payload = PersistedSettings {
        context_lines: 11,
        ..PersistedSettings::default()
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
        PersistedSettings::default().diff_font_size
    );
}

#[test]
fn load_settings_round_trips_through_persist_settings() {
    let path = temp_path("round-trip");
    let repo = SettingsRepository::new(path);
    let payload = PersistedSettings {
        context_lines: 8,
        explorer_compact: true,
        ..PersistedSettings::default()
    };
    persist_settings(&payload, &repo);

    let (settings, _resolution) = load_settings(&repo);

    assert_eq!(settings.context_lines, 8);
    assert!(settings.explorer_compact);
}

// ── RFC-076 patch 6: reset_settings / recovery_modal ──────────────────────

fn resolution(outcome: SettingsRuntimeOutcome) -> SettingsRuntimeResolution {
    SettingsRuntimeResolution {
        value: PersistedSettings::default(),
        write_disabled: true,
        outcome,
        raw_bytes: None,
    }
}

#[test]
fn reset_settings_writes_backup_and_new_value_through_the_real_repository() {
    let path = temp_path("reset");
    fs::write(&path, "{not valid json").unwrap();
    let repo = SettingsRepository::new(path.clone());
    let value = PersistedSettings {
        context_lines: 5,
        ..PersistedSettings::default()
    };

    reset_settings(&value, b"{not valid json", &repo).expect("reset must succeed");

    let backup_path = path.with_file_name("settings.json.reset.bak");
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "{not valid json",
        "the corrupt original must be preserved under a distinct .reset.bak name"
    );
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value.context_lines, 5),
        other => panic!("expected Current after reset_settings, got {other:?}"),
    }
}

#[test]
fn reset_settings_rejects_stale_bytes_after_external_change() {
    let path = temp_path("reset-stale");
    fs::write(&path, "{not valid json").unwrap();
    let repo = SettingsRepository::new(path.clone());

    let result = reset_settings(&PersistedSettings::default(), b"different bytes", &repo);

    assert!(
        result.is_err(),
        "must refuse to reset over bytes that no longer match what's on disk"
    );
}

#[test]
fn recovery_modal_is_none_for_outcomes_without_a_dialog() {
    for outcome in [
        SettingsRuntimeOutcome::Fresh,
        SettingsRuntimeOutcome::Current,
        SettingsRuntimeOutcome::Migrated(MigrationCommitOutcome::Committed { backup_path: None }),
        SettingsRuntimeOutcome::Migrated(MigrationCommitOutcome::DeferredByConflict),
    ] {
        assert!(recovery_modal(&resolution(outcome)).is_none());
    }
}

#[test]
fn recovery_modal_is_some_for_outcomes_with_a_dialog() {
    let res = resolution(SettingsRuntimeOutcome::Incompatible {
        schema: "settings".into(),
        version: 99,
    });
    assert!(matches!(
        recovery_modal(&res),
        Some(Modal::SettingsRecovery(_))
    ));
}
