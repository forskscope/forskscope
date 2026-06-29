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
use std::path::PathBuf;
use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTree, use_scan_driver};

use forskscope_core::dir::file_digest_equal;

use crate::i18n::t;
use crate::state::Store;
use forskscope_ui_logic::compute_aligned_rows;
use crate::ui::view::dir_pane::{DigestState, FilteringExecutor, NavHistory, PathBar, home_dir, short_name};

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

/// Which pane currently receives keyboard events (RFC-061).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane { Left, Right }

impl FocusedPane {
    pub fn toggle(self) -> Self { match self { Self::Left => Self::Right, Self::Right => Self::Left } }
    pub fn is_left(self)  -> bool { self == Self::Left }
    pub fn is_right(self) -> bool { self == Self::Right }
}

/// A user's pending pick in one pane.
#[derive(Clone, PartialEq, Eq)]
pub enum PickKind { File(PathBuf), Dir(PathBuf) }

impl PickKind {
    pub fn path(&self) -> &PathBuf { match self { Self::File(p) | Self::Dir(p) => p } }
    pub fn is_file(&self) -> bool { matches!(self, Self::File(_)) }
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
        (Some(PickKind::File(l)), Some(PickKind::File(r))) =>
            CompareAction::Files(l.clone(), r.clone()),
        (Some(PickKind::Dir(l)), Some(PickKind::Dir(r))) =>
            CompareAction::Dirs(l.clone(), r.clone()),
        _ => CompareAction::None,
    }
}

// ── Explorer root component ───────────────────────────────────────────────────

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    let ignore         = store.settings.read().ignore_rules();
    let binary_enabled = store.settings.read().enable_binary_comparison;
    let compact_mode   = store.settings.read().explorer_compact;

    // Binary sniff cache — cleared on directory change (RFC-066).
    let mut binary_cache: Signal<HashMap<PathBuf, bool>> = use_signal(Default::default);

    // ── Left pane ─────────────────────────────────────────────────────────────
    let remember = store.settings.read().remember_explorer_dirs;
    let init_l = if remember {
        store.settings.read().last_left_dir.clone().unwrap_or_else(default_explorer_dir)
    } else {
        default_explorer_dir()
    };
    let left_dir:      Signal<PathBuf>    = use_signal(|| init_l.clone());
    let mut left_hist: Signal<NavHistory> = use_signal(NavHistory::default);
    use_hook(|| left_hist.write().push(init_l.clone()));

    let exec_l = Arc::new(FilteringExecutor { rules: ignore.clone() });
    let mut tree_l: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(init_l.clone()));
    let scans_l = use_scan_driver(tree_l, exec_l);

    use_effect(move || {
        let root = left_dir.read().cloned();
        let mut nt = DirectoryTree::new(root.clone());
        binary_cache.write().clear();
        if let Some(req) = nt.on_toggled(&root) { tree_l.set(nt); scans_l.send(req); }
        else { tree_l.set(nt); }
    });

    // ── Right pane ────────────────────────────────────────────────────────────
    let init_r = if remember {
        store.settings.read().last_right_dir.clone().unwrap_or_else(default_explorer_dir)
    } else {
        default_explorer_dir()
    };
    let right_dir:      Signal<PathBuf>    = use_signal(|| init_r.clone());
    let mut right_hist: Signal<NavHistory> = use_signal(NavHistory::default);
    use_hook(|| right_hist.write().push(init_r.clone()));

    let exec_r = Arc::new(FilteringExecutor { rules: ignore });
    let mut tree_r: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(init_r.clone()));
    let scans_r = use_scan_driver(tree_r, exec_r);

    use_effect(move || {
        let root = right_dir.read().cloned();
        let mut nt = DirectoryTree::new(root.clone());
        binary_cache.write().clear();
        if let Some(req) = nt.on_toggled(&root) { tree_r.set(nt); scans_r.send(req); }
        else { tree_r.set(nt); }
    });

    // ── Digest map ────────────────────────────────────────────────────────────
    let mut digest_map:   Signal<HashMap<DigestKey, DigestState>> = use_signal(HashMap::new);
    let mut digest_roots: Signal<(PathBuf, PathBuf)> = use_signal(|| (PathBuf::new(), PathBuf::new()));

    use_effect(move || {
        let l_root = left_dir.read().cloned();
        let r_root = right_dir.read().cloned();
        if l_root.as_os_str().is_empty() || r_root.as_os_str().is_empty() { return; }

        {
            let roots   = digest_roots.read();
            let changed = roots.0 != l_root || roots.1 != r_root;
            drop(roots);
            if changed {
                digest_map.write().clear();
                digest_roots.set((l_root.clone(), r_root.clone()));
            }
        }

        let left_entries: Vec<(PathBuf, bool)> = tree_l.read().visible_rows().into_iter()
            .filter_map(|(n, _)| {
                let rel = n.path.strip_prefix(&l_root).ok()?.to_path_buf();
                if rel.as_os_str().is_empty() { return None; }
                Some((rel, n.is_dir))
            })
            .collect();

        for (rel, is_dir) in left_entries {
            if digest_map.read().contains_key(&DigestKey::Common(rel.clone())) { continue; }
            let cp = r_root.join(&rel);
            if is_dir {
                let state = if cp.is_dir() { DigestState::Equal } else { DigestState::Unique };
                digest_map.write().insert(DigestKey::Common(rel), state);
            } else {
                if !cp.is_file() {
                    digest_map.write().insert(DigestKey::Common(rel.clone()), DigestState::Unique);
                    continue;
                }
                let lp = l_root.join(&rel);
                let key = rel.clone();
                let mut dmap = digest_map;
                dmap.write().insert(DigestKey::Common(key.clone()), DigestState::Computing);
                spawn(async move {
                    let eq = tokio::task::spawn_blocking(move || {
                        file_digest_equal(&lp, &cp).unwrap_or(false)
                    }).await.unwrap_or(false);
                    dmap.write().insert(DigestKey::Common(key), if eq { DigestState::Equal } else { DigestState::Different });
                });
            }
        }

        let r_root2 = right_dir.read().cloned();
        let l_root2 = left_dir.read().cloned();
        let right_entries: Vec<PathBuf> = tree_r.read().visible_rows().into_iter()
            .filter_map(|(n, _)| {
                let rel = n.path.strip_prefix(&r_root2).ok()?.to_path_buf();
                if rel.as_os_str().is_empty() { return None; }
                Some(rel)
            })
            .collect();
        for rel in right_entries {
            let key = DigestKey::RightOnly(rel.clone());
            if digest_map.read().contains_key(&key) { continue; }
            if !l_root2.join(&rel).exists() {
                digest_map.write().insert(key, DigestState::Unique);
            }
        }
    });

    // ── Filter state ──────────────────────────────────────────────────────────
    let filter_open:    Signal<bool>   = use_signal(|| false);
    let filter_query:   Signal<String> = use_signal(String::new);
    let filter_hide_bin:Signal<bool>   = use_signal(|| false);
    let filter_hide_eq: Signal<bool>   = use_signal(|| false);

    // ── Picks ─────────────────────────────────────────────────────────────────
    let left_pick:  Signal<Option<PickKind>> = use_signal(|| None);
    let right_pick: Signal<Option<PickKind>> = use_signal(|| None);

    let mut focused_pane: Signal<FocusedPane> = use_signal(|| FocusedPane::Left);

    use_effect(move || {
        let lp = left_pick.read();
        store.left_pick.set(lp.as_ref().filter(|p| p.is_file()).map(|p| p.path().clone()));
    });
    use_effect(move || {
        let rp = right_pick.read();
        store.right_pick.set(rp.as_ref().filter(|p| p.is_file()).map(|p| p.path().clone()));
    });

    // ── Compute rows ──────────────────────────────────────────────────────────
    let l_root_snap = left_dir.read().cloned();
    let r_root_snap = right_dir.read().cloned();

    let left_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_l.read().visible_rows()
        .into_iter().filter(|(n, _)| n.path != l_root_snap)
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d)).collect();
    let right_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_r.read().visible_rows()
        .into_iter().filter(|(n, _)| n.path != r_root_snap)
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d)).collect();

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
                        on_back:    move |_| { let p = left_hist.write().back();    if let Some(p) = p { crate::ui::view::dir_pane::navigate_to(p, true,  store, left_hist,  left_dir); } },
                        on_forward: move |_| { let p = left_hist.write().forward(); if let Some(p) = p { crate::ui::view::dir_pane::navigate_to(p, true,  store, left_hist,  left_dir); } },
                        on_navigate: move |p| crate::ui::view::dir_pane::navigate_to(p, true, store, left_hist, left_dir),
                        lang,
                    }
                    PathBar {
                        path: right_dir.read().cloned(),
                        can_back:    right_hist.read().can_back(),
                        can_forward: right_hist.read().can_forward(),
                        on_back:    move |_| { let p = right_hist.write().back();    if let Some(p) = p { crate::ui::view::dir_pane::navigate_to(p, false, store, right_hist, right_dir); } },
                        on_forward: move |_| { let p = right_hist.write().forward(); if let Some(p) = p { crate::ui::view::dir_pane::navigate_to(p, false, store, right_hist, right_dir); } },
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
