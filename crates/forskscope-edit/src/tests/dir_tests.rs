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

/// F235: "Paths for actions are never altered. Opening, comparing,
/// copying and saving use the real name." - a name carrying an RLO
/// (the classic spoofing shape) is still just bytes to the filesystem;
/// this proves the whole chain (load, diff, copy) never substitutes,
/// sanitizes or otherwise rewrites it along the way. Falsified by
/// temporarily inserting a `.replace('\u{202E}', "")` before any one of
/// the three steps below: each time, either the load fails (wrong path),
/// the diff is empty (wrong content loaded), or the final `fs::` checks
/// fail (the raw character is gone from the real on-disk name) -
/// confirmed live, then reverted.
#[test]
fn opening_comparing_and_copying_a_bidi_control_named_file_use_the_real_name_throughout() {
    let dir = temp_dir("bidi-name-roundtrip");
    let src = dir.join("invoice\u{202E}fdp.exe");
    let dst = dir.join("copied-invoice\u{202E}fdp.exe");
    fs::write(&src, "left content\n").unwrap();

    // Open + compare: both load with the real, un-rewritten path, and
    // the diff reflects the real content (not some name-derived stand-in).
    let loaded = forskscope_compare::load_path(&src, forskscope_compare::LoadOptions::default())
        .expect("load must use the real path, control character and all");
    let diff = forskscope_compare::compute_diff(
        loaded.diff_text(),
        "left content\nchanged\n",
        forskscope_compare::DiffOptions::default(),
    );
    assert!(
        !diff.hunks.is_empty(),
        "the real content must be loaded, not an empty/garbled stand-in"
    );

    // Copy: the destination path keeps the same control character, and
    // its content matches the source read through that same real path.
    let outcome = crate::dir::copy_file(&src, &dst, crate::save::BackupPolicy::None)
        .expect("copy must use the real path throughout");
    assert_eq!(outcome.dst, dst);
    assert!(
        dst.to_string_lossy().contains('\u{202E}'),
        "the copy's own destination path must still carry the real control character: {dst:?}"
    );
    assert_eq!(fs::read_to_string(&dst).unwrap(), "left content\n");
}
