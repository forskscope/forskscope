//! Conflict navigator rail view-model (RFC-034 §"Conflict navigator", Slice 6).
//!
//! [`ConflictNavView`] is a fully-resolved snapshot of the navigator rail
//! the three-way merge workspace renders. It derives from a
//! [`ConflictNavigator`] and contains everything the Dioxus component needs
//! without any further access to the navigator or session.
//!
//! ## What the Dioxus component does with this
//!
//! 1. Render `rows` as the vertical rail: one `ConflictRailRow` per entry.
//! 2. Display `progress_text` in the navigator footer.
//! 3. Enable/disable the "Save" button using `can_save`.
//! 4. Call `prev_id` / `next_id` when the user presses Alt+↑ / Alt+↓.

use forskscope_core::conflict_nav::{ConflictNavigator, NavigatorSummary};
use forskscope_core::ConflictId;

// ── Rail row ──────────────────────────────────────────────────────────────────

/// One row in the conflict navigator rail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRailRow {
    /// The conflict this row refers to.
    pub conflict_id: ConflictId,
    /// 1-based display number shown in the rail (`"1"`, `"2"`, …).
    pub display_num: usize,
    /// Single-character glyph: `!` unresolved, `L` left, `R` right, `B` both,
    /// `~` manual, `-` ignored.
    pub glyph:       char,
    /// Short accessible text alternative for the glyph.
    pub status_text: &'static str,
    /// CSS class for the rail row, e.g. `"fsk-conflict-unresolved"`.
    pub css_class:   &'static str,
    /// Whether this is the currently focused conflict.
    pub is_focused:  bool,
}

// ── ConflictNavView ───────────────────────────────────────────────────────────

/// The complete navigator rail snapshot for one three-way merge session.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictNavView {
    /// Ordered rail rows (filtered by `ConflictNavigator::filter`).
    pub rows:          Vec<ConflictRailRow>,
    /// Footer text, e.g. `"2 of 5 resolved"` or `"All resolved"`.
    pub progress_text: String,
    /// Whether the session can be saved (all conflicts resolved).
    pub can_save:      bool,
    /// ID of the conflict to navigate to when the user presses "previous".
    pub prev_id:       Option<ConflictId>,
    /// ID of the conflict to navigate to when "next" is pressed.
    pub next_id:       Option<ConflictId>,
    /// Raw summary counts for further derivation if needed.
    pub summary:       NavigatorSummary,
}

impl ConflictNavView {
    /// Build a `ConflictNavView` from a [`ConflictNavigator`] and the
    /// session's `can_save()` predicate.
    pub fn from_navigator(nav: &ConflictNavigator, can_save: bool) -> Self {
        let rows = nav.entries.iter().map(|e| ConflictRailRow {
            conflict_id: e.conflict_id,
            display_num: e.display_num,
            glyph:       e.display.glyph,
            status_text: e.display.text,
            css_class:   e.css_class(),
            is_focused:  e.is_focused,
        }).collect();

        Self {
            rows,
            progress_text: format_progress(&nav.summary),
            can_save,
            prev_id:  nav.prev_id(),
            next_id:  nav.next_id(),
            summary:  nav.summary,
        }
    }

    /// Number of rows in the rail.
    pub fn len(&self) -> usize { self.rows.len() }

    /// `true` when the rail is empty (no conflicts, or all filtered out).
    pub fn is_empty(&self) -> bool { self.rows.is_empty() }

    /// Return the focused row, if any.
    pub fn focused_row(&self) -> Option<&ConflictRailRow> {
        self.rows.iter().find(|r| r.is_focused)
    }
}

/// Format the navigator footer progress text.
fn format_progress(summary: &NavigatorSummary) -> String {
    if summary.total == 0 {
        return "No conflicts".into();
    }
    if summary.unresolved == 0 {
        return "All resolved".into();
    }
    format!("{} of {} resolved", summary.resolved, summary.total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::conflict_nav::{ConflictFilter, ConflictNavigator};
    use forskscope_core::merge::{ThreeWayMergeSession};

    fn session_with_conflicts() -> ThreeWayMergeSession {
        ThreeWayMergeSession::from_texts(
            "base\nline\n",
            "left changed\nline\n",
            "right changed\nline\n",
        )
    }

    fn nav_from_session(session: &ThreeWayMergeSession) -> ConflictNavigator {
        ConflictNavigator::build(session, None, ConflictFilter::All)
    }

    // ── Basic construction ────────────────────────────────────────────────────

    #[test]
    fn view_from_session_with_conflicts_is_non_empty() {
        let sess = session_with_conflicts();
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        assert!(!view.is_empty());
        assert!(!view.rows.is_empty());
    }

    #[test]
    fn no_conflicts_view_is_empty() {
        let sess = ThreeWayMergeSession::from_texts("same\n", "same\n", "same\n");
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        assert!(view.is_empty());
    }

    // ── Row fields ────────────────────────────────────────────────────────────

    #[test]
    fn rows_have_non_zero_display_nums() {
        let sess = session_with_conflicts();
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        for row in &view.rows {
            assert!(row.display_num >= 1, "display_num must be 1-based");
        }
    }

    #[test]
    fn unresolved_rows_have_exclamation_glyph() {
        let sess = session_with_conflicts();
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        for row in &view.rows {
            assert_eq!(row.glyph, '!', "fresh conflicts must be unresolved (!)");
        }
    }

    #[test]
    fn rows_css_class_starts_with_fsk() {
        let sess = session_with_conflicts();
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        for row in &view.rows {
            assert!(row.css_class.starts_with("fsk-"),
                "css_class must have fsk- prefix: {}", row.css_class);
        }
    }

    // ── Progress text ─────────────────────────────────────────────────────────

    #[test]
    fn progress_text_shows_unresolved_count() {
        let sess = session_with_conflicts();
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        // "0 of N resolved" or similar — must not be "All resolved"
        assert_ne!(view.progress_text, "All resolved",
            "fresh session must not show 'All resolved'");
    }

    #[test]
    fn no_conflicts_shows_no_conflicts_text() {
        let sess = ThreeWayMergeSession::from_texts("a\n", "a\n", "a\n");
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        assert_eq!(view.progress_text, "No conflicts");
    }

    // ── can_save follows session ──────────────────────────────────────────────

    #[test]
    fn can_save_false_when_unresolved_conflicts_remain() {
        let sess = session_with_conflicts();
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        assert!(!view.can_save, "must not be saveable while conflicts unresolved");
    }

    #[test]
    fn can_save_true_when_no_conflicts() {
        let sess = ThreeWayMergeSession::from_texts("a\n", "a\n", "a\n");
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        assert!(view.can_save, "must be saveable with no conflicts");
    }

    // ── len matches rows ──────────────────────────────────────────────────────

    #[test]
    fn len_matches_rows_len() {
        let sess = session_with_conflicts();
        let nav  = nav_from_session(&sess);
        let view = ConflictNavView::from_navigator(&nav, sess.can_save());
        assert_eq!(view.len(), view.rows.len());
    }
}
