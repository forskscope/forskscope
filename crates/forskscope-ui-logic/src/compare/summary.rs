//! Compare summary and navigation state view-models (RFC-003, RFC-006).
//!
//! [`CompareStatusSummary`] consolidates everything the status bar and tab
//! title need from a comparison tab — change counts, dirty state, encoding —
//! into one tested computation instead of duplicating the logic in each
//! Dioxus component.
//!
//! [`DiffNavigationState`] captures the current hunk position (`1 of N`) and
//! whether prev/next are available. The diff workspace toolbar reads this to
//! render the navigation buttons and their labels.

use forskscope_core::diff::DiffStats;

// ── Compare status summary ─────────────────────────────────────────────────────

/// Everything the status bar and tab bar need to render — a single tested
/// snapshot derived from tab fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareStatusSummary {
    /// e.g. `"+12 / -5"` or `"Files are identical"` or `""` for binary.
    pub change_text:      String,
    /// Encoding label for the right (result) side, e.g. `"UTF-8"`.
    pub encoding_label:   String,
    /// Whether the result buffer has unsaved merge changes.
    pub is_dirty:         bool,
    /// Whether any merge or save operations are possible (false for binary/xlsx).
    pub is_saveable:      bool,
    /// Number of changed hunks.
    pub changed_hunks:    usize,
    /// Whether there are no differences (both sides identical).
    pub is_identical:     bool,
}

impl CompareStatusSummary {
    /// Build from raw tab fields.
    ///
    /// - `stats`: from `DiffDocument::stats`
    /// - `is_dirty`: `MergeSession::is_dirty()`
    /// - `is_saveable`: `tab.can_save` (editable text, has target path)
    /// - `encoding_label`: `tab.right_label()` (e.g. `"UTF-8"`)
    pub fn from_fields(
        stats:          &DiffStats,
        is_dirty:       bool,
        is_saveable:    bool,
        encoding_label: String,
    ) -> Self {
        let is_identical = stats.hunks_changed == 0;
        let change_text = if is_identical {
            "Files are identical".into()
        } else if stats.lines_inserted > 0 || stats.lines_deleted > 0 {
            format!("+{} / -{}", stats.lines_inserted, stats.lines_deleted)
        } else {
            // Changed hunks but no line count (e.g. whitespace-only).
            format!("{} change{}", stats.hunks_changed,
                if stats.hunks_changed == 1 { "" } else { "s" })
        };

        Self {
            change_text,
            encoding_label,
            is_dirty,
            is_saveable,
            changed_hunks: stats.hunks_changed,
            is_identical,
        }
    }

    /// Dirty marker character — `"●"` when dirty, `""` when clean.
    pub fn dirty_marker(&self) -> &'static str {
        if self.is_dirty { "●" } else { "" }
    }

    /// CSS class for the dirty dot in the tab title.
    pub fn dirty_css_class(&self) -> &'static str {
        if self.is_dirty { "dirty-dot" } else { "" }
    }
}

// ── Diff navigation state ──────────────────────────────────────────────────────

/// The current hunk navigation position and button availability.
///
/// Derived from `focused_change` (0-based index) and `total_changes` (count
/// of changed hunks). Both prev and next wrap around, so they are always
/// available when `total_changes > 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffNavigationState {
    /// 0-based index of the focused change. `None` when no changes exist.
    pub focused_index:    Option<usize>,
    /// Total number of changed hunks.
    pub total_changes:    usize,
}

impl DiffNavigationState {
    /// Build from the raw `focused_change` field and the change count.
    pub fn new(focused_change: usize, total_changes: usize) -> Self {
        let focused_index = if total_changes == 0 { None } else { Some(focused_change) };
        Self { focused_index, total_changes }
    }

    /// `true` when there is at least one changed hunk to navigate.
    pub fn has_changes(&self) -> bool { self.total_changes > 0 }

    /// Both prev and next wrap, so they are available iff changes exist.
    pub fn prev_available(&self) -> bool { self.has_changes() }
    pub fn next_available(&self) -> bool { self.has_changes() }

    /// 1-based display position, e.g. `"3 of 7"`. Returns `""` when no changes.
    pub fn position_label(&self) -> String {
        match self.focused_index {
            Some(i) => format!("{} of {}", i + 1, self.total_changes),
            None    => String::new(),
        }
    }

    /// Short accessibility label for the previous-hunk button.
    pub fn prev_aria_label(&self) -> String {
        match self.focused_index {
            Some(i) if i > 0 =>
                format!("Previous change ({} of {})", i, self.total_changes),
            _ => "Previous change (wraps to last)".into(),
        }
    }

    /// Short accessibility label for the next-hunk button.
    pub fn next_aria_label(&self) -> String {
        match self.focused_index {
            Some(i) if i + 1 < self.total_changes =>
                format!("Next change ({} of {})", i + 2, self.total_changes),
            _ => "Next change (wraps to first)".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(changed: usize, inserted: usize, deleted: usize) -> DiffStats {
        DiffStats {
            hunks_total:   changed,
            hunks_changed: changed,
            lines_inserted: inserted,
            lines_deleted:  deleted,
        }
    }

    // ── CompareStatusSummary ──────────────────────────────────────────────────

    #[test]
    fn identical_files_produce_identical_text() {
        let s = CompareStatusSummary::from_fields(&stats(0, 0, 0), false, true, "UTF-8".into());
        assert!(s.is_identical);
        assert_eq!(s.change_text, "Files are identical");
        assert_eq!(s.changed_hunks, 0);
        assert!(!s.is_dirty);
    }

    #[test]
    fn changed_files_produce_plus_minus_text() {
        let s = CompareStatusSummary::from_fields(&stats(3, 12, 5), false, true, "UTF-8".into());
        assert!(!s.is_identical);
        assert_eq!(s.change_text, "+12 / -5");
        assert_eq!(s.changed_hunks, 3);
    }

    #[test]
    fn whitespace_only_changes_produce_n_changes_text() {
        // hunks_changed > 0 but no line counts (whitespace-only diff mode).
        let s = CompareStatusSummary::from_fields(&stats(2, 0, 0), false, true, "UTF-8".into());
        assert_eq!(s.change_text, "2 changes");
    }

    #[test]
    fn single_hunk_no_lines_produces_singular_text() {
        let s = CompareStatusSummary::from_fields(&stats(1, 0, 0), false, true, "UTF-8".into());
        assert_eq!(s.change_text, "1 change");
    }

    #[test]
    fn dirty_flag_reflects_input() {
        let s_clean = CompareStatusSummary::from_fields(&stats(1, 1, 0), false, true, "UTF-8".into());
        let s_dirty = CompareStatusSummary::from_fields(&stats(1, 1, 0), true,  true, "UTF-8".into());
        assert!(!s_clean.is_dirty);
        assert!( s_dirty.is_dirty);
        assert_eq!(s_clean.dirty_marker(), "");
        assert_eq!(s_dirty.dirty_marker(), "●");
    }

    #[test]
    fn unsaveable_tab_is_not_dirty_even_if_changed() {
        // Binary/xlsx tabs have is_saveable = false.
        let s = CompareStatusSummary::from_fields(&stats(2, 5, 3), false, false, "(binary)".into());
        assert!(!s.is_saveable);
        assert!(!s.is_dirty);
    }

    #[test]
    fn encoding_label_passes_through() {
        let s = CompareStatusSummary::from_fields(&stats(0, 0, 0), false, true, "Shift_JIS".into());
        assert_eq!(s.encoding_label, "Shift_JIS");
    }

    // ── DiffNavigationState ───────────────────────────────────────────────────

    #[test]
    fn no_changes_produces_none_focused_index() {
        let nav = DiffNavigationState::new(0, 0);
        assert!(!nav.has_changes());
        assert!(nav.focused_index.is_none());
        assert!(!nav.prev_available());
        assert!(!nav.next_available());
        assert_eq!(nav.position_label(), "");
    }

    #[test]
    fn first_change_of_seven_produces_correct_label() {
        let nav = DiffNavigationState::new(0, 7);
        assert!(nav.has_changes());
        assert!(nav.prev_available());
        assert!(nav.next_available());
        assert_eq!(nav.position_label(), "1 of 7");
    }

    #[test]
    fn last_change_position_label_is_correct() {
        let nav = DiffNavigationState::new(6, 7);
        assert_eq!(nav.position_label(), "7 of 7");
    }

    #[test]
    fn prev_aria_label_mentions_position() {
        let nav = DiffNavigationState::new(2, 5);
        let label = nav.prev_aria_label();
        assert!(label.contains("2"), "prev label must reference previous position");
        assert!(label.contains("5"), "prev label must reference total");
    }

    #[test]
    fn next_aria_label_mentions_position() {
        let nav = DiffNavigationState::new(2, 5);
        let label = nav.next_aria_label();
        assert!(label.contains("4"), "next label must reference next position");
        assert!(label.contains("5"), "next label must reference total");
    }

    #[test]
    fn first_change_prev_aria_wraps() {
        let nav = DiffNavigationState::new(0, 5);
        assert!(nav.prev_aria_label().contains("last"),
            "at first change, prev wraps to last");
    }

    #[test]
    fn last_change_next_aria_wraps() {
        let nav = DiffNavigationState::new(4, 5);
        assert!(nav.next_aria_label().contains("first"),
            "at last change, next wraps to first");
    }

    #[test]
    fn single_change_nav_is_at_1_of_1() {
        let nav = DiffNavigationState::new(0, 1);
        assert_eq!(nav.position_label(), "1 of 1");
        assert!(nav.prev_available());
        assert!(nav.next_available());
    }
}
