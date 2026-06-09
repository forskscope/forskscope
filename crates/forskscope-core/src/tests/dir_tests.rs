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
