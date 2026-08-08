//! RFC-077: `TargetPrecondition`/`check_precondition` and the no-clobber
//! commit (`persist_noclobber`), which `save_text` routes through for
//! `TargetPrecondition::MustBeAbsent` (see `save_tests.rs` for coverage of
//! that integration; this file tests the primitives directly).

use std::fs;
use std::path::PathBuf;

use crate::document::FileFingerprint;
use crate::error::CoreError;
use crate::save::{
    TargetPrecondition, check_precondition, persist_noclobber, persist_noclobber_with_hook,
};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-save-target-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

// ── check_precondition: MustMatch ───────────────────────────────────────────

#[test]
fn must_match_succeeds_when_target_is_unchanged() {
    let dir = temp_dir("must-match-clean");
    let path = dir.join("target.txt");
    fs::write(&path, "content\n").unwrap();
    let fp = FileFingerprint::capture(&path, None).unwrap();

    assert!(check_precondition(&path, &TargetPrecondition::MustMatch(fp)).is_ok());
}

#[test]
fn must_match_conflicts_when_content_changed() {
    let dir = temp_dir("must-match-changed");
    let path = dir.join("target.txt");
    fs::write(&path, "original\n").unwrap();
    let fp = FileFingerprint::capture(&path, None).unwrap();
    // Sleep-free: write different-length content so len alone differs,
    // regardless of filesystem mtime resolution.
    fs::write(&path, "a very different and much longer line of content\n").unwrap();

    let result = check_precondition(&path, &TargetPrecondition::MustMatch(fp));

    assert!(matches!(result, Err(CoreError::Conflict { .. })));
}

#[test]
fn must_match_conflicts_when_target_deleted() {
    let dir = temp_dir("must-match-deleted");
    let path = dir.join("target.txt");
    fs::write(&path, "content\n").unwrap();
    let fp = FileFingerprint::capture(&path, None).unwrap();
    fs::remove_file(&path).unwrap();

    let result = check_precondition(&path, &TargetPrecondition::MustMatch(fp));

    assert!(
        matches!(result, Err(CoreError::Conflict { .. })),
        "a deleted target must be a Conflict, not an Io error — it's a stale-read situation, \
         not a filesystem failure"
    );
}

#[test]
fn must_match_conflicts_when_target_replaced_by_a_directory() {
    let dir = temp_dir("must-match-replaced");
    let path = dir.join("target.txt");
    fs::write(&path, "content\n").unwrap();
    let fp = FileFingerprint::capture(&path, None).unwrap();
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let result = check_precondition(&path, &TargetPrecondition::MustMatch(fp));

    assert!(matches!(result, Err(CoreError::Conflict { .. })));
}

// ── check_precondition: MustBeAbsent ────────────────────────────────────────

#[test]
fn must_be_absent_succeeds_when_nothing_is_there() {
    let dir = temp_dir("must-be-absent-ok");
    let path = dir.join("does-not-exist.txt");
    let _ = fs::remove_file(&path);

    assert!(check_precondition(&path, &TargetPrecondition::MustBeAbsent).is_ok());
}

#[test]
fn must_be_absent_conflicts_when_a_file_exists() {
    let dir = temp_dir("must-be-absent-file");
    let path = dir.join("appeared.txt");
    fs::write(&path, "surprise\n").unwrap();

    let result = check_precondition(&path, &TargetPrecondition::MustBeAbsent);

    assert!(matches!(result, Err(CoreError::Conflict { .. })));
}

#[test]
fn must_be_absent_conflicts_when_a_directory_exists() {
    let dir = temp_dir("must-be-absent-dir");
    let path = dir.join("appeared-dir");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();

    let result = check_precondition(&path, &TargetPrecondition::MustBeAbsent);

    assert!(matches!(result, Err(CoreError::Conflict { .. })));
}

#[cfg(unix)]
#[test]
fn must_be_absent_conflicts_on_a_dangling_symlink() {
    // Review 046 N1: `Path::exists()` follows symlinks and reports a
    // dangling one as absent — `check_precondition` must not, since a
    // symlink entry at the path is still an entry.
    let dir = temp_dir("must-be-absent-dangling-symlink");
    let path = dir.join("dangling-link");
    let _ = fs::remove_file(&path);
    std::os::unix::fs::symlink(dir.join("nonexistent-target"), &path).unwrap();

    let result = check_precondition(&path, &TargetPrecondition::MustBeAbsent);

    assert!(
        matches!(result, Err(CoreError::Conflict { .. })),
        "a dangling symlink is still an entry at the path, not an absent target"
    );
}

#[cfg(unix)]
#[test]
fn must_be_absent_propagates_a_genuine_read_failure_as_io_not_ok() {
    // Review 046 N1: a metadata read failure (here, permission denied on the
    // parent directory) must surface as `CoreError::Io`, not be silently
    // treated as "absent".
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("must-be-absent-unreadable-parent");
    let blocked = dir.join("blocked");
    let _ = fs::remove_dir_all(&blocked);
    fs::create_dir(&blocked).unwrap();
    let path = blocked.join("target.txt");
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();

    // Running as root bypasses directory permission bits entirely, so
    // `0o000` would not actually block the read on such a runner — confirm
    // the restriction actually took effect before trusting the assertion
    // below, rather than assuming non-root (e.g. some containerized CI runs
    // as root).
    let restriction_is_effective = fs::metadata(&path).is_err();

    let result = check_precondition(&path, &TargetPrecondition::MustBeAbsent);

    // Restore permissions before any assertion can panic and leak an
    // unreadable directory into the shared temp root.
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755)).unwrap();

    if !restriction_is_effective {
        eprintln!(
            "skipping assertion: 0o000 did not block metadata reads (likely running as root)"
        );
        return;
    }
    assert!(
        matches!(result, Err(CoreError::Io { .. })),
        "expected Io for a genuine read failure, got {result:?}"
    );
}

// ── check_precondition: Force ───────────────────────────────────────────────

#[test]
fn force_always_succeeds_regardless_of_target_state() {
    let dir = temp_dir("force");
    let missing = dir.join("missing.txt");
    let _ = fs::remove_file(&missing);
    assert!(check_precondition(&missing, &TargetPrecondition::Force).is_ok());

    let existing = dir.join("existing.txt");
    fs::write(&existing, "anything\n").unwrap();
    assert!(check_precondition(&existing, &TargetPrecondition::Force).is_ok());
}

// ── persist_noclobber ────────────────────────────────────────────────────────

#[test]
fn persist_noclobber_creates_a_missing_target_and_its_parent_directories() {
    let dir = temp_dir("noclobber-create");
    let path = dir.join("new").join("nested").join("out.txt");
    let _ = fs::remove_dir_all(dir.join("new"));

    persist_noclobber(&path, b"created content\n").expect("must succeed for a missing target");

    assert_eq!(fs::read(&path).unwrap(), b"created content\n");
}

#[test]
fn persist_noclobber_fails_and_leaves_an_existing_target_untouched() {
    let dir = temp_dir("noclobber-existing");
    let path = dir.join("already-there.txt");
    fs::write(&path, "original bytes\n").unwrap();

    let result = persist_noclobber(&path, b"attempted overwrite\n");

    assert!(matches!(result, Err(CoreError::Conflict { .. })));
    assert_eq!(
        fs::read(&path).unwrap(),
        b"original bytes\n",
        "a failed no-clobber commit must never touch the existing target"
    );
}

#[test]
fn persist_noclobber_race_via_before_commit_hook_reports_conflict_and_preserves_the_winner() {
    // RFC-077's named no-sleep race: a competing writer creates the target
    // after our temp file is fully written but before we commit — exercised
    // deterministically via the test-only hook, not a sleep.
    let dir = temp_dir("noclobber-race");
    let path = dir.join("raced.txt");
    let _ = fs::remove_file(&path);

    let result = persist_noclobber_with_hook(&path, b"our content\n", || {
        fs::write(&path, b"the other writer's content\n").unwrap();
    });

    assert!(matches!(result, Err(CoreError::Conflict { .. })));
    assert_eq!(
        fs::read(&path).unwrap(),
        b"the other writer's content\n",
        "the competing writer's bytes must survive untouched — we must not clobber them"
    );
}

#[test]
fn persist_noclobber_leaves_no_temp_file_behind_on_conflict() {
    let dir = temp_dir("noclobber-cleanup");
    let path = dir.join("target.txt");
    fs::write(&path, "existing\n").unwrap();
    let before: Vec<_> = fs::read_dir(&dir).unwrap().collect();

    let _ = persist_noclobber(&path, b"attempted\n");

    let after: Vec<_> = fs::read_dir(&dir).unwrap().collect();
    assert_eq!(
        before.len(),
        after.len(),
        "a failed commit must not leak its temp file into the target directory"
    );
}

#[cfg(unix)]
#[test]
fn persist_noclobber_output_is_not_left_with_tempfiles_narrow_default_permissions() {
    // NamedTempFile defaults to 0600; a mergetool output file that ends up
    // more restrictive than an ordinary save would be a quiet surprise for
    // the next tool that reads it. Found while checking runtime evidence for
    // RFC-077 patch 4b — `ls -la` on a freshly-created merged output showed
    // 0600 where a normally-saved file showed 0644.
    //
    // Asserted against a same-directory `fs::write`-created file's own mode
    // rather than a hardcoded `0o644` (F38, review 051 §3.3): the property
    // is "the permissions the normal save path would have produced," which
    // is umask-derived and only equals `0o644` under `umask 022`. Comparing
    // against a reference file created the same way in the same process
    // makes the test correct under any umask.
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("noclobber-permissions");
    let path = dir.join("new-output.txt");
    let _ = fs::remove_file(&path);
    let reference_path = dir.join("reference-output.txt");
    fs::write(&reference_path, b"reference\n").unwrap();
    let expected_mode = fs::metadata(&reference_path).unwrap().permissions().mode() & 0o777;

    persist_noclobber(&path, b"content\n").unwrap();

    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode, expected_mode,
        "expected {expected_mode:o} (a plain fs::write's mode in the same directory), \
         got {mode:o} — a no-clobber-created file must not be more restrictive \
         than an ordinary save"
    );
}
