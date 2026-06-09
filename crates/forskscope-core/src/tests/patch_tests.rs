//! Patch export tests (RFC-039 §"Export Patch", §"Patch Review").
//!
//! These validate the design contract: deterministic unified-diff output,
//! correct hunk ranges and coalescing, no-newline-at-EOF handling, summary
//! statistics, and directory-scope assembly. They test the specification,
//! not incidental implementation details.

use std::fs;
use std::path::PathBuf;

use crate::diff::{DiffOptions, compute_diff};
use crate::patch::{
    LineOrigin, PatchFileChange, PatchOptions, patch_from_directories, patch_from_file_diff,
    to_unified,
};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-patch-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn diff(left: &str, right: &str) -> crate::diff::DiffDocument {
    compute_diff(left, right, DiffOptions::default())
}

#[test]
fn identical_inputs_produce_no_patch() {
    let d = diff("a\nb\nc\n", "a\nb\nc\n");
    assert!(patch_from_file_diff("f.txt", &d, PatchOptions::default()).is_none());
}

#[test]
fn single_line_change_emits_one_hunk_with_context() {
    let left = "one\ntwo\nthree\nfour\nfive\n";
    let right = "one\ntwo\nCHANGED\nfour\nfive\n";
    let d = diff(left, right);
    let patch = patch_from_file_diff("f.txt", &d, PatchOptions::default()).unwrap();

    // One modified file, one hunk.
    assert_eq!(patch.files.len(), 1);
    let PatchFileChange::Modify { hunks, .. } = &patch.files[0] else {
        panic!("expected Modify");
    };
    assert_eq!(hunks.len(), 1);

    // The change touched line 3; with 3 context lines the hunk spans the
    // whole 5-line file: old 1,5 and new 1,5.
    let h = &hunks[0];
    assert_eq!((h.old_start, h.old_len), (1, 5));
    assert_eq!((h.new_start, h.new_len), (1, 5));

    // Exactly one delete and one insert.
    let deletes = h.lines.iter().filter(|l| l.origin == LineOrigin::Delete).count();
    let inserts = h.lines.iter().filter(|l| l.origin == LineOrigin::Insert).count();
    assert_eq!((deletes, inserts), (1, 1));
}

#[test]
fn summary_counts_additions_and_deletions() {
    let left = "a\nb\nc\n";
    let right = "a\nB1\nB2\nc\n";
    let d = diff(left, right);
    let patch = patch_from_file_diff("f.txt", &d, PatchOptions::default()).unwrap();
    // One line deleted (b), two inserted (B1, B2).
    assert_eq!(patch.summary.deletions, 1);
    assert_eq!(patch.summary.additions, 2);
    assert_eq!(patch.summary.files_changed, 1);
}

#[test]
fn unified_output_has_expected_headers_and_markers() {
    let left = "alpha\nbeta\ngamma\n";
    let right = "alpha\nBETA\ngamma\n";
    let d = diff(left, right);
    let patch = patch_from_file_diff("src/m.rs", &d, PatchOptions::default()).unwrap();
    let text = to_unified(&patch);

    assert!(text.contains("--- a/src/m.rs"), "missing old header:\n{text}");
    assert!(text.contains("+++ b/src/m.rs"), "missing new header:\n{text}");
    assert!(text.contains("@@ -1,3 +1,3 @@"), "missing hunk header:\n{text}");
    assert!(text.contains("-beta"), "missing deletion:\n{text}");
    assert!(text.contains("+BETA"), "missing insertion:\n{text}");
    assert!(text.contains(" alpha"), "missing context:\n{text}");
    // Summary header is present.
    assert!(text.starts_with("# forskscope patch:"), "missing summary:\n{text}");
}

#[test]
fn missing_trailing_newline_emits_no_newline_marker() {
    // Right side has no final newline.
    let left = "a\nb\n";
    let right = "a\nB";
    let d = diff(left, right);
    let patch = patch_from_file_diff("f.txt", &d, PatchOptions::default()).unwrap();
    let text = to_unified(&patch);
    assert!(
        text.contains("\\ No newline at end of file"),
        "expected no-newline marker:\n{text}"
    );
}

#[test]
fn output_is_byte_for_byte_deterministic() {
    let left = "1\n2\n3\n4\n5\n6\n7\n8\n";
    let right = "1\nX\n3\n4\n5\n6\nY\n8\n";
    let d = diff(left, right);
    let p1 = patch_from_file_diff("f.txt", &d, PatchOptions::default()).unwrap();
    let p2 = patch_from_file_diff("f.txt", &d, PatchOptions::default()).unwrap();
    assert_eq!(to_unified(&p1), to_unified(&p2));
}

#[test]
fn distant_changes_split_into_separate_hunks() {
    // Two changes far apart (line 2 and line 11) must not coalesce with
    // the default 3-line context.
    let left = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\n";
    let right = "a\nB\nc\nd\ne\nf\ng\nh\ni\nj\nK\nl\n";
    let d = diff(left, right);
    let patch = patch_from_file_diff("f.txt", &d, PatchOptions::default()).unwrap();
    let PatchFileChange::Modify { hunks, .. } = &patch.files[0] else {
        panic!("expected Modify");
    };
    assert_eq!(hunks.len(), 2, "distant changes should be two hunks");
}

#[test]
fn adjacent_changes_coalesce_into_one_hunk() {
    // Changes on consecutive lines share context and merge.
    let left = "a\nb\nc\nd\ne\n";
    let right = "a\nB\nC\nd\ne\n";
    let d = diff(left, right);
    let patch = patch_from_file_diff("f.txt", &d, PatchOptions::default()).unwrap();
    let PatchFileChange::Modify { hunks, .. } = &patch.files[0] else {
        panic!("expected Modify");
    };
    assert_eq!(hunks.len(), 1);
}

#[test]
fn directory_patch_covers_modify_add_and_delete() {
    let base = temp_dir("dir");
    let left = base.join("left");
    let right = base.join("right");
    let _ = fs::remove_dir_all(&left);
    let _ = fs::remove_dir_all(&right);
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();

    // common.txt: modified
    fs::write(left.join("common.txt"), "a\nb\nc\n").unwrap();
    fs::write(right.join("common.txt"), "a\nB\nc\n").unwrap();
    // same.txt: identical (must be excluded)
    fs::write(left.join("same.txt"), "x\n").unwrap();
    fs::write(right.join("same.txt"), "x\n").unwrap();
    // only_left.txt: deleted
    fs::write(left.join("only_left.txt"), "gone\n").unwrap();
    // only_right.txt: added
    fs::write(right.join("only_right.txt"), "new\n").unwrap();

    let patch = patch_from_directories(
        &left,
        &right,
        DiffOptions::default(),
        PatchOptions::default(),
    )
    .unwrap();

    assert_eq!(patch.summary.files_changed, 1);
    assert_eq!(patch.summary.files_added, 1);
    assert_eq!(patch.summary.files_deleted, 1);

    let kinds: Vec<&str> = patch
        .files
        .iter()
        .map(|f| match f {
            PatchFileChange::Modify { .. } => "M",
            PatchFileChange::Add { .. } => "A",
            PatchFileChange::Delete { .. } => "D",
            PatchFileChange::BinaryNotice { .. } => "B",
        })
        .collect();
    assert!(kinds.contains(&"M"));
    assert!(kinds.contains(&"A"));
    assert!(kinds.contains(&"D"));

    let text = to_unified(&patch);
    // Added file references /dev/null on the old side.
    assert!(text.contains("--- /dev/null"), "add should use /dev/null:\n{text}");
    // Deleted file references /dev/null on the new side.
    assert!(text.contains("+++ /dev/null"), "delete should use /dev/null:\n{text}");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn creation_deletion_can_be_disabled() {
    let base = temp_dir("nocreate");
    let left = base.join("left");
    let right = base.join("right");
    let _ = fs::remove_dir_all(&left);
    let _ = fs::remove_dir_all(&right);
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    fs::write(left.join("only_left.txt"), "gone\n").unwrap();
    fs::write(right.join("only_right.txt"), "new\n").unwrap();

    let options = PatchOptions {
        include_creation_deletion: false,
        ..PatchOptions::default()
    };
    let patch =
        patch_from_directories(&left, &right, DiffOptions::default(), options).unwrap();
    assert!(patch.is_empty(), "one-sided files should be skipped");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn pure_insertion_uses_zero_old_count() {
    // Insert two lines at the top of a file; the old side of the hunk has
    // start 0, len 0 by unified-diff convention when nothing precedes it.
    let left = "keep\n";
    let right = "new1\nnew2\nkeep\n";
    let d = diff(left, right);
    let patch = patch_from_file_diff("f.txt", &d, PatchOptions::default()).unwrap();
    let PatchFileChange::Modify { hunks, .. } = &patch.files[0] else {
        panic!("expected Modify");
    };
    let h = &hunks[0];
    // Two insertions, one context line "keep".
    assert_eq!(h.new_len, 3);
    assert_eq!(h.old_len, 1);
}
