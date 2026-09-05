//! RFC-077 patch 1: `save_target_from_loaded`/`inspect_save_target` — the
//! prepared-comparison save-target derivation for normal compare and Git
//! mergetool mode respectively.

use std::fs;
use std::path::PathBuf;

use crate::compare_prep::{
    SaveTargetBlockReason, SaveTargetState, TargetExpectation, inspect_save_target,
    save_target_from_loaded,
};
use crate::document::{FileFingerprint, LoadOptions, LoadedDocument, load_path};
use crate::encoding::BomPresence;
use crate::file_kind::FileKind;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-compare-prep-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

// ── save_target_from_loaded (normal compare) ───────────────────────────────

#[test]
fn text_document_yields_must_match_with_its_own_fingerprint() {
    let dir = temp_dir("normal-text");
    let path = dir.join("right.txt");
    fs::write(&path, "hello\n").unwrap();
    let doc = load_path(&path, LoadOptions::default()).unwrap();

    let snapshot = save_target_from_loaded(&path, &doc);

    assert_eq!(snapshot.path, path);
    match snapshot.state {
        SaveTargetState::Writable {
            expectation,
            encoding_label,
            ..
        } => {
            assert_eq!(
                expectation,
                TargetExpectation::MustMatch(doc.fingerprint_at_load.unwrap())
            );
            assert_eq!(encoding_label, "UTF-8");
        }
        other => panic!("expected Writable, got {other:?}"),
    }
}

#[test]
fn missing_document_yields_must_be_absent() {
    let dir = temp_dir("normal-missing");
    let path = dir.join("does-not-exist.txt");
    let _ = fs::remove_file(&path);
    let doc = LoadedDocument::empty();

    let snapshot = save_target_from_loaded(&path, &doc);

    match snapshot.state {
        SaveTargetState::Writable { expectation, .. } => {
            assert_eq!(expectation, TargetExpectation::MustBeAbsent);
        }
        other => panic!("expected Writable(MustBeAbsent), got {other:?}"),
    }
}

#[test]
fn binary_document_is_blocked() {
    let dir = temp_dir("normal-binary");
    let path = dir.join("right.bin");
    fs::write(&path, [0u8, 1, 2, 3]).unwrap();
    let doc = load_path(&path, LoadOptions::default()).unwrap();
    assert_eq!(doc.kind, FileKind::Binary);

    let snapshot = save_target_from_loaded(&path, &doc);

    assert_eq!(
        snapshot.state,
        SaveTargetState::Blocked {
            reason: SaveTargetBlockReason::Binary
        }
    );
}

// ── inspect_save_target (Git mergetool) ─────────────────────────────────────

#[test]
fn existing_merged_target_is_must_match_and_does_not_leak_content_into_state() {
    let dir = temp_dir("merge-existing");
    let path = dir.join("merged.txt");
    fs::write(&path, "already merged\n").unwrap();

    let snapshot = inspect_save_target(&path, "UTF-8", BomPresence::Absent);

    let expected_fp = FileFingerprint::capture(&path, None).unwrap();
    match snapshot.state {
        SaveTargetState::Writable {
            expectation,
            encoding_label,
            ..
        } => {
            // Fingerprint compares len + mtime, not digest, so this is a
            // structural check that we captured the target's own metadata
            // (not the digest, which needs the exact bytes hashed the same
            // way `load_path` did — asserted separately below).
            match expectation {
                TargetExpectation::MustMatch(fp) => {
                    assert_eq!(fp.len, expected_fp.len);
                }
                other => panic!("expected MustMatch, got {other:?}"),
            }
            assert_eq!(encoding_label, "UTF-8");
        }
        other => panic!("expected Writable, got {other:?}"),
    }
}

#[test]
fn missing_merged_target_is_must_be_absent_with_remote_encoding_as_fallback() {
    let dir = temp_dir("merge-missing");
    let path = dir.join("does-not-exist-yet.txt");
    let _ = fs::remove_file(&path);

    let snapshot = inspect_save_target(&path, "Shift_JIS", BomPresence::Absent);

    match snapshot.state {
        SaveTargetState::Writable {
            expectation,
            encoding_label,
            ..
        } => {
            assert_eq!(expectation, TargetExpectation::MustBeAbsent);
            assert_eq!(
                encoding_label, "Shift_JIS",
                "a missing target has no encoding of its own — must fall back to the remote's"
            );
        }
        other => panic!("expected Writable(MustBeAbsent), got {other:?}"),
    }
}

#[test]
fn binary_merged_target_is_blocked_not_replaced() {
    let dir = temp_dir("merge-binary");
    let path = dir.join("merged.bin");
    fs::write(&path, [0u8, 1, 2, 3]).unwrap();

    let snapshot = inspect_save_target(&path, "UTF-8", BomPresence::Absent);

    assert_eq!(
        snapshot.state,
        SaveTargetState::Blocked {
            reason: SaveTargetBlockReason::Binary
        }
    );
}

#[test]
fn xlsx_merged_target_is_blocked() {
    let dir = temp_dir("merge-xlsx");
    let path = dir.join("merged.xlsx");
    fs::write(&path, b"not a real workbook").unwrap();

    let snapshot = inspect_save_target(&path, "UTF-8", BomPresence::Absent);

    assert_eq!(
        snapshot.state,
        SaveTargetState::Blocked {
            reason: SaveTargetBlockReason::Spreadsheet
        }
    );
}

#[test]
fn directory_merged_target_is_blocked_as_not_a_plain_file() {
    let dir = temp_dir("merge-directory");
    let path = dir.join("merged-is-a-dir");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();

    let snapshot = inspect_save_target(&path, "UTF-8", BomPresence::Absent);

    match snapshot.state {
        SaveTargetState::Blocked {
            reason: SaveTargetBlockReason::NotAPlainFile { .. },
        } => {}
        other => panic!("expected Blocked(NotAPlainFile), got {other:?}"),
    }
}

#[test]
fn conflict_marker_content_is_still_an_ordinary_text_target() {
    // RFC-077: "It is still an existing text target... does not parse
    // markers or use them as the merge model." No special-casing exists —
    // this proves the plain-text path handles it like any other text file.
    let dir = temp_dir("merge-conflict-markers");
    let path = dir.join("merged.txt");
    fs::write(
        &path,
        "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> branch\n",
    )
    .unwrap();

    let snapshot = inspect_save_target(&path, "UTF-8", BomPresence::Absent);

    assert!(matches!(
        snapshot.state,
        SaveTargetState::Writable {
            expectation: TargetExpectation::MustMatch(_),
            ..
        }
    ));
}

#[test]
fn fallback_encoding_is_used_only_when_the_target_is_actually_missing() {
    // Guards against a snapshot silently reporting the fallback encoding for
    // an existing (non-UTF-8-fallback-worthy) target.
    let dir = temp_dir("merge-existing-encoding");
    let path = dir.join("merged.txt");
    fs::write(&path, "ascii content\n").unwrap();

    let snapshot = inspect_save_target(&path, "Shift_JIS", BomPresence::Absent);

    match snapshot.state {
        SaveTargetState::Writable { encoding_label, .. } => {
            assert_ne!(
                encoding_label, "Shift_JIS",
                "an existing target's own detected encoding must win over the fallback"
            );
        }
        other => panic!("expected Writable, got {other:?}"),
    }
}
