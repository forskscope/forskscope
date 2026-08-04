//! RFC-076 patch 4: proves `save_session`/`restore_session`'s Store-independent
//! halves actually go through `SessionRepository` — not just that the
//! lower-level core parsers work (handoff §6).
//!
//! Review 041 C1: also proves the CLI-mode regression directly —
//! `save_session_if_allowed` must leave a future-version file untouched
//! even when tabs are open, which is exactly what a CLI launch
//! (`forskscope left right`) does.

use std::fs;
use std::path::PathBuf;

use forskscope_core::persist::schema::PersistenceLoad;
use forskscope_core::persist::schema::session::runtime::{
    MigrationCommitOutcome, SessionRuntimeOutcome, SessionRuntimeResolution,
};
use forskscope_core::persist::schema::session::{
    PersistedComparePair, PersistedSession, SessionRepository,
};

use super::{
    build_save_payload, load_session, persist_session, recovery_modal, reset_session,
    save_session_if_allowed,
};
use crate::state::Modal;

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-ui-session-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir.join("session.json")
}

#[test]
fn build_save_payload_keeps_only_pairs_with_both_paths() {
    let pairs = vec![
        (
            Some(PathBuf::from("/old/a.rs")),
            Some(PathBuf::from("/new/a.rs")),
        ),
        (Some(PathBuf::from("/only/left")), None),
    ];

    let payload = build_save_payload(&pairs);

    assert_eq!(
        payload.tabs,
        vec![PersistedComparePair {
            left: "/old/a.rs".into(),
            right: "/new/a.rs".into(),
        }]
    );
    assert!(payload.active_tab.is_none());
    assert!(payload.explorer_roots.is_none());
}

#[test]
fn persist_session_writes_through_the_real_repository() {
    let path = temp_path("persist");
    let repo = SessionRepository::new(path);
    let pairs = vec![(
        Some(PathBuf::from("/old/a.rs")),
        Some(PathBuf::from("/new/a.rs")),
    )];

    persist_session(&build_save_payload(&pairs), &repo);

    match repo.load() {
        PersistenceLoad::Current { value } => {
            assert_eq!(value.tabs[0].left, PathBuf::from("/old/a.rs"));
            assert_eq!(value.tabs[0].right, PathBuf::from("/new/a.rs"));
        }
        other => panic!("expected Current after persist_session, got {other:?}"),
    }
}

#[test]
fn load_session_resolves_missing_file_as_fresh_empty_session() {
    let path = temp_path("load-missing");
    let _ = fs::remove_file(&path);
    let repo = SessionRepository::new(path);

    let resolution = load_session(&repo);

    assert!(resolution.value.tabs.is_empty());
    assert!(!resolution.write_disabled);
}

#[test]
fn load_session_round_trips_through_persist_session() {
    let path = temp_path("round-trip");
    let repo = SessionRepository::new(path);
    let pairs = vec![
        (
            Some(PathBuf::from("/old/a.rs")),
            Some(PathBuf::from("/new/a.rs")),
        ),
        (
            Some(PathBuf::from("/old/b.rs")),
            Some(PathBuf::from("/new/b.rs")),
        ),
    ];
    persist_session(&build_save_payload(&pairs), &repo);

    let resolution = load_session(&repo);

    assert_eq!(resolution.value.tabs.len(), 2);
    assert_eq!(resolution.value.tabs[0].left, PathBuf::from("/old/a.rs"));
    assert_eq!(resolution.value.tabs[1].right, PathBuf::from("/new/b.rs"));
}

#[test]
fn future_version_session_stays_byte_identical_through_a_disabled_save() {
    // Simulates the CLI-mode path review 041 C1 found unprotected: a
    // future-version session.json, then a save attempt with a tab open (as
    // `forskscope left right` would trigger via open_compare's tabs
    // use_effect) — the file must not be touched.
    let path = temp_path("future-version-cli");
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

    let resolution = load_session(&repo);
    assert!(
        resolution.write_disabled,
        "a future-version file must disable writes"
    );

    let pairs = vec![(Some(PathBuf::from("/left")), Some(PathBuf::from("/right")))];
    save_session_if_allowed(
        resolution.write_disabled,
        &build_save_payload(&pairs),
        &repo,
    );

    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        raw,
        "a write-disabled source must stay byte-identical even with tabs open"
    );
}

// ── RFC-076 patch 6: reset_session / recovery_modal ────────────────────────

fn resolution(outcome: SessionRuntimeOutcome) -> SessionRuntimeResolution {
    SessionRuntimeResolution {
        value: PersistedSession::default(),
        write_disabled: true,
        outcome,
        raw_bytes: None,
    }
}

#[test]
fn reset_session_writes_backup_and_new_value_through_the_real_repository() {
    let path = temp_path("reset");
    fs::write(&path, "{not valid json").unwrap();
    let repo = SessionRepository::new(path.clone());
    let value = PersistedSession {
        tabs: vec![PersistedComparePair {
            left: "/a".into(),
            right: "/b".into(),
        }],
        active_tab: None,
        explorer_roots: None,
    };

    reset_session(&value, b"{not valid json", &repo).expect("reset must succeed");

    let backup_path = path.with_file_name("session.json.reset.bak");
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "{not valid json",
        "the corrupt original must be preserved under a distinct .reset.bak name"
    );
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value.tabs.len(), 1),
        other => panic!("expected Current after reset_session, got {other:?}"),
    }
}

#[test]
fn reset_session_rejects_stale_bytes_after_external_change() {
    let path = temp_path("reset-stale");
    fs::write(&path, "{not valid json").unwrap();
    let repo = SessionRepository::new(path.clone());

    let result = reset_session(&PersistedSession::default(), b"different bytes", &repo);

    assert!(
        result.is_err(),
        "must refuse to reset over bytes that no longer match what's on disk"
    );
}

#[test]
fn recovery_modal_is_none_for_outcomes_without_a_dialog() {
    for outcome in [
        SessionRuntimeOutcome::Fresh,
        SessionRuntimeOutcome::Current,
        SessionRuntimeOutcome::Migrated(MigrationCommitOutcome::Committed { backup_path: None }),
        SessionRuntimeOutcome::Migrated(MigrationCommitOutcome::DeferredByConflict),
    ] {
        assert!(recovery_modal(&resolution(outcome)).is_none());
    }
}

#[test]
fn recovery_modal_is_some_for_outcomes_with_a_dialog() {
    let res = resolution(SessionRuntimeOutcome::Incompatible {
        schema: "session".into(),
        version: 99,
    });
    assert!(matches!(
        recovery_modal(&res),
        Some(Modal::SessionRecovery(_))
    ));
}
