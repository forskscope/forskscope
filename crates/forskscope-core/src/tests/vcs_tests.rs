//! VCS provider tests (RFC-038 §"Acceptance Criteria").
//!
//! Uses real git repositories in temp directories. Each test creates its own
//! repo so tests are fully isolated and hermetic.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::vcs::{GitProvider, VcsFileChange, VcsProvider, VcsRevision, detect};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("fsk-vcs-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .expect("git command failed to start");
    assert!(status.success(), "git {:?} failed in {:?}", args, dir);
}

/// Initialise a minimal git repo with one commit.
fn init_repo(dir: &std::path::Path) {
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "test@test.com"]);
    git(dir, &["config", "user.name", "Test"]);
    fs::write(dir.join("README.md"), "# test\n").unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-m", "initial"]);
}

// ── detect() ─────────────────────────────────────────────────────────────────

#[test]
fn detect_returns_git_provider_inside_repo() {
    let dir = tmp("detect-git");
    init_repo(&dir);
    let provider = detect(&dir);
    assert!(provider.is_some(), "must detect git provider inside a repo");
    assert_eq!(provider.unwrap().system_name(), "git");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn detect_returns_none_outside_any_repo() {
    let dir = tmp("detect-none");
    // No git init — plain directory.
    let provider = detect(&dir);
    assert!(provider.is_none(), "must return None outside a VCS repo");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn detect_finds_repo_from_subdirectory() {
    let dir = tmp("detect-subdir");
    init_repo(&dir);
    let subdir = dir.join("src");
    fs::create_dir_all(&subdir).unwrap();
    let provider = detect(&subdir);
    assert!(provider.is_some(), "must detect git from subdirectory");
    let _ = fs::remove_dir_all(&dir);
}

// ── GitProvider::detect ───────────────────────────────────────────────────────

#[test]
fn git_provider_root_is_the_repo_root() {
    let dir = tmp("root");
    init_repo(&dir);
    let git = GitProvider::detect(&dir).unwrap();
    assert_eq!(
        git.root().canonicalize().unwrap(),
        dir.canonicalize().unwrap(),
        "root() must be the repo root, not a subdirectory"
    );
    let _ = fs::remove_dir_all(&dir);
}

// ── status() ─────────────────────────────────────────────────────────────────

#[test]
fn status_empty_for_clean_working_tree() {
    let dir = tmp("status-clean");
    init_repo(&dir);
    let git = GitProvider::detect(&dir).unwrap();
    let status = git.status().unwrap();
    assert!(status.is_empty(), "clean working tree should have no status entries");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn status_reports_untracked_file_as_added() {
    let dir = tmp("status-untracked");
    init_repo(&dir);
    fs::write(dir.join("new.rs"), "fn main() {}").unwrap();
    let git = GitProvider::detect(&dir).unwrap();
    let status = git.status().unwrap();
    let entry = status.iter().find(|s| s.path.ends_with("new.rs"));
    assert!(entry.is_some(), "new untracked file must appear in status");
    assert!(matches!(entry.unwrap().change, VcsFileChange::Added),
        "untracked file must be reported as Added");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn status_reports_modified_file() {
    let dir = tmp("status-modified");
    init_repo(&dir);
    fs::write(dir.join("README.md"), "# modified\n").unwrap();
    let git = GitProvider::detect(&dir).unwrap();
    let status = git.status().unwrap();
    let entry = status.iter().find(|s| s.path.ends_with("README.md"));
    assert!(entry.is_some(), "modified file must appear in status");
    assert!(matches!(entry.unwrap().change, VcsFileChange::Modified),
        "modified file must be reported as Modified");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn status_reports_deleted_file() {
    let dir = tmp("status-deleted");
    init_repo(&dir);
    fs::remove_file(dir.join("README.md")).unwrap();
    let git = GitProvider::detect(&dir).unwrap();
    let status = git.status().unwrap();
    let entry = status.iter().find(|s| s.path.ends_with("README.md"));
    assert!(entry.is_some(), "deleted file must appear in status");
    assert!(matches!(entry.unwrap().change, VcsFileChange::Deleted),
        "deleted file must be reported as Deleted");
    let _ = fs::remove_dir_all(&dir);
}

// ── read_revision_file() ──────────────────────────────────────────────────────

#[test]
fn read_head_file_returns_content_at_head() {
    let dir = tmp("read-head");
    init_repo(&dir);
    // README.md was committed with "# test\n".
    let git = GitProvider::detect(&dir).unwrap();
    let bytes = git.read_revision_file(&VcsRevision::head(), "README.md".as_ref()).unwrap();
    assert_eq!(bytes, b"# test\n", "HEAD file content must match committed content");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn read_revision_file_errors_for_nonexistent_path() {
    let dir = tmp("read-nonexistent");
    init_repo(&dir);
    let git = GitProvider::detect(&dir).unwrap();
    let result = git.read_revision_file(&VcsRevision::head(), "does-not-exist.rs".as_ref());
    assert!(result.is_err(), "reading nonexistent path must return Err");
    let _ = fs::remove_dir_all(&dir);
}

// ── merge_base() ─────────────────────────────────────────────────────────────

#[test]
fn merge_base_of_head_with_itself_returns_head_hash() {
    let dir = tmp("mergebase");
    init_repo(&dir);
    let git = GitProvider::detect(&dir).unwrap();
    let head = &VcsRevision::head();
    let base = git.merge_base(head, head).unwrap();
    assert!(base.is_some(), "merge-base of HEAD with HEAD must exist");
    let _ = fs::remove_dir_all(&dir);
}

// ── Non-VCS path degrades gracefully ─────────────────────────────────────────

#[test]
fn git_provider_detect_returns_none_outside_git_repo() {
    let dir = tmp("no-git");
    let result = GitProvider::detect(&dir);
    assert!(result.is_none(), "GitProvider::detect must return None outside a repo");
    let _ = fs::remove_dir_all(&dir);
}

// ── VcsRevision helpers ───────────────────────────────────────────────────────

#[test]
fn vcs_revision_display() {
    assert_eq!(VcsRevision::head().to_string(), "HEAD");
    assert_eq!(VcsRevision::working_tree().to_string(), "WORKING");
}
