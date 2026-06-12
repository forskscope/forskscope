//! Deep compare filter and summary view-model (RFC-037, RFC-038).
//!
//! [`DeepFilter`] controls which entries are shown in the `DeepCompareView`.
//! [`DeepCompareSummary`] derives counts and visibility from a slice of
//! `RecEntry`s and the active filter, replacing the inline arithmetic
//! scattered through `deep_compare.rs`.

use forskscope_core::dir::{RecEntry, RecStatus};

// ── Filter ────────────────────────────────────────────────────────────────────

/// Which recursive comparison entries to show (RFC-037 §"Filter").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeepFilter {
    /// Show only entries that differ (default view).
    #[default]
    Different,
    /// Show all entries.
    All,
    /// Show only entries that are equal.
    Equal,
}

impl DeepFilter {
    /// `true` when this entry passes the filter.
    pub fn matches(&self, entry: &RecEntry) -> bool {
        match self {
            Self::Different => entry.status != RecStatus::Equal,
            Self::All       => true,
            Self::Equal     => entry.status == RecStatus::Equal,
        }
    }

    /// Human-readable button label for the filter selector.
    pub fn label(self) -> &'static str {
        match self {
            Self::Different => "Different",
            Self::All       => "All",
            Self::Equal     => "Equal only",
        }
    }

    /// CSS class for the filter button — `"filter-btn active"` when selected.
    pub fn button_class(self, active: DeepFilter) -> &'static str {
        if self == active { "filter-btn active" } else { "filter-btn" }
    }
}

// ── Summary counts ────────────────────────────────────────────────────────────

/// Derived counts and visibility for the `DeepCompareView` footer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepCompareSummary {
    /// Total entries (including Computing/Symlink).
    pub total:        usize,
    /// Entries with `Changed | LeftOnly | RightOnly`.
    pub different:    usize,
    /// Entries with `Equal`.
    pub equal:        usize,
    /// Entries still being hashed.
    pub computing:    usize,
    /// Number of visible entries under the current filter.
    pub visible:      usize,
    /// Active filter used to derive this summary.
    pub filter:       DeepFilter,
}

impl DeepCompareSummary {
    /// Build from a slice of entries and the current filter.
    pub fn from_entries(entries: &[RecEntry], filter: DeepFilter) -> Self {
        let total     = entries.len();
        let different = entries.iter().filter(|e| is_different(&e.status)).count();
        let equal     = entries.iter().filter(|e| e.status == RecStatus::Equal).count();
        let computing = entries.iter().filter(|e| e.status == RecStatus::Computing).count();
        let visible   = entries.iter().filter(|e| filter.matches(e)).count();
        Self { total, different, equal, computing, visible, filter }
    }

    /// Footer text, e.g. `"3 different · 12 equal · 15 total"`.
    pub fn footer_text(&self) -> String {
        format!("{} different · {} equal · {} total",
            self.different, self.equal, self.total)
    }

    /// `true` when all common entries have been hashed (no Computing entries).
    pub fn is_fully_computed(&self) -> bool {
        self.computing == 0
    }

    /// `true` when there are no entries at all.
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

/// Filter a slice of entries, returning those that match the active filter.
pub fn apply_filter(entries: &[RecEntry], filter: DeepFilter) -> Vec<&RecEntry> {
    entries.iter().filter(|e| filter.matches(e)).collect()
}

fn is_different(status: &RecStatus) -> bool {
    matches!(status, RecStatus::Changed | RecStatus::LeftOnly | RecStatus::RightOnly)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(status: RecStatus) -> RecEntry {
        RecEntry { rel_path: PathBuf::from("f.txt"), status, left_size: None, right_size: None }
    }

    fn entries() -> Vec<RecEntry> {
        vec![
            entry(RecStatus::Equal),
            entry(RecStatus::Changed),
            entry(RecStatus::LeftOnly),
            entry(RecStatus::RightOnly),
            entry(RecStatus::Computing),
            entry(RecStatus::Symlink),
        ]
    }

    // ── DeepFilter::matches ───────────────────────────────────────────────────

    #[test]
    fn different_filter_excludes_equal() {
        assert!(!DeepFilter::Different.matches(&entry(RecStatus::Equal)));
    }

    #[test]
    fn different_filter_includes_changed_left_right() {
        assert!(DeepFilter::Different.matches(&entry(RecStatus::Changed)));
        assert!(DeepFilter::Different.matches(&entry(RecStatus::LeftOnly)));
        assert!(DeepFilter::Different.matches(&entry(RecStatus::RightOnly)));
    }

    #[test]
    fn all_filter_includes_everything() {
        for e in &entries() {
            assert!(DeepFilter::All.matches(e), "{:?} must pass All filter", e.status);
        }
    }

    #[test]
    fn equal_filter_includes_only_equal() {
        assert!( DeepFilter::Equal.matches(&entry(RecStatus::Equal)));
        assert!(!DeepFilter::Equal.matches(&entry(RecStatus::Changed)));
        assert!(!DeepFilter::Equal.matches(&entry(RecStatus::LeftOnly)));
    }

    // ── DeepFilter labels ─────────────────────────────────────────────────────

    #[test]
    fn all_filter_labels_are_non_empty() {
        for f in [DeepFilter::Different, DeepFilter::All, DeepFilter::Equal] {
            assert!(!f.label().is_empty());
        }
    }

    #[test]
    fn button_class_active_when_selected() {
        assert!(DeepFilter::All.button_class(DeepFilter::All).contains("active"));
        assert!(!DeepFilter::Different.button_class(DeepFilter::All).contains("active"));
    }

    // ── DeepCompareSummary ────────────────────────────────────────────────────

    #[test]
    fn summary_counts_all_statuses_correctly() {
        let s = DeepCompareSummary::from_entries(&entries(), DeepFilter::All);
        assert_eq!(s.total, 6);
        assert_eq!(s.different, 3); // Changed + LeftOnly + RightOnly
        assert_eq!(s.equal, 1);
        assert_eq!(s.computing, 1);
    }

    #[test]
    fn visible_count_matches_filter_different() {
        let s = DeepCompareSummary::from_entries(&entries(), DeepFilter::Different);
        // Different = Changed + LeftOnly + RightOnly + Computing + Symlink (not Equal)
        assert_eq!(s.visible, 5);
    }

    #[test]
    fn visible_count_matches_filter_equal() {
        let s = DeepCompareSummary::from_entries(&entries(), DeepFilter::Equal);
        assert_eq!(s.visible, 1);
    }

    #[test]
    fn visible_count_matches_filter_all() {
        let s = DeepCompareSummary::from_entries(&entries(), DeepFilter::All);
        assert_eq!(s.visible, s.total);
    }

    #[test]
    fn footer_text_contains_counts() {
        let s = DeepCompareSummary::from_entries(&entries(), DeepFilter::All);
        let text = s.footer_text();
        assert!(text.contains('3'), "footer must contain diff count 3: {text}");
        assert!(text.contains('1'), "footer must contain equal count 1: {text}");
        assert!(text.contains('6'), "footer must contain total count 6: {text}");
    }

    #[test]
    fn is_fully_computed_false_while_computing() {
        let s = DeepCompareSummary::from_entries(&entries(), DeepFilter::All);
        assert!(!s.is_fully_computed());
    }

    #[test]
    fn is_fully_computed_true_when_no_computing_entries() {
        let no_computing = vec![entry(RecStatus::Equal), entry(RecStatus::Changed)];
        let s = DeepCompareSummary::from_entries(&no_computing, DeepFilter::All);
        assert!(s.is_fully_computed());
    }

    #[test]
    fn empty_entries_produce_empty_summary() {
        let s = DeepCompareSummary::from_entries(&[], DeepFilter::All);
        assert!(s.is_empty());
        assert_eq!(s.total, 0);
        assert_eq!(s.different, 0);
    }

    // ── apply_filter ──────────────────────────────────────────────────────────

    #[test]
    fn apply_filter_returns_matching_entries() {
        let ents = entries();
        let visible = apply_filter(&ents, DeepFilter::Equal);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].status, RecStatus::Equal);
    }
}
