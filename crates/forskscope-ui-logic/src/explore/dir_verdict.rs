//! Tier 1 of RFC-080: what a directory pair's *fast recursive listing* can
//! conclude, and no more.
//!
//! > A cheap scan can prove two directories **differ**. No cheap scan can prove
//! > they are **identical**.
//!
//! [`dir_verdict`] folds a [`RecursiveScan`] (as returned by
//! `list_recursive_for_display_with_rules`, which reads no file contents) into
//! one of three answers. It is caller-side logic on purpose: core marks every
//! common file `Computing` whether or not the sizes differ, and `RecStatus` is
//! shared with Deep Compare, so its meaning must not shift underneath it.
//!
//! ## The verdicts
//!
//! - [`DirVerdict::Different`] — an entry on one side only, or a common file
//!   whose two sizes differ (or a file already known `Changed`). Certain.
//! - [`DirVerdict::Unknown`] — the walk did not see everything, or met something
//!   it cannot judge: an `Unreadable` entry, a `Symlink` (core reports it and
//!   does not follow it, so nothing is known about what it points at), or either
//!   root unreadable. **Never a verdict.**
//! - [`DirVerdict::MetadataMatch`] — otherwise: names and sizes match, contents
//!   were never read. **Not** "identical".
//!
//! ## Precedence (decided, and tested in both orders of input)
//!
//! A definite `Different` **stands** even when something else is unreadable: a
//! one-sided entry or a size mismatch is a fact the walk established, and a
//! failed read elsewhere cannot un-establish it. `Unknown` outranks only
//! `MetadataMatch`, because an unreadable entry means the walk did not see
//! everything, so a match derived from the rest is not trustworthy.
//!
//! **One refinement, found by testing it against real unreadable trees.** A
//! one-sided entry is a fact only where the walk could see *both* sides. Beneath
//! an `Unreadable` directory the other side's files come back as one-sided —
//! `right/locked/x.txt` "right only" because `left/locked` could not be opened —
//! and when a whole **root** cannot be opened every entry of the other side does
//! the same. Those are artefacts of the failed read, not differences, so they are
//! not evidence: they make the verdict `Unknown`, never `Different`. (RFC-080 §1
//! names the root case: "a definite verdict produced by a failed read".)
//!
//! ## What it cannot see
//!
//! The listing records files, not directories. A directory that exists on one
//! side only **and is empty** produces no entry, so this cannot call it
//! `Different`; the pair reads `MetadataMatch`, whose wording ("names and sizes
//! match") is then slightly too strong for that one case. Core would have to
//! report directories to close it.

use forskscope_core::dir::{RecStatus, RecursiveScan};

/// What tier 1 concluded about one directory pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirVerdict {
    /// The trees differ. Certain.
    Different,
    /// Names and sizes match; contents were never read. Not equality.
    MetadataMatch,
    /// The walk was incomplete or met something it cannot judge. Not a verdict.
    Unknown,
}

/// Fold a fast recursive listing into a tier-1 verdict. A free function over
/// borrowed data, testable with no runtime. See the module doc for precedence.
pub fn dir_verdict(scan: &RecursiveScan) -> DirVerdict {
    // A root that could not be opened: the other side's entries all read as
    // one-sided, and nothing was compared at all.
    if scan.left_root_unreadable || scan.right_root_unreadable {
        return DirVerdict::Unknown;
    }
    // Collected first, so the answer does not depend on the order of `entries`.
    let unreadable: Vec<&std::path::Path> = scan
        .entries
        .iter()
        .filter(|e| e.status == RecStatus::Unreadable)
        .map(|e| e.rel_path.as_path())
        .collect();
    let mut incomplete = false;
    for entry in &scan.entries {
        match entry.status {
            // One-sided beneath a directory that could not be read: an artefact.
            RecStatus::LeftOnly | RecStatus::RightOnly
                if unreadable
                    .iter()
                    .any(|u| entry.rel_path.starts_with(u) && entry.rel_path != *u) =>
            {
                incomplete = true;
            }
            // Facts the walk established.
            RecStatus::LeftOnly | RecStatus::RightOnly | RecStatus::Changed => {
                return DirVerdict::Different;
            }
            RecStatus::Computing => match (entry.left_size, entry.right_size) {
                (Some(l), Some(r)) if l != r => return DirVerdict::Different,
                (Some(_), Some(_)) => {}
                // A common file with a size missing was not measured.
                _ => incomplete = true,
            },
            // Nothing measured, or nothing that can be judged.
            RecStatus::Unreadable | RecStatus::Symlink => incomplete = true,
            // A full comparison already established equality for this file;
            // it is not a tier-1 result, but it is no difference either.
            RecStatus::Equal => {}
        }
    }
    if incomplete {
        DirVerdict::Unknown
    } else {
        DirVerdict::MetadataMatch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::dir::RecEntry;
    use std::path::PathBuf;

    fn entry(name: &str, status: RecStatus, l: Option<u64>, r: Option<u64>) -> RecEntry {
        RecEntry {
            rel_path: PathBuf::from(name),
            status,
            left_size: l,
            right_size: r,
        }
    }

    fn common(name: &str, l: u64, r: u64) -> RecEntry {
        entry(name, RecStatus::Computing, Some(l), Some(r))
    }

    fn scan(entries: Vec<RecEntry>) -> RecursiveScan {
        RecursiveScan {
            entries,
            left_root_unreadable: false,
            right_root_unreadable: false,
        }
    }

    #[test]
    fn matching_names_and_sizes_are_a_metadata_match_never_more() {
        let s = scan(vec![common("a", 3, 3), common("d/b", 9, 9)]);
        assert_eq!(dir_verdict(&s), DirVerdict::MetadataMatch);
    }

    #[test]
    fn two_empty_trees_are_a_metadata_match() {
        assert_eq!(dir_verdict(&scan(vec![])), DirVerdict::MetadataMatch);
    }

    #[test]
    fn a_one_sided_entry_is_different() {
        for status in [RecStatus::LeftOnly, RecStatus::RightOnly] {
            let s = scan(vec![common("a", 1, 1), entry("x", status, Some(1), None)]);
            assert_eq!(dir_verdict(&s), DirVerdict::Different, "{status:?}");
        }
    }

    #[test]
    fn a_size_mismatch_is_different() {
        let s = scan(vec![common("a", 1, 1), common("b", 1, 2)]);
        assert_eq!(dir_verdict(&s), DirVerdict::Different);
    }

    #[test]
    fn a_file_already_known_changed_is_different() {
        let s = scan(vec![entry("a", RecStatus::Changed, Some(1), Some(1))]);
        assert_eq!(dir_verdict(&s), DirVerdict::Different);
    }

    /// Criterion 4: an unreadable entry, an unreadable subtree (reported as an
    /// `Unreadable` entry) and an unreadable root are each `Unknown` — neither
    /// a verdict nor a tier-1 match.
    #[test]
    fn unreadable_things_are_unknown() {
        let unreadable = scan(vec![
            common("a", 1, 1),
            entry("locked", RecStatus::Unreadable, None, None),
        ]);
        assert_eq!(dir_verdict(&unreadable), DirVerdict::Unknown);

        let mut left_root = scan(vec![]);
        left_root.left_root_unreadable = true;
        assert_eq!(dir_verdict(&left_root), DirVerdict::Unknown);

        let mut right_root = scan(vec![common("a", 1, 1)]);
        right_root.right_root_unreadable = true;
        assert_eq!(dir_verdict(&right_root), DirVerdict::Unknown);
    }

    /// A symlink is neither proven equal nor proven different (§2): `Unknown`.
    #[test]
    fn a_symlink_is_unknown() {
        let s = scan(vec![
            common("a", 1, 1),
            entry("link", RecStatus::Symlink, None, None),
        ]);
        assert_eq!(dir_verdict(&s), DirVerdict::Unknown);
    }

    #[test]
    fn a_common_file_missing_a_size_was_not_measured() {
        let s = scan(vec![entry("a", RecStatus::Computing, Some(1), None)]);
        assert_eq!(dir_verdict(&s), DirVerdict::Unknown);
    }

    /// Precedence, both orders: a definite `Different` stands even when another
    /// entry is unreadable (the ruling); `Unknown` wins only over a match.
    #[test]
    fn a_definite_different_stands_over_an_unreadable_entry_in_either_order() {
        let different = entry("x", RecStatus::LeftOnly, Some(1), None);
        let mismatch = common("m", 1, 2);
        let unreadable = entry("locked", RecStatus::Unreadable, None, None);
        for definite in [different, mismatch] {
            let first = scan(vec![definite.clone(), unreadable.clone()]);
            let second = scan(vec![unreadable.clone(), definite.clone()]);
            assert_eq!(dir_verdict(&first), DirVerdict::Different);
            assert_eq!(dir_verdict(&second), DirVerdict::Different);
        }
        // ... and over a symlink.
        let s = scan(vec![
            entry("link", RecStatus::Symlink, None, None),
            common("m", 1, 2),
        ]);
        assert_eq!(dir_verdict(&s), DirVerdict::Different);
    }

    /// The refinement: one-sided entries beneath a directory that could not be
    /// read are artefacts of the failed read (the other side's files, seen against
    /// a side that could not be opened) and are **not** evidence; elsewhere in the
    /// same tree a one-sided entry still is. Both orders of input.
    /// Falsify by dropping the `unreadable.iter().any(..)` arm: the first
    /// assertion returns `Different`.
    #[test]
    fn one_sided_entries_under_an_unreadable_directory_are_not_evidence() {
        let locked = entry("locked", RecStatus::Unreadable, None, None);
        let under = entry("locked/x.txt", RecStatus::RightOnly, None, Some(1));
        let elsewhere = entry("other.txt", RecStatus::LeftOnly, Some(1), None);
        let sibling = entry("lockedness.txt", RecStatus::RightOnly, None, Some(1));

        for order in [[&locked, &under], [&under, &locked]] {
            let s = scan(order.iter().map(|e| (*e).clone()).collect());
            assert_eq!(
                dir_verdict(&s),
                DirVerdict::Unknown,
                "under an unreadable dir"
            );
        }
        // A one-sided entry outside it stands, in either order.
        for order in [[&locked, &under, &elsewhere], [&elsewhere, &under, &locked]] {
            let s = scan(order.iter().map(|e| (*e).clone()).collect());
            assert_eq!(dir_verdict(&s), DirVerdict::Different, "elsewhere");
        }
        // Prefix by path component, not by string: `lockedness.txt` is not inside `locked`.
        let s = scan(vec![locked.clone(), sibling]);
        assert_eq!(dir_verdict(&s), DirVerdict::Different);
    }

    /// An unreadable root makes every entry of the other side one-sided; that is
    /// never a verdict (RFC-080 §1), whichever root failed.
    #[test]
    fn an_unreadable_root_is_unknown_even_though_every_entry_reads_one_sided() {
        let mut left = scan(vec![entry("a", RecStatus::RightOnly, None, Some(1))]);
        left.left_root_unreadable = true;
        assert_eq!(dir_verdict(&left), DirVerdict::Unknown);
        let mut right = scan(vec![entry("a", RecStatus::LeftOnly, Some(1), None)]);
        right.right_root_unreadable = true;
        assert_eq!(dir_verdict(&right), DirVerdict::Unknown);
    }

    #[test]
    fn unknown_outranks_only_a_metadata_match() {
        let s = scan(vec![
            common("a", 1, 1),
            entry("locked", RecStatus::Unreadable, None, None),
            common("b", 2, 2),
        ]);
        assert_eq!(dir_verdict(&s), DirVerdict::Unknown);
    }
}
