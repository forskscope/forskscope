//! Three-way merge session (RFC-033 §"Data Model", §"Save Policy").
//!
//! The session owns the reconciled region list, structured conflict records
//! with durable IDs, resolution operations, an undo/redo log, and the
//! canonical merged result text. Conflicts are represented as metadata —
//! ForskScope never writes conflict markers into the result unless the user
//! explicitly exports a marker file.

use crate::fnv1a64;

use super::engine::{MergeRegion, RegionKind, diff3};
use super::line::{MergeLine, render_lines, split_lines};

/// Durable identity of one conflict within a session, stable across
/// resolution operations so the UI and undo log can reference it.
pub type ConflictId = u64;

/// Resolution state of one conflict (RFC-033 `ConflictStatus`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictStatus {
    Unresolved,
    ResolvedLeft,
    ResolvedRight,
    /// Left then right, in that order.
    ResolvedBoth,
    /// User-supplied custom content.
    ResolvedManual,
    /// Deliberately skipped; contributes the base content to the result.
    Ignored,
}

impl ConflictStatus {
    pub fn is_resolved(&self) -> bool {
        !matches!(self, Self::Unresolved)
    }
}

/// One conflict region with its three sides and current resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeConflict {
    pub id: ConflictId,
    pub base: Vec<MergeLine>,
    pub left: Vec<MergeLine>,
    pub right: Vec<MergeLine>,
    pub status: ConflictStatus,
    /// Present only when `status == ResolvedManual`.
    pub manual: Option<Vec<MergeLine>>,
}

impl MergeConflict {
    /// Lines this conflict currently contributes to the result.
    fn resolved_content(&self) -> Vec<MergeLine> {
        match &self.status {
            ConflictStatus::Unresolved => Vec::new(),
            ConflictStatus::ResolvedLeft => self.left.clone(),
            ConflictStatus::ResolvedRight => self.right.clone(),
            ConflictStatus::ResolvedBoth => {
                let mut v = self.left.clone();
                v.extend(self.right.clone());
                v
            }
            ConflictStatus::ResolvedManual => self.manual.clone().unwrap_or_default(),
            ConflictStatus::Ignored => self.base.clone(),
        }
    }
}

/// One slice of the final result: either fixed auto-merged content or a
/// reference to a conflict whose content depends on its resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResultSegment {
    Fixed(Vec<MergeLine>),
    Conflict(ConflictId),
}

/// A reversible resolution operation for undo/redo.
#[derive(Debug, Clone)]
struct ResolutionTransaction {
    conflict_id: ConflictId,
    previous_status: ConflictStatus,
    previous_manual: Option<Vec<MergeLine>>,
}

/// Aggregate three-way merge statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThreeWayStats {
    pub regions_total: usize,
    pub auto_merged: usize,
    pub conflicts_total: usize,
    pub conflicts_unresolved: usize,
}

/// The canonical owner of three-way merge state for one session.
#[derive(Debug, Clone)]
pub struct ThreeWayMergeSession {
    segments: Vec<ResultSegment>,
    conflicts: Vec<MergeConflict>,
    auto_merged: usize,
    regions_total: usize,
    undo_stack: Vec<ResolutionTransaction>,
    redo_stack: Vec<ResolutionTransaction>,
    /// `undo_stack.len()` at the last save baseline.
    saved_baseline: usize,
}

impl ThreeWayMergeSession {
    /// Build a session from three source texts.
    pub fn from_texts(base: &str, left: &str, right: &str) -> Self {
        let base_lines = split_lines(base);
        let left_lines = split_lines(left);
        let right_lines = split_lines(right);
        let regions = diff3(&base_lines, &left_lines, &right_lines);
        Self::from_regions(regions)
    }

    fn from_regions(regions: Vec<MergeRegion>) -> Self {
        let regions_total = regions.len();
        let mut segments = Vec::with_capacity(regions.len());
        let mut conflicts = Vec::new();
        let mut auto_merged = 0usize;
        let mut conflict_ordinal = 0u64;

        for region in regions {
            if region.kind.is_conflict() {
                let id = conflict_id_for(conflict_ordinal, &region);
                conflict_ordinal += 1;
                conflicts.push(MergeConflict {
                    id,
                    base: region.base,
                    left: region.left,
                    right: region.right,
                    status: ConflictStatus::Unresolved,
                    manual: None,
                });
                segments.push(ResultSegment::Conflict(id));
            } else {
                if region.kind.is_change() {
                    auto_merged += 1;
                }
                segments.push(ResultSegment::Fixed(region.auto_content()));
            }
        }

        Self {
            segments,
            conflicts,
            auto_merged,
            regions_total,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_baseline: 0,
        }
    }

    pub fn conflicts(&self) -> &[MergeConflict] {
        &self.conflicts
    }

    pub fn conflict(&self, id: ConflictId) -> Option<&MergeConflict> {
        self.conflicts.iter().find(|c| c.id == id)
    }

    pub fn stats(&self) -> ThreeWayStats {
        ThreeWayStats {
            regions_total: self.regions_total,
            auto_merged: self.auto_merged,
            conflicts_total: self.conflicts.len(),
            conflicts_unresolved: self.unresolved_count(),
        }
    }

    pub fn unresolved_count(&self) -> usize {
        self.conflicts
            .iter()
            .filter(|c| !c.status.is_resolved())
            .count()
    }

    /// `true` when every conflict has been resolved or ignored.
    pub fn is_fully_resolved(&self) -> bool {
        self.unresolved_count() == 0
    }

    /// RFC-033 save policy: a direct save is blocked while any conflict
    /// remains unresolved. The save layer consults this before writing.
    pub fn can_save(&self) -> bool {
        self.is_fully_resolved()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.undo_stack.len() != self.saved_baseline
    }

    /// Resolve a conflict by choosing the left side.
    pub fn resolve_left(&mut self, id: ConflictId) -> crate::Result<()> {
        self.set_status(id, ConflictStatus::ResolvedLeft, None)
    }

    /// Resolve a conflict by choosing the right side.
    pub fn resolve_right(&mut self, id: ConflictId) -> crate::Result<()> {
        self.set_status(id, ConflictStatus::ResolvedRight, None)
    }

    /// Resolve a conflict by taking left then right.
    pub fn resolve_both(&mut self, id: ConflictId) -> crate::Result<()> {
        self.set_status(id, ConflictStatus::ResolvedBoth, None)
    }

    /// Ignore a conflict, taking the base content.
    pub fn ignore(&mut self, id: ConflictId) -> crate::Result<()> {
        self.set_status(id, ConflictStatus::Ignored, None)
    }

    /// Resolve a conflict with user-supplied text.
    pub fn resolve_manual(&mut self, id: ConflictId, text: &str) -> crate::Result<()> {
        let lines = split_lines(text);
        self.set_status(id, ConflictStatus::ResolvedManual, Some(lines))
    }

    /// Revert a conflict to unresolved.
    pub fn reset(&mut self, id: ConflictId) -> crate::Result<()> {
        self.set_status(id, ConflictStatus::Unresolved, None)
    }

    fn set_status(
        &mut self,
        id: ConflictId,
        status: ConflictStatus,
        manual: Option<Vec<MergeLine>>,
    ) -> crate::Result<()> {
        let conflict = self
            .conflicts
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(crate::CoreError::Conflict {
                message: "unknown conflict id".into(),
            })?;
        let txn = ResolutionTransaction {
            conflict_id: id,
            previous_status: conflict.status.clone(),
            previous_manual: conflict.manual.clone(),
        };
        conflict.status = status;
        conflict.manual = manual;
        self.undo_stack.push(txn);
        self.redo_stack.clear();
        Ok(())
    }

    /// Undo the most recent resolution change.
    pub fn undo(&mut self) -> crate::Result<ConflictId> {
        let txn = self.undo_stack.pop().ok_or(crate::CoreError::Conflict {
            message: "nothing to undo".into(),
        })?;
        let id = txn.conflict_id;
        let redo = self.swap_in(txn)?;
        self.redo_stack.push(redo);
        Ok(id)
    }

    /// Redo the most recently undone resolution change.
    pub fn redo(&mut self) -> crate::Result<ConflictId> {
        let txn = self.redo_stack.pop().ok_or(crate::CoreError::Conflict {
            message: "nothing to redo".into(),
        })?;
        let id = txn.conflict_id;
        let undo = self.swap_in(txn)?;
        self.undo_stack.push(undo);
        Ok(id)
    }

    fn swap_in(&mut self, txn: ResolutionTransaction) -> crate::Result<ResolutionTransaction> {
        let conflict = self
            .conflicts
            .iter_mut()
            .find(|c| c.id == txn.conflict_id)
            .ok_or(crate::CoreError::InternalInvariant {
                message: "transaction references missing conflict".into(),
            })?;
        let inverse = ResolutionTransaction {
            conflict_id: txn.conflict_id,
            previous_status: std::mem::replace(&mut conflict.status, txn.previous_status),
            previous_manual: std::mem::replace(&mut conflict.manual, txn.previous_manual),
        };
        Ok(inverse)
    }

    /// Mark the current state as saved.
    pub fn mark_saved(&mut self) {
        self.saved_baseline = self.undo_stack.len();
        self.redo_stack.clear();
    }

    /// The canonical merged result text. Unresolved conflicts contribute
    /// nothing (they are holes the user must fill); this is why
    /// [`can_save`](Self::can_save) blocks save until resolved.
    pub fn result_text(&self) -> String {
        let mut lines: Vec<MergeLine> = Vec::new();
        for seg in &self.segments {
            match seg {
                ResultSegment::Fixed(content) => lines.extend(content.clone()),
                ResultSegment::Conflict(id) => {
                    if let Some(c) = self.conflicts.iter().find(|c| c.id == *id) {
                        lines.extend(c.resolved_content());
                    }
                }
            }
        }
        render_lines(&lines)
    }
}

/// Durable conflict identity: `hash(ordinal, base, left, right)`. Resolution
/// never changes the inputs, so the ID is stable for the session's life.
fn conflict_id_for(ordinal: u64, region: &MergeRegion) -> ConflictId {
    debug_assert_eq!(region.kind, RegionKind::Conflict);
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&ordinal.to_le_bytes());
    for side in [&region.base, &region.left, &region.right] {
        buf.push(0xff); // side separator
        for line in side {
            buf.extend_from_slice(line.content.as_bytes());
            buf.extend_from_slice(line.newline.as_str().as_bytes());
            buf.push(b'\n');
        }
    }
    fnv1a64(&buf)
}
