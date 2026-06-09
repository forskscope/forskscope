//! Merge session (RFC-006, RFC-007, RFC-015).

use crate::diff::{DiffDocument, DiffRow, HunkId, HunkKind, SideLine};
use crate::error::{CoreError, Result};

use super::transaction::MergeTransaction;

/// Merge state of one working hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkState {
    /// As produced by the diff engine.
    Original,
    /// Left content was applied onto the right side.
    AppliedLeftToRight,
}

/// One hunk in the working (mergeable) document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeHunk {
    pub hunk_id: HunkId,
    pub kind: HunkKind,
    pub state: HunkState,
    pub rows: Vec<DiffRow>,
}

impl MergeHunk {
    pub fn is_pending_change(&self) -> bool {
        self.kind.is_change() && self.state == HunkState::Original
    }
}

/// The canonical owner of merge state for one compare session.
///
/// Dirty state is derived from the number of net transactions since the
/// last save baseline; it is never inferred from rendered UI content.
#[derive(Debug, Clone)]
pub struct MergeSession {
    diff_id: u64,
    hunks: Vec<MergeHunk>,
    undo_stack: Vec<MergeTransaction>,
    redo_stack: Vec<MergeTransaction>,
    /// `undo_stack.len()` at the moment of the last successful save.
    saved_baseline: usize,
}

impl MergeSession {
    /// Build a working session from a freshly computed diff.
    pub fn from_diff(diff: &DiffDocument) -> Self {
        let hunks = diff
            .hunks
            .iter()
            .map(|h| MergeHunk {
                hunk_id: h.hunk_id,
                kind: h.kind,
                state: HunkState::Original,
                rows: h.rows.clone(),
            })
            .collect();
        Self {
            diff_id: diff.diff_id,
            hunks,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_baseline: 0,
        }
    }

    pub fn diff_id(&self) -> u64 {
        self.diff_id
    }

    pub fn hunks(&self) -> &[MergeHunk] {
        &self.hunks
    }

    /// `true` when the working result differs from the last saved state.
    pub fn is_dirty(&self) -> bool {
        self.undo_stack.len() != self.saved_baseline
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Number of changed hunks not yet applied.
    pub fn pending_changes(&self) -> usize {
        self.hunks.iter().filter(|h| h.is_pending_change()).count()
    }

    /// Apply a hunk's left content onto the right side.
    ///
    /// Stale or unknown hunk IDs are rejected (RFC-002 §6); applying an
    /// equal or already-applied hunk is a no-op error so the UI cannot
    /// silently double-apply.
    pub fn apply_left_to_right(&mut self, hunk_id: HunkId) -> Result<()> {
        let hunk = self
            .hunks
            .iter_mut()
            .find(|h| h.hunk_id == hunk_id)
            .ok_or(CoreError::Conflict {
                message: "unknown or stale hunk id".into(),
            })?;
        if !hunk.is_pending_change() {
            return Err(CoreError::Conflict {
                message: "hunk has no pending change".into(),
            });
        }
        let transaction = MergeTransaction {
            hunk_id,
            previous_rows: hunk.rows.clone(),
            previous_kind: hunk.kind,
            previous_state: hunk.state,
        };
        // Replace right side with left side content; rows with no left
        // counterpart disappear from the result.
        let mut rows: Vec<DiffRow> = Vec::with_capacity(hunk.rows.len());
        for row in &hunk.rows {
            if let Some(left) = &row.left {
                rows.push(DiffRow {
                    left: Some(left.clone()),
                    right: Some(SideLine {
                        original_line_number: None,
                        content: left.content.clone(),
                        newline: left.newline,
                    }),
                    inline: None,
                });
            } else {
                rows.push(DiffRow {
                    left: None,
                    right: None,
                    inline: None,
                });
            }
        }
        hunk.rows = rows;
        hunk.state = HunkState::AppliedLeftToRight;
        self.undo_stack.push(transaction);
        self.redo_stack.clear();
        Ok(())
    }

    /// Undo the most recent merge operation.
    pub fn undo(&mut self) -> Result<HunkId> {
        let transaction = self.undo_stack.pop().ok_or(CoreError::Conflict {
            message: "nothing to undo".into(),
        })?;
        let hunk_id = transaction.hunk_id;
        let redo = self.swap_in(transaction)?;
        self.redo_stack.push(redo);
        Ok(hunk_id)
    }

    /// Redo the most recently undone merge operation.
    pub fn redo(&mut self) -> Result<HunkId> {
        let transaction = self.redo_stack.pop().ok_or(CoreError::Conflict {
            message: "nothing to redo".into(),
        })?;
        let hunk_id = transaction.hunk_id;
        let undo = self.swap_in(transaction)?;
        self.undo_stack.push(undo);
        Ok(hunk_id)
    }

    /// Install a transaction's stored hunk state, returning the inverse.
    fn swap_in(&mut self, transaction: MergeTransaction) -> Result<MergeTransaction> {
        let hunk = self
            .hunks
            .iter_mut()
            .find(|h| h.hunk_id == transaction.hunk_id)
            .ok_or(CoreError::InternalInvariant {
                message: "transaction references missing hunk".into(),
            })?;
        let inverse = MergeTransaction {
            hunk_id: transaction.hunk_id,
            previous_rows: std::mem::replace(&mut hunk.rows, transaction.previous_rows),
            previous_kind: std::mem::replace(&mut hunk.kind, transaction.previous_kind),
            previous_state: std::mem::replace(&mut hunk.state, transaction.previous_state),
        };
        Ok(inverse)
    }

    /// Mark the current state as saved; dirty becomes `false`.
    pub fn mark_saved(&mut self) {
        self.saved_baseline = self.undo_stack.len();
        // Redo across a save boundary would desynchronize the baseline.
        self.redo_stack.clear();
    }

    /// Canonical right-side result text, reconstructed from the working
    /// hunks with original newline markers preserved. This — not any UI
    /// buffer — is what the save layer writes.
    pub fn result_text(&self) -> String {
        let mut out = String::new();
        for hunk in &self.hunks {
            for row in &hunk.rows {
                if let Some(right) = &row.right {
                    out.push_str(&right.content);
                    out.push_str(right.newline.as_str());
                }
            }
        }
        out
    }
}
