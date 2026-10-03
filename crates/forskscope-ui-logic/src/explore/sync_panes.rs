//! F151 (issue #148): the Explorer's "sync panes" toggle — location-mirroring,
//! not movement-mirroring.
//!
//! The design discussion settled this precisely, and it is easy to re-derive
//! the wrong thing from "the two panes show the same location" alone, so the
//! distinction is worth restating here: a **movement**-mirroring design
//! (considered and rejected — a `Shift` modifier family replaying the same
//! gesture on both panes, e.g. `Shift+↑` = up on both) replays the *key*, which
//! acts on each pane's own independent state — `Shift+Enter`/`Shift+→` would
//! descend into whatever each pane's *own* cursor happens to be on, with no
//! requirement that the two names match. Two panes that have already diverged
//! into differently-named subtrees stay diverged forever under that scheme,
//! which is what created the rejected design's need for a second, explicit
//! "re-join" control.
//!
//! **Location**-mirroring instead ties the mirrored move to the *name* the
//! triggering pane actually navigated to or ascended to, re-applied onto the
//! other pane's own current directory. This is what lets a single subsequent
//! navigation move two diverged panes together with no second control: the
//! moment both sides have a child (or an ancestor) of the same name,
//! mirroring a descend or ascend into it succeeds on both, regardless of how
//! different the rest of each tree looks - though an existing divergence is
//! carried forward, not cancelled, by either direction (review 138 §3): the
//! mirrored move always shares the triggering pane's name, not the triggering
//! pane's absolute position, so any prior offset between the two panes
//! persists across it.
//!
//! Ascend used to go up the same *number of levels* the triggering pane went
//! up, which is only correct while both panes sit at the same depth. Once
//! they have diverged - which the "stays where it is" policy above
//! deliberately allows - the two depths can differ, and counting levels on
//! the other pane can overshoot past its own starting point entirely: review
//! 138 §2 found `left /v1/src -> /v1` (one level up) while the right pane
//! already sat at `/v2` flinging the right pane to `/`, out of the
//! comparison altogether, from one ordinary "go up". Ascend now mirrors by
//! *name*, the same way descend already did: go to the nearest ancestor of
//! the other pane's current directory that shares the name of the directory
//! the triggering pane arrived at. This is symmetric with descend, and it
//! self-corrects after a divergence instead of compounding it - if the other
//! pane has no such ancestor (most visibly, at its own starting directory,
//! since the two starting directories are expected to differ in name - that
//! is the whole point of comparing two different trees), it is left exactly
//! where it is, never moved somewhere the user has not navigated near.
//!
//! [`mirror_target`] is the pure computation at the centre of this: given the
//! triggering pane's directory before and after one navigation, and the other
//! pane's current directory, what (if anything) the other pane should also
//! navigate to. It does not touch the filesystem or decide whether the result
//! is actually a real directory — the caller does that (the "no counterpart"
//! case, handoff 062 §2: "it stays where it is and the mode stays on" is a
//! caller-side `is_dir()` check away from this function's `None`/`Some`).

use std::path::{Path, PathBuf};

/// What the other pane should navigate to, mirroring one pane's navigation
/// from `old` to `new`, relative to the other pane's own `other_current`
/// directory. `None` when the navigation has no derivable relative structure
/// at all (an unrelated jump — `Home`, a typed absolute path, or the folder
/// picker landing somewhere with no ancestor/descendant relation to `old`):
/// there is nothing to mirror, by design, not a failure to compute one.
///
/// Three cases, in order:
/// - **Descend** (`new` is `old` plus one or more path components): mirror the
///   same relative tail onto `other_current`. Covers double-clicking a row
///   (one component) and any deeper jump that happens to land inside `old`.
/// - **Ascend** (`old` is `new` plus one or more components — `new` is an
///   ancestor of `old`): go to the nearest ancestor of `other_current` that
///   shares `new`'s name — not up the same *number* of levels, which
///   overshoots once the two panes are at different depths (review 138 §2).
///   Covers `↑`, breadcrumb clicks (always to an ancestor), and `Alt+↑`.
/// - **Neither**: no relative structure to mirror.
///
/// A no-op navigation (`old == new`) returns `None` — nothing moved, so there
/// is nothing to mirror; this also keeps a spurious identical re-apply from
/// ever reaching the caller's `is_dir()`/navigate step.
pub fn mirror_target(old: &Path, new: &Path, other_current: &Path) -> Option<PathBuf> {
    if old == new {
        return None;
    }
    if let Ok(rel) = new.strip_prefix(old) {
        return Some(other_current.join(rel));
    }
    if old.strip_prefix(new).is_ok() {
        let arrived_at_name = new.file_name()?;
        return other_current
            .ancestors()
            .find(|ancestor| ancestor.file_name() == Some(arrived_at_name))
            .map(Path::to_path_buf);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Descend: the other pane gets the same child name, not whatever its own
    /// cursor happened to be on — the exact property that re-joins diverged
    /// panes, which a gesture-replay design cannot offer.
    #[test]
    fn descending_one_level_mirrors_the_same_child_name() {
        let target = mirror_target(
            Path::new("/left/src"),
            Path::new("/left/src/utils"),
            Path::new("/right/src"),
        );
        assert_eq!(target, Some(PathBuf::from("/right/src/utils")));
    }

    /// A multi-component descend (e.g. a tree row several levels below the
    /// current directory) mirrors the whole relative tail, not just one name.
    #[test]
    fn descending_several_levels_mirrors_the_whole_relative_tail() {
        let target = mirror_target(
            Path::new("/left/src"),
            Path::new("/left/src/a/b/c"),
            Path::new("/right/src"),
        );
        assert_eq!(target, Some(PathBuf::from("/right/src/a/b/c")));
    }

    /// Ascend: name-based, like descend — going up one level lands on the
    /// nearest ancestor of the other pane sharing the arrived-at name, not
    /// "whatever is one level up" (that was the defect: review 138 §2).
    #[test]
    fn ascending_one_level_goes_up_to_the_ancestor_with_the_same_name() {
        let target = mirror_target(
            Path::new("/left/src/utils"),
            Path::new("/left/src"),
            Path::new("/right/src/different-name"),
        );
        assert_eq!(target, Some(PathBuf::from("/right/src")));
    }

    /// A multi-level ascend (breadcrumb click several segments up) still
    /// matches by the arrived-at name, not by counting levels — so it finds
    /// the right ancestor even when the other pane sits at a different depth
    /// below it (here, one level, not three), which is exactly the shape a
    /// divergence creates.
    #[test]
    fn ascending_several_levels_finds_the_name_regardless_of_the_other_panes_depth() {
        let target = mirror_target(
            Path::new("/left/proj/a/b/c"),
            Path::new("/left/proj"),
            Path::new("/right/proj/x"),
        );
        assert_eq!(target, Some(PathBuf::from("/right/proj")));
    }

    /// The defect itself (review 138 §2), falsified directly: after a
    /// divergence has left the two panes at different depths, an ascent on
    /// one side used to go up the same *number* of levels on the other,
    /// overshooting straight past the other pane's own starting point and
    /// out into unrelated territory. `/v1` and `/v2` are named differently by
    /// design — comparing two differently-named trees is the point of this
    /// product — so there is no ancestor of `/v2` named `v1`, and the right
    /// pane must stay exactly where it is rather than being flung to `/`.
    #[test]
    fn ascending_after_a_divergence_does_not_fling_the_other_pane_past_its_own_start() {
        let target = mirror_target(Path::new("/v1/src"), Path::new("/v1"), Path::new("/v2"));
        assert_eq!(target, None);
    }

    /// Divergence, decided (handoff 062 §2): an unrelated jump - Home, a typed
    /// absolute path, the folder picker - has no ancestor/descendant relation
    /// to derive a mirror from. `None` here is what lets the caller leave the
    /// other pane exactly where it is, with sync still on, rather than forcing
    /// an arbitrary jump neither pane asked for.
    #[test]
    fn an_unrelated_jump_has_no_mirror() {
        let target = mirror_target(
            Path::new("/left/project/src"),
            Path::new("/home/user"),
            Path::new("/right/project/src"),
        );
        assert_eq!(target, None);
    }

    /// When the other pane has no ancestor by the arrived-at name at all -
    /// here, nothing on `/x`'s side is named `a` - there is nothing to
    /// mirror, so it stays exactly where it is rather than landing on some
    /// unrelated ancestor just because one happened to exist.
    #[test]
    fn ascending_to_a_name_the_other_pane_has_nowhere_has_no_mirror() {
        let target = mirror_target(Path::new("/a/b/c"), Path::new("/a"), Path::new("/x"));
        assert_eq!(target, None);
    }

    /// A no-op navigation (already there) mirrors nothing - there was no
    /// move to mirror, and this is what stops a spurious re-apply from ever
    /// reaching the caller's directory check.
    #[test]
    fn a_no_op_navigation_mirrors_nothing() {
        let target = mirror_target(
            Path::new("/left/src"),
            Path::new("/left/src"),
            Path::new("/right/src"),
        );
        assert_eq!(target, None);
    }

    /// Falsifies the "same name, not same position" property directly: if a
    /// future change made this return the other pane's *own* child by
    /// position/index rather than by the triggering pane's actual name, this
    /// would still pass accidentally unless the names differ - they do here,
    /// so only a name-correct implementation satisfies both assertions.
    #[test]
    fn the_mirrored_name_is_the_triggering_panes_name_not_a_coincidence() {
        let target = mirror_target(
            Path::new("/left"),
            Path::new("/left/alpha"),
            Path::new("/right"),
        );
        assert_eq!(target, Some(PathBuf::from("/right/alpha")));
        assert_ne!(target, Some(PathBuf::from("/right/beta")));
    }
}
