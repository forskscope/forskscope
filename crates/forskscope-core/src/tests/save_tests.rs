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
