//! RFC-076 patch 3: runtime adapter tests (implementation sequence step 4,
//! "Add runtime adapter tests before changing `App`").
//!
//! Exercises [`crate::persist::schema::settings::runtime::resolve_and_commit`]
//! and its session mirror end to end against real temp-path repositories:
//! fresh/current pass through unchanged, legacy files are migrated and
//! durably committed, future/corrupt files resolve to temporary defaults
//! with writes disabled and the original bytes untouched, and a commit that
//! fails for a persistent reason (review 038 C1/C2) is reported with its
//! cause and disables writes rather than silently retrying forever.

use std::fs;
use std::path::PathBuf;

use crate::persist::schema::PersistenceLoad;
use crate::persist::schema::session::runtime::{
    MigrationCommitOutcome as SessionMigrationCommitOutcome, SessionRuntimeOutcome,
    resolve_and_commit as resolve_session,
};
use crate::persist::schema::session::{PersistedSession, SessionRepository};
use crate::persist::schema::settings::runtime::{
    MigrationCommitOutcome as SettingsMigrationCommitOutcome, SettingsRuntimeOutcome,
    resolve_and_commit as resolve_settings,
};
use crate::persist::schema::settings::{PersistedSettings, SettingsRepository};

fn temp_path(tag: &str, file_name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fsk-persist-v2-runtime-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::create_dir_all(&dir);
    dir.join(file_name)
}

fn backup_path_for(path: &std::path::Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.pre-v2.bak",
        path.file_name().unwrap().to_string_lossy()
    ))
}

/// Strips write permission from `dir` so a *new* file cannot be created in
/// it — `atomic_replace`'s `tempfile_in(dir)` step, specifically — while a
/// read of an existing file inside it still succeeds (read+execute is kept).
/// Returns `false` (skip, don't assert) if the change had no effect, e.g.
/// running as root — the same verify-before-assert pattern
/// `dir_unreadable_tests.rs` established for handoff 006's permission tests.
#[cfg(unix)]
fn make_dir_readonly(dir: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let probe = dir.join(".fsk-writability-probe");
    let _ = fs::remove_file(&probe);
    fs::set_permissions(dir, fs::Permissions::from_mode(0o555)).unwrap();
    fs::write(&probe, b"x").is_err()
}

#[cfg(unix)]
fn restore_dir_writable(dir: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
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
    assert_eq!(resolved.value, PersistedSettings::default());
    assert!(!resolved.write_disabled);
    assert_eq!(resolved.outcome, SettingsRuntimeOutcome::Fresh);
}

#[test]
fn settings_resolve_current_passes_through_unchanged() {
    let path = temp_path("settings-current", "settings.json");
    let repo = SettingsRepository::new(path);
    let value = PersistedSettings {
        diff_font_size: 22,
        ..PersistedSettings::default()
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
    assert_eq!(resolved.value, PersistedSettings::default());
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
    assert_eq!(resolved.value, PersistedSettings::default());
    assert!(resolved.write_disabled);
    assert!(matches!(
        resolved.outcome,
        SettingsRuntimeOutcome::CorruptPreserved { .. }
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), "{not valid json");
    // Patch 6: raw_bytes must be populated for Corrupt, or an explicit
    // reset (SettingsRepository::reset_with_backup) has nothing to back up.
    assert_eq!(
        resolved.raw_bytes.as_deref(),
        Some(b"{not valid json".as_slice())
    );
}

#[cfg(unix)]
#[test]
fn settings_resolve_surfaces_a_failed_commit_and_disables_writes() {
    // F89/RFC-082 §D5: used to obstruct the sibling temp path
    // `atomic_replace` wrote to — no longer possible once that path became
    // a random `tempfile`-generated name (see the repository failure-window
    // test's comment for the full account). Obstructed at the directory
    // level instead: the backup this flow would create is pre-created here
    // so `ensure_pre_v2_backup` finds it already present and never needs to
    // write, then the directory is made read-only so `atomic_replace`'s
    // `tempfile_in` cannot create its temp file — a persistent failure
    // (review 038 C1), not the benign N1 conflict race.
    let path = temp_path("settings-migrate-failed", "settings.json");
    let legacy_raw = fixture("settings-v0.json");
    fs::write(&path, &legacy_raw).unwrap();
    fs::write(backup_path_for(&path), &legacy_raw).unwrap();
    let dir = path.parent().unwrap();

    if !make_dir_readonly(dir) {
        restore_dir_writable(dir);
        eprintln!(
            "skipping settings_resolve_surfaces_a_failed_commit_and_disables_writes: \
             the directory permission change had no effect (running as root?)"
        );
        return;
    }

    let repo = SettingsRepository::new(path.clone());
    let resolved = resolve_settings(&repo);
    restore_dir_writable(dir);

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
    assert_eq!(resolved.value, PersistedSession::default());
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

#[cfg(unix)]
#[test]
fn session_resolve_surfaces_a_failed_commit_and_disables_writes() {
    // Session mirror of the settings test above — see its comment for why
    // this is a directory-level obstruction rather than the pre-F89
    // predictable-temp-path one.
    let path = temp_path("session-migrate-failed", "session.json");
    let legacy_raw = fixture("session-v0.json");
    fs::write(&path, &legacy_raw).unwrap();
    fs::write(backup_path_for(&path), &legacy_raw).unwrap();
    let dir = path.parent().unwrap();

    if !make_dir_readonly(dir) {
        restore_dir_writable(dir);
        eprintln!(
            "skipping session_resolve_surfaces_a_failed_commit_and_disables_writes: \
             the directory permission change had no effect (running as root?)"
        );
        return;
    }

    let repo = SessionRepository::new(path.clone());
    let resolved = resolve_session(&repo);
    restore_dir_writable(dir);

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
    assert_eq!(resolved.value, PersistedSession::default());
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
    assert_eq!(resolved.value, PersistedSession::default());
    assert!(resolved.write_disabled);
    assert!(matches!(
        resolved.outcome,
        SessionRuntimeOutcome::CorruptPreserved { .. }
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), "{not valid json");
    assert_eq!(
        resolved.raw_bytes.as_deref(),
        Some(b"{not valid json".as_slice())
    );
}
