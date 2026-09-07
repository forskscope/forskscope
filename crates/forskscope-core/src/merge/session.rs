//! Merge session (RFC-006, RFC-007, RFC-015).

use crate::diff::{DiffDocument, DiffRow, HunkId, HunkKind, SideLine};
use crate::error::{CoreError, Result};
use crate::fnv1a64;

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
/// Dirty state is content identity, not undo-stack depth (F86/RFC-082 §D1):
/// `is_dirty()` compares a hash of the current [`result_text`](Self::result_text)
/// against a hash of what was last saved, so an undo that lands the buffer
/// back on the saved text reports clean, and a save-then-undo-then-different-
/// edit reports dirty, in both cases regardless of how deep the stack is.
///
/// Review 086 §3: `is_dirty()` is read once per tab on every render
/// (`ui/layout/tabs.rs`, `ui/layout/statusbar.rs`), so it must be O(1) — the
/// hash of the *current* content is therefore maintained incrementally by
/// every mutator (`apply_left_to_right`, and `swap_in`, which both `undo`
/// and `redo` route through), not recomputed from `result_text()` on read.
#[derive(Debug, Clone)]
pub struct MergeSession {
    hunks: Vec<MergeHunk>,
    undo_stack: Vec<MergeTransaction>,
    redo_stack: Vec<MergeTransaction>,
    /// FNV-1a 64-bit hash of [`result_text`](Self::result_text) as it
    /// stands right now — kept in sync by every mutator, so `is_dirty()`
    /// never needs to walk the hunks. Same hash this module's sibling
    /// already trusts for conflict identity
    /// (`three_way::session::conflict_id_for`).
    current_hash: u64,
    /// `current_hash`'s value at the moment of the last successful save (or
    /// at construction, if never saved) — content identity, not
    /// undo-stack depth.
    saved_hash: u64,
}

impl MergeSession {
    /// An empty placeholder used while a tab is in the Loading state (RFC-065).
    pub fn empty() -> Self {
        let hash = fnv1a64(b"");
        Self {
            hunks: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_hash: hash,
            saved_hash: hash,
        }
    }

    /// Build a working session from a freshly computed diff.
    pub fn from_diff(diff: &DiffDocument) -> Self {
        let hunks: Vec<MergeHunk> = diff
            .hunks
            .iter()
            .map(|h| MergeHunk {
                hunk_id: h.hunk_id,
                kind: h.kind,
                state: HunkState::Original,
                rows: h.rows.clone(),
            })
            .collect();
        debug_assert!(
            has_unique_hunk_ids(&hunks),
            "hunk ids must be unique within a session (RFC-086 §4) — two \
             hunks in the same DiffDocument hashed to the same id"
        );
        let mut session = Self {
            hunks,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_hash: 0,
            saved_hash: 0,
        };
        session.current_hash = session.content_hash();
        session.saved_hash = session.current_hash;
        session
    }

    fn content_hash(&self) -> u64 {
        fnv1a64(self.result_text().as_bytes())
    }

    pub fn hunks(&self) -> &[MergeHunk] {
        &self.hunks
    }

    /// `true` when every hunk this session currently tracks also exists,
    /// by id, in `diff` (RFC-086 §4/§5's compatibility test). Recomputing
    /// the *same* document under different `DiffOptions` that happen not
    /// to move any hunk boundary yields exactly this: identical ids, in
    /// which case `diff`'s hunks describe the same regions this session's
    /// working state (applied hunks, undo/redo stacks) already refers to,
    /// and can be adopted without rebuilding anything.
    ///
    /// This is deliberately the *whole*-session check, not just the hunks
    /// referenced by the undo/redo log: if even one untouched hunk moved,
    /// `tab.diff` and this session's own `hunks` (which is what actually
    /// renders — see `ui/view/diff.rs`) would describe different
    /// structures, which is a hazard independent of whether anything was
    /// ever applied.
    ///
    /// A `false` result means only "do not adopt `diff` as-is" — it is
    /// never safe to bridge the gap by rebasing the log onto `diff`'s
    /// hunks (RFC-086 §5 rejects heuristic rebasing outright: a wrong
    /// match applies stored rows to a hunk the user did not choose).
    /// Callers that get `false` must discard and rebuild via
    /// [`from_diff`](Self::from_diff), never patch this session in place.
    pub fn is_compatible_with(&self, diff: &DiffDocument) -> bool {
        self.hunks
            .iter()
            .all(|h| diff.hunks.iter().any(|d| d.hunk_id == h.hunk_id))
    }

    /// `true` when the working result differs from the last saved state.
    /// O(1): compares two stored hashes, never walks `hunks`.
    pub fn is_dirty(&self) -> bool {
        self.current_hash != self.saved_hash
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
        let hunk =
            self.hunks
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
        self.current_hash = self.content_hash();
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
    /// Shared by [`undo`](Self::undo) and [`redo`](Self::redo) — both
    /// mutate `hunks` only through here, so refreshing `current_hash` in
    /// this one place keeps it in sync for both callers.
    ///
    /// RFC-086 §3: a missing hunk is reported as [`CoreError::Conflict`],
    /// the same variant [`apply_left_to_right`](Self::apply_left_to_right)
    /// already uses for "unknown or stale hunk id" — not
    /// `InternalInvariant`, which should mean "this cannot happen" rather
    /// than a condition a caller mixing session state could reach.
    fn swap_in(&mut self, transaction: MergeTransaction) -> Result<MergeTransaction> {
        let hunk = self
            .hunks
            .iter_mut()
            .find(|h| h.hunk_id == transaction.hunk_id)
            .ok_or(CoreError::Conflict {
                message: "unknown or stale hunk id".into(),
            })?;
        let inverse = MergeTransaction {
            hunk_id: transaction.hunk_id,
            previous_rows: std::mem::replace(&mut hunk.rows, transaction.previous_rows),
            previous_kind: std::mem::replace(&mut hunk.kind, transaction.previous_kind),
            previous_state: std::mem::replace(&mut hunk.state, transaction.previous_state),
        };
        self.current_hash = self.content_hash();
        Ok(inverse)
    }

    /// Mark the current state as saved; dirty becomes `false`.
    pub fn mark_saved(&mut self) {
        // O(1): a save doesn't itself change the content, so current_hash
        // is already correct — no need to recompute it from result_text().
        self.saved_hash = self.current_hash;
        // F86: the baseline is content now, so a redo across this boundary
        // would no longer desynchronize it the way it did against a depth
        // counter — replaying a redone transaction still lands on the same
        // content it always would have. Left unchanged anyway: this handoff
        // is about what "dirty" means, not about redo-after-save behavior,
        // and removing the clear is its own user-visible change with its
        // own risk (e.g. a *different* edit made after this save, then
        // undone, then an old redo entry reapplied out of the edit's
        // context) that deserves its own review rather than riding along.
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

/// `true` when no two hunks share a `hunk_id` — the invariant
/// [`MergeSession::from_diff`] asserts in debug builds (RFC-086 §4).
fn has_unique_hunk_ids(hunks: &[MergeHunk]) -> bool {
    let mut ids: Vec<HunkId> = hunks.iter().map(|h| h.hunk_id).collect();
    ids.sort_unstable();
    ids.windows(2).all(|w| w[0] != w[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffHunk, DiffOptions, DiffStats, LineRange};

    fn hunk_doc(ids: &[HunkId]) -> DiffDocument {
        DiffDocument {
            options: DiffOptions::default(),
            hunks: ids
                .iter()
                .map(|&hunk_id| DiffHunk {
                    hunk_id,
                    kind: HunkKind::Insert,
                    left_range: LineRange::new(1, 0),
                    right_range: LineRange::new(1, 1),
                    rows: Vec::new(),
                })
                .collect(),
            stats: DiffStats::default(),
            warnings: Vec::new(),
        }
    }

    // RFC-086 §6 falsification 2: removing the `debug_assert` in
    // `from_diff` must let a colliding session through silently; with it
    // present, constructing one panics instead.
    #[test]
    #[should_panic(expected = "hunk ids must be unique")]
    fn from_diff_panics_on_duplicate_hunk_ids_in_debug_builds() {
        let doc = hunk_doc(&[1, 1]);
        let _ = MergeSession::from_diff(&doc);
    }

    #[test]
    fn from_diff_accepts_distinct_hunk_ids() {
        let doc = hunk_doc(&[1, 2, 3]);
        let session = MergeSession::from_diff(&doc);
        assert_eq!(session.hunks().len(), 3);
    }

    // RFC-086 §3: `swap_in` — reached only through `undo`/`redo` — must
    // report a stale/missing hunk as a real, named `Conflict`, not
    // `InternalInvariant`. `MergeSession`'s own public API can never
    // produce this state (hunks are never removed after construction, and
    // every transaction is only ever pushed after a successful lookup by
    // id), so this constructs the invalid state directly to test
    // `swap_in`'s own contract in isolation from what callers currently do.
    #[test]
    fn swap_in_reports_a_missing_hunk_as_conflict_not_internal_invariant() {
        let mut session = MergeSession::empty();
        let stale = MergeTransaction {
            hunk_id: 0xdead_beef,
            previous_rows: Vec::new(),
            previous_kind: HunkKind::Insert,
            previous_state: HunkState::Original,
        };
        let result = session.swap_in(stale);
        assert!(
            matches!(result, Err(CoreError::Conflict { .. })),
            "expected Conflict, got {result:?}"
        );
    }
}
