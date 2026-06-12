//! Text editing operation model (RFC-032).
//!
//! Defines the types that flow between the editor adapter and the core when
//! the user makes manual text edits or applies merge commands. Core owns text
//! truth; the editor adapter reports operations; core acknowledges or rejects
//! them.
//!
//! ## Operation rules (RFC-032 §"Operation Rules")
//!
//! 1. The editor sends an operation tagged with the document revision it observed.
//! 2. Core accepts the operation only if `base_revision` matches the current
//!    document revision.
//! 3. On acceptance core applies the edit, bumps the revision, and returns
//!    [`OperationAck`].
//! 4. The UI updates editor state from the acknowledged revision.
//! 5. On conflict (stale `base_revision`) core returns [`OperationReject`];
//!    the editor adapter must reconcile with the current document state.
//!
//! ## What is not in this module
//!
//! - The actual text buffer (`String` / rope). `EditBuffer` in the UI state
//!   layer owns that; this module owns only the operation descriptors.
//! - File I/O. `save_text` in `save.rs` handles that.
//! - Diff recomputation scheduling. The `diff_invalidated` flag in
//!   [`OperationAck`] signals the UI to trigger a fresh diff; the timing is
//!   the UI layer's responsibility.

use std::time::SystemTime;

// ── Identity types ────────────────────────────────────────────────────────────

/// Identifies one document in the editing session.
///
/// Stable for the lifetime of the tab; regenerated on reload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentId(pub String);

impl DocumentId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// Monotonically increasing document revision.
///
/// Starts at 0 (clean load). Incremented on every accepted operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RevisionId(pub u64);

impl RevisionId {
    pub fn initial() -> Self { Self(0) }
    pub fn next(self) -> Self { Self(self.0 + 1) }
    pub fn is_initial(self) -> bool { self.0 == 0 }
}

/// A unique identifier for one edit transaction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransactionId(pub String);

impl TransactionId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
}

// ── Position types ────────────────────────────────────────────────────────────

/// A byte-offset position within a document's text content.
///
/// Byte offsets are used throughout because Rust string indexing is
/// byte-based. Callers must ensure offsets fall on character boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TextOffset(pub usize);

impl TextOffset {
    pub fn zero() -> Self { Self(0) }
    pub fn as_usize(self) -> usize { self.0 }
}

/// A byte-range within a document (start inclusive, end exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: TextOffset,
    pub end:   TextOffset,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "TextRange start must be ≤ end");
        Self { start: TextOffset(start), end: TextOffset(end) }
    }

    pub fn empty_at(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    pub fn len(self) -> usize {
        self.end.0.saturating_sub(self.start.0)
    }

    pub fn is_empty(self) -> bool { self.start == self.end }

    /// `true` when `offset` falls within this range (start inclusive, end exclusive).
    pub fn contains(self, offset: TextOffset) -> bool {
        offset >= self.start && offset < self.end
    }

    /// `true` when this range overlaps `other`.
    pub fn overlaps(self, other: TextRange) -> bool {
        self.start < other.end && other.start < self.end
    }
}

// ── Edit operation ────────────────────────────────────────────────────────────

/// One atomic text edit sent from the editor adapter to core (RFC-032 §"Core Types").
///
/// All operations are tagged with the `base_revision` the editor observed.
/// Core rejects the operation if its current revision differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEditOperation {
    /// Insert `text` at `offset`.
    Insert {
        document:      DocumentId,
        base_revision: RevisionId,
        offset:        TextOffset,
        text:          String,
    },
    /// Delete the text in `range`.
    Delete {
        document:      DocumentId,
        base_revision: RevisionId,
        range:         TextRange,
    },
    /// Replace the text in `range` with `text`.
    Replace {
        document:      DocumentId,
        base_revision: RevisionId,
        range:         TextRange,
        text:          String,
    },
}

impl TextEditOperation {
    pub fn document_id(&self) -> &DocumentId {
        match self {
            Self::Insert { document, .. }  => document,
            Self::Delete { document, .. }  => document,
            Self::Replace { document, .. } => document,
        }
    }

    pub fn base_revision(&self) -> RevisionId {
        match self {
            Self::Insert { base_revision, .. }  => *base_revision,
            Self::Delete { base_revision, .. }  => *base_revision,
            Self::Replace { base_revision, .. } => *base_revision,
        }
    }

    /// The byte range this operation affects in the *pre-edit* text.
    pub fn affected_range(&self) -> TextRange {
        match self {
            Self::Insert { offset, .. } => TextRange::empty_at(offset.0),
            Self::Delete { range, .. }  => *range,
            Self::Replace { range, .. } => *range,
        }
    }

    /// Whether this operation inserts any text.
    pub fn inserts_text(&self) -> bool {
        match self {
            Self::Insert { .. } => true,
            Self::Replace { text, .. } => !text.is_empty(),
            Self::Delete { .. } => false,
        }
    }

    /// Whether this operation deletes any text.
    pub fn deletes_text(&self) -> bool {
        match self {
            Self::Delete { range, .. } => !range.is_empty(),
            Self::Replace { range, .. } => !range.is_empty(),
            Self::Insert { .. } => false,
        }
    }
}

// ── Operation outcome ─────────────────────────────────────────────────────────

/// Core's acceptance response to a [`TextEditOperation`] (RFC-032 §"Operation Rules").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationAck {
    pub document:        DocumentId,
    pub new_revision:    RevisionId,
    /// The range that changed in the *post-edit* text.
    pub affected_range:  TextRange,
    /// `true` when the edit changes content that the current diff was computed
    /// from, so the UI should schedule a diff recomputation.
    pub diff_invalidated: bool,
}

/// Core's rejection response — the editor must reconcile with `current_revision`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationReject {
    pub document:         DocumentId,
    pub submitted_revision: RevisionId,
    pub current_revision:   RevisionId,
    pub reason:           RejectReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// The `base_revision` is stale — the document was modified after the
    /// editor took its snapshot.
    StaleRevision,
    /// The range or offset falls outside the current document length.
    OutOfBounds,
    /// The document identified by `document_id` is not open or is read-only.
    DocumentNotEditable,
}

/// The result of submitting a [`TextEditOperation`] to core.
pub type OperationResult = Result<OperationAck, OperationReject>;

// ── Transaction model ─────────────────────────────────────────────────────────

/// Human-readable label for an edit transaction (for undo menu display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionLabel(pub String);

impl TransactionLabel {
    pub fn new(label: impl Into<String>) -> Self { Self(label.into()) }
    pub fn as_str(&self) -> &str { &self.0 }

    // ── Well-known labels ──────────────────────────────────────────────────
    pub fn merge_hunk_left_to_right() -> Self { Self::new("Apply hunk left to right") }
    pub fn merge_hunk_right_to_left() -> Self { Self::new("Apply hunk right to left") }
    pub fn manual_edit() -> Self { Self::new("Edit") }
    pub fn paste() -> Self { Self::new("Paste") }
    pub fn delete_selection() -> Self { Self::new("Delete") }
}

/// A group of operations that form a single undo unit (RFC-032 §"Transaction Model").
///
/// Merge commands and manual edits both become transactions so undo/redo
/// remains consistent across user text edits and merge actions.
#[derive(Debug, Clone)]
pub struct EditTransaction {
    pub id:         TransactionId,
    pub label:      TransactionLabel,
    pub operations: Vec<TextEditOperation>,
    /// The inverse operations that undo this transaction, in reverse order.
    pub inverse:    Vec<TextEditOperation>,
    pub timestamp:  SystemTime,
}

impl EditTransaction {
    pub fn new(
        id:         TransactionId,
        label:      TransactionLabel,
        operations: Vec<TextEditOperation>,
        inverse:    Vec<TextEditOperation>,
    ) -> Self {
        Self {
            id, label, operations, inverse,
            timestamp: SystemTime::now(),
        }
    }

    /// `true` when this transaction contains at least one operation.
    pub fn is_empty(&self) -> bool { self.operations.is_empty() }

    /// `true` when this transaction can be undone (has inverse operations).
    pub fn is_reversible(&self) -> bool { !self.inverse.is_empty() }
}

// ── Revision compatibility check ──────────────────────────────────────────────

/// Check whether `op_revision` is compatible with `current_revision`.
///
/// An operation is compatible when its `base_revision` exactly matches the
/// current document revision. "Last-write-wins" or "any past revision" are
/// intentionally not supported — the editor must always reconcile before
/// retrying.
pub fn is_revision_compatible(op_revision: RevisionId, current_revision: RevisionId) -> bool {
    op_revision == current_revision
}
