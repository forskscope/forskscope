//! Comparison tab model: `CompareTab`, `TabState`, and tab-level mutations.

use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_core::compare_prep::{
    SaveCapability, SaveTargetSnapshot, save_capability, save_target_from_loaded,
};
use forskscope_core::diff::DiffDocument;
use forskscope_core::document::LoadedDocument;
use forskscope_core::{DiffOptions, MergeSession, compute_diff};
use forskscope_ui_logic::{CompareTabId, LoadGeneration};

use crate::state::save_session;
use crate::state::settings::Lang;

/// Lifecycle state of a comparison tab (RFC-065).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TabState {
    /// Background load is in progress. Tab shows a spinner.
    Loading,
    /// Load and diff complete. Tab shows the diff view.
    Ready,
    /// Load or diff failed. Tab shows a recoverable error message.
    Error(String),
}

/// How a tab was launched, and — for Git mergetool mode — the distinct
/// output path a save writes to (RFC-077). Carrying `merged` here (rather
/// than a separate field on `CompareTab`) means `launch_mode` alone is
/// enough to reconstruct the tab's `SaveDestination` on reload.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CompareLaunchMode {
    /// Two-argument CLI / `git difftool`: save target is the right input.
    Normal,
    /// Three-argument CLI / `git mergetool`: `left_path`/`right_path` are
    /// the compared local/remote inputs; save writes to `merged`, a
    /// genuinely distinct path never aliased to either compared input.
    MergeTool { merged: PathBuf },
}

#[derive(Clone)]
pub struct CompareTab {
    /// Process-local identity; never persisted or derived from vector position.
    pub id: CompareTabId,
    /// Identity of the current file-I/O load attempt for this tab.
    pub load_generation: LoadGeneration,
    pub title: String,
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    /// Lifecycle state — `Loading` until background task completes (RFC-065).
    pub state: TabState,
    pub left_doc: LoadedDocument,
    pub right_doc: LoadedDocument,
    pub diff: DiffDocument,
    pub merge: MergeSession,
    pub diff_options: DiffOptions,
    /// Derived from `save_capability.is_saveable()` — kept as its own field
    /// since most call sites only need this yes/no answer (F88/RFC-082 §D3).
    pub can_save: bool,
    /// One source of truth for whether, and how, a save is possible
    /// (F88/RFC-082 §D3) — `can_save` above is this collapsed to a bool for
    /// the many call sites that only need that; `build_request` reads this
    /// field directly to decide whether a save must be blocked and
    /// explained instead of attempted.
    pub save_capability: SaveCapability,
    pub char_mode: bool,
    pub word_wrap: bool,
    pub focused_change: usize,
    /// Where a save on this tab will go, and under what precondition
    /// (RFC-077). `None` while `state == TabState::Loading` — the same
    /// prepared-comparison commit that installs `left_doc`/`right_doc`/
    /// `diff`/`merge` installs this too, so a tab is never left with some
    /// pieces from an old load and others from a new one.
    pub save_target: Option<SaveTargetSnapshot>,
    /// How this tab was launched (RFC-077). Set once at tab creation and
    /// never mutated afterward — reload derives its `CompareRequest` from
    /// this rather than re-deciding launch mode.
    pub launch_mode: CompareLaunchMode,
}

impl CompareTab {
    pub fn right_label(&self) -> String {
        self.right_doc
            .text
            .as_ref()
            .map(|t| t.encoding.label.clone())
            .unwrap_or_else(|| "—".into())
    }
}

pub fn recompute_diff(tab: &mut CompareTab) {
    let diff = compute_diff(
        tab.left_doc.diff_text(),
        tab.right_doc.diff_text(),
        tab.diff_options,
    );
    tab.merge = MergeSession::from_diff(&diff);
    tab.diff = diff;
    tab.focused_change = 0;
    tab.char_mode = false;
}

pub fn swap_sides(store: &mut crate::state::Store, index: usize) {
    {
        let mut tabs = store.tabs.write();
        let Some(tab) = tabs.get_mut(index) else {
            return;
        };
        std::mem::swap(&mut tab.left_doc, &mut tab.right_doc);
        std::mem::swap(&mut tab.left_path, &mut tab.right_path);
        recompute_diff(tab);
        // save_target must be refreshed before recomputing save_capability
        // — the capability's third input is the *current* target state.
        refresh_save_target(tab);
        refresh_save_capability(tab);
    }
    // F61: swap_sides changes left_path/right_path, which is exactly what
    // session persistence needs to reflect - see open_compare_request's
    // save_session call for why this is now explicit rather than reactive.
    save_session(store);
}

/// F85/RFC-082 §D2: `save_target` is a function of `save_destination`
/// (`launch_mode`), re-derived exactly when the inputs it derives from
/// change — never left stale after a mutation of the compared panes.
///
/// `Normal` mode's destination *is* the right input, already loaded — the
/// same `save_target_from_loaded` call `load_and_diff` uses, re-run here
/// against the (now swapped) `right_path`/`right_doc`, no extra I/O.
///
/// `MergeTool` mode's destination is `$MERGED`, independent of both panes —
/// deliberately a no-op here, not "re-derive unconditionally" (§3a): calling
/// `inspect_save_target($MERGED)` on every swap would refresh its
/// `MustMatch` fingerprint against a file this tab has not re-read, so an
/// external change to `$MERGED` between load and swap would be silently
/// adopted as expected — destroying the external-modification detection
/// mergetool mode exists to provide for that file.
fn refresh_save_target(tab: &mut CompareTab) {
    if let CompareLaunchMode::Normal = tab.launch_mode {
        let right_path = tab.right_path.clone().unwrap_or_default();
        tab.save_target = Some(save_target_from_loaded(&right_path, &tab.right_doc));
    }
}

/// F88/RFC-082 §D3: recomputes `save_capability` (and the `can_save` bool
/// derived from it) from the tab's *current* `left_doc`/`right_doc`/
/// `save_target` — the same composed function `load_and_diff` uses, so a
/// mutation of the compared panes never leaves either field stale. A no-op
/// if `save_target` is `None` (not expected for a `Ready` tab, the only
/// state this is ever called from, but left both fields exactly as they
/// were rather than guessing at a value if it somehow were).
fn refresh_save_capability(tab: &mut CompareTab) {
    let Some(save_target) = tab.save_target.clone() else {
        return;
    };
    tab.save_capability = save_capability(
        &tab.left_doc.kind,
        &tab.right_doc.kind,
        tab.left_doc.editability(),
        tab.right_doc.editability(),
        tab.left_doc.had_decode_errors(),
        tab.right_doc.had_decode_errors(),
        &save_target.state,
    );
    tab.can_save = tab.save_capability.is_saveable();
}

/// The F85/RFC-082 §D2 invariant in `Normal` launch mode: `save_target`
/// equals what `save_target_from_loaded` derives from the tab's *current*
/// `right_path`/`right_doc` — the exact formula both `refresh_save_target`
/// (above) and the load path (`compare::load_and_diff`) use. Shared by the
/// swap test (`tab::tests`) and the load/reload test (`compare::tests`), so
/// a regression in either path is caught by the same assertion, per the
/// handoff's "write the test against the invariant, not against
/// `swap_sides`" (§3b).
#[cfg(test)]
pub(crate) fn assert_save_target_matches_right_input(tab: &CompareTab) {
    let right_path = tab.right_path.clone().unwrap_or_default();
    let expected = save_target_from_loaded(&right_path, &tab.right_doc);
    assert_eq!(
        tab.save_target.as_ref(),
        Some(&expected),
        "save_target must equal save_target_from_loaded(right_path, right_doc) \
         in Normal launch mode"
    );
}

/// Installs `next` and recomputes the diff immediately, discarding any
/// applied merge work and undo/redo history without asking. Used when a tab
/// isn't dirty, and by `ConfirmDiffOptionChangeModal`'s confirm button after
/// the user has already been warned (F40) — never call this from anywhere
/// that hasn't already checked `is_dirty()` or gotten explicit confirmation.
pub fn set_diff_options(store: &mut crate::state::Store, index: usize, next: DiffOptions) {
    let mut tabs = store.tabs.write();
    if let Some(tab) = tabs.get_mut(index) {
        tab.diff_options = next;
        recompute_diff(tab);
    }
}

/// Changes a tab's diff options, guarding against silently discarding
/// applied merge work and the undo/redo stack (F40): `recompute_diff`
/// rebuilds `MergeSession` from scratch, so a dirty tab defers to
/// `Modal::ConfirmDiffOptionChange` instead of applying `next` immediately —
/// the same class of hazard `swap_sides`'s `ConfirmSwap` guard already
/// covers for side-swapping. `next` is computed by the caller (at click
/// time, from the tab's current `diff_options`) so this function stays
/// agnostic to which control was used.
///
/// This does not implement RFC-015 §8 rule 4 ("recomputing diff after an
/// edit must not erase undo history"): once the user confirms, applied
/// merges and the undo stack are discarded, not preserved and reapplied
/// against the new hunks. Hunk identity is not stable across a recompute
/// (`DiffId` is a fresh global counter on every `compute_diff` call), so
/// reapplication would need a rebasing rule this slice does not implement —
/// see RFC-015's recorded gap.
pub fn change_diff_options(store: &mut crate::state::Store, index: usize, next: DiffOptions) {
    let dirty = store
        .tabs
        .read()
        .get(index)
        .map(|t| t.merge.is_dirty())
        .unwrap_or(false);
    if dirty {
        store
            .modal
            .set(crate::state::Modal::ConfirmDiffOptionChange(index, next));
    } else {
        set_diff_options(store, index, next);
    }
}

/// Derive a human-readable tab title from the two file paths.
pub(crate) fn tab_title(l: &std::path::Path, r: &std::path::Path, lang: Lang) -> String {
    use crate::i18n::t;
    let ln = l.file_name().map(|n| n.to_string_lossy().into_owned());
    let rn = r.file_name().map(|n| n.to_string_lossy().into_owned());
    match (ln, rn) {
        (Some(a), Some(b)) if a == b => a,
        (Some(a), Some(b)) => format!("{a} ↔ {b}"),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => t(lang, "comparison"),
    }
}

#[cfg(test)]
mod tests;
