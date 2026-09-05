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
use crate::encoding::BomPresence;
use crate::file_kind::{EditabilityClass, FileKind, classify};
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
    pub save_capability: SaveCapability,
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
        /// The BOM to preserve on save (RFC-083 §2) — tracked alongside
        /// `encoding_label` for the same reason: both are properties of
        /// "what this target's bytes currently look like," re-derived
        /// whenever the target is (re-)inspected.
        bom: BomPresence,
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
    let bom = || doc.text.as_ref().map(|t| t.bom).unwrap_or_default();
    let state = match &doc.kind {
        FileKind::Text => SaveTargetState::Writable {
            expectation: match doc.fingerprint_at_load {
                Some(fp) => TargetExpectation::MustMatch(fp),
                None => TargetExpectation::MustBeAbsent,
            },
            encoding_label: encoding_label(),
            bom: bom(),
        },
        FileKind::Missing => SaveTargetState::Writable {
            expectation: TargetExpectation::MustBeAbsent,
            encoding_label: encoding_label(),
            bom: bom(),
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
/// input"). `fallback_encoding_label`/`fallback_bom` are the remote input's
/// encoding and BOM presence, used as the initial output values when the
/// target doesn't exist yet — the same fallback relationship for both,
/// since both describe "what this target's bytes currently look like."
pub fn inspect_save_target(
    path: &Path,
    fallback_encoding_label: &str,
    fallback_bom: BomPresence,
) -> SaveTargetSnapshot {
    // Classify first, rather than going through `load_path` alone: its
    // `Unsupported` case (e.g. a directory) surfaces as `Err`, which would
    // otherwise collapse into the same generic `Unreadable` block reason as
    // a genuine I/O failure — RFC-077 wants "replaced by a directory"
    // distinguishable from "target is unreadable."
    let state = match classify(path) {
        Ok(FileKind::Missing) => SaveTargetState::Writable {
            expectation: TargetExpectation::MustBeAbsent,
            encoding_label: fallback_encoding_label.to_string(),
            bom: fallback_bom,
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
                    bom: doc.text.as_ref().map(|t| t.bom).unwrap_or(fallback_bom),
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

// ── F88/RFC-082 §D3: one source of truth for whether a save is possible ───

/// Whether the merge result derived from two loaded sides can be written to
/// the resolved save target — composed from the sides' [`FileKind`], their
/// [`EditabilityClass`], and the target's [`SaveTargetState`]. Never
/// assembled as a boolean at the call site; [`save_capability`] is the one
/// function that answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveCapability {
    /// The merge result can be written directly.
    Saveable,
    /// At least one loaded side needed replacement characters at decode
    /// time — the merge result in memory cannot reproduce that side's
    /// original bytes, and no save-time encoding choice can fix it (unlike
    /// a merely non-UTF-8 encoding with a *clean* decode, which
    /// `save::save_text` already checks precisely at save time —
    /// F87/RFC-082 §D4). Saving must be blocked and explained, never
    /// attempted silently.
    SaveableWithGuard,
    /// Cannot be saved at all, and why.
    Blocked(SaveCapabilityBlockReason),
}

impl SaveCapability {
    /// `true` for `Saveable` and `SaveableWithGuard` — a save can be
    /// attempted (edited, toolbar shown) either way; only `Blocked` means no.
    pub fn is_saveable(&self) -> bool {
        !matches!(self, Self::Blocked(_))
    }

    /// `true` only for `SaveableWithGuard` — the caller must block the
    /// write and explain, rather than calling `save_text`.
    pub fn requires_guard(&self) -> bool {
        matches!(self, Self::SaveableWithGuard)
    }
}

/// Why [`SaveCapability::Blocked`] applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveCapabilityBlockReason {
    /// One or both sides cannot be represented as mergeable text at all
    /// (binary, spreadsheet, or otherwise unsupported). `Missing` is not
    /// this — it is empty text, not an unsupported kind (RFC-082 §D3 §3).
    NotMergeableText,
    /// The resolved save target itself cannot be written to.
    Target(SaveTargetBlockReason),
}

/// Composes the three facts (RFC-082 §D3): both sides' [`FileKind`] decide
/// whether a text merge is possible at all; both sides' [`EditabilityClass`]
/// — together with the `had_decode_errors` each was derived from — decide
/// whether saving must be blocked and explained; `target_state` decides
/// whether the resolved destination can be written to at all.
///
/// `EditabilityClass::from_kind` maps `Missing -> ReadOnly` — correct for a
/// document (there is nothing to edit), but wrong for this question, and
/// that mapping is not changed here. A missing side is empty text: it
/// contributes no content and needs no guard, carved out explicitly below
/// rather than by touching `from_kind`.
///
/// **Why `had_decode_errors` is checked directly, not just
/// `requires_save_guard()`:** that predicate is `true` for two situations
/// `from_kind` deliberately does not distinguish — decode substitution
/// (`had_decode_errors`, unrecoverable: the original bytes are already gone
/// from memory, and no save-time choice restores them) and a merely
/// non-UTF-8 encoding that decoded *cleanly* (recoverable per-character,
/// already checked precisely at save time by `save::save_text` — F87). Both
/// map to the same `ReadWriteWithGuard` value, so using `requires_save_guard()`
/// alone here would also block the second case — the common case for
/// nearly every legacy-encoded file that decoded without error — always,
/// unconditionally, with no escape, even when a save of it would round-trip
/// perfectly. Only the first is this function's concern. The
/// `debug_assert!` below documents (and checks, in every debug build and
/// test run) that `had_decode_errors` is always a genuine subset of
/// `requires_save_guard()` — not an unrelated condition smuggled in instead
/// of it.
pub fn save_capability(
    left_kind: &FileKind,
    right_kind: &FileKind,
    left_editability: EditabilityClass,
    right_editability: EditabilityClass,
    left_had_decode_errors: bool,
    right_had_decode_errors: bool,
    target_state: &SaveTargetState,
) -> SaveCapability {
    if let SaveTargetState::Blocked { reason } = target_state {
        return SaveCapability::Blocked(SaveCapabilityBlockReason::Target(reason.clone()));
    }

    let mergeable = |kind: &FileKind| kind.is_mergeable_text() || matches!(kind, FileKind::Missing);
    if !mergeable(left_kind) || !mergeable(right_kind) {
        return SaveCapability::Blocked(SaveCapabilityBlockReason::NotMergeableText);
    }

    let needs_guard = |kind: &FileKind, editability: EditabilityClass, had_decode_errors: bool| {
        if matches!(kind, FileKind::Missing) {
            // Empty text: nothing was decoded, so nothing could have
            // needed a replacement character.
            return false;
        }
        debug_assert!(
            !had_decode_errors || editability.requires_save_guard(),
            "had_decode_errors=true must always imply requires_save_guard()"
        );
        had_decode_errors
    };

    if needs_guard(left_kind, left_editability, left_had_decode_errors)
        || needs_guard(right_kind, right_editability, right_had_decode_errors)
    {
        SaveCapability::SaveableWithGuard
    } else {
        SaveCapability::Saveable
    }
}
