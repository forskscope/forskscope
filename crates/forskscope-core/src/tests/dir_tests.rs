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
        listing
            .files
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
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
    let left = root.join("left");
    let right = root.join("right");
    for d in [&left, &right] {
        fs::create_dir_all(d).unwrap();
    }

    // equal file
    fs::write(left.join("same.txt"), "x").unwrap();
    fs::write(right.join("same.txt"), "x").unwrap();
    // changed file
    fs::write(left.join("diff.txt"), "v1").unwrap();
    fs::write(right.join("diff.txt"), "v2").unwrap();
    // left-only
    fs::write(left.join("left_only.txt"), "l").unwrap();
    // right-only
    fs::write(right.join("right_only.txt"), "r").unwrap();

    let entries = crate::dir::recursive_diff(&left, &right).entries;
    let status = |name: &str| {
        entries
            .iter()
            .find(|e| e.rel_path.to_str() == Some(name))
            .map(|e| e.status)
            .unwrap()
    };
    use crate::dir::RecStatus;
    assert_eq!(status("same.txt"), RecStatus::Equal);
    assert_eq!(status("diff.txt"), RecStatus::Changed);
    assert_eq!(status("left_only.txt"), RecStatus::LeftOnly);
    assert_eq!(status("right_only.txt"), RecStatus::RightOnly);
}

#[test]
fn recursive_diff_descends_into_subdirectories() {
    let root = temp_dir("rec-nested");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(left.join("sub")).unwrap();
    fs::create_dir_all(right.join("sub")).unwrap();
    fs::write(left.join("sub").join("a.rs"), "old").unwrap();
    fs::write(right.join("sub").join("a.rs"), "new").unwrap();

    let entries = crate::dir::recursive_diff(&left, &right).entries;
    assert!(
        entries
            .iter()
            .any(|e| e.rel_path == std::path::Path::new("sub/a.rs")
                && e.status == crate::dir::RecStatus::Changed)
    );
}

// ── F135: an empty one-sided directory ────────────────────────────────────────

/// The defect itself, falsified directly: before this fix, a directory that
/// exists on one side only and is empty produced no entry at all - the pair
/// looked exactly like two trees with nothing one-sided in them.
#[test]
fn an_empty_left_only_directory_is_reported() {
    let root = temp_dir("rec-empty-left");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(left.join("empty")).unwrap();
    fs::create_dir_all(&right).unwrap();

    let entries = crate::dir::recursive_diff(&left, &right).entries;
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].rel_path, std::path::Path::new("empty"));
    assert_eq!(entries[0].status, crate::dir::RecStatus::LeftOnlyDir);
}

#[test]
fn an_empty_right_only_directory_is_reported() {
    let root = temp_dir("rec-empty-right");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(right.join("empty")).unwrap();

    let entries = crate::dir::recursive_diff(&left, &right).entries;
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].rel_path, std::path::Path::new("empty"));
    assert_eq!(entries[0].status, crate::dir::RecStatus::RightOnlyDir);
}

/// The fast listing (tier 1's own walk, `walk_and_merge_fast`) is a
/// separate code path from `recursive_diff`'s - checked here directly
/// rather than assumed from the full walk alone.
#[test]
fn the_fast_listing_also_reports_an_empty_one_sided_directory() {
    let root = temp_dir("rec-empty-fast");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(left.join("empty")).unwrap();
    fs::create_dir_all(&right).unwrap();

    let entries = crate::dir::list_recursive_for_display(&left, &right).entries;
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].status, crate::dir::RecStatus::LeftOnlyDir);
}

/// A directory empty on *both* sides must stay invisible - unchanged from
/// before this fix, and the one case the register was explicit must not
/// regress: this is not a new kind of noise, only the one-sided case closes.
#[test]
fn a_directory_empty_on_both_sides_stays_invisible() {
    let root = temp_dir("rec-empty-both");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(left.join("empty")).unwrap();
    fs::create_dir_all(right.join("empty")).unwrap();

    let entries = crate::dir::recursive_diff(&left, &right).entries;
    assert!(entries.is_empty(), "{entries:?}");
}

/// A one-sided directory that is *not* empty is unaffected: its files are
/// already individually one-sided, exactly as before this fix, and the
/// directory itself earns no separate marker - reported once, through its
/// contents, not twice.
#[test]
fn a_non_empty_one_sided_directory_is_unaffected() {
    let root = temp_dir("rec-nonempty-left");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(&right).unwrap();
    fs::create_dir_all(left.join("only")).unwrap();
    fs::write(left.join("only/a.txt"), "x").unwrap();

    let entries = crate::dir::recursive_diff(&left, &right).entries;
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].rel_path, std::path::Path::new("only/a.txt"));
    assert_eq!(entries[0].status, crate::dir::RecStatus::LeftOnly);
}

/// A chain of empty one-sided directories reports once, at the leaf -
/// mirroring how a one-sided *file* reports once at its own path, with
/// every ancestor directory staying implicit.
#[test]
fn a_nested_empty_one_sided_directory_reports_once_at_the_leaf() {
    let root = temp_dir("rec-nested-empty");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(left.join("a/b/c")).unwrap();
    fs::create_dir_all(&right).unwrap();

    let entries = crate::dir::recursive_diff(&left, &right).entries;
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].rel_path, std::path::Path::new("a/b/c"));
    assert_eq!(entries[0].status, crate::dir::RecStatus::LeftOnlyDir);
}

/// A directory that looks empty only because its one file was excluded by
/// ignore rules must not be reported as a one-sided difference - the
/// module doc's own rule ("an ignored entry present on one side only never
/// becomes a one-sided difference") applies to a directory's *emptiness*
/// too, not only to the ignored file itself. Falsify by dropping the
/// `ignored_anything` propagation in `walk`/`walk_and_merge*`: `sub` then
/// reports as `LeftOnlyDir`, which this fails on.
#[test]
fn a_directory_emptied_only_by_an_ignore_rule_is_not_reported() {
    let root = temp_dir("rec-ignored-empty");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(&right).unwrap();
    fs::create_dir_all(left.join("sub")).unwrap();
    fs::write(left.join("sub/ignored.log"), "x").unwrap();

    let rules = crate::IgnoreRules::from_settings("log", "");
    let token = crate::CancellationToken::new();
    let entries = crate::dir::recursive_diff_with_rules(&left, &right, &token, &rules).entries;
    assert!(
        entries.is_empty(),
        "an ignored-into-emptiness directory must not be reported: {entries:?}"
    );
}

// ── v0.34.0 additions ─────────────────────────────────────────────────────────

#[test]
fn list_dir_on_empty_directory_returns_no_entries() {
    let dir = temp_dir("empty-list");
    let listing = crate::dir::list_dir(Some(&dir)).unwrap();
    assert!(
        listing.files.is_empty(),
        "empty directory must have no file entries"
    );
    assert!(
        listing.dirs.is_empty(),
        "empty directory must have no dir entries"
    );
}

#[test]
fn list_dir_reports_last_modified_for_files() {
    let dir = temp_dir("mtime");
    let f = dir.join("check.txt");
    fs::write(&f, "hello").unwrap();
    let listing = crate::dir::list_dir(Some(&dir)).unwrap();
    let entry = listing
        .files
        .iter()
        .find(|e| e.name == "check.txt")
        .expect("file must appear");
    // last_modified is a formatted string; just verify it's non-empty.
    assert!(
        !entry.last_modified.is_empty(),
        "last_modified must be populated for real files"
    );
}

#[test]
fn list_dir_uses_current_dir_when_path_is_none() {
    // When no path is given, listing should return without panicking.
    let result = crate::dir::list_dir(None);
    assert!(result.is_ok(), "list_dir(None) must succeed");
}

#[test]
fn recursive_diff_returns_empty_for_two_empty_directories() {
    let root = temp_dir("rec-empty");
    let left = root.join("l");
    let right = root.join("r");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    let entries = crate::dir::recursive_diff(&left, &right).entries;
    assert!(
        entries.is_empty(),
        "two empty directories have no diff entries"
    );
}

/// RFC-080 tier 1: a names-and-sizes match is a completed measurement with an
/// incomplete conclusion — none of equal, different or pending. Falsify by adding
/// `MetadataMatch` to `is_equal`: the first assertion fails (it would let *hide
/// identical* hide it and would assert what tier 1 cannot).
#[test]
fn a_tier_1_match_is_neither_equal_nor_different_nor_pending() {
    use crate::dir::EqualityEvidence as E;
    let m = E::MetadataMatch;
    assert!(!m.is_equal());
    assert!(!m.is_different());
    assert!(!m.is_pending());
    assert!(m.present_on_both_sides());

    let d = E::TreeDifferent;
    assert!(d.is_different());
    assert!(!d.is_equal());
    assert!(!d.is_pending());
}

/// RFC-080 tier 2: `TreeIdentical` is the only evidence a *directory*
/// comparison is entitled to call equal — reached by `recursive_diff`
/// actually reading contents, unlike `MetadataMatch` above. Falsify by
/// removing it from `is_equal`'s match: the first assertion fails.
#[test]
fn tree_identical_is_equal_and_present_on_both_sides() {
    use crate::dir::EqualityEvidence as E;
    let i = E::TreeIdentical;
    assert!(i.is_equal());
    assert!(!i.is_different());
    assert!(!i.is_pending());
    assert!(i.present_on_both_sides());
}
