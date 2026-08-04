//! Prepared-comparison and save-target types (RFC-077, audit finding B3).
//!
//! Compared inputs and save output are different identities. Normal two-file
//! mode compares left/right and saves to the right input; Git mergetool mode
//! compares local/remote but saves to a distinct merged output. This module
//! gives that save destination its own type — [`SaveTargetSnapshot`] — so a
//! tab can never confuse "what I'm comparing" with "what I'll write to."
//!
//! [`save_target_from_loaded`] derives a snapshot from an already-loaded
//! document (normal compare: the target *is* the right input, already read).
//! [`inspect_save_target`] independently inspects a path on disk without
//! treating its content as a comparison input (Git mergetool: the merged
//! output is never the diffed remote).
//!
//! [`PreparedCompare`] is the type the blocking preparation step will return
//! (RFC-077 "Prepared result") — declared here now so callers can be written
//! against it; the refactor of `forskscope-ui`'s `load_and_diff` to actually
//! construct one is a later patch in the same milestone.

use std::path::{Path, PathBuf};

use crate::diff::DiffDocument;
use crate::document::{FileFingerprint, LoadOptions, LoadedDocument, load_path};
use crate::file_kind::{FileKind, classify};
use crate::merge::MergeSession;

/// The result of blocking comparison preparation: loaded documents, the
/// computed diff, a fresh merge session, and the save target — committed
/// together so a tab is never left with mismatched pieces (RFC-075 token
/// discipline governs *when* this is installed; this type only describes
/// *what* is installed).
#[derive(Debug, Clone)]
pub struct PreparedCompare {
    pub left: LoadedDocument,
    pub right: LoadedDocument,
    pub diff: DiffDocument,
    pub merge: MergeSession,
    pub save_target: SaveTargetSnapshot,
    pub can_save: bool,
}

/// Where a save will go, and whether it's currently possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveTargetSnapshot {
    pub path: PathBuf,
    pub state: SaveTargetState,
}

/// Whether the save target accepts a write right now, and under what
/// precondition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveTargetState {
    Writable {
        expectation: TargetExpectation,
        encoding_label: String,
    },
    Blocked {
        reason: SaveTargetBlockReason,
    },
}

/// The load-time precondition a save must satisfy. Mirrors
/// [`crate::save::TargetPrecondition`] one-for-one, minus `Force` — a
/// snapshot is never captured with force already decided; `Force` exists
/// only as an explicit, one-time save-time override (RFC-077 "`Force` is
/// constructed only after an explicit overwrite confirmation... never stored
/// as the tab's load-time snapshot").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetExpectation {
    MustMatch(FileFingerprint),
    MustBeAbsent,
}

/// Why a target cannot be written to. None of these are silently replaced —
/// each needs an explicit user choice (Save As, or fixing the target) before
/// any write is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveTargetBlockReason {
    Binary,
    Spreadsheet,
    /// Exists but isn't a plain file (e.g. a directory) — carries
    /// [`FileKind::Unsupported`]'s reason string.
    NotAPlainFile {
        reason: String,
    },
    Unreadable {
        message: String,
    },
}

/// Normal two-file compare: the save target *is* the already-loaded right
/// document — no independent disk inspection, per RFC-077 ("Normal compare
/// derives the snapshot from the loaded right document").
pub fn save_target_from_loaded(path: &Path, doc: &LoadedDocument) -> SaveTargetSnapshot {
    let encoding_label = || {
        doc.text
            .as_ref()
            .map(|t| t.encoding.label.clone())
            .unwrap_or_else(|| "UTF-8".into())
    };
    let state = match &doc.kind {
        FileKind::Text => SaveTargetState::Writable {
            expectation: match doc.fingerprint_at_load {
                Some(fp) => TargetExpectation::MustMatch(fp),
                None => TargetExpectation::MustBeAbsent,
            },
            encoding_label: encoding_label(),
        },
        FileKind::Missing => SaveTargetState::Writable {
            expectation: TargetExpectation::MustBeAbsent,
            encoding_label: encoding_label(),
        },
        FileKind::Binary => blocked(SaveTargetBlockReason::Binary),
        FileKind::ExcelXlsx => blocked(SaveTargetBlockReason::Spreadsheet),
        FileKind::Unsupported { reason } => blocked(SaveTargetBlockReason::NotAPlainFile {
            reason: reason.clone(),
        }),
    };
    SaveTargetSnapshot {
        path: path.to_path_buf(),
        state,
    }
}

/// Git mergetool mode: independently inspects `path` on disk — reading only
/// enough to classify, decode, and fingerprint it — without ever feeding its
/// content into the left/right comparison (RFC-077 "Mergetool target
/// preparation": "Do not use its content as the right-side comparison
/// input"). `fallback_encoding_label` is the remote input's encoding, used
/// as the initial output encoding when the target doesn't exist yet.
pub fn inspect_save_target(path: &Path, fallback_encoding_label: &str) -> SaveTargetSnapshot {
    // Classify first, rather than going through `load_path` alone: its
    // `Unsupported` case (e.g. a directory) surfaces as `Err`, which would
    // otherwise collapse into the same generic `Unreadable` block reason as
    // a genuine I/O failure — RFC-077 wants "replaced by a directory"
    // distinguishable from "target is unreadable."
    let state = match classify(path) {
        Ok(FileKind::Missing) => SaveTargetState::Writable {
            expectation: TargetExpectation::MustBeAbsent,
            encoding_label: fallback_encoding_label.to_string(),
        },
        Ok(FileKind::Text) => {
            match load_path(
                path,
                LoadOptions {
                    allow_missing: false,
                },
            ) {
                Ok(doc) => SaveTargetState::Writable {
                    expectation: TargetExpectation::MustMatch(
                        doc.fingerprint_at_load
                            .expect("a Text load always captures a fingerprint"),
                    ),
                    encoding_label: doc
                        .text
                        .as_ref()
                        .map(|t| t.encoding.label.clone())
                        .unwrap_or_else(|| fallback_encoding_label.to_string()),
                },
                Err(e) => blocked(SaveTargetBlockReason::Unreadable {
                    message: e.to_string(),
                }),
            }
        }
        Ok(FileKind::Binary) => blocked(SaveTargetBlockReason::Binary),
        Ok(FileKind::ExcelXlsx) => blocked(SaveTargetBlockReason::Spreadsheet),
        Ok(FileKind::Unsupported { reason }) => {
            blocked(SaveTargetBlockReason::NotAPlainFile { reason })
        }
        Err(e) => blocked(SaveTargetBlockReason::Unreadable {
            message: e.to_string(),
        }),
    };
    SaveTargetSnapshot {
        path: path.to_path_buf(),
        state,
    }
}

fn blocked(reason: SaveTargetBlockReason) -> SaveTargetState {
    SaveTargetState::Blocked { reason }
}
