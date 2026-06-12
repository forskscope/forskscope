use std::fs;
use std::path::PathBuf;

use crate::document::FileFingerprint;
use crate::error::CoreError;
use crate::save::{BackupPolicy, SaveRequest, save_text};

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
        expected_fingerprint: None,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "merged\nresult\n");
    assert_eq!(outcome.written_bytes, 14);
    assert!(!outcome.encoding_fallback_to_utf8);
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
        expected_fingerprint: Some(fingerprint),
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
        expected_fingerprint: Some(stale),
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
        target:           target.clone(),
        content:          "nested\n".to_string(),
        encoding_label:   "UTF-8".to_string(),
        expected_fingerprint: None,
        backup:           crate::save::BackupPolicy::None,
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
        target:           target.clone(),
        content:          "overwritten\n".to_string(),
        encoding_label:   "UTF-8".to_string(),
        expected_fingerprint: None,
        backup:           crate::save::BackupPolicy::None,
    };
    crate::save::save_text(&req).unwrap();
    let bak = dir.join("file.txt.bak");
    assert!(!bak.exists(), "no backup should be created when policy is None");
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
        target:           target.clone(),
        content:          "v3-ours\n".to_string(),
        encoding_label:   "UTF-8".to_string(),
        expected_fingerprint: Some(fp),
        backup:           crate::save::BackupPolicy::None,
    };
    let err = crate::save::save_text(&req).unwrap_err();
    // The error should be a Conflict variant.
    assert!(matches!(err, crate::CoreError::Conflict { .. }),
        "should report Conflict when file was externally changed");
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
        expected_fingerprint: None,
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
        expected_fingerprint: None,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    assert!(outcome.backup_path.is_none(),
        "backup_path must be None when BackupPolicy::None is used");
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
        expected_fingerprint: None,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    // The new fingerprint must differ from the original.
    assert_ne!(outcome.new_fingerprint.len, original_fp.len,
        "new_fingerprint must reflect the updated file size");
    // Re-capturing should give the same fingerprint as the outcome.
    let recaptured = FileFingerprint::capture(&target, None).unwrap();
    assert_eq!(outcome.new_fingerprint.len, recaptured.len,
        "new_fingerprint must match a fresh capture after write");
}

#[test]
fn encoding_fallback_to_utf8_is_true_for_unknown_encoding() {
    let dir = temp_dir("enc-fallback");
    let target = dir.join("file.txt");
    let request = SaveRequest {
        target: target.clone(),
        content: "hello world\n".into(),
        encoding_label: "DEFINITELY-NOT-A-REAL-ENCODING-LABEL".into(),
        expected_fingerprint: None,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    // Unknown encoding → UTF-8 fallback used → flag is true.
    assert!(outcome.encoding_fallback_to_utf8,
        "encoding_fallback_to_utf8 must be true when the label is unknown");
    // Content must still have been written (as UTF-8).
    assert_eq!(fs::read_to_string(&target).unwrap(), "hello world\n");
}

#[test]
fn written_bytes_matches_content_length() {
    let dir = temp_dir("written-bytes");
    let target = dir.join("file.txt");
    let content = "line1\nline2\nline3\n";  // 18 bytes
    let request = SaveRequest {
        target: target.clone(),
        content: content.into(),
        encoding_label: "UTF-8".into(),
        expected_fingerprint: None,
        backup: BackupPolicy::None,
    };
    let outcome = save_text(&request).unwrap();
    assert_eq!(outcome.written_bytes, 18,
        "written_bytes must equal the byte length of the content");
}
