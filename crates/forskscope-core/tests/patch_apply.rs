//! Integration test: patches produced by `forskscope-core` must apply
//! cleanly with the system `patch` tool and with `git apply` (RFC-039
//! acceptance criterion — "users can export a unified diff for selected
//! changes"; RFC-084 — CRLF, mixed-newline, and space-in-path
//! conformance). This validates the export format against real consumers
//! rather than only against the library's own reader.
//!
//! Each differential case is skipped automatically (with a message, not
//! silently) when the tool it needs is unavailable.

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

fn have_git() -> bool {
    Command::new("git")
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

/// Apply `patch_text` (targeting `rel_name`, bytes not text — the CRLF and
/// mixed-newline cases need exact terminators preserved) with GNU `patch`
/// and return the resulting bytes.
fn apply_with_patch(
    dir: &std::path::Path,
    rel_name: &str,
    original: &[u8],
    patch_text: &str,
) -> Vec<u8> {
    let target = dir.join(rel_name);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&target, original).unwrap();
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
    fs::read(&target).unwrap()
}

/// Apply `patch_text` (targeting `rel_name`) with `git apply` and return
/// the resulting bytes. `git apply` needs no repository — it only reads
/// and writes the working-tree file named in the patch headers.
fn apply_with_git(
    dir: &std::path::Path,
    rel_name: &str,
    original: &[u8],
    patch_text: &str,
) -> Vec<u8> {
    let target = dir.join(rel_name);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&target, original).unwrap();
    fs::write(dir.join("change.patch"), patch_text).unwrap();
    let out = Command::new("git")
        .current_dir(dir)
        .args(["apply", "--unsafe-paths", "change.patch"])
        .output()
        .expect("run git apply");
    assert!(
        out.status.success(),
        "git apply failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    fs::read(&target).unwrap()
}

/// Apply `patch_text` to a file containing `original` and return the result.
fn apply(dir: &std::path::Path, original: &str, patch_text: &str) -> String {
    String::from_utf8(apply_with_patch(
        dir,
        "f.txt",
        original.as_bytes(),
        patch_text,
    ))
    .unwrap()
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

// ── RFC-084 §1: CRLF and mixed-newline conformance ──────────────────────

#[test]
fn crlf_patch_applies_with_both_tools() {
    let left = "line one\r\nline two\r\nline three\r\n";
    let right = "line one\r\nline TWO changed\r\nline three\r\n";

    let diff = compute_diff(left, right, DiffOptions::default());
    let patch = patch_from_file_diff("f.txt", &diff, PatchOptions::default()).unwrap();
    let text = to_unified(&patch);

    if have_patch() {
        let dir = workdir("crlf-patch");
        let result = apply_with_patch(&dir, "f.txt", left.as_bytes(), &text);
        assert_eq!(
            result,
            right.as_bytes(),
            "GNU patch must reproduce the CRLF right side byte-for-byte"
        );
        let _ = fs::remove_dir_all(&dir);
    } else {
        eprintln!(
            "skipping crlf_patch_applies_with_both_tools (patch -p1): GNU patch not available"
        );
    }

    if have_git() {
        let dir = workdir("crlf-git");
        let result = apply_with_git(&dir, "f.txt", left.as_bytes(), &text);
        assert_eq!(
            result,
            right.as_bytes(),
            "git apply must reproduce the CRLF right side byte-for-byte"
        );
        let _ = fs::remove_dir_all(&dir);
    } else {
        eprintln!("skipping crlf_patch_applies_with_both_tools (git apply): git not available");
    }

    if !have_patch() && !have_git() {
        panic!("neither GNU patch nor git is available — cannot validate CRLF conformance at all");
    }
}

#[test]
fn mixed_newline_file_round_trips() {
    // Left is entirely LF; the changed line and one context line switch to
    // CRLF on the right, so the same file carries both terminators.
    let left = "alpha\nbeta\ngamma\ndelta\n";
    let right = "alpha\r\nBETA changed\r\ngamma\ndelta\n";

    let diff = compute_diff(left, right, DiffOptions::default());
    let patch = patch_from_file_diff("f.txt", &diff, PatchOptions::default()).unwrap();
    let text = to_unified(&patch);

    if have_patch() {
        let dir = workdir("mixed-patch");
        let result = apply_with_patch(&dir, "f.txt", left.as_bytes(), &text);
        assert_eq!(
            result,
            right.as_bytes(),
            "GNU patch must round-trip a mixed-newline file byte-for-byte"
        );
        let _ = fs::remove_dir_all(&dir);
    } else {
        eprintln!("skipping mixed_newline_file_round_trips (patch -p1): GNU patch not available");
    }

    if have_git() {
        let dir = workdir("mixed-git");
        let result = apply_with_git(&dir, "f.txt", left.as_bytes(), &text);
        assert_eq!(
            result,
            right.as_bytes(),
            "git apply must round-trip a mixed-newline file byte-for-byte"
        );
        let _ = fs::remove_dir_all(&dir);
    } else {
        eprintln!("skipping mixed_newline_file_round_trips (git apply): git not available");
    }

    if !have_patch() && !have_git() {
        panic!(
            "neither GNU patch nor git is available — cannot validate mixed-newline conformance at all"
        );
    }
}

// ── RFC-084 §2: a path with a space defeats `patch -p1` without a
// disambiguating trailing tab ───────────────────────────────────────────

#[test]
fn space_in_path_applies_with_both_tools() {
    let rel = "dir with space/file name.txt";
    let left = "one\ntwo\nthree\n";
    let right = "one\nTWO changed\nthree\n";

    let diff = compute_diff(left, right, DiffOptions::default());
    let patch = patch_from_file_diff(rel, &diff, PatchOptions::default()).unwrap();
    let text = to_unified(&patch);

    if have_patch() {
        let dir = workdir("space-patch");
        let result = apply_with_patch(&dir, rel, left.as_bytes(), &text);
        assert_eq!(
            result,
            right.as_bytes(),
            "GNU patch must locate and patch a space-containing path"
        );
        let _ = fs::remove_dir_all(&dir);
    } else {
        eprintln!(
            "skipping space_in_path_applies_with_both_tools (patch -p1): GNU patch not available"
        );
    }

    if have_git() {
        let dir = workdir("space-git");
        let result = apply_with_git(&dir, rel, left.as_bytes(), &text);
        assert_eq!(
            result,
            right.as_bytes(),
            "git apply must locate and patch a space-containing path"
        );
        let _ = fs::remove_dir_all(&dir);
    } else {
        eprintln!("skipping space_in_path_applies_with_both_tools (git apply): git not available");
    }

    if !have_patch() && !have_git() {
        panic!(
            "neither GNU patch nor git is available — cannot validate space-in-path conformance at all"
        );
    }
}
