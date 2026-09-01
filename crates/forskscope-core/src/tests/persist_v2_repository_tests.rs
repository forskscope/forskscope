//! RFC-076 patch 2/3: explicit-path repository safe-write tests.
//!
//! Covers the handoff's "Repository and safe-write tests" acceptance:
//! `Missing` on an absent file, save/load round-trip, migration-commit
//! backup semantics (created once, never overwritten by a retry), that an
//! ordinary save leaves no stray temp file behind, and (patch 3, review 037
//! N1) that `commit_migration` refuses to proceed when the file changed
//! since the caller's `load_with_raw`, plus a real failure-window case
//! proving the backup survives a write that fails after it succeeds. Every
//! path here is a temporary explicit path — never the developer's real
//! config directory.

use std::fs;
use std::path::PathBuf;

use crate::persist::schema::session::{PersistedSession, SessionRepository};
use crate::persist::schema::settings::{PersistedSettings, SettingsRepository};
use crate::persist::schema::{PersistenceCommitError, PersistenceLoad};

fn temp_path(tag: &str, file_name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-persist-v2-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir.join(file_name)
}

fn backup_path_for(path: &std::path::Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.pre-v2.bak",
        path.file_name().unwrap().to_string_lossy()
    ))
}

fn reset_backup_path_for(path: &std::path::Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.reset.bak",
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

// ── Settings ─────────────────────────────────────────────────────────────

#[test]
fn settings_missing_file_returns_defaults() {
    let path = temp_path("settings-missing", "settings.json");
    let _ = fs::remove_file(&path);
    let repo = SettingsRepository::new(path);
    match repo.load() {
        PersistenceLoad::Missing { defaults } => {
            assert_eq!(defaults, PersistedSettings::default())
        }
        other => panic!("expected Missing, got {other:?}"),
    }
}

#[test]
fn settings_save_then_load_round_trips() {
    let path = temp_path("settings-roundtrip", "settings.json");
    let repo = SettingsRepository::new(path);
    let value = PersistedSettings {
        diff_font_size: 20,
        ignore_extensions: "o, tmp".into(),
        ..PersistedSettings::default()
    };
    repo.save(&value).expect("save must succeed");
    match repo.load() {
        PersistenceLoad::Current { value: loaded } => assert_eq!(loaded, value),
        other => panic!("expected Current, got {other:?}"),
    }
}

/// F89/RFC-082 §D5 §3a: `atomic_replace` itself must not create parent
/// directories — that stays `atomic_write_envelope`'s job, deliberately
/// (F61/F62: the first write a fresh install makes, into
/// `~/.config/forskscope`, which nothing else creates beforehand). This is
/// the caller-level proof that the split still holds: a settings write into
/// a directory that does not exist yet — not even created by `temp_path`'s
/// usual `create_dir_all` — must still succeed, via the repository's own
/// parent-creation call.
#[test]
fn settings_save_into_a_missing_parent_directory_still_succeeds() {
    let dir = std::env::temp_dir().join(format!(
        "fsk-persist-v2-settings-fresh-install-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    assert!(
        !dir.exists(),
        "test setup: the directory must not exist yet"
    );

    let path = dir.join("settings.json");
    let repo = SettingsRepository::new(path.clone());
    repo.save(&PersistedSettings::default())
        .expect("save into a missing parent directory must still succeed");

    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSettings::default()),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn settings_save_leaves_no_stray_temp_file() {
    // F89/RFC-082 §D5: filtering by name for "fsk-tmp" would now be
    // vacuous — `atomic_replace`'s temp file has a random `tempfile`-
    // generated name (verified empirically: a real save's directory
    // listing mid-flight shows no "fsk-tmp" substring anywhere), so that
    // filter would always find nothing regardless of whether a stray file
    // is actually left behind. Assert against every entry that isn't the
    // target file instead — a check that doesn't depend on knowing the
    // implementation's temp-naming scheme.
    let path = temp_path("settings-notemp", "settings.json");
    let repo = SettingsRepository::new(path.clone());
    repo.save(&PersistedSettings::default()).unwrap();
    let dir = path.parent().unwrap();
    let target_name = path.file_name().unwrap().to_string_lossy().into_owned();
    let stray: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| *name != target_name)
        .collect();
    assert!(
        stray.is_empty(),
        "stray file(s) left behind besides the target: {stray:?}"
    );
}

#[test]
fn settings_resave_preserves_created_unix() {
    let path = temp_path("settings-created", "settings.json");
    let repo = SettingsRepository::new(path.clone());
    repo.save(&PersistedSettings::default()).unwrap();
    let first_raw = fs::read_to_string(&path).unwrap();
    let first_created = created_unix_of(&first_raw);

    // A second save, with different content, should not reset created_unix.
    let changed = PersistedSettings {
        context_lines: 9,
        ..PersistedSettings::default()
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
        .commit_migration(&PersistedSettings::default(), original_bytes)
        .expect("commit_migration must succeed");

    let backup_path = outcome.backup_path.expect("backup path must be reported");
    assert_eq!(fs::read(&backup_path).unwrap(), original_bytes);
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSettings::default()),
        other => panic!("expected Current after migration commit, got {other:?}"),
    }
}

#[test]
fn settings_commit_migration_does_not_overwrite_existing_backup() {
    // A backup already on disk (e.g. left by a prior attempt that crashed
    // after the backup but before the replace — see
    // `settings_commit_migration_survives_failure_between_backup_and_replace`
    // below) must survive a subsequent successful commit untouched.
    let path = temp_path("settings-migrate-retry", "settings.json");
    let original = b"current original bytes";
    fs::write(&path, original).unwrap();
    let backup_path = backup_path_for(&path);
    let stale_backup = b"a backup already on disk from a prior attempt";
    fs::write(&backup_path, stale_backup).unwrap();

    let repo = SettingsRepository::new(path.clone());
    let outcome = repo
        .commit_migration(&PersistedSettings::default(), original)
        .expect("commit_migration must succeed");

    assert_eq!(outcome.backup_path.as_deref(), Some(backup_path.as_path()));
    assert_eq!(fs::read(&backup_path).unwrap(), stale_backup);
}

#[test]
fn settings_commit_migration_rejects_stale_bytes_after_external_change() {
    let path = temp_path("settings-migrate-conflict", "settings.json");
    let original = br#"{"theme":"dark"}"#;
    fs::write(&path, original).unwrap();
    let repo = SettingsRepository::new(path.clone());

    // Something touches the file after the caller's `load_with_raw` produced
    // `original` but before `commit_migration` runs.
    let changed = b"changed after load, before commit";
    fs::write(&path, changed).unwrap();

    let err = repo
        .commit_migration(&PersistedSettings::default(), original)
        .expect_err("must reject stale original_bytes");
    assert_eq!(err, PersistenceCommitError::Conflict);
    assert_eq!(
        fs::read(&path).unwrap(),
        changed,
        "target must be untouched"
    );
    assert!(
        !backup_path_for(&path).exists(),
        "no backup should be created for a rejected commit"
    );
}

#[cfg(unix)]
#[test]
fn settings_commit_migration_survives_failure_between_backup_and_replace() {
    // Review 037 §4.4: exercise the actual failure window (backup succeeds,
    // then the write fails), without needing OS-level fault injection.
    //
    // F89/RFC-082 §D5: this used to obstruct the sibling temp path
    // `atomic_replace` wrote to (`temp_write_path_for`) with a directory —
    // that stopped working once `atomic_replace` moved to a random O_EXCL
    // name (a different name is simply tried next). Forced at the
    // *directory* level instead: the backup is pre-created so
    // `ensure_pre_v2_backup` finds it already there and never needs to
    // write (no directory permission required for that branch), then the
    // directory is made read-only so `atomic_replace`'s `tempfile_in`
    // cannot create its new temp file at all, regardless of its name.
    let path = temp_path("settings-migrate-failwindow", "settings.json");
    let original = br#"{"theme":"dark"}"#;
    fs::write(&path, original).unwrap();
    fs::write(backup_path_for(&path), original).unwrap();
    let dir = path.parent().unwrap();

    if !make_dir_readonly(dir) {
        restore_dir_writable(dir);
        eprintln!(
            "skipping settings_commit_migration_survives_failure_between_backup_and_replace: \
             the directory permission change had no effect (running as root?)"
        );
        return;
    }

    let repo = SettingsRepository::new(path.clone());
    let result = repo.commit_migration(&PersistedSettings::default(), original);
    restore_dir_writable(dir);

    assert!(
        result.is_err(),
        "the write must fail because the directory cannot accept a new file"
    );
    assert_eq!(
        fs::read(backup_path_for(&path)).unwrap(),
        original,
        "backup must survive the failed write"
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        original,
        "target must be untouched by the failed write"
    );
}

#[test]
fn settings_reset_with_backup_creates_a_distinct_backup_and_writes_the_value() {
    let path = temp_path("settings-reset", "settings.json");
    let corrupt = b"{not valid json";
    fs::write(&path, corrupt).unwrap();

    let repo = SettingsRepository::new(path.clone());
    let outcome = repo
        .reset_with_backup(&PersistedSettings::default(), corrupt)
        .expect("reset_with_backup must succeed");

    let backup_path = outcome.backup_path.expect("backup path must be reported");
    assert_eq!(backup_path, reset_backup_path_for(&path));
    assert_ne!(
        backup_path,
        backup_path_for(&path),
        "a reset backup must not reuse the .pre-v2.bak migration-backup name"
    );
    assert_eq!(fs::read(&backup_path).unwrap(), corrupt);
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSettings::default()),
        other => panic!("expected Current after reset, got {other:?}"),
    }
}

#[test]
fn settings_reset_with_backup_does_not_overwrite_existing_backup() {
    let path = temp_path("settings-reset-retry", "settings.json");
    let corrupt = b"{not valid json";
    fs::write(&path, corrupt).unwrap();
    let backup_path = reset_backup_path_for(&path);
    let stale_backup = b"a reset backup already on disk from a prior attempt";
    fs::write(&backup_path, stale_backup).unwrap();

    let repo = SettingsRepository::new(path.clone());
    let outcome = repo
        .reset_with_backup(&PersistedSettings::default(), corrupt)
        .expect("reset_with_backup must succeed");

    assert_eq!(outcome.backup_path.as_deref(), Some(backup_path.as_path()));
    assert_eq!(fs::read(&backup_path).unwrap(), stale_backup);
}

#[test]
fn settings_reset_with_backup_rejects_stale_bytes_after_external_change() {
    let path = temp_path("settings-reset-conflict", "settings.json");
    let corrupt = b"{not valid json";
    fs::write(&path, corrupt).unwrap();
    let repo = SettingsRepository::new(path.clone());

    let changed = b"changed after load, before reset";
    fs::write(&path, changed).unwrap();

    let err = repo
        .reset_with_backup(&PersistedSettings::default(), corrupt)
        .expect_err("must reject stale original_bytes");
    assert_eq!(err, PersistenceCommitError::Conflict);
    assert_eq!(
        fs::read(&path).unwrap(),
        changed,
        "target must be untouched"
    );
    assert!(
        !reset_backup_path_for(&path).exists(),
        "no backup should be created for a rejected reset"
    );
}

#[test]
fn settings_load_with_raw_pairs_bytes_with_the_loaded_value() {
    let path = temp_path("settings-load-raw", "settings.json");
    let repo = SettingsRepository::new(path.clone());
    repo.save(&PersistedSettings::default()).unwrap();

    let (load, raw) = repo.load_with_raw();
    match load {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSettings::default()),
        other => panic!("expected Current, got {other:?}"),
    }
    assert_eq!(raw.unwrap(), fs::read(&path).unwrap());
}

#[test]
fn settings_load_with_raw_returns_no_bytes_when_missing() {
    let path = temp_path("settings-load-raw-missing", "settings.json");
    let _ = fs::remove_file(&path);
    let repo = SettingsRepository::new(path);
    let (_, raw) = repo.load_with_raw();
    assert_eq!(raw, None);
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
            assert_eq!(defaults, PersistedSession::default())
        }
        other => panic!("expected Missing, got {other:?}"),
    }
}

#[test]
fn session_save_then_load_round_trips() {
    let path = temp_path("session-roundtrip", "session.json");
    let repo = SessionRepository::new(path);
    let value = PersistedSession::default();
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
        .commit_migration(&PersistedSession::default(), original_bytes)
        .expect("commit_migration must succeed");

    let backup_path = outcome.backup_path.expect("backup path must be reported");
    assert_eq!(fs::read(&backup_path).unwrap(), original_bytes);
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSession::default()),
        other => panic!("expected Current after migration commit, got {other:?}"),
    }
}

#[test]
fn session_commit_migration_does_not_overwrite_existing_backup() {
    let path = temp_path("session-migrate-retry", "session.json");
    let original = br#"{"tabs":[]}"#;
    fs::write(&path, original).unwrap();
    let backup_path = backup_path_for(&path);
    let stale_backup = b"a backup already on disk from a prior attempt";
    fs::write(&backup_path, stale_backup).unwrap();

    let repo = SessionRepository::new(path.clone());
    let outcome = repo
        .commit_migration(&PersistedSession::default(), original)
        .expect("commit_migration must succeed");

    assert_eq!(outcome.backup_path.as_deref(), Some(backup_path.as_path()));
    assert_eq!(fs::read(&backup_path).unwrap(), stale_backup);
}

#[test]
fn session_commit_migration_rejects_stale_bytes_after_external_change() {
    let path = temp_path("session-migrate-conflict", "session.json");
    let original = br#"{"tabs":[]}"#;
    fs::write(&path, original).unwrap();
    let repo = SessionRepository::new(path.clone());

    let changed = b"changed after load, before commit";
    fs::write(&path, changed).unwrap();

    let err = repo
        .commit_migration(&PersistedSession::default(), original)
        .expect_err("must reject stale original_bytes");
    assert_eq!(err, PersistenceCommitError::Conflict);
    assert_eq!(
        fs::read(&path).unwrap(),
        changed,
        "target must be untouched"
    );
    assert!(
        !backup_path_for(&path).exists(),
        "no backup should be created for a rejected commit"
    );
}

#[cfg(unix)]
#[test]
fn session_commit_migration_survives_failure_between_backup_and_replace() {
    // Session mirror of the settings test above — see its comment for why
    // this is a directory-level obstruction rather than the pre-F89
    // predictable-temp-path one.
    let path = temp_path("session-migrate-failwindow", "session.json");
    let original = br#"{"tabs":[]}"#;
    fs::write(&path, original).unwrap();
    fs::write(backup_path_for(&path), original).unwrap();
    let dir = path.parent().unwrap();

    if !make_dir_readonly(dir) {
        restore_dir_writable(dir);
        eprintln!(
            "skipping session_commit_migration_survives_failure_between_backup_and_replace: \
             the directory permission change had no effect (running as root?)"
        );
        return;
    }

    let repo = SessionRepository::new(path.clone());
    let result = repo.commit_migration(&PersistedSession::default(), original);
    restore_dir_writable(dir);

    assert!(
        result.is_err(),
        "the write must fail because the directory cannot accept a new file"
    );
    assert_eq!(
        fs::read(backup_path_for(&path)).unwrap(),
        original,
        "backup must survive the failed write"
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        original,
        "target must be untouched by the failed write"
    );
}

#[test]
fn session_reset_with_backup_creates_a_distinct_backup_and_writes_the_value() {
    let path = temp_path("session-reset", "session.json");
    let corrupt = b"{not valid json";
    fs::write(&path, corrupt).unwrap();

    let repo = SessionRepository::new(path.clone());
    let outcome = repo
        .reset_with_backup(&PersistedSession::default(), corrupt)
        .expect("reset_with_backup must succeed");

    let backup_path = outcome.backup_path.expect("backup path must be reported");
    assert_eq!(backup_path, reset_backup_path_for(&path));
    assert_ne!(
        backup_path,
        backup_path_for(&path),
        "a reset backup must not reuse the .pre-v2.bak migration-backup name"
    );
    assert_eq!(fs::read(&backup_path).unwrap(), corrupt);
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSession::default()),
        other => panic!("expected Current after reset, got {other:?}"),
    }
}

#[test]
fn session_reset_with_backup_does_not_overwrite_existing_backup() {
    let path = temp_path("session-reset-retry", "session.json");
    let corrupt = b"{not valid json";
    fs::write(&path, corrupt).unwrap();
    let backup_path = reset_backup_path_for(&path);
    let stale_backup = b"a reset backup already on disk from a prior attempt";
    fs::write(&backup_path, stale_backup).unwrap();

    let repo = SessionRepository::new(path.clone());
    let outcome = repo
        .reset_with_backup(&PersistedSession::default(), corrupt)
        .expect("reset_with_backup must succeed");

    assert_eq!(outcome.backup_path.as_deref(), Some(backup_path.as_path()));
    assert_eq!(fs::read(&backup_path).unwrap(), stale_backup);
}

#[test]
fn session_reset_with_backup_rejects_stale_bytes_after_external_change() {
    let path = temp_path("session-reset-conflict", "session.json");
    let corrupt = b"{not valid json";
    fs::write(&path, corrupt).unwrap();
    let repo = SessionRepository::new(path.clone());

    let changed = b"changed after load, before reset";
    fs::write(&path, changed).unwrap();

    let err = repo
        .reset_with_backup(&PersistedSession::default(), corrupt)
        .expect_err("must reject stale original_bytes");
    assert_eq!(err, PersistenceCommitError::Conflict);
    assert_eq!(
        fs::read(&path).unwrap(),
        changed,
        "target must be untouched"
    );
    assert!(
        !reset_backup_path_for(&path).exists(),
        "no backup should be created for a rejected reset"
    );
}

#[test]
fn session_load_with_raw_pairs_bytes_with_the_loaded_value() {
    let path = temp_path("session-load-raw", "session.json");
    let repo = SessionRepository::new(path.clone());
    repo.save(&PersistedSession::default()).unwrap();

    let (load, raw) = repo.load_with_raw();
    match load {
        PersistenceLoad::Current { value } => assert_eq!(value, PersistedSession::default()),
        other => panic!("expected Current, got {other:?}"),
    }
    assert_eq!(raw.unwrap(), fs::read(&path).unwrap());
}

#[test]
fn session_load_with_raw_returns_no_bytes_when_missing() {
    let path = temp_path("session-load-raw-missing", "session.json");
    let _ = fs::remove_file(&path);
    let repo = SessionRepository::new(path);
    let (_, raw) = repo.load_with_raw();
    assert_eq!(raw, None);
}
