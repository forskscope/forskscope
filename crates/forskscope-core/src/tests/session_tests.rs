//! Workspace session model tests (RFC-011 §13 testing requirements,
//! §14 acceptance criteria).
//!
//! Every test requirement from RFC-011 §13 is covered by at least one test.
//! Requirements are called out inline as `// RFC-011 §13: <requirement>`.

use std::path::PathBuf;

use crate::persist::MigrationPolicy;
use crate::session::{
    BinaryTabSession, CloseResult, DiffTabSession, ErrorTabSession,
    ExcelTabSession, RecentKind, RecentSessionEntry, SessionId, TabId,
    Timestamp, WorkspaceRoot, WorkspaceSession, WorkspaceTab,
    SESSION_SCHEMA_VERSION,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn left()  -> PathBuf { PathBuf::from("/left/file.rs") }
fn right() -> PathBuf { PathBuf::from("/right/file.rs") }
fn ldir()  -> PathBuf { PathBuf::from("/left/project") }
fn rdir()  -> PathBuf { PathBuf::from("/right/project") }

fn diff_tab(left: &str, right: &str) -> WorkspaceTab {
    WorkspaceTab::Diff(DiffTabSession {
        tab_id:     TabId::new(),
        left_path:  PathBuf::from(left),
        right_path: PathBuf::from(right),
        is_dirty:   false,
    })
}

// ── RFC-011 §13: Create an empty session ─────────────────────────────────────

#[test]
fn empty_session_has_no_tabs_and_empty_root() {
    let s = WorkspaceSession::empty();
    assert!(matches!(s.root, WorkspaceRoot::Empty));
    assert!(s.tabs.is_empty());
    assert!(s.active_tab_id.is_none());
    assert!(!s.any_dirty());
}

// ── RFC-011 §13: Create a file-pair session from startup args ─────────────────

#[test]
fn from_file_pair_opens_one_diff_tab_as_active() {
    let s = WorkspaceSession::from_file_pair(left(), right());
    assert!(matches!(s.root, WorkspaceRoot::FilePair(_)));
    assert_eq!(s.tabs.len(), 1);
    assert!(s.active_tab_id.is_some());
    assert_eq!(s.active_tab_id, Some(s.tabs[0].tab_id().clone()));
    // Tab carries the correct paths.
    if let WorkspaceTab::Diff(t) = &s.tabs[0] {
        assert_eq!(t.left_path,  left());
        assert_eq!(t.right_path, right());
    } else {
        panic!("expected Diff tab");
    }
}

// ── RFC-011 §13: Create a directory-pair session from startup args ────────────

#[test]
fn from_directory_pair_has_no_tabs_but_correct_root() {
    let s = WorkspaceSession::from_directory_pair(ldir(), rdir());
    assert!(matches!(s.root, WorkspaceRoot::DirectoryPair(_)));
    assert!(s.tabs.is_empty());
    assert!(s.active_tab_id.is_none());
}

// ── RFC-011 §13: Open multiple diff tabs ──────────────────────────────────────

#[test]
fn open_multiple_tabs_tracks_active_and_count() {
    let mut s = WorkspaceSession::empty();
    s.open_tab(diff_tab("/a.rs", "/b.rs"));
    s.open_tab(diff_tab("/c.rs", "/d.rs"));
    assert_eq!(s.tabs.len(), 2);
    // Most recently opened tab is active.
    assert_eq!(s.active_tab_id, Some(s.tabs[1].tab_id().clone()));
}

// ── RFC-011 §13: Close clean tab ──────────────────────────────────────────────

#[test]
fn close_clean_tab_removes_it_and_returns_closed() {
    let mut s = WorkspaceSession::empty();
    s.open_tab(diff_tab("/a.rs", "/b.rs"));
    s.open_tab(diff_tab("/c.rs", "/d.rs"));
    let id = s.tabs[0].tab_id().clone();
    let result = s.close_tab(&id);
    assert_eq!(result, CloseResult::Closed);
    assert_eq!(s.tabs.len(), 1);
    assert!(s.tabs.iter().all(|t| t.tab_id() != &id));
}

#[test]
fn close_nonexistent_tab_returns_not_found() {
    let mut s = WorkspaceSession::empty();
    let result = s.close_tab(&TabId("does-not-exist".into()));
    assert_eq!(result, CloseResult::NotFound);
}

// ── RFC-011 §13: Attempt to close dirty tab and cancel ───────────────────────

#[test]
fn close_dirty_tab_returns_blocked_dirty() {
    let mut s = WorkspaceSession::empty();
    s.open_tab(diff_tab("/a.rs", "/b.rs"));
    let id = s.tabs[0].tab_id().clone();
    s.mark_tab_dirty(&id);
    let result = s.close_tab(&id);
    // RFC-011 §5.4: must be blocked; UI shows the unsaved-changes dialog.
    assert_eq!(result, CloseResult::BlockedDirty);
    assert_eq!(s.tabs.len(), 1, "dirty tab must not be removed");
}

// ── RFC-011 §13: Save dirty tab and close ────────────────────────────────────

#[test]
fn mark_clean_then_close_succeeds() {
    let mut s = WorkspaceSession::empty();
    s.open_tab(diff_tab("/a.rs", "/b.rs"));
    let id = s.tabs[0].tab_id().clone();
    s.mark_tab_dirty(&id);
    assert!(s.any_dirty());
    s.mark_tab_clean(&id);
    assert!(!s.any_dirty());
    let result = s.close_tab(&id);
    assert_eq!(result, CloseResult::Closed);
}

// ── RFC-011 §13: Recent session with existing paths ───────────────────────────

#[test]
fn recent_session_entry_paths_available_for_existing_paths() {
    // Use /tmp which is guaranteed to exist on any platform.
    let entry = RecentSessionEntry {
        session_id:     SessionId("s1".into()),
        title:          "test".into(),
        left_path:      PathBuf::from("/tmp"),
        right_path:     PathBuf::from("/tmp"),
        kind:           RecentKind::DirectoryPair,
        last_opened_at: Timestamp::now(),
    };
    assert!(entry.paths_available(),
        "existing paths must be reported as available");
}

// ── RFC-011 §13: Recent session with missing paths ────────────────────────────

#[test]
fn recent_session_entry_paths_not_available_for_missing_paths() {
    let entry = RecentSessionEntry {
        session_id:     SessionId("s2".into()),
        title:          "gone".into(),
        left_path:      PathBuf::from("/definitely/does/not/exist/left"),
        right_path:     PathBuf::from("/definitely/does/not/exist/right"),
        kind:           RecentKind::FilePair,
        last_opened_at: Timestamp::now(),
    };
    assert!(!entry.paths_available(),
        "missing paths must be reported as unavailable");
}

// ── RFC-011 §13: Validate schema-version compatibility ───────────────────────

#[test]
fn json_round_trip_restores_session_fields() {
    let s = WorkspaceSession::from_file_pair(left(), right());
    let json = s.to_json();
    let parsed = WorkspaceSession::from_json(&json).unwrap();
    assert_eq!(parsed.migration, MigrationPolicy::CompatibleRead);
    assert_eq!(parsed.session.session_id.0, s.session_id.0);
    assert!(matches!(parsed.session.root, WorkspaceRoot::FilePair(_)));
}

#[test]
fn json_round_trip_preserves_directory_pair_root() {
    let s = WorkspaceSession::from_directory_pair(ldir(), rdir());
    let json = s.to_json();
    let parsed = WorkspaceSession::from_json(&json).unwrap();
    if let WorkspaceRoot::DirectoryPair(p) = &parsed.session.root {
        assert_eq!(p.left,  ldir());
        assert_eq!(p.right, rdir());
    } else {
        panic!("expected DirectoryPair root");
    }
}

#[test]
fn json_round_trip_preserves_empty_session() {
    let s = WorkspaceSession::empty();
    let json = s.to_json();
    let parsed = WorkspaceSession::from_json(&json).unwrap();
    assert!(matches!(parsed.session.root, WorkspaceRoot::Empty));
}

#[test]
fn newer_schema_returns_too_new_error() {
    let s = WorkspaceSession::empty();
    let json = s.to_json();
    // Simulate a file from a future app version by bumping the schema version
    // in the JSON text.
    let future_json = json.replace(
        &format!("\"schema_version\": {SESSION_SCHEMA_VERSION}"),
        &format!("\"schema_version\": {}", SESSION_SCHEMA_VERSION + 10),
    );
    let result = WorkspaceSession::from_json(&future_json);
    assert!(matches!(result,
        Err(crate::session::SessionParseError::TooNew { .. })),
        "file from newer ForskScope must return TooNew error");
}

// ── RFC-011 §14: Acceptance criteria ─────────────────────────────────────────

#[test]
fn session_identity_stable_across_open_close() {
    // RFC-011 §14: "Session identity is not lost during UI redraw."
    let mut s = WorkspaceSession::from_file_pair(left(), right());
    let orig_id = s.session_id.0.clone();
    s.open_tab(diff_tab("/c.rs", "/d.rs"));
    let id = s.tabs[1].tab_id().clone();
    s.close_tab(&id);
    assert_eq!(s.session_id.0, orig_id, "session_id must not change on tab operations");
}

#[test]
fn dirty_tabs_visible_and_any_dirty_correct() {
    // RFC-011 §14: "Users can see what is open and what is dirty."
    let mut s = WorkspaceSession::empty();
    s.open_tab(diff_tab("/a.rs", "/b.rs"));
    s.open_tab(diff_tab("/c.rs", "/d.rs"));
    let id0 = s.tabs[0].tab_id().clone();
    s.mark_tab_dirty(&id0);
    assert!(s.any_dirty());
    assert_eq!(s.dirty_tabs().len(), 1);
    assert!(!s.tabs[1].is_dirty());
}

#[test]
fn recent_entry_does_not_store_file_contents() {
    // RFC-011 §14: "Recent sessions do not store file contents."
    // RecentSessionEntry has no content field — verified by type inspection.
    let entry = RecentSessionEntry {
        session_id:     SessionId::new(),
        title:          "test".into(),
        left_path:      left(),
        right_path:     right(),
        kind:           RecentKind::FilePair,
        last_opened_at: Timestamp::now(),
    };
    // If this compiles and the struct has no `content` or `bytes` field,
    // the requirement is met structurally.
    let _ = entry;
}

// ── WorkspaceTab variants ─────────────────────────────────────────────────────

#[test]
fn binary_and_excel_tabs_are_never_dirty() {
    let binary = WorkspaceTab::Binary(BinaryTabSession {
        tab_id: TabId::new(), left_path: left(), right_path: right(),
    });
    let excel = WorkspaceTab::Excel(ExcelTabSession {
        tab_id: TabId::new(), left_path: left(), right_path: right(),
    });
    assert!(!binary.is_dirty());
    assert!(!excel.is_dirty());
}

#[test]
fn error_tab_has_message_and_is_never_dirty() {
    let error = WorkspaceTab::Error(ErrorTabSession {
        tab_id:  TabId::new(),
        message: "load failed".into(),
    });
    assert!(!error.is_dirty());
}

#[test]
fn force_close_removes_dirty_tab_without_check() {
    let mut s = WorkspaceSession::empty();
    s.open_tab(diff_tab("/a.rs", "/b.rs"));
    let id = s.tabs[0].tab_id().clone();
    s.mark_tab_dirty(&id);
    s.force_close_tab(&id);
    assert!(s.tabs.is_empty(), "force_close must remove dirty tab unconditionally");
}

#[test]
fn active_tab_returns_correct_tab() {
    let mut s = WorkspaceSession::empty();
    s.open_tab(diff_tab("/a.rs", "/b.rs"));
    s.open_tab(diff_tab("/c.rs", "/d.rs"));
    let active = s.active_tab().unwrap();
    assert_eq!(active.tab_id(), s.tabs.last().unwrap().tab_id());
}
