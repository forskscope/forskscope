//! Tests for F79: an unreadable entry must be visible - never silently
//! dropped, never asserted as a verdict, and an unopenable root must be
//! distinguishable from an empty tree (handoff 006).
//!
//! Permission tricks here are `#[cfg(unix)]` only - there is no one-line
//! equivalent on Windows, and a test that silently no-ops there is worse
//! than one that is honestly absent (handoff 006 §8's falsifiability
//! hazard). Every permission-dependent test also verifies the permission
//! change actually took effect before asserting anything: `chmod` does not
//! stop root, and a suite running as root (containers often do) would
//! otherwise pass every one of these for the wrong reason. When the check
//! fails, the test skips with an explicit message rather than asserting.

use std::fs;
use std::path::PathBuf;

use crate::CancellationToken;
use crate::dir::{RecStatus, list_recursive_for_display_with_cancel, recursive_diff_with_cancel};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fsk-dirunreadable-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn write(base: &std::path::Path, rel: &str, content: &str) {
    let p = base.join(rel);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(p, content).unwrap();
}

/// `chmod 000` on `dir` so `fs::read_dir(dir)` itself fails - models a
/// directory that cannot be opened. Returns `false` (skip, don't assert)
/// if the change had no effect, e.g. running as root.
#[cfg(unix)]
fn make_dir_unopenable(dir: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o000));
    fs::read_dir(dir).is_err()
}

/// Removes execute (search) permission from `parent_dir` - `read_dir` on it
/// still lists names (read permission kept), but `stat`-ing any child by
/// path now fails, since path resolution needs execute on every ancestor.
/// This is the specific shape of "a per-entry `metadata()` failure" the
/// walk hits, distinct from `make_dir_unopenable`'s "the directory itself
/// cannot be opened". Returns `false` (skip) if the change had no effect.
#[cfg(unix)]
fn make_child_metadata_unreadable(parent_dir: &std::path::Path, child: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(parent_dir, fs::Permissions::from_mode(0o400));
    fs::metadata(child).is_err()
}

#[cfg(unix)]
fn restore_perms(dir: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(mode));
}

// ── §8.1: an unreadable file appears as Unreadable, not absent ──────────────

#[cfg(unix)]
#[test]
fn an_unreadable_file_appears_as_unreadable_not_absent() {
    let base = tmp("file");
    let left = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(left.join("blocked")).unwrap();
    fs::create_dir_all(&right).unwrap();
    write(&left, "blocked/file.txt", "secret");
    // No right-side counterpart at all - with the discarded-error defect,
    // this entry is not just misclassified, it is entirely absent from the
    // result (nothing on either side ever produces it).

    if !make_child_metadata_unreadable(&left.join("blocked"), &left.join("blocked/file.txt")) {
        eprintln!(
            "skipping an_unreadable_file_appears_as_unreadable_not_absent: \
             chmod had no effect (running as root?)"
        );
        restore_perms(&left.join("blocked"), 0o755);
        let _ = fs::remove_dir_all(&base);
        return;
    }

    let scan = recursive_diff_with_cancel(&left, &right, &CancellationToken::new());
    let entry = scan
        .entries
        .iter()
        .find(|e| e.rel_path == std::path::Path::new("blocked/file.txt"));

    assert_eq!(
        entry.map(|e| e.status),
        Some(RecStatus::Unreadable),
        "an unreadable file must appear as Unreadable, not be dropped from the result: {:?}",
        scan.entries
    );

    restore_perms(&left.join("blocked"), 0o755);
    let _ = fs::remove_dir_all(&base);
}

// ── A discovered subtlety while implementing §8.1 ────────────────────────────
//
// Marking the left side Unreadable is not enough on its own: if the right
// side has a normal, readable counterpart at the same path, the merge pass
// (`walk_and_merge`) previously ran a digest comparison against it
// unconditionally - which fails to open the left path for the same reason
// metadata() already failed, and the pre-existing (F76-territory) I/O-error
// fallback would silently turn `Unreadable` back into `Changed`. This test
// exercises that merge-guard fix directly.

#[cfg(unix)]
#[test]
fn an_unreadable_left_file_is_not_reclassified_by_a_readable_right_counterpart() {
    let base = tmp("merge-guard");
    let left = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(left.join("blocked")).unwrap();
    fs::create_dir_all(right.join("blocked")).unwrap();
    write(&left, "blocked/file.txt", "left content");
    write(&right, "blocked/file.txt", "right content");

    if !make_child_metadata_unreadable(&left.join("blocked"), &left.join("blocked/file.txt")) {
        eprintln!(
            "skipping an_unreadable_left_file_is_not_reclassified_by_a_readable_right_counterpart: \
             chmod had no effect (running as root?)"
        );
        restore_perms(&left.join("blocked"), 0o755);
        let _ = fs::remove_dir_all(&base);
        return;
    }

    let scan = recursive_diff_with_cancel(&left, &right, &CancellationToken::new());
    let entry = scan
        .entries
        .iter()
        .find(|e| e.rel_path == std::path::Path::new("blocked/file.txt"));

    assert_eq!(
        entry.map(|e| e.status),
        Some(RecStatus::Unreadable),
        "a left-side Unreadable entry must not be silently reclassified \
         just because the right side has a normal, readable counterpart: {:?}",
        scan.entries
    );

    restore_perms(&left.join("blocked"), 0o755);
    let _ = fs::remove_dir_all(&base);
}

// Review 076 §3: the sibling guard in `walk_and_merge_fast` was fixed by
// inspection alongside `walk_and_merge`'s (same shape, same reasoning) but
// only the latter was independently falsified - and the reviewer checked
// rather than trusted that reasoning: removing `walk_and_merge_fast`'s
// guard left the whole workspace suite green. That matters because
// `walk_and_merge_fast` is reached through
// `list_recursive_for_display_with_cancel` - Deep Compare's phase 1, the
// interactive path a user actually sees - while `walk_and_merge` (the one
// that was falsified) is reached through `recursive_diff`, whose consumers
// are `patch/directory.rs` and `merge_plan.rs`. The guard protecting what a
// user sees was the unprotected one. This test closes that gap directly.

#[cfg(unix)]
#[test]
fn an_unreadable_left_file_is_not_reclassified_in_the_fast_listing_either() {
    let base = tmp("merge-guard-fast");
    let left = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(left.join("blocked")).unwrap();
    fs::create_dir_all(right.join("blocked")).unwrap();
    write(&left, "blocked/file.txt", "left content");
    write(&right, "blocked/file.txt", "right content");

    if !make_child_metadata_unreadable(&left.join("blocked"), &left.join("blocked/file.txt")) {
        eprintln!(
            "skipping an_unreadable_left_file_is_not_reclassified_in_the_fast_listing_either: \
             chmod had no effect (running as root?)"
        );
        restore_perms(&left.join("blocked"), 0o755);
        let _ = fs::remove_dir_all(&base);
        return;
    }

    let scan = list_recursive_for_display_with_cancel(&left, &right, &CancellationToken::new());
    let entry = scan
        .entries
        .iter()
        .find(|e| e.rel_path == std::path::Path::new("blocked/file.txt"));

    assert_eq!(
        entry.map(|e| e.status),
        Some(RecStatus::Unreadable),
        "a left-side Unreadable entry must not be silently promoted to Computing \
         by the fast listing just because the right side has a normal, \
         readable counterpart: {:?}",
        scan.entries
    );

    restore_perms(&left.join("blocked"), 0o755);
    let _ = fs::remove_dir_all(&base);
}

// ── §8.2: an unreadable directory appears as Unreadable, and its parent
//    still lists ─────────────────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn an_unreadable_directory_appears_as_unreadable_and_its_parent_still_lists() {
    let base = tmp("dir");
    let left = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(left.join("parent/blocked_dir")).unwrap();
    fs::create_dir_all(&right).unwrap();
    write(&left, "parent/sibling.txt", "still here");
    write(&left, "parent/blocked_dir/inner.txt", "unreachable");

    let blocked_dir = left.join("parent/blocked_dir");
    if !make_dir_unopenable(&blocked_dir) {
        eprintln!(
            "skipping an_unreadable_directory_appears_as_unreadable_and_its_parent_still_lists: \
             chmod had no effect (running as root?)"
        );
        restore_perms(&blocked_dir, 0o755);
        let _ = fs::remove_dir_all(&base);
        return;
    }

    let scan = recursive_diff_with_cancel(&left, &right, &CancellationToken::new());

    let blocked_entry = scan
        .entries
        .iter()
        .find(|e| e.rel_path == std::path::Path::new("parent/blocked_dir"));
    assert_eq!(
        blocked_entry.map(|e| e.status),
        Some(RecStatus::Unreadable),
        "an unreadable directory must appear as Unreadable at its own path: {:?}",
        scan.entries
    );

    let sibling_entry = scan
        .entries
        .iter()
        .find(|e| e.rel_path == std::path::Path::new("parent/sibling.txt"));
    assert!(
        sibling_entry.is_some(),
        "the parent directory must still list its other children: {:?}",
        scan.entries
    );

    // The subtree of an unreadable directory stays absent - unavoidable,
    // nothing read it (handoff 006 §14). Confirmed, not merely assumed.
    let inner_entry = scan
        .entries
        .iter()
        .find(|e| e.rel_path == std::path::Path::new("parent/blocked_dir/inner.txt"));
    assert!(
        inner_entry.is_none(),
        "an unreadable directory's subtree cannot be recovered: {:?}",
        scan.entries
    );

    restore_perms(&blocked_dir, 0o755);
    let _ = fs::remove_dir_all(&base);
}

// ── §8.3: an unopenable root is distinguishable from an empty tree ──────────

#[cfg(unix)]
#[test]
fn an_unopenable_root_is_distinguishable_from_an_empty_tree() {
    let base = tmp("root");
    let left = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    write(&left, "a.txt", "content-a");
    write(&left, "b.txt", "content-b");

    if !make_dir_unopenable(&right) {
        eprintln!(
            "skipping an_unopenable_root_is_distinguishable_from_an_empty_tree: \
             chmod had no effect (running as root?)"
        );
        restore_perms(&right, 0o755);
        let _ = fs::remove_dir_all(&base);
        return;
    }

    let scan = recursive_diff_with_cancel(&left, &right, &CancellationToken::new());

    assert!(
        scan.right_root_unreadable,
        "an unopenable right root must be flagged, not silently treated as empty"
    );
    assert!(
        !scan.left_root_unreadable,
        "the left root opened fine and must not be flagged"
    );
    // The old defect this replaces: every left-side file reads as a
    // confident `LeftOnly` verdict here, which is indistinguishable from
    // "the right tree really is empty" unless the caller also has this
    // flag - asserting it exists and is set is the point of this test.
    assert_eq!(
        scan.entries
            .iter()
            .filter(|e| e.status == RecStatus::LeftOnly)
            .count(),
        2,
        "the entries alone still look exactly like a genuinely empty right tree: {:?}",
        scan.entries
    );

    restore_perms(&right, 0o755);
    let _ = fs::remove_dir_all(&base);
}

/// Contrast for the test above: a genuinely empty (but readable) right root
/// must NOT set the flag - otherwise the flag itself would be meaningless
/// noise rather than a real signal. Not permission-dependent, so it runs on
/// every platform (unlike the rest of this file).
#[test]
fn a_genuinely_empty_right_root_is_not_flagged_unreadable() {
    let base = tmp("root-empty");
    let left = base.join("l");
    let right = base.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    write(&left, "a.txt", "x");

    let scan = recursive_diff_with_cancel(&left, &right, &CancellationToken::new());

    assert!(!scan.right_root_unreadable);
    assert!(!scan.left_root_unreadable);

    let _ = fs::remove_dir_all(&base);
}
