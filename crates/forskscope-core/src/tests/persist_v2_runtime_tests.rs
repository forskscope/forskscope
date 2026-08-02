//! RFC-076 patch 3: runtime adapter tests (implementation sequence step 4,
//! "Add runtime adapter tests before changing `App`").
//!
//! Exercises [`crate::persist::v2::settings::runtime::resolve_and_commit`]
//! and its session mirror end to end against real temp-path repositories:
//! fresh/current pass through unchanged, legacy and core-v1 files are
//! migrated and durably committed, future/corrupt files resolve to
//! temporary defaults with writes disabled and the original bytes
//! untouched, and a commit that fails for a persistent reason (review 038
//! C1/C2) is reported with its cause and disables writes rather than
//! silently retrying forever.

use std::fs;
use std::path::PathBuf;

use crate::persist::v2::PersistenceLoad;
use crate::persist::v2::session::runtime::{
    MigrationCommitOutcome as SessionMigrationCommitOutcome, SessionRuntimeOutcome,
    resolve_and_commit as resolve_session,
};
use crate::persist::v2::session::{PersistedSessionV2, SessionRepository};
use crate::persist::v2::settings::runtime::{
    MigrationCommitOutcome as SettingsMigrationCommitOutcome, SettingsRuntimeOutcome,
    resolve_and_commit as resolve_settings,
};
use crate::persist::v2::settings::{PersistedSettingsV2, SettingsRepository};

fn temp_path(tag: &str, file_name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fsk-persist-v2-runtime-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::create_dir_all(&dir);
    dir.join(file_name)
}

fn temp_write_path_for(path: &std::path::Path) -> PathBuf {
    path.with_file_name(format!(
        ".{}.fsk-tmp",
        path.file_name().unwrap().to_string_lossy()
    ))
}

fn fixture(name: &str) -> String {
    fs::read_to_string(format!(
        "{}/src/tests/fixtures/persistence/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

// ── Settings ─────────────────────────────────────────────────────────────

#[test]
fn settings_resolve_missing_is_fresh_and_writable() {
    let path = temp_path("settings-fresh", "settings.json");
    let _ = fs::remove_file(&path);
    let repo = SettingsRepository::new(path);

    let resolved = resolve_settings(&repo);
    assert_eq!(resolved.value, PersistedSettingsV2::default());
    assert!(!resolved.write_disabled);
    assert_eq!(resolved.outcome, SettingsRuntimeOutcome::Fresh);
}

#[test]
fn settings_resolve_current_passes_through_unchanged() {
    let path = temp_path("settings-current", "settings.json");
    let repo = SettingsRepository::new(path);
    let value = PersistedSettingsV2 {
        diff_font_size: 22,
        ..PersistedSettingsV2::default()
    };
    repo.save(&value).unwrap();

    let resolved = resolve_settings(&repo);
    assert_eq!(resolved.value, value);
    assert!(!resolved.write_disabled);
    assert_eq!(resolved.outcome, SettingsRuntimeOutcome::Current);
}

#[test]
fn settings_resolve_migrates_legacy_v0_and_commits_durably() {
    let path = temp_path("settings-migrate-v0", "settings.json");
    let legacy_raw = fixture("settings-v0.json");
    fs::write(&path, &legacy_raw).unwrap();
    let repo = SettingsRepository::new(path.clone());

    let resolved = resolve_settings(&repo);
    match &resolved.outcome {
        SettingsRuntimeOutcome::Migrated(SettingsMigrationCommitOutcome::Committed {
            backup_path,
        }) => {
            let backup_path = backup_path.as_ref().expect("backup path must be reported");
            assert_eq!(fs::read_to_string(backup_path).unwrap(), legacy_raw);
        }
        other => panic!("expected Migrated(Committed), got {other:?}"),
    }
    assert!(!resolved.write_disabled);

    // The migration must be durably written: a fresh load sees v2 Current.
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, resolved.value),
        other => panic!("expected Current after durable commit, got {other:?}"),
    }
}

#[test]
fn settings_resolve_migrates_core_v1_envelope_and_commits_durably() {
    let path = temp_path("settings-migrate-v1", "settings.json");
    fs::write(&path, fixture("settings-v1-envelope.json")).unwrap();
    let repo = SettingsRepository::new(path.clone());

    let resolved = resolve_settings(&repo);
    assert!(matches!(
        resolved.outcome,
        SettingsRuntimeOutcome::Migrated(SettingsMigrationCommitOutcome::Committed { .. })
    ));
    match repo.load() {
        PersistenceLoad::Current { .. } => {}
        other => panic!("expected Current after durable commit, got {other:?}"),
    }
}

#[test]
fn settings_resolve_future_version_disables_writes_and_preserves_bytes() {
    let path = temp_path("settings-future", "settings.json");
    let raw = serde_json::json!({
        "schema_name": "settings",
        "schema_version": 99,
        "app_version": "0.165.1",
        "created_unix": 0,
        "updated_unix": 0,
        "payload": {},
    })
    .to_string();
    fs::write(&path, &raw).unwrap();
    let repo = SettingsRepository::new(path.clone());

    let resolved = resolve_settings(&repo);
    assert_eq!(resolved.value, PersistedSettingsV2::default());
    assert!(resolved.write_disabled);
    match resolved.outcome {
        SettingsRuntimeOutcome::Incompatible { schema, version } => {
            assert_eq!(schema, "settings");
            assert_eq!(version, 99);
        }
        other => panic!("expected Incompatible, got {other:?}"),
    }
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        raw,
        "bytes must be untouched"
    );
}

#[test]
fn settings_resolve_corrupt_disables_writes_and_preserves_bytes() {
    let path = temp_path("settings-corrupt", "settings.json");
    fs::write(&path, "{not valid json").unwrap();
    let repo = SettingsRepository::new(path.clone());

    let resolved = resolve_settings(&repo);
    assert_eq!(resolved.value, PersistedSettingsV2::default());
    assert!(resolved.write_disabled);
    assert!(matches!(
        resolved.outcome,
        SettingsRuntimeOutcome::CorruptPreserved { .. }
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), "{not valid json");
}

#[test]
fn settings_resolve_surfaces_a_failed_commit_and_disables_writes() {
    // Obstructs the sibling temp path `atomic_replace` writes to (same
    // technique as the repository failure-window test), so the commit's
    // final write fails after its backup already succeeded — a persistent
    // failure (review 038 C1), not the benign N1 conflict race.
    let path = temp_path("settings-migrate-failed", "settings.json");
    let legacy_raw = fixture("settings-v0.json");
    fs::write(&path, &legacy_raw).unwrap();
    fs::create_dir_all(temp_write_path_for(&path)).unwrap();
    let repo = SettingsRepository::new(path.clone());

    let resolved = resolve_settings(&repo);
    match resolved.outcome {
        SettingsRuntimeOutcome::Migrated(SettingsMigrationCommitOutcome::Failed { detail }) => {
            assert!(!detail.is_empty());
        }
        other => panic!("expected Migrated(Failed), got {other:?}"),
    }
    assert!(
        resolved.write_disabled,
        "a refused/failed migration commit must not leave the file writable (review 038 C2)"
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        legacy_raw,
        "the un-migrated file must be untouched by the failed commit"
    );
}

// ── Session ──────────────────────────────────────────────────────────────

#[test]
fn session_resolve_missing_is_fresh_and_writable() {
    let path = temp_path("session-fresh", "session.json");
    let _ = fs::remove_file(&path);
    let repo = SessionRepository::new(path);

    let resolved = resolve_session(&repo);
    assert_eq!(resolved.value, PersistedSessionV2::default());
    assert!(!resolved.write_disabled);
    assert_eq!(resolved.outcome, SessionRuntimeOutcome::Fresh);
}

#[test]
fn session_resolve_migrates_legacy_v0_and_commits_durably() {
    let path = temp_path("session-migrate-v0", "session.json");
    let legacy_raw = fixture("session-v0.json");
    fs::write(&path, &legacy_raw).unwrap();
    let repo = SessionRepository::new(path.clone());

    let resolved = resolve_session(&repo);
    match &resolved.outcome {
        SessionRuntimeOutcome::Migrated(SessionMigrationCommitOutcome::Committed {
            backup_path,
        }) => {
            let backup_path = backup_path.as_ref().expect("backup path must be reported");
            assert_eq!(fs::read_to_string(backup_path).unwrap(), legacy_raw);
        }
        other => panic!("expected Migrated(Committed), got {other:?}"),
    }
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, resolved.value),
        other => panic!("expected Current after durable commit, got {other:?}"),
    }
}

#[test]
fn session_resolve_surfaces_a_failed_commit_and_disables_writes() {
    let path = temp_path("session-migrate-failed", "session.json");
    let legacy_raw = fixture("session-v0.json");
    fs::write(&path, &legacy_raw).unwrap();
    fs::create_dir_all(temp_write_path_for(&path)).unwrap();
    let repo = SessionRepository::new(path.clone());

    let resolved = resolve_session(&repo);
    match resolved.outcome {
        SessionRuntimeOutcome::Migrated(SessionMigrationCommitOutcome::Failed { detail }) => {
            assert!(!detail.is_empty());
        }
        other => panic!("expected Migrated(Failed), got {other:?}"),
    }
    assert!(
        resolved.write_disabled,
        "a refused/failed migration commit must not leave the file writable (review 038 C2)"
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        legacy_raw,
        "the un-migrated file must be untouched by the failed commit"
    );
}

#[test]
fn session_resolve_future_version_disables_writes_and_preserves_bytes() {
    let path = temp_path("session-future", "session.json");
    let raw = serde_json::json!({
        "schema_name": "session",
        "schema_version": 99,
        "app_version": "0.165.1",
        "created_unix": 0,
        "updated_unix": 0,
        "payload": {},
    })
    .to_string();
    fs::write(&path, &raw).unwrap();
    let repo = SessionRepository::new(path.clone());

    let resolved = resolve_session(&repo);
    assert_eq!(resolved.value, PersistedSessionV2::default());
    assert!(resolved.write_disabled);
    assert!(matches!(
        resolved.outcome,
        SessionRuntimeOutcome::Incompatible { .. }
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), raw);
}

#[test]
fn session_resolve_corrupt_disables_writes_and_preserves_bytes() {
    let path = temp_path("session-corrupt", "session.json");
    fs::write(&path, "{not valid json").unwrap();
    let repo = SessionRepository::new(path.clone());

    let resolved = resolve_session(&repo);
    assert_eq!(resolved.value, PersistedSessionV2::default());
    assert!(resolved.write_disabled);
    assert!(matches!(
        resolved.outcome,
        SessionRuntimeOutcome::CorruptPreserved { .. }
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), "{not valid json");
}
