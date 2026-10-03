//! F151 (issue #148): the Explorer's "sync panes" toggle — anchored
//! location-mirroring (review 139 §3), not movement-mirroring and not a
//! delta against the other pane's live position.
//!
//! The design discussion settled "the two panes show the same location,"
//! and two prior shapes were tried and found wanting before this one, which
//! is worth restating so a future change does not re-derive either:
//!
//! - A **movement**-mirroring design (considered and rejected before this
//!   module existed — a `Shift` modifier family replaying the same gesture
//!   on both panes) replays the *key*, acting on each pane's own independent
//!   cursor with no requirement that the two land on the same name. Two
//!   panes that diverge stay diverged forever under that scheme.
//! - A **delta**-against-current-position design (this module's first two
//!   revisions) computed the mirror from the other pane's *live* directory —
//!   by level count (review 138 §2: overshot past the other pane's own
//!   starting point once the two panes were at different depths), then by
//!   matching the arrived-at name among the other pane's ancestors (review
//!   139 §1: stalled the moment the triggering pane ascended to or above its
//!   own root, because the two roots are named differently by design — the
//!   whole point of comparing two different trees). Both were correct
//!   projections of a model that was never written down, which is why each
//!   repair fixed one case and broke another.
//!
//! **The model**: syncing means keeping the same path *relative to where
//! each pane started*. The caller captures that start — each pane's
//! directory at the moment sync is switched on — once per pane, and
//! [`mirror_target`] always computes the mirror as "the triggering pane's
//! current path relative to its own anchor, rejoined under the other pane's
//! anchor." This never depends on where the other pane currently sits, so a
//! prior divergence cannot feed back into the next mirror: the moment the
//! triggering pane's anchor-relative path exists on the other side too, that
//! mirror succeeds again, regardless of how far apart the two panes drifted
//! in between. Navigating above your own anchor is a divergence like any
//! other — that pane moves, the other stays — and switching sync off and
//! back on re-anchors both panes from wherever they are then.
//!
//! Neither function touches the filesystem or decides whether a computed
//! target is actually a real directory — the caller does that (the "no
//! counterpart" case, handoff 062 §2: "it stays where it is and the mode
//! stays on" is a caller-side `is_dir()` check away from `mirror_target`'s
//! `None`/`Some`).

use std::path::{Path, PathBuf};

/// What the other pane should navigate to, mirroring the triggering pane's
/// current directory (`new`) relative to its own anchor (`anchor_mine`),
/// rejoined under the other pane's anchor (`anchor_other`). Always
/// recomputed from the anchors, never from the other pane's current
/// directory — there is no such parameter, which is the fix (review 139
/// §3): nothing about the other pane's live position can feed back into
/// this and make it wrong.
///
/// `None` when `new` is not at or under `anchor_mine` at all — this covers
/// both an unrelated jump (`Home`, a typed absolute path, the folder picker
/// landing somewhere unrelated) and navigating *above* your own anchor
/// (including up to a same-named root under a different parent, review 139
/// §1's case E): there is no relative path to mirror, by design, not a
/// failure to compute one.
pub fn mirror_target(new: &Path, anchor_mine: &Path, anchor_other: &Path) -> Option<PathBuf> {
    let rel = new.strip_prefix(anchor_mine).ok()?;
    Some(anchor_other.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Review 139's case D: descending inside the anchors mirrors the
    /// relative path onto the other anchor.
    #[test]
    fn in_step_descent_mirrors_the_relative_path() {
        let target = mirror_target(
            Path::new("/v1/src/utils"),
            Path::new("/v1"),
            Path::new("/v2"),
        );
        assert_eq!(target, Some(PathBuf::from("/v2/src/utils")));
    }

    /// Review 139's case B: ascending one level while still under the
    /// anchor mirrors correctly - unaffected by where the other pane
    /// currently is, because that is no longer a parameter at all.
    #[test]
    fn in_step_ascent_one_level_mirrors_the_shorter_relative_path() {
        let target = mirror_target(Path::new("/v1/src"), Path::new("/v1"), Path::new("/v2"));
        assert_eq!(target, Some(PathBuf::from("/v2/src")));
    }

    /// Review 139's cases A and C collapse into one: ascending all the way
    /// back to your own anchor mirrors to the other pane's anchor exactly -
    /// whether the other pane was already there (case A, the review 138 §2
    /// bug: no longer flung anywhere) or still mid-tree in step (case C, the
    /// defect this review found: it used to not follow at all). Under the
    /// anchored model there is nothing left to distinguish the two inputs:
    /// the other pane's prior position was the only thing that ever made
    /// them different, and it is gone from the signature.
    #[test]
    fn ascending_to_your_own_anchor_mirrors_to_the_other_panes_anchor() {
        let target = mirror_target(Path::new("/v1"), Path::new("/v1"), Path::new("/v2"));
        assert_eq!(target, Some(PathBuf::from("/v2")));
    }

    /// Review 139 §1's case E: same-named roots under different parents
    /// still have no mirror once the triggering pane goes *above* its own
    /// anchor, even though the two anchors happen to share a leaf name -
    /// this was never about names matching, only about staying at or under
    /// your own anchor.
    #[test]
    fn navigating_above_your_own_anchor_has_no_mirror_even_with_a_shared_leaf_name() {
        let target = mirror_target(Path::new("/a"), Path::new("/a/proj"), Path::new("/b/proj"));
        assert_eq!(target, None);
    }

    /// An unrelated jump - Home, a typed absolute path, the folder picker -
    /// is just another way of landing outside your own anchor. `None` here
    /// is what lets the caller leave the other pane exactly where it is,
    /// with sync still on, rather than forcing an arbitrary jump neither
    /// pane asked for (handoff 062 §2).
    #[test]
    fn an_unrelated_jump_has_no_mirror() {
        let target = mirror_target(
            Path::new("/home/user"),
            Path::new("/v1/project/src"),
            Path::new("/v2/project/src"),
        );
        assert_eq!(target, None);
    }

    /// Falsifies "the other anchor's own child by position" directly: if a
    /// future change made this return something keyed off the other
    /// anchor's own structure rather than the triggering pane's actual
    /// relative path, this would still pass accidentally unless the names
    /// differ - they do here, so only a correct implementation satisfies
    /// both assertions.
    #[test]
    fn the_mirrored_path_is_the_triggering_panes_relative_path_not_a_coincidence() {
        let target = mirror_target(Path::new("/v1/alpha"), Path::new("/v1"), Path::new("/v2"));
        assert_eq!(target, Some(PathBuf::from("/v2/alpha")));
        assert_ne!(target, Some(PathBuf::from("/v2/beta")));
    }
}
