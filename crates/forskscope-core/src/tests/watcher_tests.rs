//! FileChangeMonitor trait and MockFileChangeMonitor tests (RFC-036
//! §"Watcher Boundary", §"Platform Considerations").

use std::path::PathBuf;

use crate::watcher::{
    FileChangeEvent, FileChangeKind, FileChangeMonitor, MockFileChangeMonitor,
    WatchError, WatchToken,
};

fn path(s: &str) -> PathBuf { PathBuf::from(s) }

// ── MockFileChangeMonitor basic operation ─────────────────────────────────────

#[test]
fn new_mock_is_active() {
    let m = MockFileChangeMonitor::new();
    assert!(m.is_active());
}

#[test]
fn watch_returns_distinct_tokens_for_each_call() {
    let mut m = MockFileChangeMonitor::new();
    let t1 = m.watch(&path("/a.rs")).unwrap();
    let t2 = m.watch(&path("/b.rs")).unwrap();
    assert_ne!(t1, t2, "each watch call must return a distinct token");
}

#[test]
fn poll_events_returns_empty_when_no_events_injected() {
    let mut m = MockFileChangeMonitor::new();
    m.watch(&path("/a.rs")).unwrap();
    assert!(m.poll_events().is_empty());
}

#[test]
fn inject_event_appears_in_poll_events() {
    let mut m = MockFileChangeMonitor::new();
    let token = m.watch(&path("/src/main.rs")).unwrap();
    m.inject_event(token, path("/src/main.rs"), FileChangeKind::Modified);
    let events = m.poll_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, FileChangeKind::Modified);
    assert_eq!(events[0].token, token);
    assert_eq!(events[0].path, path("/src/main.rs"));
}

#[test]
fn poll_events_drains_the_queue() {
    let mut m = MockFileChangeMonitor::new();
    let token = m.watch(&path("/f.rs")).unwrap();
    m.inject_event(token, path("/f.rs"), FileChangeKind::Modified);
    let _ = m.poll_events(); // drain
    assert!(m.poll_events().is_empty(), "second poll must return empty after drain");
}

#[test]
fn multiple_injected_events_all_returned_in_one_poll() {
    let mut m = MockFileChangeMonitor::new();
    let t1 = m.watch(&path("/a.rs")).unwrap();
    let t2 = m.watch(&path("/b.rs")).unwrap();
    m.inject_event(t1, path("/a.rs"), FileChangeKind::Modified);
    m.inject_event(t2, path("/b.rs"), FileChangeKind::Deleted);
    let events = m.poll_events();
    assert_eq!(events.len(), 2);
}

#[test]
fn unwatch_removes_path_from_watched_list() {
    let mut m = MockFileChangeMonitor::new();
    let token = m.watch(&path("/a.rs")).unwrap();
    assert_eq!(m.watched_paths().len(), 1);
    m.unwatch(token);
    assert!(m.watched_paths().is_empty());
}

#[test]
fn unwatch_unknown_token_is_noop() {
    let mut m = MockFileChangeMonitor::new();
    let _ = m.watch(&path("/a.rs")).unwrap();
    m.unwatch(WatchToken(999)); // unknown token — must not panic
    assert_eq!(m.watched_paths().len(), 1, "valid watch must not be removed");
}

#[test]
fn watch_on_inactive_monitor_returns_error() {
    let mut m = MockFileChangeMonitor::new();
    m.set_active(false);
    let result = m.watch(&path("/a.rs"));
    assert!(matches!(result, Err(WatchError::BackendUnavailable(_))));
    assert!(!m.is_active());
}

// ── FileChangeEvent ───────────────────────────────────────────────────────────

#[test]
fn file_change_event_stores_all_fields() {
    let token = WatchToken(42);
    let p     = path("/src/lib.rs");
    let event = FileChangeEvent::new(token, p.clone(), FileChangeKind::Renamed);
    assert_eq!(event.token, token);
    assert_eq!(event.path,  p);
    assert_eq!(event.kind,  FileChangeKind::Renamed);
}

// ── FileChangeKind exhaustiveness ─────────────────────────────────────────────

#[test]
fn all_file_change_kinds_are_distinct() {
    let kinds = [
        FileChangeKind::Modified,
        FileChangeKind::Deleted,
        FileChangeKind::Created,
        FileChangeKind::Renamed,
        FileChangeKind::Unknown,
    ];
    let unique: std::collections::HashSet<String> =
        kinds.iter().map(|k| format!("{k:?}")).collect();
    assert_eq!(unique.len(), kinds.len());
}

// ── WatchError display ────────────────────────────────────────────────────────

#[test]
fn watch_error_display_is_non_empty_for_all_variants() {
    let errors = vec![
        WatchError::PathNotFound(path("/missing")),
        WatchError::BackendUnavailable("inotify limit".into()),
        WatchError::AlreadyWatched(path("/watched")),
        WatchError::Other("unexpected".into()),
    ];
    for e in &errors {
        assert!(!e.to_string().is_empty(), "{e:?} must have non-empty display");
    }
}

// ── RFC-036 safety constraint ─────────────────────────────────────────────────

#[test]
fn watcher_events_are_advisory_not_authoritative() {
    // This test documents the RFC-036 safety rule: the watcher is an
    // optimization layer only. Receiving a Modified event does not mean
    // the file has *actually* changed — it is a hint to call
    // `check_external_state`. We verify that the mock delivers events
    // that consumers can *choose* to act on or ignore.
    let mut m = MockFileChangeMonitor::new();
    let token = m.watch(&path("/data/file.txt")).unwrap();
    // Inject an event but do not call check_external_state here —
    // the event is advisory and the consumer decides what to do with it.
    m.inject_event(token, path("/data/file.txt"), FileChangeKind::Modified);
    let events = m.poll_events();
    assert_eq!(events.len(), 1);
    // Consumer must call check_external_state before taking any
    // destructive action. This test verifies the event is readable
    // but does not take any action itself.
    let _ = events[0].kind; // just read it
}
