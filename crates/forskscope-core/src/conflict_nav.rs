//! Conflict resolution workspace navigator (RFC-034 §"Conflict navigator").
//!
//! `ConflictNavigator` is the view-model between `ThreeWayMergeSession` and
//! the four-region workspace UI. It tracks which conflict is focused, drives
//! the navigator rail, and exposes the prev/next traversal that the action
//! bar buttons and keyboard shortcuts call.
//!
//! ## Design
//!
//! - Pure data: no Dioxus, no file I/O. Built from `ThreeWayMergeSession`
//!   and rebuilt whenever the session mutates.
//! - The `ConflictNavigatorEntry` per conflict carries the RFC-034 glyph
//!   and text label so the rail component needs no match on `ConflictStatus`.
//! - `ConflictFilter` lets the UI show only unresolved conflicts without
//!   rebuilding the full entry list.

use crate::merge::{ConflictId, ConflictStatus, ThreeWayMergeSession};

// ── Filter ────────────────────────────────────────────────────────────────────

/// Which conflicts to show in the navigator rail (RFC-034 §"Conflict navigator").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictFilter {
    /// Show all conflicts (resolved and unresolved).
    #[default]
    All,
    /// Show only `Unresolved` conflicts.
    UnresolvedOnly,
}

// ── Status display ────────────────────────────────────────────────────────────

/// Display metadata for one conflict status (RFC-034 §"Conflict navigator" table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConflictStatusDisplay {
    /// Single-character glyph (never the sole cue; paired with `text`).
    pub glyph: char,
    /// Short text label for the navigator row (accessible alternative to glyph).
    pub text:  &'static str,
}

impl ConflictStatusDisplay {
    pub fn for_status(status: ConflictStatus) -> Self {
        match status {
            ConflictStatus::Unresolved     => Self { glyph: '!', text: "unresolved" },
            ConflictStatus::ResolvedLeft   => Self { glyph: 'L', text: "left"       },
            ConflictStatus::ResolvedRight  => Self { glyph: 'R', text: "right"      },
            ConflictStatus::ResolvedBoth   => Self { glyph: 'B', text: "both"       },
            ConflictStatus::ResolvedManual => Self { glyph: '~', text: "manual"     },
            ConflictStatus::Ignored        => Self { glyph: '-', text: "ignored"    },
        }
    }
}

// ── Navigator entry ───────────────────────────────────────────────────────────

/// One row in the conflict navigator rail (RFC-034 §"Conflict navigator").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictNavigatorEntry {
    pub conflict_id: ConflictId,
    /// 1-based display number (not the `ConflictId`).
    pub display_num: usize,
    pub status:      ConflictStatus,
    pub display:     ConflictStatusDisplay,
    /// `true` when this is the currently focused conflict.
    pub is_focused:  bool,
}

impl ConflictNavigatorEntry {
    /// The CSS class token for this entry's status indicator.
    pub fn css_class(&self) -> &'static str {
        match self.status {
            ConflictStatus::Unresolved     => "fsk-conflict-unresolved",
            ConflictStatus::ResolvedLeft   => "fsk-conflict-left",
            ConflictStatus::ResolvedRight  => "fsk-conflict-right",
            ConflictStatus::ResolvedBoth   => "fsk-conflict-both",
            ConflictStatus::ResolvedManual => "fsk-conflict-manual",
            ConflictStatus::Ignored        => "fsk-conflict-ignored",
        }
    }
}

// ── Navigator summary counts ──────────────────────────────────────────────────

/// Summary counts shown in the navigator footer (RFC-034 §"Navigator footer").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavigatorSummary {
    pub total:      usize,
    pub resolved:   usize,
    pub unresolved: usize,
    pub auto_merged: usize,
}

impl NavigatorSummary {
    pub fn progress_fraction(&self) -> f32 {
        if self.total == 0 { return 1.0; }
        self.resolved as f32 / self.total as f32
    }
}

// ── Navigator ─────────────────────────────────────────────────────────────────

/// View-model for the conflict navigator rail (RFC-034 §"Conflict navigator").
///
/// Rebuilt on every session mutation; cheap to reconstruct because it holds
/// only IDs and metadata, not line content.
#[derive(Debug, Clone)]
pub struct ConflictNavigator {
    pub entries:  Vec<ConflictNavigatorEntry>,
    pub filter:   ConflictFilter,
    pub summary:  NavigatorSummary,
    focused_idx:  Option<usize>,
}

impl ConflictNavigator {
    /// Build a navigator from a `ThreeWayMergeSession`.
    ///
    /// `focused_id` is the currently focused conflict, if any.
    /// `filter` controls which entries appear in the rail.
    pub fn build(
        session:    &ThreeWayMergeSession,
        focused_id: Option<ConflictId>,
        filter:     ConflictFilter,
    ) -> Self {
        let stats = session.stats();

        let summary = NavigatorSummary {
            total:       stats.conflicts_total,
            resolved:    stats.conflicts_total.saturating_sub(stats.conflicts_unresolved),
            unresolved:  stats.conflicts_unresolved,
            auto_merged: stats.auto_merged,
        };

        let entries: Vec<ConflictNavigatorEntry> = session.conflicts()
            .iter()
            .enumerate()
            .filter(|(_, c)| match filter {
                ConflictFilter::All           => true,
                ConflictFilter::UnresolvedOnly => c.status == ConflictStatus::Unresolved,
            })
            .map(|(i, c)| ConflictNavigatorEntry {
                conflict_id: c.id,
                display_num: i + 1,
                status:      c.status.clone(),
                display:     ConflictStatusDisplay::for_status(c.status.clone()),
                is_focused:  focused_id == Some(c.id),
            })
            .collect();

        let focused_idx = focused_id.and_then(|id|
            entries.iter().position(|e| e.conflict_id == id)
        );

        Self { entries, filter, summary, focused_idx }
    }

    /// The currently focused entry, if any.
    pub fn focused_entry(&self) -> Option<&ConflictNavigatorEntry> {
        self.focused_idx.and_then(|i| self.entries.get(i))
    }

    /// The `ConflictId` of the next entry after the current focus,
    /// wrapping around. Returns `None` when there are no entries.
    pub fn next_id(&self) -> Option<ConflictId> {
        let n = self.entries.len();
        if n == 0 { return None; }
        let next = self.focused_idx.map(|i| (i + 1) % n).unwrap_or(0);
        Some(self.entries[next].conflict_id)
    }

    /// The `ConflictId` of the previous entry before the current focus,
    /// wrapping around. Returns `None` when there are no entries.
    pub fn prev_id(&self) -> Option<ConflictId> {
        let n = self.entries.len();
        if n == 0 { return None; }
        let prev = self.focused_idx
            .map(|i| if i == 0 { n - 1 } else { i - 1 })
            .unwrap_or(n - 1);
        Some(self.entries[prev].conflict_id)
    }

    /// The first unresolved conflict ID, if any.
    pub fn first_unresolved_id(&self) -> Option<ConflictId> {
        self.entries.iter()
            .find(|e| e.status == ConflictStatus::Unresolved)
            .map(|e| e.conflict_id)
    }

    /// `true` when all conflicts are resolved.
    pub fn is_fully_resolved(&self) -> bool {
        self.summary.unresolved == 0
    }

    /// `true` when `filter` hides at least one entry.
    pub fn has_hidden_entries(&self) -> bool {
        self.filter == ConflictFilter::UnresolvedOnly
            && self.summary.resolved > 0
    }
}
