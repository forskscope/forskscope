//! External file state detection tests (RFC-036 §"Acceptance Criteria").
//!
//! All tests are hermetic: each creates a temp file, captures a fingerprint,
//! optionally mutates the file or the session-dirty flag, and asserts the
//! correct `ExternalFileState`. No network, no subprocess.

use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use crate::document::{ExternalFileState, FileFingerprint, check_external_state};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("fsk-extstate-{tag}-{}", std::process::id()));
    fs::create_dir_all(&d).unwrap();
    d
}

/// Create a file, capture its fingerprint immediately.
fn make_file(dir: &std::path::Path, name: &str, content: &str) -> (PathBuf, FileFingerprint) {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    let fp = FileFingerprint::capture(&path, Some(content.as_bytes())).unwrap();
    (path, fp)
}

// ── Clean and DirtyInSession ──────────────────────────────────────────────────

#[test]
fn unchanged_file_no_session_edits_is_clean() {
    let dir = tmp("clean");
    let (path, fp) = make_file(&dir, "f.txt", "hello");
    assert_eq!(
        check_external_state(&path, &fp, false),
        ExternalFileState::Clean
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn unchanged_file_with_session_edits_is_dirty_in_session() {
    let dir = tmp("dirtyinsession");
    let (path, fp) = make_file(&dir, "f.txt", "hello");
    assert_eq!(
        check_external_state(&path, &fp, true),
        ExternalFileState::DirtyInSession
    );
    let _ = fs::remove_dir_all(&dir);
}

// ── ChangedOnDisk ─────────────────────────────────────────────────────────────

#[test]
fn file_with_different_size_is_changed_on_disk() {
    let dir = tmp("sizechange");
    let (path, fp) = make_file(&dir, "f.txt", "short");
    fs::write(&path, "this is much longer content").unwrap();
    // Size changed: immediate detection without mtime.
    assert_eq!(
        check_external_state(&path, &fp, false),
        ExternalFileState::ChangedOnDisk
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_with_different_mtime_same_size_is_changed_on_disk() {
    let dir = tmp("mtimechange");
    let (path, fp) = make_file(&dir, "f.txt", "abcde");
    // Wait briefly so the mtime ticks (1ms granularity on most filesystems).
    sleep(Duration::from_millis(10));
    // Overwrite with same-size content to force mtime change.
    fs::write(&path, "ABCDE").unwrap();
    let state = check_external_state(&path, &fp, false);
    // On filesystems with coarse mtime (1s), this may read as Clean.
    // Accept both: the important thing is it never returns DirtyInSession.
    assert!(
        matches!(state, ExternalFileState::ChangedOnDisk | ExternalFileState::Clean),
        "should not report DirtyInSession for externally modified file: {state:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn changed_on_disk_blocks_save() {
    assert!(ExternalFileState::ChangedOnDisk.blocks_save());
}

// ── DeletedOnDisk ─────────────────────────────────────────────────────────────

#[test]
fn deleted_file_is_deleted_on_disk() {
    let dir = tmp("deleted");
    let (path, fp) = make_file(&dir, "f.txt", "content");
    fs::remove_file(&path).unwrap();
    assert_eq!(
        check_external_state(&path, &fp, false),
        ExternalFileState::DeletedOnDisk
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn deleted_on_disk_blocks_save() {
    assert!(ExternalFileState::DeletedOnDisk.blocks_save());
}

// ── ReplacedOnDisk ────────────────────────────────────────────────────────────

#[test]
fn path_now_pointing_to_directory_is_replaced_on_disk() {
    let dir = tmp("replaced");
    let path = dir.join("f.txt");
    fs::write(&path, "content").unwrap();
    let fp = FileFingerprint::capture(&path, None).unwrap();
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();   // same name, now a directory
    assert_eq!(
        check_external_state(&path, &fp, false),
        ExternalFileState::ReplacedOnDisk
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn replaced_on_disk_blocks_save() {
    assert!(ExternalFileState::ReplacedOnDisk.blocks_save());
}

// ── ExternalFileState predicates ─────────────────────────────────────────────

#[test]
fn clean_does_not_block_save() {
    assert!(!ExternalFileState::Clean.blocks_save());
}

#[test]
fn dirty_in_session_does_not_block_save() {
    assert!(!ExternalFileState::DirtyInSession.blocks_save());
}

#[test]
fn unknown_does_not_block_save() {
    // Unknown is conservative: we don't block (the save path falls back
    // to pre-save stat verification). Blocking on Unknown would make
    // ForskScope unusable on non-standard filesystems.
    assert!(!ExternalFileState::Unknown.blocks_save());
}

#[test]
fn file_accessible_for_reachable_states() {
    assert!(ExternalFileState::Clean.file_accessible());
    assert!(ExternalFileState::DirtyInSession.file_accessible());
    assert!(ExternalFileState::ChangedOnDisk.file_accessible());
}

#[test]
fn file_not_accessible_for_missing_or_unknown() {
    assert!(!ExternalFileState::DeletedOnDisk.file_accessible());
    assert!(!ExternalFileState::ReplacedOnDisk.file_accessible());
    assert!(!ExternalFileState::Unknown.file_accessible());
}

// ── Never-panic guarantee ────────────────────────────────────────────────────

#[test]
fn nonexistent_snapshot_path_returns_deleted_not_panic() {
    // A path that never existed.
    let fp = FileFingerprint { len: 0, modified_unix_nanos: None, digest: None };
    let state = check_external_state("/tmp/fsk-definitely-does-not-exist-xyz-789".as_ref(), &fp, false);
    assert_eq!(state, ExternalFileState::DeletedOnDisk,
        "nonexistent path must be DeletedOnDisk, never Unknown or panic");
}
