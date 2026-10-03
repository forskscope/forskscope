//! Deep compare filter and summary view-model (RFC-037, RFC-038).
//!
//! [`DeepFilter`] controls which entries are shown in the `DeepCompareView`.
//! [`DeepCompareSummary`] derives counts and visibility from a slice of
//! `RecEntry`s and the active filter, replacing the inline arithmetic
//! scattered through `deep_compare.rs`. [`demote_entries_under_an_unreadable_root`]
//! (F110) is the Deep Compare half of the ruling `dir_verdict`'s module doc
//! states for tier 1: a root that could not be opened makes every entry on
//! the other side an artefact of the failed read, not a fact about the trees.

use forskscope_core::dir::{RecEntry, RecStatus, RecursiveScan};

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
            Self::All => true,
            Self::Equal => entry.status == RecStatus::Equal,
        }
    }

    /// Human-readable button label for the filter selector.
    pub fn label(self) -> &'static str {
        match self {
            Self::Different => "Different",
            Self::All => "All",
            Self::Equal => "Equal only",
        }
    }

    /// CSS class for the filter button — `"filter-btn active"` when selected.
    pub fn button_class(self, active: DeepFilter) -> &'static str {
        if self == active {
            "filter-btn active"
        } else {
            "filter-btn"
        }
    }
}

// ── Root-unreadable demotion (F110) ──────────────────────────────────────────

/// Deep Compare has no "Unknown" verdict of its own, unlike tier 1's
/// [`crate::DirVerdict`] — but `RecStatus::Unreadable` already means exactly
/// that: nothing measured, not a verdict, never copyable (see its own doc
/// comment). When either root could not be opened, every entry the walk did
/// manage to collect came from the *other* side alone — a confident
/// `LeftOnly`/`RightOnly`/`Symlink` there is an artefact of the failed read,
/// not a fact about the trees (the exact finding review 129 §1 made for
/// tier 1, and F110 is the same ruling for this view, which had been
/// discarding the flags entirely — see `deep_compare.rs`'s own prior comment
/// recording that). Demoting every entry to `Unreadable` keeps that promise
/// using the vocabulary this view already has, rather than inventing a
/// second "Unknown".
///
/// A scan where neither root is unreadable is returned with its entries
/// untouched — this is a no-op in the overwhelmingly common case.
pub fn demote_entries_under_an_unreadable_root(scan: RecursiveScan) -> Vec<RecEntry> {
    if !scan.left_root_unreadable && !scan.right_root_unreadable {
        return scan.entries;
    }
    scan.entries
        .into_iter()
        .map(|mut e| {
            e.status = RecStatus::Unreadable;
            e
        })
        .collect()
}

// ── Summary counts ────────────────────────────────────────────────────────────

/// Derived counts and visibility for the `DeepCompareView` footer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepCompareSummary {
    /// Total entries (including Computing/Symlink).
    pub total: usize,
    /// Entries with `Changed | LeftOnly | RightOnly` — any non-equal verdict.
    pub different: usize,
    /// Entries with `Changed` specifically (present on both sides, content
    /// differs) — the shipped stats line breaks this out from
    /// `left_only`/`right_only` rather than folding all three into one
    /// number (F75).
    pub changed: usize,
    /// Entries with `LeftOnly`.
    pub left_only: usize,
    /// Entries with `RightOnly`.
    pub right_only: usize,
    /// Entries with `Equal`.
    pub equal: usize,
    /// Entries with `Unreadable` — not a verdict, counted separately from
    /// `different`/`equal` (F79).
    pub unreadable: usize,
    /// Entries still being hashed.
    pub computing: usize,
    /// Number of visible entries under the current filter.
    pub visible: usize,
    /// Active filter used to derive this summary.
    pub filter: DeepFilter,
}

impl DeepCompareSummary {
    /// Build from a slice of entries and the current filter.
    pub fn from_entries(entries: &[RecEntry], filter: DeepFilter) -> Self {
        let count = |pred: &dyn Fn(&RecEntry) -> bool| entries.iter().filter(|e| pred(e)).count();
        Self {
            total: entries.len(),
            different: count(&|e| is_different(&e.status)),
            changed: count(&|e| e.status == RecStatus::Changed),
            left_only: count(&|e| e.status == RecStatus::LeftOnly),
            right_only: count(&|e| e.status == RecStatus::RightOnly),
            equal: count(&|e| e.status == RecStatus::Equal),
            unreadable: count(&|e| e.status == RecStatus::Unreadable),
            computing: count(&|e| e.status == RecStatus::Computing),
            visible: count(&|e| filter.matches(e)),
            filter,
        }
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
    // F79: `RecStatus::Unreadable` is deliberately absent from this list -
    // nothing was measured for it, so it is not a verdict and must not be
    // counted as "different" alongside entries that were actually compared.
    matches!(
        status,
        RecStatus::Changed | RecStatus::LeftOnly | RecStatus::RightOnly
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(status: RecStatus) -> RecEntry {
        RecEntry {
            rel_path: PathBuf::from("f.txt"),
            status,
            left_size: None,
            right_size: None,
        }
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
            assert!(
                DeepFilter::All.matches(e),
                "{:?} must pass All filter",
                e.status
            );
        }
    }

    #[test]
    fn equal_filter_includes_only_equal() {
        assert!(DeepFilter::Equal.matches(&entry(RecStatus::Equal)));
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
        assert!(
            DeepFilter::All
                .button_class(DeepFilter::All)
                .contains("active")
        );
        assert!(
            !DeepFilter::Different
                .button_class(DeepFilter::All)
                .contains("active")
        );
    }

    // ── DeepCompareSummary ────────────────────────────────────────────────────

    #[test]
    fn summary_counts_all_statuses_correctly() {
        let s = DeepCompareSummary::from_entries(&entries(), DeepFilter::All);
        assert_eq!(s.total, 6);
        assert_eq!(s.different, 3); // Changed + LeftOnly + RightOnly
        assert_eq!(s.changed, 1);
        assert_eq!(s.left_only, 1);
        assert_eq!(s.right_only, 1);
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

    // ── F79: RecStatus::Unreadable ────────────────────────────────────────────
    //
    // A one-off entry vec rather than adding `Unreadable` to `entries()` -
    // that fixture backs every count assertion above (`total == 6`,
    // `different == 3`, etc.), and changing its shape would mean re-deriving
    // every one of those numbers for a variant this file's tests don't
    // otherwise need shared fixture data for.

    #[test]
    fn is_different_does_not_count_unreadable_it_is_not_a_verdict() {
        assert!(!is_different(&RecStatus::Unreadable));
    }

    #[test]
    fn different_filter_includes_unreadable() {
        assert!(DeepFilter::Different.matches(&entry(RecStatus::Unreadable)));
    }

    #[test]
    fn equal_filter_excludes_unreadable() {
        assert!(!DeepFilter::Equal.matches(&entry(RecStatus::Unreadable)));
    }

    #[test]
    fn summary_does_not_count_unreadable_as_different_or_equal() {
        let ents = vec![entry(RecStatus::Unreadable), entry(RecStatus::Changed)];
        let s = DeepCompareSummary::from_entries(&ents, DeepFilter::All);
        assert_eq!(s.total, 2);
        assert_eq!(s.different, 1, "only the Changed entry is different");
        assert_eq!(s.equal, 0);
        assert_eq!(s.unreadable, 1);
    }

    // ── F110: demote_entries_under_an_unreadable_root ────────────────────────
    //
    // Issue reproduced directly against the real walk and `batch_copy` before
    // this fix existed: an unreadable right root left every left-side file at
    // `LeftOnly`, which `BatchCopyButtons` would offer in a "copy all left to
    // right" manifest. The copy itself failed for every entry (permission
    // denied on the same unreadable directory it could not open), so this was
    // a display defect, not a data-safety one - but the confident `LeftOnly`
    // was still wrong: nothing was established about the right side at all.

    fn scan_with(
        entries: Vec<RecEntry>,
        left_root_unreadable: bool,
        right_root_unreadable: bool,
    ) -> RecursiveScan {
        RecursiveScan {
            entries,
            left_root_unreadable,
            right_root_unreadable,
        }
    }

    #[test]
    fn an_unreadable_right_root_demotes_every_left_only_entry() {
        let scan = scan_with(
            vec![entry(RecStatus::LeftOnly), entry(RecStatus::LeftOnly)],
            false,
            true,
        );
        let demoted = demote_entries_under_an_unreadable_root(scan);
        assert!(
            demoted.iter().all(|e| e.status == RecStatus::Unreadable),
            "{demoted:?}"
        );
        // Not dropped - the paths are still listed, as `Unreadable` rows
        // already render elsewhere in this view.
        assert_eq!(demoted.len(), 2);
    }

    #[test]
    fn an_unreadable_left_root_demotes_every_right_only_entry() {
        let scan = scan_with(vec![entry(RecStatus::RightOnly)], true, false);
        let demoted = demote_entries_under_an_unreadable_root(scan);
        assert_eq!(demoted[0].status, RecStatus::Unreadable);
    }

    #[test]
    fn a_symlink_under_an_unreadable_root_is_demoted_too() {
        // Not merely one-sided entries: nothing was established about the
        // other side for this path either, so "not followed" is no less an
        // artefact of the failed read than "left only" is.
        let scan = scan_with(vec![entry(RecStatus::Symlink)], false, true);
        let demoted = demote_entries_under_an_unreadable_root(scan);
        assert_eq!(demoted[0].status, RecStatus::Unreadable);
    }

    #[test]
    fn an_already_unreadable_entry_under_an_unreadable_root_stays_unreadable() {
        let scan = scan_with(vec![entry(RecStatus::Unreadable)], false, true);
        let demoted = demote_entries_under_an_unreadable_root(scan);
        assert_eq!(demoted[0].status, RecStatus::Unreadable);
    }

    #[test]
    fn neither_root_unreadable_leaves_entries_exactly_as_given() {
        let ents = entries();
        let scan = scan_with(ents.clone(), false, false);
        assert_eq!(demote_entries_under_an_unreadable_root(scan), ents);
    }

    #[test]
    fn both_roots_unreadable_is_the_same_demotion() {
        let scan = scan_with(vec![], true, true);
        assert_eq!(demote_entries_under_an_unreadable_root(scan), vec![]);
    }

    /// The chain Deep Compare actually runs, against a real `chmod 000`
    /// right root rather than a synthetic `RecursiveScan` - the handoff's own
    /// standard ("checked through the real verdict and not only a unit
    /// test"). `#[cfg(unix)]`: there is no one-line equivalent on Windows,
    /// following `dir_unreadable_tests.rs`'s established pattern, including
    /// the root-skip guard for a suite running as root.
    #[cfg(unix)]
    #[test]
    fn a_real_unreadable_right_root_demotes_the_real_walks_left_only_entries() {
        use std::os::unix::fs::PermissionsExt;

        let base = std::env::temp_dir().join(format!("fsk-f110-reallroot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let left = base.join("l");
        let right = base.join("r");
        std::fs::create_dir_all(&left).unwrap();
        std::fs::write(left.join("a.txt"), "left a").unwrap();
        std::fs::write(left.join("b.txt"), "left b").unwrap();
        std::fs::create_dir_all(&right).unwrap();
        std::fs::set_permissions(&right, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read_dir(&right).is_ok() {
            crate::test_support::permission_guard_failed();
            std::fs::set_permissions(&right, std::fs::Permissions::from_mode(0o755)).unwrap();
            let _ = std::fs::remove_dir_all(&base);
            return;
        }

        let token = forskscope_core::CancellationToken::new();
        let scan = forskscope_core::dir::list_recursive_for_display_with_rules(
            &left,
            &right,
            &token,
            &forskscope_core::IgnoreRules::default(),
        );
        // The premise, from the real walk: exactly what F110 reported before
        // this fix - a confident `LeftOnly` for every left-side file.
        assert!(scan.right_root_unreadable);
        assert_eq!(
            scan.entries
                .iter()
                .filter(|e| e.status == RecStatus::LeftOnly)
                .count(),
            2,
            "the premise: the real walk still reports confident LeftOnly entries: {:?}",
            scan.entries
        );

        let demoted = demote_entries_under_an_unreadable_root(scan);
        assert!(
            demoted.iter().all(|e| e.status == RecStatus::Unreadable),
            "every entry from the real walk must be demoted: {demoted:?}"
        );
        assert_eq!(demoted.len(), 2, "the paths are kept, not dropped");

        std::fs::set_permissions(&right, std::fs::Permissions::from_mode(0o755)).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }
}
