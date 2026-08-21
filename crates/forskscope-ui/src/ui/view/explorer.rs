//! Explorer workspace: two-pane directory browser (RFC-054).
//!
//! This file owns signal setup, digest computation, and top-level layout.
//! Sub-components live in the `explorer/` subdirectory:
//!
//! - `tree.rs`    — aligned two-pane tree with keyboard navigation
//! - `compact.rs` — compact (unaligned) tree view (RFC-068)
//! - `filter.rs`  — filter bar UI and filter predicate (RFC-067)
//! - `footer.rs`  — targets label and Compare button (RFC-069)

pub mod compact;
pub mod filter;
pub mod footer;
pub mod tree;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTree, use_scan_driver};

use forskscope_core::dir::{DigestOutcome, file_digest_equal_with_cancel};

use crate::i18n::t;
use crate::state::Store;
use crate::ui::view::digest_epoch::{DigestEpoch, EpochStamp};
use crate::ui::view::dir_pane::{
    DigestState, FilteringExecutor, NavHistory, PathBar, home_dir, short_name,
};
use forskscope_ui_logic::compute_aligned_rows;

use compact::CompactTree;
use filter::{FilterBar, apply_filter};
use footer::ExplorerFooter;
use tree::AlignedTree;

// ── Shared types ──────────────────────────────────────────────────────────────

/// Default directory for an explorer pane when no directory has been persisted
/// (e.g. first boot with no saved settings).
///
/// Preference order:
/// 1. the user's home directory (the most useful starting point), then
/// 2. the process working directory, then
/// 3. the filesystem root as a last resort.
///
/// Home is preferred over the working directory because at first launch the
/// working directory is wherever the app was started from — often `/` for a
/// desktop launcher — which is not a useful place to begin browsing.
///
/// Uses [`dir_pane::home_dir`] (HOME / USERPROFILE), already used elsewhere in
/// the explorer, falling back to the working directory only if home cannot be
/// resolved.
fn default_explorer_dir() -> PathBuf {
    let home = home_dir();
    if home.as_os_str().is_empty() || home == std::path::Path::new("/") {
        std::env::current_dir().unwrap_or(home)
    } else {
        home
    }
}

/// Typed key for the digest map (RFC-059 §M2).
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum DigestKey {
    Common(PathBuf),
    RightOnly(PathBuf),
}

/// F74: the state for a directory row that exists on the left side, given
/// whether a same-named entry on the right is *also* a directory.
/// `counterpart_is_dir` true means both sides have a same-named directory -
/// this says nothing about whether their *contents* match (the Explorer
/// never examines directory contents; that verdict is Deep Compare's job,
/// design decision handoff 002 §2), so the only honest state is
/// `NotCompared`, never `Equal`. `counterpart_is_dir` false means the
/// right side has no directory here at all (either nothing, or a file of
/// the same name - a type mismatch, not a match) - correctly `Unique`,
/// unchanged from before F74.
fn dir_common_state(counterpart_is_dir: bool) -> DigestState {
    if counterpart_is_dir {
        DigestState::NotCompared
    } else {
        DigestState::Unique
    }
}

/// F74 review 072: what a left-side entry's classification will be, before
/// any async work starts. `Final` is inserted immediately; `NeedsDigest`
/// means the caller inserts `Computing` and starts the real digest
/// comparison - a file present on both sides is the one case this
/// function cannot resolve synchronously.
#[derive(Debug, PartialEq)]
enum EntryClassification {
    Final(DigestState),
    NeedsDigest {
        left_abs: PathBuf,
        right_abs: PathBuf,
    },
}

/// F74 review 072: the per-entry classification, extracted from the
/// `use_effect` below so a test can drive it against real directories -
/// this is the exact call site `9f355c6` got wrong (`if cp.is_dir() {
/// DigestState::Equal }`), now `dir_common_state`-backed and reachable
/// without a `VirtualDom`. Takes only plain values - no signal reads here,
/// per the extraction risk `dir_pane.rs`'s `apply_navigation` split
/// already established (keep signal reads in the closure, pass plain
/// values in).
fn classify_entry(rel: &Path, is_dir: bool, l_root: &Path, r_root: &Path) -> EntryClassification {
    let cp = r_root.join(rel);
    if is_dir {
        EntryClassification::Final(dir_common_state(cp.is_dir()))
    } else if !cp.is_file() {
        EntryClassification::Final(DigestState::Unique)
    } else {
        EntryClassification::NeedsDigest {
            left_abs: l_root.join(rel),
            right_abs: cp,
        }
    }
}

/// F78: applies a completed async digest result to `digest_map` only if
/// `stamp` is still current for `epoch` - a comparison spawned under one
/// pair of roots that finishes after the roots changed must not land on
/// whatever entry now occupies that key. This is required even with the
/// epoch's own cancellation token: cancellation is inherently racy (a
/// comparison can finish in the window between the root change and the
/// token being observed), so the token stops wasted work while this guard
/// stops a wrong result from reaching a live key - neither substitutes for
/// the other. Takes the map and epoch directly (not `Signal`s) so a test
/// can drive it without a Dioxus runtime - the same reason F77's
/// `apply_digest_result` took the map by value, now backed by
/// `DigestEpoch::is_current` instead of a raw generation-number compare.
fn apply_epoch_result(
    digest_map: &mut HashMap<DigestKey, DigestState>,
    key: DigestKey,
    state: DigestState,
    stamp: EpochStamp,
    epoch: &DigestEpoch,
) {
    if epoch.is_current(stamp) {
        digest_map.insert(key, state);
    }
}

/// Which pane currently receives keyboard events (RFC-061).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Left,
    Right,
}

impl FocusedPane {
    pub fn toggle(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
    pub fn is_left(self) -> bool {
        self == Self::Left
    }
    pub fn is_right(self) -> bool {
        self == Self::Right
    }
}

/// A user's pending pick in one pane.
#[derive(Clone, PartialEq, Eq)]
pub enum PickKind {
    File(PathBuf),
    Dir(PathBuf),
}

impl PickKind {
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::File(p) | Self::Dir(p) => p,
        }
    }
    pub fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }
}

/// Derived action from the current left + right picks.
#[derive(Clone, PartialEq, Eq)]
pub enum CompareAction {
    Files(PathBuf, PathBuf),
    Dirs(PathBuf, PathBuf),
    None,
}

pub fn compare_action(lp: &Option<PickKind>, rp: &Option<PickKind>) -> CompareAction {
    match (lp, rp) {
        (Some(PickKind::File(l)), Some(PickKind::File(r))) => {
            CompareAction::Files(l.clone(), r.clone())
        }
        (Some(PickKind::Dir(l)), Some(PickKind::Dir(r))) => {
            CompareAction::Dirs(l.clone(), r.clone())
        }
        _ => CompareAction::None,
    }
}

// ── Explorer root component ───────────────────────────────────────────────────

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    let ignore = store.settings.read().ignore_rules();
    let binary_enabled = store.settings.read().enable_binary_comparison;
    let compact_mode = store.settings.read().explorer_compact;

    // Binary sniff cache — cleared on directory change (RFC-066).
    let mut binary_cache: Signal<HashMap<PathBuf, bool>> = use_signal(Default::default);

    // ── Left pane ─────────────────────────────────────────────────────────────
    let remember = store.settings.read().remember_explorer_dirs;
    let init_l = if remember {
        store
            .settings
            .read()
            .last_left_dir
            .clone()
            .unwrap_or_else(default_explorer_dir)
    } else {
        default_explorer_dir()
    };
    let left_dir: Signal<PathBuf> = use_signal(|| init_l.clone());
    let mut left_hist: Signal<NavHistory> = use_signal(NavHistory::default);
    use_hook(|| left_hist.write().push(init_l.clone()));

    let exec_l = Arc::new(FilteringExecutor {
        rules: ignore.clone(),
    });
    let mut tree_l: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(init_l.clone()));
    let scans_l = use_scan_driver(tree_l, exec_l);

    use_effect(move || {
        let root = left_dir.read().cloned();
        let mut nt = DirectoryTree::new(root.clone());
        binary_cache.write().clear();
        if let Some(req) = nt.on_toggled(&root) {
            tree_l.set(nt);
            scans_l.send(req);
        } else {
            tree_l.set(nt);
        }
    });

    // ── Right pane ────────────────────────────────────────────────────────────
    let init_r = if remember {
        store
            .settings
            .read()
            .last_right_dir
            .clone()
            .unwrap_or_else(default_explorer_dir)
    } else {
        default_explorer_dir()
    };
    let right_dir: Signal<PathBuf> = use_signal(|| init_r.clone());
    let mut right_hist: Signal<NavHistory> = use_signal(NavHistory::default);
    use_hook(|| right_hist.write().push(init_r.clone()));

    let exec_r = Arc::new(FilteringExecutor { rules: ignore });
    let mut tree_r: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(init_r.clone()));
    let scans_r = use_scan_driver(tree_r, exec_r);

    use_effect(move || {
        let root = right_dir.read().cloned();
        let mut nt = DirectoryTree::new(root.clone());
        binary_cache.write().clear();
        if let Some(req) = nt.on_toggled(&root) {
            tree_r.set(nt);
            scans_r.send(req);
        } else {
            tree_r.set(nt);
        }
    });

    // ── Digest map ────────────────────────────────────────────────────────────
    let mut digest_map: Signal<HashMap<DigestKey, DigestState>> = use_signal(HashMap::new);
    let mut digest_roots: Signal<(PathBuf, PathBuf)> =
        use_signal(|| (PathBuf::new(), PathBuf::new()));
    // F78: `digest_epoch` owns the generation guard, the cancellation
    // token, and (new for the Explorer) the concurrency bound that
    // `deep_compare.rs` already had and this view didn't - one
    // `spawn_blocking` per common file, unbounded, before this change.
    // `restart()` is called at the same place `digest_map` is cleared -
    // see the `changed` block below.
    let mut digest_epoch: Signal<DigestEpoch> =
        use_signal(|| DigestEpoch::new(forskscope_core::DIGEST_CONCURRENCY_LIMIT));

    use_effect(move || {
        let l_root = left_dir.read().cloned();
        let r_root = right_dir.read().cloned();
        if l_root.as_os_str().is_empty() || r_root.as_os_str().is_empty() {
            return;
        }

        {
            let roots = digest_roots.read();
            let changed = roots.0 != l_root || roots.1 != r_root;
            drop(roots);
            if changed {
                // Cancel outstanding comparisons under the old roots
                // before anything else observes the new ones - stops
                // wasted work for whatever hasn't finished yet.
                digest_epoch.write().restart();
                digest_map.write().clear();
                digest_roots.set((l_root.clone(), r_root.clone()));
            }
        }

        let left_entries: Vec<(PathBuf, bool)> = tree_l
            .read()
            .visible_rows()
            .into_iter()
            .filter_map(|(n, _)| {
                let rel = n.path.strip_prefix(&l_root).ok()?.to_path_buf();
                if rel.as_os_str().is_empty() {
                    return None;
                }
                Some((rel, n.is_dir))
            })
            .collect();

        for (rel, is_dir) in left_entries {
            if digest_map
                .read()
                .contains_key(&DigestKey::Common(rel.clone()))
            {
                continue;
            }
            match classify_entry(&rel, is_dir, &l_root, &r_root) {
                EntryClassification::Final(state) => {
                    digest_map.write().insert(DigestKey::Common(rel), state);
                }
                EntryClassification::NeedsDigest {
                    left_abs,
                    right_abs,
                } => {
                    let key = rel.clone();
                    let mut dmap = digest_map;
                    dmap.write()
                        .insert(DigestKey::Common(key.clone()), DigestState::Computing);
                    // F78: captured now, at spawn time - the stamp this
                    // comparison belongs to, the token that can stop it if
                    // the roots change before it finishes, and the
                    // semaphore permit that bounds how many of these run
                    // at once (new for the Explorer - see digest_epoch.rs).
                    let (stamp, token, sem) = digest_epoch.read().begin_task();
                    let epoch_signal = digest_epoch;
                    spawn(async move {
                        // Acquired here, inside the task, after the
                        // `begin_task()` read guard above has already been
                        // dropped - never held across an await.
                        let _permit = sem.acquire_owned().await;
                        let outcome = tokio::task::spawn_blocking(move || {
                            file_digest_equal_with_cancel(&left_abs, &right_abs, &token)
                        })
                        .await
                        // A join error (the blocking task panicked) has no
                        // real verdict either - same fallback the old code
                        // used for any failure, preserved as-is (a separate,
                        // already-tracked finding, not this handoff's scope).
                        .unwrap_or(Ok(DigestOutcome::Different));
                        let state = match outcome {
                            Ok(DigestOutcome::Equal) => DigestState::Equal,
                            Ok(DigestOutcome::Different) => DigestState::Different,
                            Err(_) => DigestState::Different,
                            Ok(DigestOutcome::Cancelled) => {
                                // Established nothing - there is no verdict
                                // to (conditionally or not) apply. The root
                                // change that cancelled this also cleared
                                // digest_map already.
                                return;
                            }
                        };
                        apply_epoch_result(
                            &mut dmap.write(),
                            DigestKey::Common(key),
                            state,
                            stamp,
                            &epoch_signal.read(),
                        );
                    });
                }
            }
        }

        let r_root2 = right_dir.read().cloned();
        let l_root2 = left_dir.read().cloned();
        let right_entries: Vec<PathBuf> = tree_r
            .read()
            .visible_rows()
            .into_iter()
            .filter_map(|(n, _)| {
                let rel = n.path.strip_prefix(&r_root2).ok()?.to_path_buf();
                if rel.as_os_str().is_empty() {
                    return None;
                }
                Some(rel)
            })
            .collect();
        for rel in right_entries {
            let key = DigestKey::RightOnly(rel.clone());
            if digest_map.read().contains_key(&key) {
                continue;
            }
            if !l_root2.join(&rel).exists() {
                digest_map.write().insert(key, DigestState::Unique);
            }
        }
    });

    // ── Filter state ──────────────────────────────────────────────────────────
    let filter_open: Signal<bool> = use_signal(|| false);
    let filter_query: Signal<String> = use_signal(String::new);
    let filter_hide_bin: Signal<bool> = use_signal(|| false);
    let filter_hide_eq: Signal<bool> = use_signal(|| false);

    // ── Picks ─────────────────────────────────────────────────────────────────
    let left_pick: Signal<Option<PickKind>> = use_signal(|| None);
    let right_pick: Signal<Option<PickKind>> = use_signal(|| None);

    let mut focused_pane: Signal<FocusedPane> = use_signal(|| FocusedPane::Left);

    use_effect(move || {
        let lp = left_pick.read();
        store.left_pick.set(
            lp.as_ref()
                .filter(|p| p.is_file())
                .map(|p| p.path().clone()),
        );
    });
    use_effect(move || {
        let rp = right_pick.read();
        store.right_pick.set(
            rp.as_ref()
                .filter(|p| p.is_file())
                .map(|p| p.path().clone()),
        );
    });

    // ── Compute rows ──────────────────────────────────────────────────────────
    let l_root_snap = left_dir.read().cloned();
    let r_root_snap = right_dir.read().cloned();

    let left_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_l
        .read()
        .visible_rows()
        .into_iter()
        .filter(|(n, _)| n.path != l_root_snap)
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d))
        .collect();
    let right_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_r
        .read()
        .visible_rows()
        .into_iter()
        .filter(|(n, _)| n.path != r_root_snap)
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d))
        .collect();

    let aligned = compute_aligned_rows(&left_flat, &right_flat, &l_root_snap, &r_root_snap);
    let aligned = apply_filter(
        aligned,
        &filter_query.read().to_lowercase(),
        *filter_hide_bin.read(),
        *filter_hide_eq.read(),
        binary_enabled,
        &digest_map.read(),
        &mut binary_cache,
    );

    rsx! {
        div { class: "explorer",
            div { class: "explorer-browse",

                // ── Path bars ─────────────────────────────────────────────
                div { class: "explorer-path-bars",
                    PathBar {
                        path: left_dir.read().cloned(),
                        can_back:    left_hist.read().can_back(),
                        can_forward: left_hist.read().can_forward(),
                        on_back:    move |_| { let p = left_hist.write().back();    if let Some(p) = p { crate::ui::view::dir_pane::navigate_to_from_history(p, true,  store, left_dir); } },
                        on_forward: move |_| { let p = left_hist.write().forward(); if let Some(p) = p { crate::ui::view::dir_pane::navigate_to_from_history(p, true,  store, left_dir); } },
                        on_navigate: move |p| crate::ui::view::dir_pane::navigate_to(p, true, store, left_hist, left_dir),
                        lang,
                    }
                    PathBar {
                        path: right_dir.read().cloned(),
                        can_back:    right_hist.read().can_back(),
                        can_forward: right_hist.read().can_forward(),
                        on_back:    move |_| { let p = right_hist.write().back();    if let Some(p) = p { crate::ui::view::dir_pane::navigate_to_from_history(p, false, store, right_dir); } },
                        on_forward: move |_| { let p = right_hist.write().forward(); if let Some(p) = p { crate::ui::view::dir_pane::navigate_to_from_history(p, false, store, right_dir); } },
                        on_navigate: move |p| crate::ui::view::dir_pane::navigate_to(p, false, store, right_hist, right_dir),
                        lang,
                    }
                }

                // ── Filter bar ────────────────────────────────────────────
                FilterBar { lang, filter_open, filter_query, filter_hide_bin, filter_hide_eq }

                // ── Pane-root labels ──────────────────────────────────────
                div { class: "pane-root-bar",
                    div {
                        class: if focused_pane.read().is_left() { "pane-root-cell pane-focused" } else { "pane-root-cell" },
                        role: "heading",
                        aria_label: format!("{} — {}", t(lang, "Left pane"), short_name(&l_root_snap)),
                        onclick: move |_| focused_pane.set(FocusedPane::Left),
                        span { class: "root-label", "📁 " }
                        span { class: "root-name", title: "{l_root_snap.display()}", {short_name(&l_root_snap)} }
                    }
                    div {
                        class: if focused_pane.read().is_right() { "pane-root-cell pane-focused" } else { "pane-root-cell" },
                        role: "heading",
                        aria_label: format!("{} — {}", t(lang, "Right pane"), short_name(&r_root_snap)),
                        onclick: move |_| focused_pane.set(FocusedPane::Right),
                        span { class: "root-label", "📁 " }
                        span { class: "root-name", title: "{r_root_snap.display()}", {short_name(&r_root_snap)} }
                    }
                }

                // ── Tree ──────────────────────────────────────────────────
                if !compact_mode {
                    AlignedTree {
                        lang, aligned,
                        tree_l, tree_r, scans_l, scans_r,
                        left_dir, right_dir, left_hist, right_hist,
                        left_pick, right_pick, focused_pane,
                        digest_map, binary_cache, binary_enabled,
                    }
                } else {
                    CompactTree {
                        lang,
                        left_flat, right_flat,
                        l_root: l_root_snap.clone(), r_root: r_root_snap.clone(),
                        tree_l, tree_r, scans_l, scans_r,
                        left_dir, right_dir, left_hist, right_hist,
                        left_pick, right_pick,
                        digest_map, binary_cache, binary_enabled,
                        filter_query,
                    }
                }

                // ── Footer ────────────────────────────────────────────────
                ExplorerFooter { lang, left_pick, right_pick }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // F74: a same-named directory on both sides must never be reported
    // `Equal` - its contents were never examined. Two directories with
    // differing contents (the exact false-positive the bug report hit) end
    // up with `counterpart_is_dir == true` here (the right side genuinely
    // is a directory, differing contents notwithstanding - the Explorer
    // has no way to know they differ without recursing, which is
    // deliberately not this function's job), so this asserts the honest
    // state rather than a claimed one.
    #[test]
    fn a_same_named_directory_on_both_sides_is_never_reported_equal() {
        let state = dir_common_state(true);
        assert_eq!(state, DigestState::NotCompared);
        assert_ne!(state, DigestState::Equal);
    }

    #[test]
    fn a_directory_with_no_directory_counterpart_is_unique() {
        // Either nothing on the other side, or a file of the same name -
        // a type mismatch, not a match. Unchanged behavior from before F74.
        assert_eq!(dir_common_state(false), DigestState::Unique);
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fsk-ui-explorer-f74-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    // F74 review 072: drives `classify_entry` - the real call site, not
    // just `dir_common_state` in isolation - against two real directories
    // that share a name and genuinely differ. Confirmed to fail with the
    // original `9f355c6` defect restored (see the commit message/review
    // request for the observed failure output).
    #[test]
    fn real_directories_with_the_same_name_and_differing_contents_are_not_compared() {
        let base = temp_dir("real-path-not-equal");
        let l_root = base.join("left");
        let r_root = base.join("right");
        std::fs::create_dir_all(l_root.join("sub")).unwrap();
        std::fs::create_dir_all(r_root.join("sub")).unwrap();
        std::fs::write(l_root.join("sub/a.txt"), "left content").unwrap();
        std::fs::write(
            r_root.join("sub/a.txt"),
            "right content, genuinely different",
        )
        .unwrap();

        let rel = std::path::PathBuf::from("sub");
        let result = classify_entry(&rel, true, &l_root, &r_root);

        assert_eq!(
            result,
            EntryClassification::Final(DigestState::NotCompared),
            "a same-named directory whose contents differ must never be \
             classified Equal - see 9f355c6's original defect"
        );
    }

    // F78: drives `apply_epoch_result` directly against a real `HashMap`
    // and a real `DigestEpoch` - no Dioxus runtime needed, since the
    // function takes both by value/reference rather than as `Signal`s.
    // Per handoff 004 §14 (still true here): the race that makes this
    // defect *reachable* is timing-dependent, but the guard itself is
    // not - it is a plain stamp/generation comparison, tested
    // deterministically by constructing the stale case directly rather
    // than trying to race a real root change against a real spawn.
    //
    // This replaces F77's `a_stale_generation_result_does_not_mutate_the_map`
    // / `a_current_generation_result_is_applied`, which drove the removed
    // `apply_digest_result` - review 074's falsification was re-run against
    // this converted call site by temporarily short-circuiting the
    // `if epoch.is_current(stamp)` check above to always insert, and
    // confirming the first assertion below then fails; restored.
    #[test]
    fn a_stale_stamp_result_does_not_mutate_the_map() {
        let mut epoch = DigestEpoch::new(4);
        let (stale_stamp, _, _) = epoch.begin_task();
        // The roots changed after `stale_stamp` was taken.
        epoch.restart();

        let mut map: HashMap<DigestKey, DigestState> = HashMap::new();
        let key = DigestKey::Common(PathBuf::from("a.txt"));

        apply_epoch_result(
            &mut map,
            key.clone(),
            DigestState::Equal,
            stale_stamp,
            &epoch,
        );

        assert!(
            !map.contains_key(&key),
            "a stale-stamp result must not mutate the map"
        );
    }

    #[test]
    fn a_current_stamp_result_is_applied() {
        let epoch = DigestEpoch::new(4);
        let (stamp, _, _) = epoch.begin_task();

        let mut map: HashMap<DigestKey, DigestState> = HashMap::new();
        let key = DigestKey::Common(PathBuf::from("a.txt"));

        apply_epoch_result(&mut map, key.clone(), DigestState::Equal, stamp, &epoch);

        assert_eq!(map.get(&key), Some(&DigestState::Equal));
    }
}
