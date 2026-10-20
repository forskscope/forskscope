//! `forskscope_compare::dir` owns the read half (listing, recursive diff,
//! digests) — see its own `dir_tests.rs`. This crate owns the write half:
//! `copy_file`, tested here (RFC-093 §1 split).

use std::fs;
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-dir-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn copy_file_creates_backup_and_overwrites() {
    let dir = temp_dir("copy");
    let src = dir.join("src.txt");
    let dst = dir.join("dst.txt");
    fs::write(&src, "new content").unwrap();
    fs::write(&dst, "old content").unwrap();

    let outcome = crate::dir::copy_file(&src, &dst, crate::save::BackupPolicy::SiblingBak).unwrap();
    assert_eq!(fs::read_to_string(&dst).unwrap(), "new content");
    let bak = outcome.backup_path.expect("backup created");
    assert_eq!(fs::read_to_string(&bak).unwrap(), "old content");
}

#[test]
fn copy_file_creates_destination_parent_dirs() {
    let dir = temp_dir("copy-nested");
    let src = dir.join("src.txt");
    let dst = dir.join("deep").join("nested").join("dst.txt");
    fs::write(&src, "hello").unwrap();

    crate::dir::copy_file(&src, &dst, crate::save::BackupPolicy::None).unwrap();
    assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
}
