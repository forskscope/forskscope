//! Tests for `CancellationToken`, cancellable directory recursion, and
//! symlink handling (RFC-037 §"Cancellation", §"Symlink Policy").

use std::fs;
use std::path::PathBuf;

use crate::CancellationToken;
use crate::dir::{RecStatus, list_recursive_for_display_with_cancel, recursive_diff_with_cancel};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("fsk-dircancel-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn write(base: &std::path::Path, rel: &str, content: &str) {
    let p = base.join(rel);
    if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
    fs::write(p, content).unwrap();
}

// ── CancellationToken unit tests ─────────────────────────────────────────────

#[test]
fn token_starts_uncancelled() {
    assert!(!CancellationToken::new().is_cancelled());
}

#[test]
fn cancel_is_observed_by_all_clones() {
    let t = CancellationToken::new();
    let c1 = t.clone();
    let c2 = t.clone();
    t.cancel();
    assert!(c1.is_cancelled());
    assert!(c2.is_cancelled());
}

#[test]
fn cancel_on_clone_propagates_back() {
    let t = CancellationToken::new();
    let c = t.clone();
    c.cancel();
    assert!(t.is_cancelled());
}

// ── Non-cancellable behaviour preserved ──────────────────────────────────────

#[test]
fn uncancelled_token_produces_same_result_as_non_cancellable_api() {
    let base = tmp("compat");
    let left  = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    write(&left,  "common.txt", "hello");
    write(&right, "common.txt", "hello");
    write(&left,  "only_l.txt", "x");
    write(&right, "only_r.txt", "y");

    let token = CancellationToken::new();
    let cancellable = recursive_diff_with_cancel(&left, &right, &token);
    let standard    = crate::dir::recursive_diff(&left, &right);

    assert_eq!(cancellable.len(), standard.len());
    for (a, b) in cancellable.iter().zip(standard.iter()) {
        assert_eq!(a.rel_path, b.rel_path);
        assert_eq!(a.status,   b.status);
    }
    let _ = fs::remove_dir_all(&base);
}

// ── Pre-cancelled token returns immediately ───────────────────────────────────

#[test]
fn pre_cancelled_token_returns_empty_or_partial() {
    let base = tmp("precancelled");
    let left  = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    for i in 0..20 {
        write(&left,  &format!("file{i}.txt"), &format!("left {i}"));
        write(&right, &format!("file{i}.txt"), &format!("right {i}"));
    }

    let token = CancellationToken::new();
    token.cancel();   // cancel BEFORE the scan starts
    let result = recursive_diff_with_cancel(&left, &right, &token);

    // The result may be empty or partial, but must not be the full 20-file
    // set with all digests computed (because we cancelled before any work).
    // The key assertion: we got no panic, no block, and the token says cancelled.
    assert!(token.is_cancelled());
    // At most the initial walk could have run on left before the check.
    // Nothing should show Changed (that requires digest comparison).
    let changed = result.iter().filter(|e| e.status == RecStatus::Changed).count();
    assert_eq!(changed, 0, "no digest should run after pre-cancel");
    let _ = fs::remove_dir_all(&base);
}

// ── Cancel mid-scan reduces the result set ───────────────────────────────────

#[test]
fn cancel_during_scan_produces_partial_results_without_panic() {
    let base = tmp("midscan");
    let left  = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    // Enough files that cancellation mid-scan gives a partial result.
    for i in 0..50 {
        write(&left,  &format!("file{i:03}.txt"), &"x".repeat(1024));
        write(&right, &format!("file{i:03}.txt"), &"y".repeat(1024));
    }

    let token = CancellationToken::new();
    let t2 = token.clone();

    // Cancel from a thread after a tiny delay so the scan has started.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_micros(100));
        t2.cancel();
    });

    // This must return (not block forever), even if only partially done.
    let result = recursive_diff_with_cancel(&left, &right, &token);
    assert!(result.len() <= 50, "no extra entries");
    let _ = fs::remove_dir_all(&base);
}

// ── Symlink handling ──────────────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn symlinks_reported_as_symlink_status_not_silently_skipped() {
    let base = tmp("symlink");
    let left  = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    write(&left, "real.txt", "content");
    // Create a symlink on the left side.
    std::os::unix::fs::symlink("/etc/hostname", left.join("link.txt")).unwrap();

    let result = crate::dir::recursive_diff(&left, &right);
    let statuses: Vec<_> = result.iter().map(|e| (e.rel_path.to_str().unwrap(), e.status)).collect();

    // real.txt: LeftOnly (exists only on left)
    // link.txt: Symlink (not silently skipped, not treated as regular file)
    let link_entry = result.iter().find(|e| e.rel_path.ends_with("link.txt"));
    assert!(link_entry.is_some(), "symlink must appear in results (not silently skipped): {statuses:?}");
    assert_eq!(
        link_entry.unwrap().status,
        RecStatus::Symlink,
        "symlink must have Symlink status: {statuses:?}"
    );
    let _ = fs::remove_dir_all(&base);
}

#[cfg(unix)]
#[test]
fn symlink_in_fast_listing_also_gets_symlink_status() {
    let base = tmp("symlink-fast");
    let left  = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    std::os::unix::fs::symlink("/etc/hostname", left.join("link.txt")).unwrap();

    let token = CancellationToken::new();
    let result = list_recursive_for_display_with_cancel(&left, &right, &token);
    let link = result.iter().find(|e| e.rel_path.ends_with("link.txt")).unwrap();
    assert_eq!(link.status, RecStatus::Symlink);
    let _ = fs::remove_dir_all(&base);
}
