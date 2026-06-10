//! Transaction log — uniform history model for merge operations (RFC-015).
//!
//! Both the two-way [`MergeSession`](super::MergeSession) and the three-way
//! [`ThreeWayMergeSession`](super::ThreeWayMergeSession) maintain their own
//! private undo/redo stacks. This module adds the *enrichment* layer RFC-015
//! calls for: typed [`TransactionKind`], user-visible labels, timestamps,
//! and a queryable [`TransactionLog`] that records every operation in session
//! order.
//!
//! The design rule is **additive**: the existing session stacks and their
//! undo/redo semantics are unchanged. [`TransactionLog`] is a companion
//! struct you attach to a session to gain history introspection, replay
//! capability, and the clean-baseline revision concept — without coupling
//! the session internals to the log.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::diff::HunkId;
use crate::merge::three_way::ConflictId;

// ── Types ─────────────────────────────────────────────────────────────────────

/// A monotonically increasing revision counter for a session.
///
/// Revision 0 is the initial (pre-any-operation) state. Each committed
/// operation increments the revision. The save baseline is captured as
/// a `SessionRevision` so dirty-state can be computed as
/// `current > save_baseline`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct SessionRevision(pub u64);

impl SessionRevision {
    pub const INITIAL: Self = Self(0);

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl std::fmt::Display for SessionRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r{}", self.0)
    }
}

/// Which merge operation was performed (RFC-015 §"Transaction Kinds").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionKind {
    /// Copied one hunk from the left/old side into the right/new side.
    ApplyHunkLeftToRight { hunk_id: HunkId },
    /// Reverted one previously applied hunk back to original diff state.
    RevertHunk { hunk_id: HunkId },
    /// Copied all pending hunks left-to-right in one operation.
    ApplyAllLeftToRight,
    /// A three-way conflict was resolved by choosing the left side.
    ResolveConflictLeft { conflict_id: ConflictId },
    /// A three-way conflict was resolved by choosing the right side.
    ResolveConflictRight { conflict_id: ConflictId },
    /// A three-way conflict was resolved by taking both sides (left then right).
    ResolveConflictBoth { conflict_id: ConflictId },
    /// A three-way conflict was resolved with user-supplied text.
    ResolveConflictManual { conflict_id: ConflictId },
    /// A three-way conflict was ignored (base content used).
    IgnoreConflict { conflict_id: ConflictId },
    /// A three-way conflict was reopened (reset to unresolved).
    ReopenConflict { conflict_id: ConflictId },
    /// A manual text edit was committed to the result buffer.
    ManualTextEdit,
    /// An externally applied patch was accepted.
    ApplyExternalPatch,
}

impl TransactionKind {
    /// Short human-readable description for the history panel (English).
    /// Localization maps these through the i18n layer.
    pub fn label(&self) -> String {
        match self {
            Self::ApplyHunkLeftToRight { hunk_id } =>
                format!("Apply hunk #{hunk_id}"),
            Self::RevertHunk { hunk_id } =>
                format!("Revert hunk #{hunk_id}"),
            Self::ApplyAllLeftToRight =>
                "Apply all changes".into(),
            Self::ResolveConflictLeft { conflict_id } =>
                format!("Resolve conflict #{conflict_id} — use left"),
            Self::ResolveConflictRight { conflict_id } =>
                format!("Resolve conflict #{conflict_id} — use right"),
            Self::ResolveConflictBoth { conflict_id } =>
                format!("Resolve conflict #{conflict_id} — use both"),
            Self::ResolveConflictManual { conflict_id } =>
                format!("Resolve conflict #{conflict_id} — manual edit"),
            Self::IgnoreConflict { conflict_id } =>
                format!("Ignore conflict #{conflict_id}"),
            Self::ReopenConflict { conflict_id } =>
                format!("Reopen conflict #{conflict_id}"),
            Self::ManualTextEdit =>
                "Manual edit".into(),
            Self::ApplyExternalPatch =>
                "Apply external patch".into(),
        }
    }
}

/// Unix seconds since epoch — lightweight timestamp, no chrono dependency here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct UnixTimestamp(pub u64);

impl UnixTimestamp {
    pub fn now() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        Self(secs)
    }
}

/// One entry in the transaction log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionEntry {
    /// The revision *after* this transaction was applied.
    pub revision:  SessionRevision,
    pub kind:      TransactionKind,
    /// Human-readable label (same as `kind.label()` unless overridden).
    pub label:     String,
    pub timestamp: UnixTimestamp,
    /// Whether this entry is currently on the undo stack (true) or was
    /// popped to the redo stack (false). The log keeps all entries even
    /// after undo so the history panel can show what was undone.
    pub active:    bool,
}

// ── TransactionLog ────────────────────────────────────────────────────────────

/// A queryable log of every merge operation performed in a session.
///
/// Usage pattern:
///
/// ```rust,no_run
/// # use forskscope_core::merge::{TransactionLog, TransactionKind};
/// let mut log = TransactionLog::new();
/// // …after each session.apply_left_to_right(id) call:
/// log.push(TransactionKind::ApplyHunkLeftToRight { hunk_id: 1 });
/// // …after session.undo():
/// log.record_undo();
/// // …after session.mark_saved():
/// log.mark_saved();
/// ```
///
/// The log is independent of the session stacks so it can be attached to
/// either session type without modifying them.
#[derive(Debug, Clone)]
pub struct TransactionLog {
    entries:       Vec<TransactionEntry>,
    current_rev:   SessionRevision,
    saved_rev:     SessionRevision,
    /// Pointer into `entries` for the "current" position (top of undo stack).
    /// Points past the last active entry; entries at or above this index have
    /// been undone (active == false).
    active_cursor: usize,
}

impl Default for TransactionLog {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionLog {
    pub fn new() -> Self {
        Self {
            entries:       Vec::new(),
            current_rev:   SessionRevision::INITIAL,
            saved_rev:     SessionRevision::INITIAL,
            active_cursor: 0,
        }
    }

    // ── Mutation ──────────────────────────────────────────────────────────────

    /// Record a new operation. Clears any undone (redo) entries above the
    /// current cursor, consistent with undo semantics (new operation after
    /// undo discards the redo branch).
    pub fn push(&mut self, kind: TransactionKind) {
        // Discard undone entries beyond the cursor.
        self.entries.truncate(self.active_cursor);
        self.current_rev = self.current_rev.next();
        let label = kind.label();
        self.entries.push(TransactionEntry {
            revision:  self.current_rev,
            kind,
            label,
            timestamp: UnixTimestamp::now(),
            active:    true,
        });
        self.active_cursor = self.entries.len();
    }

    /// Record that the most recent active operation was undone.
    /// Marks the entry as inactive and steps the cursor back.
    pub fn record_undo(&mut self) {
        if self.active_cursor == 0 {
            return; // nothing to undo
        }
        self.active_cursor -= 1;
        if let Some(e) = self.entries.get_mut(self.active_cursor) {
            e.active = false;
        }
        self.current_rev = if self.active_cursor == 0 {
            SessionRevision::INITIAL
        } else {
            self.entries[self.active_cursor - 1].revision
        };
    }

    /// Record that the most recently undone operation was redone.
    /// Marks the entry as active again and advances the cursor.
    pub fn record_redo(&mut self) {
        if self.active_cursor >= self.entries.len() {
            return; // nothing to redo
        }
        if let Some(e) = self.entries.get_mut(self.active_cursor) {
            e.active = true;
            self.current_rev = e.revision;
        }
        self.active_cursor += 1;
    }

    /// Mark the current revision as the save baseline.
    /// `is_dirty()` will return `false` immediately after this call.
    pub fn mark_saved(&mut self) {
        self.saved_rev = self.current_rev;
    }

    // ── Query ─────────────────────────────────────────────────────────────────

    /// `true` when the current revision differs from the last saved revision.
    pub fn is_dirty(&self) -> bool {
        self.current_rev != self.saved_rev
    }

    pub fn current_revision(&self) -> SessionRevision {
        self.current_rev
    }

    pub fn saved_revision(&self) -> SessionRevision {
        self.saved_rev
    }

    pub fn can_undo(&self) -> bool {
        self.active_cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.active_cursor < self.entries.len()
    }

    /// Total number of entries ever pushed (including undone ones).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All entries, including undone ones (for history panel display).
    pub fn all_entries(&self) -> &[TransactionEntry] {
        &self.entries
    }

    /// Only the currently active entries (i.e. those on the logical undo
    /// stack, in chronological order).
    pub fn active_entries(&self) -> &[TransactionEntry] {
        &self.entries[..self.active_cursor]
    }

    /// Entries that have been undone and are available to redo.
    pub fn undone_entries(&self) -> &[TransactionEntry] {
        &self.entries[self.active_cursor..]
    }

    /// The most recent active entry, if any.
    pub fn last_active(&self) -> Option<&TransactionEntry> {
        if self.active_cursor == 0 {
            None
        } else {
            self.entries.get(self.active_cursor - 1)
        }
    }

    /// Number of active operations since the last save (i.e. operations
    /// that contribute to `is_dirty()`).
    pub fn active_ops_since_save(&self) -> usize {
        self.active_entries()
            .iter()
            .filter(|e| e.revision > self.saved_rev)
            .count()
    }
}
