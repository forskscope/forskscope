//! RFC-076 patch 2: explicit-path repository safe-write tests.
//!
//! Covers the handoff's "Repository and safe-write tests" acceptance:
//! `Missing` on an absent file, save/load round-trip, migration-commit
//! backup semantics (created once, never overwritten by a retry), and that
//! an ordinary save leaves no stray temp file behind. Every path here is a
//! temporary explicit path — never the developer's real config directory.

use std::fs;
use std::path::PathBuf;

use crate::persist::v2::PersistenceLoad;
use crate::persist::v2::session::{PersistedSessionV2, SessionRepository};
use crate::persist::v2::settings::{PersistedSettingsV2, SettingsRepository};

fn temp_path(tag: &str, file_name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-persist-v2-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir.join(file_name)
}

// ── Settings ─────────────────────────────────────────────────────────────

#[test]
fn settings_missing_file_returns_defaults() {
    let path = temp_path("settings-missing", "settings.json");
    let _ = fs::remove_file(&path);
    let repo = SettingsRepository::new(path);
    match repo.load() {
        PersistenceLoad::Missing { defaults } => {
            assert_eq!(defaults, PersistedSettingsV2::default())
        }
        other => panic!("expected Missing, got {other:?}"),
    }
}

#[test]
fn settings_save_then_load_round_trips() {
    let path = temp_path("settings-roundtrip", "settings.json");
    let repo = SettingsRepository::new(path);
    let value = PersistedSettingsV2 {
        diff_font_size: 20,
        ignore_extensions: "o, tmp".into(),
        ..PersistedSettingsV2::default()
    };
    repo.save(&value).expect("save must succeed");
    match repo.load() {
        PersistenceLoad::Current { value: loaded } => assert_eq!(loaded, value),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn settings_save_leaves_no_stray_temp_file() {
    let path = temp_path("settings-notemp", "settings.json");
    let repo = SettingsRepository::new(path.clone());
    repo.save(&PersistedSettingsV2::default()).unwrap();
    let dir = path.parent().unwrap();
    let stray: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("fsk-tmp"))
        .collect();
    assert!(
        stray.is_empty(),
        "stray temp file(s) left behind: {stray:?}"
    );
}

#[test]
fn settings_resave_preserves_created_unix() {
    let path = temp_path("settings-created", "settings.json");
    let repo = SettingsRepository::new(path.clone());
    repo.save(&PersistedSettingsV2::default()).unwrap();
    let first_raw = fs::read_to_string(&path).unwrap();
    let first_created = created_unix_of(&first_raw);

    // A second save, with different content, should not reset created_unix.
    let changed = PersistedSettingsV2 {
        context_lines: 9,
        ..PersistedSettingsV2::default()
    };
    repo.save(&changed).unwrap();
    let second_raw = fs::read_to_string(&path).unwrap();
    assert_eq!(created_unix_of(&second_raw), first_created);
}

#[test]
fn settings_commit_migration_creates_backup_and_v2_file() {
    let path = temp_path("settings-migrate", "settings.json");
    let original_bytes = br#"{"theme":"dark","language":"en"}"#;
    fs::write(&path, original_bytes).unwrap();

    let repo = SettingsRepository::new(path.clone());
    let outcome = repo
        .commit_migration(&PersistedSettingsV2::default(), original_bytes)
        .expect("commit_migration must succeed");

    let backup_path = outcome.backup_path.expect("backup path must be reported");
    assert_eq!(fs::read(&backup_path).unwrap(), original_bytes);
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSettingsV2::default()),
        other => panic!("expected Current after migration commit, got {other:?}"),
    }
}

#[test]
fn settings_commit_migration_does_not_overwrite_existing_backup() {
    let path = temp_path("settings-migrate-retry", "settings.json");
    let first_original = b"first original bytes";
    fs::write(&path, first_original).unwrap();

    let repo = SettingsRepository::new(path.clone());
    let first = repo
        .commit_migration(&PersistedSettingsV2::default(), first_original)
        .unwrap();
    let backup_path = first.backup_path.unwrap();

    // Simulate a retried commit with different "original" bytes (e.g. a
    // second migration attempt on an already-migrated file, or a caller
    // bug) — the first backup must survive untouched.
    let second_original = b"different bytes from a retried attempt";
    let second = repo
        .commit_migration(&PersistedSettingsV2::default(), second_original)
        .unwrap();
    assert_eq!(second.backup_path.unwrap(), backup_path);
    assert_eq!(fs::read(&backup_path).unwrap(), first_original);
}

fn created_unix_of(raw: &str) -> u64 {
    let value: serde_json::Value = serde_json::from_str(raw).unwrap();
    value.get("created_unix").unwrap().as_u64().unwrap()
}

// ── Session ──────────────────────────────────────────────────────────────

#[test]
fn session_missing_file_returns_defaults() {
    let path = temp_path("session-missing", "session.json");
    let _ = fs::remove_file(&path);
    let repo = SessionRepository::new(path);
    match repo.load() {
        PersistenceLoad::Missing { defaults } => {
            assert_eq!(defaults, PersistedSessionV2::default())
        }
        other => panic!("expected Missing, got {other:?}"),
    }
}

#[test]
fn session_save_then_load_round_trips() {
    let path = temp_path("session-roundtrip", "session.json");
    let repo = SessionRepository::new(path);
    let value = PersistedSessionV2::default();
    repo.save(&value).expect("save must succeed");
    match repo.load() {
        PersistenceLoad::Current { value: loaded } => assert_eq!(loaded, value),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn session_commit_migration_creates_backup_and_v2_file() {
    let path = temp_path("session-migrate", "session.json");
    let original_bytes = br#"{"tabs":[["a","b"]]}"#;
    fs::write(&path, original_bytes).unwrap();

    let repo = SessionRepository::new(path.clone());
    let outcome = repo
        .commit_migration(&PersistedSessionV2::default(), original_bytes)
        .expect("commit_migration must succeed");

    let backup_path = outcome.backup_path.expect("backup path must be reported");
    assert_eq!(fs::read(&backup_path).unwrap(), original_bytes);
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSessionV2::default()),
        other => panic!("expected Current after migration commit, got {other:?}"),
    }
}
