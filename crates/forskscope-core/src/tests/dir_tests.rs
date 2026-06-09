use std::fs;
use std::path::PathBuf;

use crate::dir::{dir_digest_equal, file_digest_equal, list_dir};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-dir-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn listing_separates_dirs_and_files_sorted() {
    let dir = temp_dir("list");
    fs::create_dir_all(dir.join("zsub")).unwrap();
    fs::create_dir_all(dir.join("asub")).unwrap();
    fs::write(dir.join("b.txt"), "x").unwrap();
    fs::write(dir.join("a.txt"), "xy").unwrap();

    let listing = list_dir(Some(&dir)).unwrap();
    assert_eq!(listing.dirs, vec!["asub", "zsub"]);
    assert_eq!(
        listing.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        vec!["a.txt", "b.txt"]
    );
    let a = listing.files.iter().find(|f| f.name == "a.txt").unwrap();
    assert_eq!(a.len, 2);
    assert!(a.human_size.contains("bytes"));
}

#[test]
fn file_digest_equal_compares_content() {
    let dir = temp_dir("fdigest");
    let a = dir.join("a");
    let b = dir.join("b");
    let c = dir.join("c");
    fs::write(&a, "same").unwrap();
    fs::write(&b, "same").unwrap();
    fs::write(&c, "diff").unwrap();
    assert!(file_digest_equal(&a, &b).unwrap());
    assert!(!file_digest_equal(&a, &c).unwrap());
}

#[test]
fn dir_digest_equal_is_recursive() {
    let root = temp_dir("ddigest");
    let left = root.join("left");
    let right = root.join("right");
    for base in [&left, &right] {
        fs::create_dir_all(base.join("nested")).unwrap();
        fs::write(base.join("top.txt"), "top").unwrap();
        fs::write(base.join("nested/inner.txt"), "inner").unwrap();
    }
    assert!(dir_digest_equal(&left, &right).unwrap());

    fs::write(right.join("nested/inner.txt"), "changed").unwrap();
    assert!(!dir_digest_equal(&left, &right).unwrap());
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

#[test]
fn recursive_diff_classifies_equal_changed_left_only_right_only() {
    let root = temp_dir("rec");
    let left  = root.join("left");
    let right = root.join("right");
    for d in [&left, &right] { fs::create_dir_all(d).unwrap(); }

    // equal file
    fs::write(left.join("same.txt"),    "x").unwrap();
    fs::write(right.join("same.txt"),   "x").unwrap();
    // changed file
    fs::write(left.join("diff.txt"),    "v1").unwrap();
    fs::write(right.join("diff.txt"),   "v2").unwrap();
    // left-only
    fs::write(left.join("left_only.txt"),  "l").unwrap();
    // right-only
    fs::write(right.join("right_only.txt"), "r").unwrap();

    let entries = crate::dir::recursive_diff(&left, &right);
    let status = |name: &str| entries.iter().find(|e| e.rel_path.to_str() == Some(name))
        .map(|e| e.status).unwrap();
    use crate::dir::RecStatus;
    assert_eq!(status("same.txt"),        RecStatus::Equal);
    assert_eq!(status("diff.txt"),        RecStatus::Changed);
    assert_eq!(status("left_only.txt"),   RecStatus::LeftOnly);
    assert_eq!(status("right_only.txt"),  RecStatus::RightOnly);
}

#[test]
fn recursive_diff_descends_into_subdirectories() {
    let root  = temp_dir("rec-nested");
    let left  = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(left.join("sub")).unwrap();
    fs::create_dir_all(right.join("sub")).unwrap();
    fs::write(left.join("sub").join("a.rs"),  "old").unwrap();
    fs::write(right.join("sub").join("a.rs"), "new").unwrap();

    let entries = crate::dir::recursive_diff(&left, &right);
    assert!(entries.iter().any(|e| e.rel_path == std::path::Path::new("sub/a.rs")
        && e.status == crate::dir::RecStatus::Changed));
}
