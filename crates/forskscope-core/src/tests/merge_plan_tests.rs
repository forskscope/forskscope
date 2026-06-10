//! Directory merge plan tests (RFC-022 §"Acceptance criteria", §"Test strategy").
//!
//! Tests cover: plan generation for all entry statuses, direction handling,
//! selection filters, risk summary accuracy, preflight checks, plan safety
//! predicate, execute_plan round-trip, and skip entries.

use std::fs;
use std::path::PathBuf;

use crate::dir::{
    CopyDirection, DirectoryMergeAction, EntrySelection, FileOutcome, RecEntry, RecStatus,
    BatchFailurePolicy, execute_plan, plan_operations,
};
use crate::save::BackupPolicy;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("fsk-mergeplan-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn write(base: &std::path::Path, rel: &str, content: &str) {
    let p = base.join(rel);
    if let Some(par) = p.parent() { let _ = fs::create_dir_all(par); }
    fs::write(p, content).unwrap();
}

fn entry(rel: &str, status: RecStatus, ls: Option<u64>, rs: Option<u64>) -> RecEntry {
    RecEntry { rel_path: PathBuf::from(rel), status, left_size: ls, right_size: rs }
}

// ── plan_operations: direction and status ─────────────────────────────────────

#[test]
fn left_to_right_plans_left_only_as_copy() {
    let entries = vec![entry("new.txt", RecStatus::LeftOnly, Some(100), None)];
    let plan = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    let op = &plan.operations[0];
    assert_eq!(op.action, DirectoryMergeAction::CopyLeftToRight);
    assert_eq!(op.source, Some(PathBuf::from("/l/new.txt")));
    assert_eq!(op.target, Some(PathBuf::from("/r/new.txt")));
}

#[test]
fn right_to_left_plans_right_only_as_copy() {
    let entries = vec![entry("new.txt", RecStatus::RightOnly, None, Some(80))];
    let plan = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::RightToLeft, EntrySelection::AllNonEqual);
    let op = &plan.operations[0];
    assert_eq!(op.action, DirectoryMergeAction::CopyRightToLeft);
    assert_eq!(op.source, Some(PathBuf::from("/r/new.txt")));
    assert_eq!(op.target, Some(PathBuf::from("/l/new.txt")));
}

#[test]
fn changed_entry_is_planned_for_copy_in_both_directions() {
    let entries = vec![entry("a.rs", RecStatus::Changed, Some(200), Some(210))];
    let l2r = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    assert_eq!(l2r.operations[0].action, DirectoryMergeAction::CopyLeftToRight);

    let r2l = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::RightToLeft, EntrySelection::AllNonEqual);
    assert_eq!(r2l.operations[0].action, DirectoryMergeAction::CopyRightToLeft);
}

#[test]
fn equal_entries_are_excluded_from_plan() {
    let entries = vec![
        entry("same.txt", RecStatus::Equal, Some(50), Some(50)),
        entry("diff.txt", RecStatus::Changed, Some(100), Some(110)),
    ];
    let plan = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    assert_eq!(plan.risk_summary.total_files, 1, "equal file must not count toward total");
    assert_eq!(plan.operations.len(), 1);
}

#[test]
fn left_only_entry_is_skipped_when_direction_is_right_to_left() {
    // A file only on the left side: with R→L direction it becomes a Skip,
    // because there's nothing to copy from the right.
    let entries = vec![entry("left_only.txt", RecStatus::LeftOnly, Some(50), None)];
    let plan = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::RightToLeft, EntrySelection::AllNonEqual);
    assert_eq!(plan.operations[0].action, DirectoryMergeAction::Skip);
    assert_eq!(plan.risk_summary.total_files, 0, "skip doesn't count as copy");
}

// ── EntrySelection filters ────────────────────────────────────────────────────

#[test]
fn changed_only_filter_excludes_one_sided_entries() {
    let entries = vec![
        entry("changed.txt",   RecStatus::Changed,  Some(100), Some(110)),
        entry("left_only.txt", RecStatus::LeftOnly,  Some(50),  None),
    ];
    let plan = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::LeftToRight, EntrySelection::ChangedOnly);
    assert_eq!(plan.risk_summary.total_files, 1, "only changed files should be planned");
}

#[test]
fn source_only_filter_includes_only_source_side_entries() {
    let entries = vec![
        entry("left.txt",  RecStatus::LeftOnly,  Some(50), None),
        entry("right.txt", RecStatus::RightOnly, None, Some(60)),
        entry("same.txt",  RecStatus::Changed,   Some(80), Some(90)),
    ];
    // L→R with SourceOnly: only left-only entries.
    let plan = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::LeftToRight, EntrySelection::SourceOnlyEntries);
    assert_eq!(plan.risk_summary.total_files, 1);
    assert_eq!(plan.operations.iter().filter(|o| o.action != DirectoryMergeAction::Skip).count(), 1);
}

// ── Risk summary ──────────────────────────────────────────────────────────────

#[test]
fn risk_summary_counts_are_accurate() {
    let base = tmp("risksummary");
    let left  = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    write(&left,  "new.txt",      "content");
    write(&left,  "existing.txt", "left version");
    write(&right, "existing.txt", "right version");  // will be overwritten

    let entries = vec![
        entry("new.txt",      RecStatus::LeftOnly, Some(7), None),
        entry("existing.txt", RecStatus::Changed,  Some(12), Some(13)),
    ];
    let plan = plan_operations(&entries, &left, &right,
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);

    assert_eq!(plan.risk_summary.total_files,  2);
    assert_eq!(plan.risk_summary.new_files,    1, "new.txt is a new file");
    assert_eq!(plan.risk_summary.overwrites,   1, "existing.txt is an overwrite");
    assert_eq!(plan.risk_summary.estimated_bytes, 7 + 12);
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn plan_is_safe_when_no_permission_blocks() {
    let entries = vec![entry("a.txt", RecStatus::LeftOnly, Some(10), None)];
    let plan = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    // Target /r/a.txt does not exist — no permission block (parent is writable by assumption).
    assert_eq!(plan.risk_summary.permission_blocks, 0);
    assert!(plan.is_safe_to_execute());
}

// ── Preflight ─────────────────────────────────────────────────────────────────

#[test]
fn preflight_detects_existing_target() {
    let base = tmp("preflight");
    let left  = base.join("l"); fs::create_dir_all(&left).unwrap();
    let right = base.join("r"); fs::create_dir_all(&right).unwrap();
    write(&left,  "f.txt", "left");
    write(&right, "f.txt", "right");   // target exists

    let entries = vec![entry("f.txt", RecStatus::Changed, Some(4), Some(5))];
    let plan = plan_operations(&entries, &left, &right,
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    let op = &plan.operations[0];
    assert!(op.preflight.target_exists,   "existing target must be detected");
    assert!(op.preflight.backup_required, "overwrite requires backup");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn preflight_no_backup_needed_for_new_file() {
    let base = tmp("preflight-new");
    let left  = base.join("l"); fs::create_dir_all(&left).unwrap();
    let right = base.join("r"); fs::create_dir_all(&right).unwrap();
    write(&left, "new.txt", "hello");

    let entries = vec![entry("new.txt", RecStatus::LeftOnly, Some(5), None)];
    let plan = plan_operations(&entries, &left, &right,
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    assert!(!plan.operations[0].preflight.target_exists);
    assert!(!plan.operations[0].preflight.backup_required);
    let _ = fs::remove_dir_all(&base);
}

// ── execute_plan round-trip ───────────────────────────────────────────────────

#[test]
fn execute_plan_copies_all_planned_files_successfully() {
    let base = tmp("execute");
    let left  = base.join("l"); fs::create_dir_all(&left).unwrap();
    let right = base.join("r"); fs::create_dir_all(&right).unwrap();
    write(&left, "a.txt", "alpha");
    write(&left, "b.txt", "beta");

    let entries = vec![
        entry("a.txt", RecStatus::LeftOnly, Some(5), None),
        entry("b.txt", RecStatus::LeftOnly, Some(4), None),
    ];
    let plan = plan_operations(&entries, &left, &right,
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    let report = execute_plan(&plan, BackupPolicy::None, BatchFailurePolicy::ContinueOnFailure);

    assert_eq!(report.succeeded, 2);
    assert_eq!(report.failed,    0);
    assert!(right.join("a.txt").exists(), "a.txt must be copied");
    assert!(right.join("b.txt").exists(), "b.txt must be copied");
    assert_eq!(fs::read_to_string(right.join("a.txt")).unwrap(), "alpha");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn execute_plan_creates_backup_when_overwriting() {
    let base = tmp("execute-backup");
    let left  = base.join("l"); fs::create_dir_all(&left).unwrap();
    let right = base.join("r"); fs::create_dir_all(&right).unwrap();
    write(&left,  "f.txt", "new content");
    write(&right, "f.txt", "original");

    let entries = vec![entry("f.txt", RecStatus::Changed, Some(11), Some(8))];
    let plan = plan_operations(&entries, &left, &right,
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    let report = execute_plan(&plan, BackupPolicy::SiblingBak,
        BatchFailurePolicy::StopOnFirst);

    assert_eq!(report.succeeded, 1);
    let (_, outcome) = &report.outcomes[0];
    assert!(matches!(outcome, FileOutcome::Copied { backup_created: true, .. }),
        "backup must be created for overwrite");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn execute_plan_skips_count_is_reported() {
    let entries = vec![
        // Left-only going R→L: will become a Skip.
        entry("left_only.txt", RecStatus::LeftOnly, Some(50), None),
    ];
    let plan = plan_operations(&entries, "/l".as_ref(), "/r".as_ref(),
        CopyDirection::RightToLeft, EntrySelection::AllNonEqual);
    // No actual copy to execute (all skipped) — verify skipped count.
    assert_eq!(plan.risk_summary.total_files, 0);
    let report = execute_plan(&plan, BackupPolicy::None, BatchFailurePolicy::StopOnFirst);
    assert_eq!(report.skipped, 1);
    assert_eq!(report.succeeded, 0);
}

#[test]
fn empty_entry_list_produces_empty_plan() {
    let plan = plan_operations(&[], "/l".as_ref(), "/r".as_ref(),
        CopyDirection::LeftToRight, EntrySelection::AllNonEqual);
    assert_eq!(plan.operations.len(), 0);
    assert_eq!(plan.risk_summary.total_files, 0);
}
