//! Patch document model (RFC-039 §"Patch Model").
//!
//! These types are GUI-independent and serializable to a deterministic
//! textual form by the `unified` writer. The model deliberately keeps the
//! *export* contract separate from the (still-proposed) guarded *apply*
//! contract: a `PatchDocument` describes a set of file changes; it never
//! performs I/O.

use std::path::PathBuf;

use crate::diff::NewlineMarker;

/// Patch serialization format. Only unified diff is supported in the
/// v0.39.0 export slice; the enum exists so the apply half (RFC-039) can
/// extend it without a breaking change.
/// Patch format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PatchFormat {
    /// POSIX-style unified diff with `git`-compatible file headers.
    #[default]
    Unified,
}

/// One change to one file in a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchFileChange {
    /// The file exists on both sides and its text differs.
    Modify {
        path: PathBuf,
        hunks: Vec<PatchHunk>,
    },
    /// The file exists only on the right (new) side.
    Add {
        path: PathBuf,
        /// Right-side lines, terminators stripped (re-emitted on write).
        lines: Vec<PatchLine>,
    },
    /// The file exists only on the left (old) side.
    Delete {
        path: PathBuf,
        /// Left-side lines, terminators stripped (re-emitted on write).
        lines: Vec<PatchLine>,
    },
    /// A binary or unsupported file that differs but cannot be expressed
    /// as a text patch. Emitted only when `include_binary_notices` is set.
    BinaryNotice { path: PathBuf },
}

impl PatchFileChange {
    /// The relative path this change targets, regardless of variant.
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Modify { path, .. }
            | Self::Add { path, .. }
            | Self::Delete { path, .. }
            | Self::BinaryNotice { path } => path,
        }
    }

    /// Per-file (additions, deletions) line counts contributing to the
    /// summary. Binary notices contribute nothing.
    pub(crate) fn line_deltas(&self) -> (usize, usize) {
        match self {
            Self::Modify { hunks, .. } => {
                let mut add = 0;
                let mut del = 0;
                for h in hunks {
                    for line in &h.lines {
                        match line.origin {
                            LineOrigin::Insert => add += 1,
                            LineOrigin::Delete => del += 1,
                            LineOrigin::Context => {}
                        }
                    }
                }
                (add, del)
            }
            Self::Add { lines, .. } => (lines.len(), 0),
            Self::Delete { lines, .. } => (0, lines.len()),
            Self::BinaryNotice { .. } => (0, 0),
        }
    }
}

/// One contiguous unified-diff hunk for a `Modify` change.
///
/// Ranges are 1-based and follow unified-diff conventions: a side with
/// zero lines uses a start equal to the line *before* the change (and a
/// count of zero), matching what `diff -u` and `git diff` emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchHunk {
    pub old_start: u32,
    pub old_len: u32,
    pub new_start: u32,
    pub new_len: u32,
    pub lines: Vec<PatchLine>,
}

/// The role of one line within a hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineOrigin {
    /// Unchanged context (present on both sides).
    Context,
    /// Present on the left/old side only.
    Delete,
    /// Present on the right/new side only.
    Insert,
}

impl LineOrigin {
    /// The unified-diff prefix character for this origin.
    pub fn marker(self) -> char {
        match self {
            Self::Context => ' ',
            Self::Delete => '-',
            Self::Insert => '+',
        }
    }
}

/// One line in a patch, content stored without its terminator.
///
/// `newline` records the line's own terminator from the source document, so
/// the writer can re-emit it faithfully instead of assuming LF. A source
/// line with no trailing newline (`NewlineMarker::None`) still needs its
/// terminator in the patch text stream itself — the writer falls back to
/// `\n` there to separate the content line from the following
/// `\ No newline at end of file` marker line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchLine {
    pub origin: LineOrigin,
    pub content: String,
    pub newline: NewlineMarker,
}

impl PatchLine {
    pub fn new(origin: LineOrigin, content: impl Into<String>, newline: NewlineMarker) -> Self {
        Self {
            origin,
            content: content.into(),
            newline,
        }
    }

    /// `true` when this line had no trailing newline in its source document.
    pub fn no_newline_at_eof(&self) -> bool {
        self.newline == NewlineMarker::None
    }
}

/// Aggregate patch statistics, used for the review summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PatchSummary {
    pub files_changed: usize,
    pub files_added: usize,
    pub files_deleted: usize,
    pub binary_files: usize,
    pub additions: usize,
    pub deletions: usize,
}

/// A complete, serializable patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchDocument {
    pub format: PatchFormat,
    pub files: Vec<PatchFileChange>,
    pub summary: PatchSummary,
}

impl PatchDocument {
    /// `true` when the patch carries no file changes.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Recompute `summary` from `files`. Called by builders after assembling
    /// the file list so the summary is always consistent with the contents.
    pub(crate) fn recompute_summary(&mut self) {
        let mut s = PatchSummary::default();
        for change in &self.files {
            let (add, del) = change.line_deltas();
            s.additions += add;
            s.deletions += del;
            match change {
                PatchFileChange::Modify { .. } => s.files_changed += 1,
                PatchFileChange::Add { .. } => s.files_added += 1,
                PatchFileChange::Delete { .. } => s.files_deleted += 1,
                PatchFileChange::BinaryNotice { .. } => s.binary_files += 1,
            }
        }
        self.summary = s;
    }
}
