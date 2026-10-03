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
//! triggering pane actually navigated to (or the number of levels it ascended),
//! re-applied onto the other pane's own current directory. This is what lets
//! a single subsequent navigation re-join two diverged panes with no second
//! control: the moment both sides have a child of the same name, mirroring a
//! descend into it succeeds on both, regardless of how different the rest of
//! each tree looks.
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
///   ancestor of `old`): go up the same number of levels on `other_current`.
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
    if let Ok(rel_back) = old.strip_prefix(new) {
        let levels = rel_back.components().count();
        return other_current.ancestors().nth(levels).map(Path::to_path_buf);
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

    /// Ascend: structural, not name-based — going up one level always means
    /// "the parent", so the other pane goes up one level too, whatever either
    /// pane's name at that level is.
    #[test]
    fn ascending_one_level_goes_up_one_level_on_the_other_pane() {
        let target = mirror_target(
            Path::new("/left/src/utils"),
            Path::new("/left/src"),
            Path::new("/right/src/different-name"),
        );
        assert_eq!(target, Some(PathBuf::from("/right/src")));
    }

    /// A multi-level ascend (breadcrumb click several segments up) goes up
    /// the same number of levels on the other pane.
    #[test]
    fn ascending_several_levels_goes_up_the_same_count() {
        let target = mirror_target(
            Path::new("/left/a/b/c"),
            Path::new("/left"),
            Path::new("/right/a/b/c"),
        );
        assert_eq!(target, Some(PathBuf::from("/right")));
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

    /// Ascending past the other pane's own filesystem root has no mirror -
    /// there is no "further up" to go, so this is the ascend case's own
    /// divergence, not a panic or a clamp to some arbitrary path.
    #[test]
    fn ascending_past_the_other_panes_root_has_no_mirror() {
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
