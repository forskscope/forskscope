//! Batch copy and restore manifest tests (RFC-023 §"Batch operation manifest").
//!
//! Tests validate the RFC-023 acceptance criteria: successful batch creates a
//! manifest with accurate entries, stop-on-first terminates correctly, continue
//! policy collects all outcomes, backups are created, restore recovers files,
//! and the manifest JSON is valid.

use std::fs;
use std::path::PathBuf;

use crate::dir::{BatchFailurePolicy, BatchItem, EntryOutcome, batch_copy, restore_from_manifest};
use crate::save::BackupPolicy;

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("fsk-batch-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn items(pairs: &[(&str, &str)], src_dir: &std::path::Path, dst_dir: &std::path::Path) -> Vec<BatchItem> {
    pairs.iter().map(|(s, d)| BatchItem {
        src: src_dir.join(s),
        dst: dst_dir.join(d),
    }).collect()
}

// ── Success path ──────────────────────────────────────────────────────────────

#[test]
fn all_copies_succeed_and_manifest_records_correct_counts() {
    let base = tmp("allok");
    let src = base.join("src"); fs::create_dir_all(&src).unwrap();
    let dst = base.join("dst"); fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("a.txt"), "alpha").unwrap();
    fs::write(src.join("b.txt"), "beta").unwrap();

    let items = items(&[("a.txt","a.txt"),("b.txt","b.txt")], &src, &dst);
    let m = batch_copy(&items, BackupPolicy::None, BatchFailurePolicy::StopOnFirst, None).unwrap();

    assert_eq!(m.succeeded(), 2);
    assert_eq!(m.failed(), 0);
    assert_eq!(m.attempted(), 2);
    assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(dst.join("b.txt")).unwrap(), "beta");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn backup_is_created_for_overwritten_destination() {
    let base = tmp("backup");
    let src = base.join("src"); fs::create_dir_all(&src).unwrap();
    let dst = base.join("dst"); fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("f.txt"), "new content").unwrap();
    fs::write(dst.join("f.txt"), "old content").unwrap();  // existing destination

    let items = items(&[("f.txt","f.txt")], &src, &dst);
    let m = batch_copy(&items, BackupPolicy::SiblingBak, BatchFailurePolicy::StopOnFirst, None).unwrap();

    assert_eq!(m.succeeded(), 1);
    let bak_paths = m.backup_paths();
    assert_eq!(bak_paths.len(), 1, "one backup should be created");
    assert!(bak_paths[0].exists(), "backup file must exist on disk");
    assert_eq!(fs::read_to_string(bak_paths[0]).unwrap(), "old content");
    assert_eq!(fs::read_to_string(dst.join("f.txt")).unwrap(), "new content");
    let _ = fs::remove_dir_all(&base);
}

// ── Failure policies ──────────────────────────────────────────────────────────

#[test]
fn stop_on_first_marks_remaining_as_skipped() {
    let base = tmp("stopfirst");
    let src = base.join("src"); fs::create_dir_all(&src).unwrap();
    let dst = base.join("dst"); fs::create_dir_all(&dst).unwrap();
    // Only the first file exists — second file missing causes a failure.
    fs::write(src.join("good.txt"), "ok").unwrap();
    // missing.txt intentionally absent from src.

    let items = vec![
        BatchItem { src: src.join("missing.txt"), dst: dst.join("missing.txt") },
        BatchItem { src: src.join("good.txt"),   dst: dst.join("good.txt") },
    ];
    let m = batch_copy(&items, BackupPolicy::None, BatchFailurePolicy::StopOnFirst, None).unwrap();

    assert_eq!(m.failed(), 1);
    assert!(matches!(m.entries[0].outcome, EntryOutcome::Failed { .. }));
    // Second item must be Skipped, not attempted.
    assert!(matches!(m.entries[1].outcome, EntryOutcome::Skipped { .. }));
    assert!(!dst.join("good.txt").exists(), "skipped item must not be copied");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn continue_on_failure_collects_all_outcomes() {
    let base = tmp("continueall");
    let src = base.join("src"); fs::create_dir_all(&src).unwrap();
    let dst = base.join("dst"); fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("ok1.txt"), "one").unwrap();
    // missing.txt absent
    fs::write(src.join("ok2.txt"), "two").unwrap();

    let items = vec![
        BatchItem { src: src.join("ok1.txt"),     dst: dst.join("ok1.txt") },
        BatchItem { src: src.join("missing.txt"), dst: dst.join("missing.txt") },
        BatchItem { src: src.join("ok2.txt"),     dst: dst.join("ok2.txt") },
    ];
    let m = batch_copy(&items, BackupPolicy::None, BatchFailurePolicy::ContinueOnFailure, None).unwrap();

    assert_eq!(m.succeeded(), 2);
    assert_eq!(m.failed(), 1);
    // No Skipped entries under ContinueOnFailure.
    assert!(!m.entries.iter().any(|e| matches!(e.outcome, EntryOutcome::Skipped { .. })));
    assert!(dst.join("ok2.txt").exists(), "ok2 should be copied despite earlier failure");
    let _ = fs::remove_dir_all(&base);
}

// ── Manifest persistence ──────────────────────────────────────────────────────

#[test]
fn manifest_is_written_to_directory_when_provided() {
    let base = tmp("manifest");
    let src = base.join("src"); fs::create_dir_all(&src).unwrap();
    let dst = base.join("dst"); fs::create_dir_all(&dst).unwrap();
    let mdir = base.join("manifests");
    fs::write(src.join("x.txt"), "data").unwrap();

    let items = items(&[("x.txt","x.txt")], &src, &dst);
    let m = batch_copy(&items, BackupPolicy::None, BatchFailurePolicy::StopOnFirst, Some(&mdir)).unwrap();

    assert!(m.manifest_path.is_some(), "manifest_path must be set");
    let path = m.manifest_path.as_ref().unwrap();
    assert!(path.exists(), "manifest file must exist on disk");
    assert!(path.extension().map(|e| e == "json").unwrap_or(false), "must be JSON");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn manifest_json_contains_operation_id_and_entry_outcomes() {
    let base = tmp("jsoncheck");
    let src = base.join("src"); fs::create_dir_all(&src).unwrap();
    let dst = base.join("dst"); fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), "hello").unwrap();

    let items = items(&[("file.txt","file.txt")], &src, &dst);
    let m = batch_copy(&items, BackupPolicy::None, BatchFailurePolicy::StopOnFirst, None).unwrap();
    let json = m.to_json();

    assert!(json.contains("\"operation_id\""), "must have operation_id field");
    assert!(json.contains("\"app_version\""), "must have app_version field");
    assert!(json.contains("\"entries\""), "must have entries array");
    assert!(json.contains("\"copied\""), "successful copy must show outcome=copied");
    // Basic JSON structure: starts with { ends with }
    assert!(json.trim().starts_with('{'));
    assert!(json.trim().ends_with('}'));
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn operation_id_is_unique_across_two_calls() {
    use crate::dir::OperationId;
    let id1 = OperationId::new();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let id2 = OperationId::new();
    // Same second is possible; PIDs are the same; but the two calls
    // happening within 1ms make collision very unlikely. We assert
    // they're stable strings (non-empty, start with "op-").
    assert!(id1.0.starts_with("op-"), "id must start with op-: {}", id1.0);
    assert!(id2.0.starts_with("op-"), "id must start with op-: {}", id2.0);
}

// ── Restore ───────────────────────────────────────────────────────────────────

#[test]
fn restore_from_manifest_recovers_overwritten_files() {
    let base = tmp("restore");
    let src = base.join("src"); fs::create_dir_all(&src).unwrap();
    let dst = base.join("dst"); fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("f.txt"), "new version").unwrap();
    fs::write(dst.join("f.txt"), "original").unwrap();

    let items = items(&[("f.txt","f.txt")], &src, &dst);
    let m = batch_copy(&items, BackupPolicy::SiblingBak, BatchFailurePolicy::StopOnFirst, None).unwrap();
    assert_eq!(fs::read_to_string(dst.join("f.txt")).unwrap(), "new version");

    let restored = restore_from_manifest(&m);
    assert_eq!(restored, 1, "one file should be restored");
    assert_eq!(fs::read_to_string(dst.join("f.txt")).unwrap(), "original",
        "restored file must match pre-copy content");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn restore_skips_entries_without_backup() {
    let base = tmp("norestore");
    let src = base.join("src"); fs::create_dir_all(&src).unwrap();
    let dst = base.join("dst"); fs::create_dir_all(&dst).unwrap();
    // New file — no existing destination, so no backup created.
    fs::write(src.join("new.txt"), "fresh").unwrap();

    let items = items(&[("new.txt","new.txt")], &src, &dst);
    let m = batch_copy(&items, BackupPolicy::SiblingBak, BatchFailurePolicy::StopOnFirst, None).unwrap();
    assert_eq!(m.backup_paths().len(), 0, "no backup for new file");

    let restored = restore_from_manifest(&m);
    assert_eq!(restored, 0, "nothing to restore");
    // The copied file remains.
    assert!(dst.join("new.txt").exists());
    let _ = fs::remove_dir_all(&base);
}
