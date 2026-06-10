//! VCS context integration boundary (RFC-038).
//!
//! ForskScope may use VCS information to improve comparison workflows, but
//! it must not become a VCS client. This module is **read-only**: no commits,
//! no branch operations, no history editing.
//!
//! ## Design
//!
//! - [`VcsProvider`] is the trait all providers implement. Currently only
//!   [`GitProvider`] is implemented; a JJ provider is reserved for future work.
//! - [`detect`] finds the first supported VCS at or above a given path.
//! - VCS failures degrade gracefully: if a command fails the UI falls back to
//!   normal file/directory comparison.
//!
//! ## Security
//!
//! All git commands are run with an explicit argument array — never through a
//! shell. Paths are passed as separate arguments, preventing injection.

use std::path::{Path, PathBuf};
use std::process::Command;

// ── Error type ────────────────────────────────────────────────────────────────

/// An error from a VCS provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsError {
    pub message: String,
}

impl VcsError {
    fn new(msg: impl Into<String>) -> Self { Self { message: msg.into() } }
}

impl std::fmt::Display for VcsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VcsError {}

// ── Revision ──────────────────────────────────────────────────────────────────

/// An opaque VCS revision identifier (commit hash, branch name, symbolic ref).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VcsRevision(pub String);

impl VcsRevision {
    /// The working-tree pseudo-revision used in UI labels.
    pub fn working_tree() -> Self { Self("WORKING".into()) }
    /// HEAD — the current commit.
    pub fn head() -> Self { Self("HEAD".into()) }
}

impl std::fmt::Display for VcsRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── File status ───────────────────────────────────────────────────────────────

/// How a file's working-tree state relates to the repository (RFC-038 §"VCS Changes Panel").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VcsFileChange {
    /// Modified relative to the index / HEAD.
    Modified,
    /// Added (untracked or staged new file).
    Added,
    /// Deleted from the working tree.
    Deleted,
    /// Renamed from another path.
    Renamed { from: PathBuf },
    /// Both sides modified — merge conflict.
    Conflicted,
    /// Status was returned but is not one of the above.
    Other(String),
}

/// The VCS status of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsFileStatus {
    /// Path relative to the repository root.
    pub path:   PathBuf,
    pub change: VcsFileChange,
}

// ── Provider trait ────────────────────────────────────────────────────────────

/// A read-only VCS context provider (RFC-038 §"VCS Provider Trait").
pub trait VcsProvider: Send + Sync {
    /// The root directory of the repository.
    fn root(&self) -> &Path;

    /// Which VCS system this provider represents.
    fn system_name(&self) -> &'static str;

    /// List modified, added, deleted, and conflicted files in the working tree.
    fn status(&self) -> Result<Vec<VcsFileStatus>, VcsError>;

    /// Read the content of `path` (relative to repo root) at `rev`.
    /// Returns raw bytes; the caller decodes them through `load_path`.
    fn read_revision_file(&self, rev: &VcsRevision, path: &Path)
        -> Result<Vec<u8>, VcsError>;

    /// Find the common ancestor of `left` and `right`.
    fn merge_base(&self, left: &VcsRevision, right: &VcsRevision)
        -> Result<Option<VcsRevision>, VcsError>;
}

// ── Git provider ──────────────────────────────────────────────────────────────

/// A Git working-tree provider.
///
/// Runs bounded, read-only `git` subcommands. Every command uses an explicit
/// argument array — no shell string expansion — preventing path injection.
#[derive(Debug, Clone)]
pub struct GitProvider {
    root: PathBuf,
}

impl GitProvider {
    /// Return a provider if `path` is inside a git repository, else `None`.
    /// Searches upward from `path` for a `.git` directory.
    pub fn detect(path: &Path) -> Option<Self> {
        let root = find_git_root(path)?;
        Some(Self { root })
    }

    /// Run a git command with the given args, working in the repo root.
    /// Returns stdout as UTF-8, or an error.
    fn git(&self, args: &[&str]) -> Result<String, VcsError> {
        let out = Command::new("git")
            .current_dir(&self.root)
            .args(args)
            .output()
            .map_err(|e| VcsError::new(format!("git command failed to start: {e}")))?;
        if out.status.success() {
            String::from_utf8(out.stdout)
                .map_err(|_| VcsError::new("git output is not valid UTF-8"))
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            Err(VcsError::new(format!(
                "git exited {}: {}",
                out.status.code().unwrap_or(-1),
                stderr.trim()
            )))
        }
    }
}

impl VcsProvider for GitProvider {
    fn root(&self) -> &Path { &self.root }
    fn system_name(&self) -> &'static str { "git" }

    fn status(&self) -> Result<Vec<VcsFileStatus>, VcsError> {
        let output = self.git(&["status", "--porcelain", "-u"])?;
        let mut result = Vec::new();
        for line in output.lines() {
            if line.len() < 3 { continue; }
            let xy    = &line[..2];
            let path_part = &line[3..];
            let change = parse_porcelain_status(xy, path_part);
            let path = PathBuf::from(path_part.trim_matches('"'));
            result.push(VcsFileStatus { path, change });
        }
        Ok(result)
    }

    fn read_revision_file(&self, rev: &VcsRevision, path: &Path) -> Result<Vec<u8>, VcsError> {
        let spec = format!("{}:{}", rev.0, path.display());
        let out = Command::new("git")
            .current_dir(&self.root)
            .args(["show", &spec])
            .output()
            .map_err(|e| VcsError::new(format!("git show failed: {e}")))?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            Err(VcsError::new(format!("git show: {}", stderr.trim())))
        }
    }

    fn merge_base(&self, left: &VcsRevision, right: &VcsRevision)
        -> Result<Option<VcsRevision>, VcsError>
    {
        match self.git(&["merge-base", &left.0, &right.0]) {
            Ok(out) => {
                let hash = out.trim().to_string();
                if hash.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(VcsRevision(hash)))
                }
            }
            Err(e) if e.message.contains("not a valid object name")
                   || e.message.contains("no merge base") =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Detect the VCS at or above `path`, if any.
///
/// Currently detects Git only. Returns `None` if no supported VCS is found —
/// the caller falls back to normal file/directory comparison.
///
/// ```rust,no_run
/// # use std::path::Path;
/// # use forskscope_core::vcs::detect;
/// if let Some(provider) = detect(Path::new("/my/project")) {
///     println!("found {}", provider.system_name());
/// }
/// ```
pub fn detect(path: &Path) -> Option<Box<dyn VcsProvider>> {
    if let Some(git) = GitProvider::detect(path) {
        return Some(Box::new(git));
    }
    // Future: JJ provider, Mercurial, etc.
    None
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Walk upward from `start` looking for a `.git` directory or file.
fn find_git_root(start: &Path) -> Option<PathBuf> {
    let canonical = start.canonicalize().ok()?;
    let mut cur = canonical.as_path();
    loop {
        if cur.join(".git").exists() {
            return Some(cur.to_path_buf());
        }
        cur = cur.parent()?;
    }
}

/// Parse a git `--porcelain` two-character XY code.
fn parse_porcelain_status(xy: &str, path: &str) -> VcsFileChange {
    // XY: X = index status, Y = working-tree status.
    // For conflict detection: 'U', 'A'/'A', 'D'/'D', etc.
    match xy {
        "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU" =>
            VcsFileChange::Conflicted,
        xy if xy.starts_with('R') || xy.ends_with('R') => {
            // Rename: path field is "new -> old" or just new.
            let from = path.find(" -> ")
                .map(|i| PathBuf::from(&path[i + 4..]))
                .unwrap_or_default();
            VcsFileChange::Renamed { from }
        }
        xy if xy.contains('M') => VcsFileChange::Modified,
        xy if xy.contains('A') || xy.contains('?') => VcsFileChange::Added,
        xy if xy.contains('D') => VcsFileChange::Deleted,
        other => VcsFileChange::Other(other.into()),
    }
}
