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
use forskscope_core::persist::schema::session::{PersistedComparePair, SessionRepository};

use super::{build_save_payload, load_session, persist_session, save_session_if_allowed};

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
