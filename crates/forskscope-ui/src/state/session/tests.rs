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

use dioxus::prelude::*;
use forskscope_core::persist::schema::PersistenceLoad;
use forskscope_core::persist::schema::session::runtime::{
    MigrationCommitOutcome, SessionRuntimeOutcome, SessionRuntimeResolution,
};
use forskscope_core::persist::schema::session::{
    PersistedComparePair, PersistedSession, SessionRepository,
};

use super::{
    build_save_payload, load_session, persist_session, recovery_modal, request_close_file_tab,
    reset_session, restore_tabs, save_session_if_allowed, set_active_tab,
};
use crate::state::{ConfigRootOverrideGuard, Modal, with_test_store};

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-ui-session-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir.join("session.json")
}

// ── F67: restore_tabs selects the active tab ───────────────────────────────

/// A fresh scratch directory plus a config-root override set for its
/// lifetime, so `open_compare`'s own internal `save_session` call (every
/// `restore_tabs` call triggers one per tab, per the F61 account above)
/// has somewhere real to write rather than touching this machine's actual
/// config.
struct RestoreFixture {
    dir: PathBuf,
    _config_root: ConfigRootOverrideGuard,
}

fn restore_fixture(tag: &str) -> RestoreFixture {
    let dir = std::env::temp_dir().join(format!(
        "fsk-ui-session-restore-{tag}-{}",
        std::process::id()
    ));
    let config_home = dir.join("config");
    fs::create_dir_all(&config_home).unwrap();
    let _config_root = ConfigRootOverrideGuard::set(config_home);
    RestoreFixture { dir, _config_root }
}

impl RestoreFixture {
    fn pair(&self, name: &str) -> PersistedComparePair {
        let left = self.dir.join(format!("{name}-left.txt"));
        let right = self.dir.join(format!("{name}-right.txt"));
        fs::write(&left, format!("{name} left\n")).unwrap();
        fs::write(&right, format!("{name} right\n")).unwrap();
        PersistedComparePair { left, right }
    }
}

fn resolution_of(
    tabs: Vec<PersistedComparePair>,
    active_tab: Option<usize>,
) -> SessionRuntimeResolution {
    SessionRuntimeResolution {
        value: PersistedSession { tabs, active_tab },
        write_disabled: true,
        outcome: SessionRuntimeOutcome::Fresh,
        raw_bytes: None,
    }
}

#[test]
fn restore_tabs_selects_the_tab_that_was_active() {
    // The active pair is deliberately not the last one restored: `open_compare`
    // leaves `store.active` on whichever tab it opened most recently as its
    // own side effect, so a test using the last pair as "active" would pass
    // even with that side effect alone, restore_tabs's own fix unexercised.
    let fx = restore_fixture("active");
    let a = fx.pair("a");
    let b = fx.pair("b");
    let c = fx.pair("c");
    let resolution = resolution_of(vec![a, b, c], Some(0));

    with_test_store(|store| {
        restore_tabs(store, &resolution);
        assert_eq!(store.tabs.read().len(), 3);
        assert_eq!(
            *store.active.read(),
            Some(0),
            "the first pair (file index 0) was active and must stay selected, \
             not the last-opened tab"
        );
    });
}

/// Falsifies the fix: with `restore_tabs` back to never touching
/// `store.active`, `open_compare`'s own side effect leaves the *last*
/// opened tab selected, not the one the file says was active.
#[test]
fn restore_tabs_falls_back_to_the_first_tab_when_active_was_not_restored() {
    let fx = restore_fixture("fallback-none");
    let a = fx.pair("a");
    let b = fx.pair("b");
    // No active_tab recorded at all (e.g. an older/hand-edited session file).
    let resolution = resolution_of(vec![a, b], None);

    with_test_store(|store| {
        restore_tabs(store, &resolution);
        assert_eq!(store.tabs.read().len(), 2);
        assert_eq!(*store.active.read(), Some(0));
    });
}

#[test]
fn restore_tabs_falls_back_to_the_first_tab_when_the_active_pair_was_skipped() {
    let fx = restore_fixture("fallback-skipped");
    let a = fx.pair("a");
    let missing = PersistedComparePair {
        left: fx.dir.join("gone-left.txt"),
        right: fx.dir.join("gone-right.txt"),
    };
    let c = fx.pair("c");
    // The active pair (file index 1) has neither file on disk, so it is
    // skipped during restore — the first pair that *did* restore (file
    // index 0, store index 0) is the right fallback, not file index 1's
    // now-nonexistent slot.
    let resolution = resolution_of(vec![a, missing, c], Some(1));

    with_test_store(|store| {
        restore_tabs(store, &resolution);
        assert_eq!(
            store.tabs.read().len(),
            2,
            "only the two pairs with files on disk restore"
        );
        assert_eq!(*store.active.read(), Some(0));
    });
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

    let payload = build_save_payload(&pairs, None);

    assert_eq!(
        payload.tabs,
        vec![PersistedComparePair {
            left: "/old/a.rs".into(),
            right: "/new/a.rs".into(),
        }]
    );
    assert!(payload.active_tab.is_none());
}

/// F67: the active index is computed against the *saved* list, not copied
/// verbatim — a dropped incomplete pair before the active one must shift it.
#[test]
fn build_save_payload_reindexes_active_tab_past_a_dropped_pair() {
    let pairs = vec![
        (Some(PathBuf::from("/only/left")), None), // dropped: incomplete
        (
            Some(PathBuf::from("/old/a.rs")),
            Some(PathBuf::from("/new/a.rs")),
        ),
        (
            Some(PathBuf::from("/old/b.rs")),
            Some(PathBuf::from("/new/b.rs")),
        ),
    ];

    // Index 2 in `pairs` (the b.rs pair) is active; after the incomplete
    // pair at index 0 is dropped, it must land at index 1 in `saved`.
    let payload = build_save_payload(&pairs, Some(2));

    assert_eq!(payload.tabs.len(), 2);
    assert_eq!(payload.active_tab, Some(1));
}

#[test]
fn build_save_payload_active_tab_is_none_when_active_names_a_dropped_pair() {
    let pairs = vec![
        (Some(PathBuf::from("/only/left")), None), // dropped: incomplete
        (
            Some(PathBuf::from("/old/a.rs")),
            Some(PathBuf::from("/new/a.rs")),
        ),
    ];

    // The active tab (index 0) is exactly the one with no right side.
    let payload = build_save_payload(&pairs, Some(0));

    assert_eq!(payload.tabs.len(), 1);
    assert_eq!(
        payload.active_tab, None,
        "the active pair was dropped, so no index in `saved` names it"
    );
}

#[test]
fn persist_session_writes_through_the_real_repository() {
    let path = temp_path("persist");
    let repo = SessionRepository::new(path);
    let pairs = vec![(
        Some(PathBuf::from("/old/a.rs")),
        Some(PathBuf::from("/new/a.rs")),
    )];

    persist_session(&build_save_payload(&pairs, None), &repo)
        .expect("persist_session must succeed");

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
    persist_session(&build_save_payload(&pairs, None), &repo)
        .expect("persist_session must succeed");

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
    let result = save_session_if_allowed(
        resolution.write_disabled,
        &build_save_payload(&pairs, None),
        &repo,
    );

    assert_eq!(
        result,
        Ok(()),
        "a disabled write is a deliberate no-op, not a failure"
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

// ── F194 (RFC-087 §3a, handoff 077): save on every active-tab change ───────

/// "Open two tabs. Select the first. Show that `session.json`'s
/// `active_tab` is 0 without any open or close." Falsify by reverting
/// `set_active_tab` to a bare `store.active.set(index)` with no save: this
/// then fails, since opening the two tabs already wrote `active_tab: 1`
/// (the second, last-opened one, by `open_compare`'s own side effect) and
/// nothing updates the file after that.
#[test]
fn switching_the_active_tab_saves_the_session_without_any_open_or_close() {
    let fx = restore_fixture("f194-switch");
    let left1 = fx.dir.join("a-left.txt");
    let right1 = fx.dir.join("a-right.txt");
    let left2 = fx.dir.join("b-left.txt");
    let right2 = fx.dir.join("b-right.txt");
    fs::write(&left1, "a left\n").unwrap();
    fs::write(&right1, "a right\n").unwrap();
    fs::write(&left2, "b left\n").unwrap();
    fs::write(&right2, "b right\n").unwrap();

    with_test_store(|store| {
        crate::state::open_compare(store, left1, right1);
        crate::state::open_compare(store, left2, right2);
        assert_eq!(
            *store.active.read(),
            Some(1),
            "test premise: opening leaves the second (last-opened) tab active"
        );

        // The switch itself — no tab opens or closes here.
        set_active_tab(store, Some(0));

        let session_path = fx
            .dir
            .join("config")
            .join("forskscope")
            .join("session.json");
        let repo = SessionRepository::new(session_path);
        match repo.load() {
            PersistenceLoad::Current { value } => {
                assert_eq!(value.active_tab, Some(0));
            }
            other => panic!("expected Current, got {other:?}"),
        }
    });
}

/// `request_close_file_tab`: the one path the × button, `Ctrl+W` and
/// `Delete` on a focused tab (RFC-087 §1) all go through. Falsify by
/// removing the `dirty` branch (always closing at once): the dirty case
/// then fails, since the tab would already be gone instead of waiting on
/// `Modal::ConfirmClose`.
#[test]
fn request_close_file_tab_asks_first_only_when_dirty() {
    let fx = restore_fixture("f194-request-close");
    let left = fx.dir.join("left.txt");
    let right = fx.dir.join("right.txt");
    fs::write(&left, "a\n").unwrap();
    fs::write(&right, "b\n").unwrap();

    with_test_store(|store| {
        crate::state::open_compare(store, left, right);
        assert_eq!(store.tabs.read().len(), 1, "test premise");

        // Not dirty: closes at once, no modal.
        request_close_file_tab(store, 0);
        assert_eq!(store.tabs.read().len(), 0, "a clean tab must close at once");
        assert!(matches!(*store.modal.read(), Modal::None));
    });
}

#[test]
fn request_close_file_tab_asks_first_when_dirty() {
    let fx = restore_fixture("f194-request-close-dirty");
    let left = fx.dir.join("left.txt");
    let right = fx.dir.join("right.txt");
    fs::write(&left, "a\n").unwrap();
    fs::write(&right, "b\n").unwrap();

    with_test_store(|store| {
        // A real tab (id, generation, paths) from the real open path — but
        // its diff/merge are built directly here rather than waited on from
        // `open_compare`'s spawned load task, which `with_test_store`'s bare
        // synchronous call never drives to completion (confirmed: the
        // tab's `diff.hunks` is still empty immediately after this call).
        crate::state::open_compare(store, left, right);
        let diff_options = forskscope_core::DiffOptions::default();
        let diff = forskscope_core::compute_diff("a\n", "b\n", diff_options);
        let hunk_id = diff
            .hunks
            .first()
            .expect("test premise: the differing pair must produce at least one hunk")
            .hunk_id;
        let mut merge = forskscope_core::MergeSession::from_diff(&diff);
        merge
            .apply_left_to_right(hunk_id)
            .expect("test premise: applying the hunk must succeed");
        {
            let mut tabs = store.tabs.write();
            let tab = &mut tabs[0];
            // Still `Loading` otherwise, since the real load never ran —
            // and `request_close_file_tab`'s own `is_loading` fast path
            // (RFC-065) would then skip the dirty check entirely.
            tab.state = crate::state::TabState::Ready;
            tab.diff_options = diff_options;
            tab.diff = diff;
            tab.merge = merge;
            tab.can_save = true;
        }
        assert!(
            store.tabs.read()[0].can_save && store.tabs.read()[0].merge.is_dirty(),
            "test premise: the tab must actually be dirty and saveable"
        );

        request_close_file_tab(store, 0);
        assert_eq!(
            store.tabs.read().len(),
            1,
            "a dirty tab must not close until confirmed"
        );
        assert!(matches!(*store.modal.read(), Modal::ConfirmClose(0)));
    });
}
