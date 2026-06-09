//! Integration test: patches produced by `forskscope-core` must apply
//! cleanly with the system `patch` tool (RFC-039 acceptance criterion —
//! "users can export a unified diff for selected changes"). This validates
//! the export format against a real consumer rather than only against the
//! library's own reader.
//!
//! The test is skipped automatically when GNU `patch` is unavailable.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use forskscope_core::{DiffOptions, PatchOptions, compute_diff, patch_from_file_diff, to_unified};

fn have_patch() -> bool {
    Command::new("patch")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn workdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-applytest-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Apply `patch_text` to a file containing `original` and return the result.
fn apply(dir: &std::path::Path, original: &str, patch_text: &str) -> String {
    fs::write(dir.join("f.txt"), original).unwrap();
    fs::write(dir.join("change.patch"), patch_text).unwrap();
    // -p1 strips the leading a//b/ component; --no-backup keeps the dir clean.
    let out = Command::new("patch")
        .current_dir(dir)
        .args(["-p1", "--no-backup-if-mismatch", "-i", "change.patch"])
        .output()
        .expect("run patch");
    assert!(
        out.status.success(),
        "patch failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    fs::read_to_string(dir.join("f.txt")).unwrap()
}

#[test]
fn generated_patch_transforms_left_into_right() {
    if !have_patch() {
        eprintln!("skipping: GNU patch not available");
        return;
    }
    let left = "line one\nline two\nline three\nline four\nline five\n";
    let right = "line one\nline TWO changed\nline three\nadded line\nline four\nline five\n";

    let diff = compute_diff(left, right, DiffOptions::default());
    let patch = patch_from_file_diff("f.txt", &diff, PatchOptions::default()).unwrap();
    let text = to_unified(&patch);

    let dir = workdir("modify");
    let result = apply(&dir, left, &text);
    assert_eq!(result, right, "applied patch must reproduce the right side");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generated_patch_handles_multiple_distant_hunks() {
    if !have_patch() {
        eprintln!("skipping: GNU patch not available");
        return;
    }
    let left = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\n";
    let right = "a\nB\nc\nd\ne\nf\ng\nh\ni\nj\nk\nL\nm\nn\n";

    let diff = compute_diff(left, right, DiffOptions::default());
    let patch = patch_from_file_diff("f.txt", &diff, PatchOptions::default()).unwrap();
    let text = to_unified(&patch);

    let dir = workdir("multi");
    let result = apply(&dir, left, &text);
    assert_eq!(result, right);
    let _ = fs::remove_dir_all(&dir);
}
