//! Tier 2 of RFC-080: the certain pass.
//!
//! Tier 1 ([`crate::dir_verdict`]) answers *do these differ?* from names and
//! sizes, reading no contents. Tier 2 answers *are these identical?*
//! definitively, by running `recursive_diff_with_rules` (the same walk, same
//! [`forskscope_core::IgnoreRules`] plumbing tier 1 and the Explorer tree use
//! — F149's "the tree and the walk must answer the same question" applies
//! here too) and reading every common file's contents.
//!
//! [`tier2_verdict`] folds the resulting [`RecursiveScan`] into one of three
//! answers, mirroring tier 1's precedence (see `dir_verdict`'s module doc)
//! with one difference: where tier 1's walk never resolves a common file
//! past `Computing` (it reads no contents), tier 2's walk resolves every
//! common file to `Equal` or `Changed` — `Computing` surviving into this
//! fold means the walk was cancelled before finishing that file, not that it
//! was never examined, so it is still incomplete, never a verdict.

use std::path::Path;

use forskscope_core::dir::{RecStatus, RecursiveScan};

/// What tier 2 concluded about one directory pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier2Verdict {
    /// Every common file's contents matched, and nothing was one-sided,
    /// unreadable, or a symlink. The only one of the three verdicts entitled
    /// to claim identity (RFC-080 §3/§4 criterion 3).
    Identical,
    /// The trees differ. Certain — the same claim tier 1's `Different`
    /// makes, since "a tier-1 `Different` and a tier-2 `Different` are the
    /// same claim" (RFC-080 §4).
    Different,
    /// The walk was incomplete, cancelled, or met something it cannot judge.
    /// Not a verdict.
    Unknown,
}

/// Fold a full recursive comparison into a tier-2 verdict. A free function
/// over borrowed data, testable with no runtime — the same shape as
/// [`crate::dir_verdict`].
pub fn tier2_verdict(scan: &RecursiveScan) -> Tier2Verdict {
    // A root that could not be opened: nothing was compared at all (same
    // ruling as tier 1, and F110's ruling for Deep Compare).
    if scan.left_root_unreadable || scan.right_root_unreadable {
        return Tier2Verdict::Unknown;
    }
    let unreadable: Vec<&Path> = scan
        .entries
        .iter()
        .filter(|e| e.status == RecStatus::Unreadable)
        .map(|e| e.rel_path.as_path())
        .collect();
    let mut incomplete = false;
    for entry in &scan.entries {
        match entry.status {
            // One-sided beneath a directory that could not be read: an artefact.
            RecStatus::LeftOnly
            | RecStatus::RightOnly
            | RecStatus::LeftOnlyDir
            | RecStatus::RightOnlyDir
                if unreadable
                    .iter()
                    .any(|u| entry.rel_path.starts_with(u) && entry.rel_path != *u) =>
            {
                incomplete = true;
            }
            // Facts the walk established. F135: an empty one-sided
            // directory is the same kind of fact a one-sided file is.
            RecStatus::LeftOnly
            | RecStatus::RightOnly
            | RecStatus::LeftOnlyDir
            | RecStatus::RightOnlyDir
            | RecStatus::Changed => {
                return Tier2Verdict::Different;
            }
            // Tier 2 reads every common file's contents, so `Computing`
            // surviving to here means the walk was cancelled mid-comparison
            // for this file, not that it was skipped by design (unlike
            // tier 1, which never resolves a common file past this status).
            RecStatus::Computing => incomplete = true,
            // Nothing measured, or nothing that can be judged.
            RecStatus::Unreadable | RecStatus::Symlink => incomplete = true,
            RecStatus::Equal => {}
        }
    }
    if incomplete {
        Tier2Verdict::Unknown
    } else {
        Tier2Verdict::Identical
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::dir::RecEntry;
    use std::path::PathBuf;

    fn entry(name: &str, status: RecStatus) -> RecEntry {
        RecEntry {
            rel_path: PathBuf::from(name),
            status,
            left_size: None,
            right_size: None,
        }
    }

    fn scan(entries: Vec<RecEntry>) -> RecursiveScan {
        RecursiveScan {
            entries,
            left_root_unreadable: false,
            right_root_unreadable: false,
        }
    }

    /// Criterion 3: every common file equal, nothing else present →
    /// `Identical` — the only tier entitled to it.
    #[test]
    fn an_all_equal_scan_is_identical() {
        let s = scan(vec![
            entry("a", RecStatus::Equal),
            entry("d/b", RecStatus::Equal),
        ]);
        assert_eq!(tier2_verdict(&s), Tier2Verdict::Identical);
    }

    #[test]
    fn two_empty_trees_are_identical() {
        assert_eq!(tier2_verdict(&scan(vec![])), Tier2Verdict::Identical);
    }

    /// Criterion 2: a file with real, digest-verified differing content
    /// (`Changed`, what the walk actually produces for it) is `Different`.
    #[test]
    fn a_changed_file_is_different() {
        let s = scan(vec![
            entry("a", RecStatus::Equal),
            entry("b", RecStatus::Changed),
        ]);
        assert_eq!(tier2_verdict(&s), Tier2Verdict::Different);
    }

    #[test]
    fn a_one_sided_entry_is_different() {
        for status in [RecStatus::LeftOnly, RecStatus::RightOnly] {
            let s = scan(vec![entry("a", RecStatus::Equal), entry("x", status)]);
            assert_eq!(tier2_verdict(&s), Tier2Verdict::Different, "{status:?}");
        }
    }

    /// F135: tier 2 reuses the same walk tier 1 does, so it is just as
    /// blind to an empty one-sided directory without the fix - confirmed
    /// here at this tier too, not assumed from tier 1 alone.
    #[test]
    fn an_empty_one_sided_directory_is_different() {
        for status in [RecStatus::LeftOnlyDir, RecStatus::RightOnlyDir] {
            let s = scan(vec![
                entry("a", RecStatus::Equal),
                entry("empty_dir", status),
            ]);
            assert_eq!(tier2_verdict(&s), Tier2Verdict::Different, "{status:?}");
        }
    }

    /// Cancellation mid-walk leaves at least one file at `Computing` — not
    /// examined to a conclusion, so `Unknown`, never `Identical`. This is
    /// the clause RFC-080 §3 says to keep a permanent test on: tier 2 must
    /// never repaint a row it did not finish examining.
    #[test]
    fn a_cancelled_common_file_is_unknown_never_identical() {
        let s = scan(vec![
            entry("a", RecStatus::Equal),
            entry("b", RecStatus::Computing),
        ]);
        assert_eq!(tier2_verdict(&s), Tier2Verdict::Unknown);
    }

    #[test]
    fn unreadable_things_are_unknown() {
        let unreadable = scan(vec![
            entry("a", RecStatus::Equal),
            entry("locked", RecStatus::Unreadable),
        ]);
        assert_eq!(tier2_verdict(&unreadable), Tier2Verdict::Unknown);

        let mut left_root = scan(vec![]);
        left_root.left_root_unreadable = true;
        assert_eq!(tier2_verdict(&left_root), Tier2Verdict::Unknown);

        let mut right_root = scan(vec![entry("a", RecStatus::Equal)]);
        right_root.right_root_unreadable = true;
        assert_eq!(tier2_verdict(&right_root), Tier2Verdict::Unknown);
    }

    #[test]
    fn a_symlink_is_unknown() {
        let s = scan(vec![
            entry("a", RecStatus::Equal),
            entry("link", RecStatus::Symlink),
        ]);
        assert_eq!(tier2_verdict(&s), Tier2Verdict::Unknown);
    }

    /// A definite `Different` stands even when something else under a
    /// different, unreadable directory is incomplete — same precedence tier
    /// 1 established, tested in both orders of input so the answer does not
    /// depend on entry order.
    #[test]
    fn a_definite_different_stands_over_an_unreadable_entry_in_either_order() {
        let a = scan(vec![
            entry("changed", RecStatus::Changed),
            entry("locked", RecStatus::Unreadable),
        ]);
        let b = scan(vec![
            entry("locked", RecStatus::Unreadable),
            entry("changed", RecStatus::Changed),
        ]);
        assert_eq!(tier2_verdict(&a), Tier2Verdict::Different);
        assert_eq!(tier2_verdict(&b), Tier2Verdict::Different);
    }

    /// A one-sided entry beneath an unreadable directory is an artefact of
    /// the failed read, not a fact about the trees — tier 1's refinement
    /// (review 129 §1), which applies identically here.
    #[test]
    fn a_one_sided_entry_beneath_an_unreadable_directory_is_unknown_not_different() {
        let s = scan(vec![
            entry("locked", RecStatus::Unreadable),
            entry("locked/x.txt", RecStatus::RightOnly),
        ]);
        assert_eq!(tier2_verdict(&s), Tier2Verdict::Unknown);
    }

    /// The same refinement applies to an empty one-sided directory.
    #[test]
    fn an_empty_one_sided_directory_beneath_an_unreadable_directory_is_unknown() {
        let s = scan(vec![
            entry("locked", RecStatus::Unreadable),
            entry("locked/empty", RecStatus::RightOnlyDir),
        ]);
        assert_eq!(tier2_verdict(&s), Tier2Verdict::Unknown);
    }
}
