//! RFC-076 patch 4: proves `save_session`/`restore_session`'s Store-independent
//! halves actually go through `SessionRepository` — not just that the
//! lower-level core parsers work (handoff §6).

use std::fs;
use std::path::PathBuf;

use forskscope_core::persist::v2::PersistenceLoad;
use forskscope_core::persist::v2::session::{PersistedComparePairV2, SessionRepository};

use super::{build_save_payload, load_session, persist_session};

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
        vec![PersistedComparePairV2 {
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
