use std::fs;
use std::path::PathBuf;

use crate::document::FileFingerprint;
use crate::encoding::BomPresence;
use crate::error::CoreError;
#[cfg(unix)]
use crate::save::atomic_replace;
use crate::save::{BackupPolicy, SaveRequest, TargetPrecondition, save_text};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-save-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn save_writes_content_and_returns_fingerprint() {
    let dir = temp_dir("write");
    let target = dir.join("out.txt");
    let request = SaveRequest {
        target: target.clone(),
        content: "merged\nresult\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "merged\nresult\n");
    assert_eq!(outcome.written_bytes, 14);
    assert!(!outcome.encoding_fallback_to_utf8);
}

// ── RFC-083 §2: BOM round-trip ────────────────────────────────────────────────

/// Handoff 023 §6 test 3: loaded-with-BOM saves with a BOM. Falsify by
/// dropping the `BomPolicy::Preserve` prepend in `save_text` — see the
/// review request for the real failing output.
#[test]
fn a_document_loaded_with_a_bom_is_saved_with_a_bom() {
    let dir = temp_dir("bom-round-trip-in");
    let target = dir.join("out.txt");
    let request = SaveRequest {
        target: target.clone(),
        content: "hi\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Utf8,
        precondition: TargetPrecondition::Force,
        backup: BackupPolicy::None,
    };
    save_text(&request).unwrap();
    let bytes = fs::read(&target).unwrap();
    assert_eq!(bytes, [0xEFu8, 0xBB, 0xBF, b'h', b'i', b'\n']);
}

#[test]
fn a_document_loaded_without_a_bom_is_saved_without_a_bom() {
    let dir = temp_dir("bom-round-trip-out");
    let target = dir.join("out.txt");
    let request = SaveRequest {
        target: target.clone(),
        content: "hi\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: BackupPolicy::None,
    };
    save_text(&request).unwrap();
    let bytes = fs::read(&target).unwrap();
    assert_eq!(bytes, b"hi\n");
}

#[test]
fn save_creates_sibling_backup_when_requested() {
    let dir = temp_dir("backup");
    let target = dir.join("file.txt");
    fs::write(&target, "original\n").unwrap();
    let fingerprint = FileFingerprint::capture(&target, None).unwrap();
    let request = SaveRequest {
        target: target.clone(),
        content: "updated\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustMatch(fingerprint),
        backup: BackupPolicy::SiblingBak,
    };
    let outcome = save_text(&request).unwrap();
    let bak = outcome.backup_path.expect("backup path");
    assert_eq!(fs::read_to_string(&bak).unwrap(), "original\n");
    assert_eq!(fs::read_to_string(&target).unwrap(), "updated\n");
}

#[test]
fn external_modification_is_detected_as_conflict() {
    let dir = temp_dir("conflict");
    let target = dir.join("file.txt");
    fs::write(&target, "v1\n").unwrap();
    let stale = FileFingerprint::capture(&target, None).unwrap();

    // Simulate an external edit changing length after load.
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(&target, "v2-external-edit\n").unwrap();

    let request = SaveRequest {
        target: target.clone(),
        content: "our-merge\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustMatch(stale),
        backup: BackupPolicy::None,
    };
    let err = save_text(&request).unwrap_err();
    assert!(matches!(err, CoreError::Conflict { .. }));
    // The external content must be preserved on conflict.
    assert_eq!(fs::read_to_string(&target).unwrap(), "v2-external-edit\n");
}

// ── New tests for v0.32.0 ─────────────────────────────────────────────────────

#[test]
fn save_creates_nested_parent_dirs() {
    let dir = temp_dir("save-nested");
    let target = dir.join("a").join("b").join("output.txt");
    let req = crate::save::SaveRequest {
        target: target.clone(),
        content: "nested\n".to_string(),
        encoding_label: "UTF-8".to_string(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: crate::save::BackupPolicy::None,
    };
    crate::save::save_text(&req).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "nested\n");
}

#[test]
fn save_without_backup_does_not_create_bak_file() {
    let dir = temp_dir("save-nobak");
    let target = dir.join("file.txt");
    std::fs::write(&target, "original").unwrap();
    let req = crate::save::SaveRequest {
        target: target.clone(),
        content: "overwritten\n".to_string(),
        encoding_label: "UTF-8".to_string(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: crate::save::BackupPolicy::None,
    };
    crate::save::save_text(&req).unwrap();
    let bak = dir.join("file.txt.bak");
    assert!(
        !bak.exists(),
        "no backup should be created when policy is None"
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "overwritten\n");
}

#[test]
fn conflict_error_contains_path_info() {
    let dir = temp_dir("conflict-path");
    let target = dir.join("file.txt");
    std::fs::write(&target, "v1").unwrap();

    // Capture a fingerprint before writing.
    let fp = crate::document::FileFingerprint::capture(&target, None).unwrap();

    // Modify the file to simulate external change.
    std::fs::write(&target, "v2-external").unwrap();

    let req = crate::save::SaveRequest {
        target: target.clone(),
        content: "v3-ours\n".to_string(),
        encoding_label: "UTF-8".to_string(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustMatch(fp),
        backup: crate::save::BackupPolicy::None,
    };
    let err = crate::save::save_text(&req).unwrap_err();
    // The error should be a Conflict variant.
    assert!(
        matches!(err, crate::CoreError::Conflict { .. }),
        "should report Conflict when file was externally changed"
    );
}

#[test]
fn save_with_none_fingerprint_always_succeeds() {
    let dir = temp_dir("save-any");
    let target = dir.join("f.txt");
    std::fs::write(&target, "old").unwrap();
    let req = crate::save::SaveRequest {
        target: target.clone(),
        content: "new\n".to_string(),
        encoding_label: "UTF-8".to_string(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: crate::save::BackupPolicy::None,
    };
    // No expected fingerprint → never a conflict.
    crate::save::save_text(&req).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new\n");
}

// ── SaveOutcome field coverage ────────────────────────────────────────────────

#[test]
fn backup_path_is_none_when_policy_is_none() {
    let dir = temp_dir("bak-none");
    let target = dir.join("file.txt");
    let request = SaveRequest {
        target: target.clone(),
        content: "data\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    assert!(
        outcome.backup_path.is_none(),
        "backup_path must be None when BackupPolicy::None is used"
    );
}

#[test]
fn new_fingerprint_reflects_written_content() {
    let dir = temp_dir("new-fp");
    let target = dir.join("file.txt");
    // Write an initial file, capture its fingerprint, then overwrite.
    fs::write(&target, "original\n").unwrap();
    let original_fp = FileFingerprint::capture(&target, None).unwrap();

    let request = SaveRequest {
        target: target.clone(),
        content: "updated content here\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    // The new fingerprint must differ from the original.
    assert_ne!(
        outcome.new_fingerprint.len, original_fp.len,
        "new_fingerprint must reflect the updated file size"
    );
    // Re-capturing should give the same fingerprint as the outcome.
    let recaptured = FileFingerprint::capture(&target, None).unwrap();
    assert_eq!(
        outcome.new_fingerprint.len, recaptured.len,
        "new_fingerprint must match a fresh capture after write"
    );
}

#[test]
fn encoding_fallback_to_utf8_is_true_for_unknown_encoding() {
    let dir = temp_dir("enc-fallback");
    let target = dir.join("file.txt");
    let request = SaveRequest {
        target: target.clone(),
        content: "hello world\n".into(),
        encoding_label: "DEFINITELY-NOT-A-REAL-ENCODING-LABEL".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    // Unknown encoding → UTF-8 fallback used → flag is true.
    assert!(
        outcome.encoding_fallback_to_utf8,
        "encoding_fallback_to_utf8 must be true when the label is unknown"
    );
    // Content must still have been written (as UTF-8).
    assert_eq!(fs::read_to_string(&target).unwrap(), "hello world\n");
}

#[test]
fn written_bytes_matches_content_length() {
    let dir = temp_dir("written-bytes");
    let target = dir.join("file.txt");
    let content = "line1\nline2\nline3\n"; // 18 bytes
    let request = SaveRequest {
        target: target.clone(),
        content: content.into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    assert_eq!(
        outcome.written_bytes, 18,
        "written_bytes must equal the byte length of the content"
    );
}

// ── RFC-077: save_text routed through TargetPrecondition::MustBeAbsent ────

#[test]
fn must_be_absent_precondition_creates_a_missing_target_through_save_text() {
    let dir = temp_dir("precondition-must-be-absent-create");
    let target = dir.join("new-mergetool-output.txt");
    let _ = fs::remove_file(&target);
    let request = SaveRequest {
        target: target.clone(),
        content: "merged result\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustBeAbsent,
        backup: BackupPolicy::SiblingBak,
    };

    let outcome = save_text(&request).unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "merged result\n");
    assert!(outcome.backup_path.is_none(), "nothing existed to back up");
}

#[test]
fn must_be_absent_precondition_conflicts_and_leaves_an_existing_target_untouched() {
    let dir = temp_dir("precondition-must-be-absent-conflict");
    let target = dir.join("appeared-externally.txt");
    fs::write(&target, "someone else's content\n").unwrap();
    let request = SaveRequest {
        target: target.clone(),
        content: "our merge result\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustBeAbsent,
        backup: BackupPolicy::SiblingBak,
    };

    let err = save_text(&request).unwrap_err();

    assert!(matches!(err, CoreError::Conflict { .. }));
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "someone else's content\n",
        "a MustBeAbsent conflict must never touch the existing target, and must not \
         silently replace it — RFC-077's central no-clobber guarantee"
    );
    let bak = dir.join("appeared-externally.txt.bak");
    assert!(
        !bak.exists(),
        "no backup should be attempted for a save that never wrote anything"
    );
}

// ── F89/RFC-082 §D5: atomic_replace uses an unpredictable temp file ────────

#[cfg(unix)]
#[test]
fn atomic_replace_does_not_follow_a_pre_created_symlink_at_the_old_predictable_temp_path() {
    // CWE-59/CWE-378, reproduced by the architect against the old
    // `fs::write(temp_path_for(target), …)` implementation: pre-creating
    // `.doc.txt.fsk-tmp` as a symlink to an unrelated file caused that
    // file to be silently overwritten with the user's new content, and
    // left the user's own document replaced by a symlink
    // (`doc.txt -> victim.txt`). No root-skip: this test creates its own
    // symlink, and root changes nothing about link-following behavior.
    use std::os::unix::fs::symlink;

    let dir = temp_dir("atomic-replace-symlink-attack");
    let doc = dir.join("doc.txt");
    let victim = dir.join("victim.txt");
    fs::write(&doc, "original document content\n").unwrap();
    fs::write(&victim, "victim's own content\n").unwrap();

    let predictable_temp = dir.join(".doc.txt.fsk-tmp");
    symlink(&victim, &predictable_temp).unwrap();

    atomic_replace(&doc, b"user's new content\n").unwrap();

    assert_eq!(
        fs::read_to_string(&victim).unwrap(),
        "victim's own content\n",
        "the unrelated file must be untouched — atomic_replace must never \
         write through a pre-existing symlink at any predictable path"
    );
    assert!(
        !fs::symlink_metadata(&doc).unwrap().file_type().is_symlink(),
        "doc.txt must remain a regular file, never replaced by a symlink"
    );
    assert_eq!(
        fs::read_to_string(&doc).unwrap(),
        "user's new content\n",
        "the actual target must receive the new content"
    );
    assert!(
        fs::symlink_metadata(&predictable_temp)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false),
        "test setup sanity check: the pre-created symlink must still be \
         exactly what it was — never touched, because atomic_replace's \
         real (randomly-named) temp file never has this name"
    );
}

#[cfg(unix)]
#[test]
fn atomic_replace_output_is_not_left_with_tempfiles_narrow_default_permissions() {
    // Same property persist_noclobber_with_hook's own permissions test
    // protects (see its comment in save_target_tests.rs) — NamedTempFile
    // defaults to 0600, but atomic_replace's output must have the
    // permissions an ordinary fs::write would have produced under the
    // process umask, not something more restrictive. Compared against a
    // same-directory reference file created the same way in the same
    // process, so this is correct under any umask — not a hardcoded mode
    // (F38, review 051 §3.3). Also exercises "an ordinary overwrite still
    // works" (handoff 016 §5.2): the target already exists and is
    // genuinely replaced.
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("atomic-replace-permissions");
    let target = dir.join("out.txt");
    fs::write(&target, "original\n").unwrap();
    let reference_path = dir.join("reference-output.txt");
    fs::write(&reference_path, b"reference\n").unwrap();
    let expected_mode = fs::metadata(&reference_path).unwrap().permissions().mode() & 0o777;

    atomic_replace(&target, b"new content\n").unwrap();

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "new content\n",
        "the overwrite must actually take effect"
    );
    let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode, expected_mode,
        "expected {expected_mode:o} (a plain fs::write's mode in the same directory), \
         got {mode:o} — atomic_replace's output must not be more restrictive \
         than an ordinary save"
    );
}

// ── F87/RFC-082 §D4: a save that cannot represent its content must not
// happen — refused before the backup step, nothing on disk touched ────────

#[test]
fn a_lossy_save_writes_nothing_and_never_touches_the_backup() {
    let dir = temp_dir("encode-lossy-refusal");
    let target = dir.join("doc.txt");
    fs::write(&target, "original content\n").unwrap();
    let bak = dir.join("doc.txt.bak");
    fs::write(&bak, "a prior backup that must survive untouched\n").unwrap();
    let fp = FileFingerprint::capture(&target, None).unwrap();

    let request = SaveRequest {
        target: target.clone(),
        content: "hi 😀\n".into(),
        encoding_label: "shift_jis".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustMatch(fp),
        backup: BackupPolicy::SiblingBak,
    };
    let err = save_text(&request).unwrap_err();

    assert!(
        matches!(err, CoreError::Encode { .. }),
        "expected CoreError::Encode, got {err:?}"
    );
    assert_eq!(
        fs::read(&target).unwrap(),
        b"original content\n",
        "§3's ordering requirement: nothing on disk may be touched when a \
         save is refused for this reason"
    );
    assert_eq!(
        fs::read(&bak).unwrap(),
        b"a prior backup that must survive untouched\n",
        "the backup step must never run for a refused save — a later \
         refusal would already have destroyed the user's prior backup \
         for a save that never happens"
    );
}

#[test]
fn a_lossy_save_names_the_offending_character() {
    let dir = temp_dir("encode-lossy-names-character");
    let target = dir.join("doc.txt");
    fs::write(&target, "original\n").unwrap();
    let fp = FileFingerprint::capture(&target, None).unwrap();

    let request = SaveRequest {
        target: target.clone(),
        content: "hi 😀\n".into(),
        encoding_label: "shift_jis".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustMatch(fp),
        backup: BackupPolicy::None,
    };
    match save_text(&request).unwrap_err() {
        CoreError::Encode {
            sample_characters,
            encoding_label,
            ..
        } => {
            assert_eq!(
                sample_characters,
                vec!['😀'],
                "the actual character must be named, not merely \"lossy\""
            );
            assert_eq!(encoding_label, "shift_jis");
        }
        other => panic!("expected CoreError::Encode, got {other:?}"),
    }
}

// ── F121: three write-path residuals ─────────────────────────────────────────

#[cfg(unix)]
fn force_request(target: &std::path::Path, content: &str, backup: BackupPolicy) -> SaveRequest {
    SaveRequest {
        target: target.to_path_buf(),
        content: content.into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::Force,
        backup,
    }
}

/// F121 A — the attack-regression shape F89 established: a predictable
/// sibling path pre-created as a symlink to an unrelated file. Before the fix
/// `fs::copy(target, "<name>.bak")` wrote **through** the link, and
/// `victim.txt` held `doc.txt`'s previous content. Now the link itself is
/// replaced by a real backup and the victim is untouched.
#[cfg(unix)]
#[test]
fn backup_does_not_write_through_a_pre_created_symlink_at_the_bak_path() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("backup-symlink-attack");
    let doc = dir.join("doc.txt");
    let victim = dir.join("victim.txt");
    let bak = dir.join("doc.txt.bak");
    fs::write(&doc, "original document content\n").unwrap();
    fs::write(&victim, "victim's own content\n").unwrap();
    symlink(&victim, &bak).unwrap();

    save_text(&force_request(
        &doc,
        "new content\n",
        BackupPolicy::SiblingBak,
    ))
    .unwrap();

    assert_eq!(
        fs::read_to_string(&victim).unwrap(),
        "victim's own content\n",
        "the unrelated file must be untouched — the backup must never be written through a link"
    );
    assert!(
        !fs::symlink_metadata(&bak).unwrap().file_type().is_symlink(),
        "the link must have been replaced by a real backup file"
    );
    assert_eq!(
        fs::read_to_string(&bak).unwrap(),
        "original document content\n",
        "the backup must hold the previous content"
    );
    assert_eq!(fs::read_to_string(&doc).unwrap(), "new content\n");
}

/// The documented behaviour survives the fix: a save still clobbers a
/// previous ordinary backup, and the backup keeps the source's mode (as
/// `fs::copy` did), so a `0600` file's backup is not wider than the file.
#[cfg(unix)]
#[test]
fn backup_still_replaces_an_existing_backup_and_keeps_the_sources_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("backup-replaces-and-keeps-mode");
    let doc = dir.join("doc.txt");
    let bak = dir.join("doc.txt.bak");
    fs::write(&doc, "current\n").unwrap();
    fs::set_permissions(&doc, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&bak, "an older backup\n").unwrap();

    save_text(&force_request(&doc, "next\n", BackupPolicy::SiblingBak)).unwrap();

    assert_eq!(fs::read_to_string(&bak).unwrap(), "current\n");
    let mode = fs::metadata(&bak).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the backup must not be wider than its source");
}

/// F121 B — a `0600` file was `0644` after a save. Falsify by removing the
/// `carry_mode` call from `atomic_replace_checked`.
#[cfg(unix)]
#[test]
fn saving_an_existing_file_keeps_its_permission_bits() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("save-keeps-mode");
    for mode in [0o600u32, 0o640, 0o755] {
        let doc = dir.join(format!("doc-{mode:o}.txt"));
        fs::write(&doc, "before\n").unwrap();
        fs::set_permissions(&doc, fs::Permissions::from_mode(mode)).unwrap();

        save_text(&force_request(&doc, "after\n", BackupPolicy::None)).unwrap();

        assert_eq!(fs::read_to_string(&doc).unwrap(), "after\n");
        let got = fs::metadata(&doc).unwrap().permissions().mode() & 0o7777;
        assert_eq!(
            got, mode,
            "a {mode:o} file must still be {mode:o} after a save"
        );
    }
}

/// The other half of B: a **new** file still gets the umask default, as F38
/// decided (compared against a same-directory reference, so any umask works).
#[cfg(unix)]
#[test]
fn saving_a_new_file_still_gets_the_umask_default() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("save-new-file-umask-default");
    let reference = dir.join("reference.txt");
    fs::write(&reference, "r\n").unwrap();
    let expected = fs::metadata(&reference).unwrap().permissions().mode() & 0o777;

    let fresh = dir.join("fresh.txt");
    save_text(&force_request(&fresh, "new\n", BackupPolicy::None)).unwrap();

    let got = fs::metadata(&fresh).unwrap().permissions().mode() & 0o777;
    assert_eq!(got, expected, "a new file keeps the plain-write default");
}

/// F121 C — the precondition is re-checked inside the commit. `MustMatch` is
/// satisfied at the start of `save_text`; the hook then changes the target
/// (after the first check and the backup, before the commit), and only the
/// re-check can catch it. Falsify by replacing the closure passed to
/// `atomic_replace_checked` with `|| Ok(())`: the external content is then
/// overwritten and this fails.
#[test]
fn a_change_after_the_first_check_is_caught_by_the_recheck_before_the_rename() {
    let dir = temp_dir("recheck-before-rename");
    let target = dir.join("file.txt");
    fs::write(&target, "v1\n").unwrap();
    let fingerprint = FileFingerprint::capture(&target, None).unwrap();

    let request = SaveRequest {
        target: target.clone(),
        content: "our save\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustMatch(fingerprint),
        backup: BackupPolicy::None,
    };
    let racing_target = target.clone();
    let err = crate::save::save_text_with_commit_hook(&request, move || {
        fs::write(
            &racing_target,
            "written by another process, longer than v1\n",
        )
        .unwrap();
    })
    .unwrap_err();

    assert!(matches!(err, CoreError::Conflict { .. }), "got {err:?}");
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "written by another process, longer than v1\n",
        "the other process's content must survive the refused save"
    );
    let leftovers: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        leftovers.len(),
        1,
        "no temp file may be left behind: {leftovers:?}"
    );
}

// ── F121 (handoff 050): a refused save leaves the backup alone ───────────────

/// F87: "nothing on disk is touched by a refusal". Moving the precondition
/// check to just before the rename put the backup *before* it, so a save the
/// re-check refused had already replaced `<name>.bak` with the file's current
/// content — destroying the previous backup for a save that never happened.
/// Falsified against the code as it stood at `43b3e5a`: `.bak` was replaced.
#[test]
fn a_save_refused_by_the_recheck_leaves_the_previous_backup_untouched() {
    let dir = temp_dir("refused-save-keeps-backup");
    let target = dir.join("file.txt");
    let bak = dir.join("file.txt.bak");
    fs::write(&target, "v1\n").unwrap();
    fs::write(&bak, "the previous backup, from an earlier save\n").unwrap();
    let fingerprint = FileFingerprint::capture(&target, None).unwrap();

    let request = SaveRequest {
        target: target.clone(),
        content: "our save\n".into(),
        encoding_label: "UTF-8".into(),
        bom: BomPresence::Absent,
        precondition: TargetPrecondition::MustMatch(fingerprint),
        backup: BackupPolicy::SiblingBak,
    };
    let racing = target.clone();
    let err = crate::save::save_text_with_commit_hook(&request, move || {
        fs::write(&racing, "written by another process, longer than v1\n").unwrap();
    })
    .unwrap_err();

    assert!(matches!(err, CoreError::Conflict { .. }), "got {err:?}");
    assert_eq!(
        fs::read(&bak).unwrap(),
        b"the previous backup, from an earlier save\n",
        "a refused save must leave <name>.bak byte for byte unchanged"
    );
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "written by another process, longer than v1\n"
    );
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["file.txt", "file.txt.bak"],
        "no content temp and no staged backup may be left behind"
    );
}
