//! Explorer workspace: aligned two-pane directory view (RFC-054).
//!
//! Both trees are managed here so their visible rows can be merged into an
//! aligned structure where same-name entries share the same horizontal row.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTree, DirectoryTreeEvent, SelectionMode, use_scan_driver};

use forskscope_core::dir::file_digest_equal;

use crate::i18n::t;
use crate::state::{Store, open_compare};
use crate::ui::deep_compare::DeepCompareView;
use crate::ui::explorer_align::compute_aligned_rows;
// ── Digest map key ────────────────────────────────────────────────────────────

/// Typed key for the shared digest map (RFC-059 §M2).
///
/// Using an enum removes the stringly-typed `PathBuf::from("r:").join(rel)`
/// namespace hack and eliminates the aliasing risk it created.
#[derive(Clone, PartialEq, Eq, Hash)]
enum DigestKey {
    /// A path present on both sides (keyed by relative path).
    Common(PathBuf),
    /// A path present on the right side only (keyed by relative path).
    RightOnly(PathBuf),
}
use crate::ui::dir_pane::{
    DigestState, FilteringExecutor, NavHistory, PathBar, TreeRow,
    navigate_to, short_name,
};

// ── Aligned-row types: see explorer_align.rs (RFC-059 §M5) ──────────────────

// ── Explorer mode ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ExplorerMode { #[default] Browse, Deep }

// ── Explorer component ────────────────────────────────────────────────────────

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut mode: Signal<ExplorerMode> = use_signal(ExplorerMode::default);

    let ignore = store.settings.read().ignore_rules();

    // ── Left pane state ──────────────────────────────────────────────────────
    let init_l = store.settings.read().last_left_dir.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
    let left_dir:  Signal<PathBuf>    = use_signal(|| init_l.clone());
    let mut left_hist: Signal<NavHistory> = use_signal(NavHistory::default);
    use_hook(|| left_hist.write().push(init_l.clone()));

    let exec_l = Arc::new(FilteringExecutor { rules: ignore.clone() });
    let mut tree_l: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(init_l.clone()));
    let scans_l = use_scan_driver(tree_l, exec_l);

    use_effect(move || {
        let root = left_dir.read().clone();
        let mut nt = DirectoryTree::new(root.clone());
        if let Some(req) = nt.on_toggled(&root) { tree_l.set(nt); scans_l.send(req); }
        else { tree_l.set(nt); }
    });

    // ── Right pane state ─────────────────────────────────────────────────────
    let init_r = store.settings.read().last_right_dir.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
    let right_dir:  Signal<PathBuf>    = use_signal(|| init_r.clone());
    let mut right_hist: Signal<NavHistory> = use_signal(NavHistory::default);
    use_hook(|| right_hist.write().push(init_r.clone()));

    let exec_r = Arc::new(FilteringExecutor { rules: ignore });
    let mut tree_r: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(init_r.clone()));
    let scans_r = use_scan_driver(tree_r, exec_r);

    use_effect(move || {
        let root = right_dir.read().clone();
        let mut nt = DirectoryTree::new(root.clone());
        if let Some(req) = nt.on_toggled(&root) { tree_r.set(nt); scans_r.send(req); }
        else { tree_r.set(nt); }
    });

    // ── Shared digest map ────────────────────────────────────────────────────
    let mut digest_map: Signal<HashMap<DigestKey, DigestState>> = use_signal(HashMap::new);

    // Compute digest status for entries visible in both panes.
    use_effect(move || {
        let l_root = left_dir.read().clone();
        let r_root = right_dir.read().clone();
        if r_root.as_os_str().is_empty() || l_root.as_os_str().is_empty() { return; }

        // Collect visible relative paths from the left tree.
        let left_entries: Vec<(PathBuf, bool)> = tree_l.read().visible_rows().into_iter()
            .filter_map(|(n, _)| {
                n.path.strip_prefix(&l_root).ok().map(|rel| (rel.to_path_buf(), n.is_dir))
            })
            .collect();

        for (rel, is_dir) in left_entries {
            if digest_map.read().contains_key(&DigestKey::Common(rel.clone())) { continue; }
            let cp = r_root.join(&rel);
            if is_dir {
                // Directory: existence check only (deep comparison is in Directory Report).
                let state = if cp.is_dir() { DigestState::Equal } else { DigestState::Unique };
                digest_map.write().insert(DigestKey::Common(rel), state);
            } else {
                // File: background byte comparison.
                if !cp.is_file() { digest_map.write().insert(DigestKey::Common(rel.clone()), DigestState::Unique); continue; }
                let lp = l_root.join(&rel);
                let rp = cp;
                let key = rel.clone();
                let mut dmap = digest_map;
                dmap.write().insert(DigestKey::Common(key.clone()), DigestState::Computing);
                spawn(async move {
                    let eq = tokio::task::spawn_blocking(move || {
                        file_digest_equal(&lp, &rp).unwrap_or(false)
                    }).await.unwrap_or(false);
                    dmap.write().insert(DigestKey::Common(key), if eq { DigestState::Equal } else { DigestState::Different });
                });
            }
        }
        // Right-only entries visible in the right tree get Unique status too.
        let r_root2 = right_dir.read().clone();
        let l_root2 = left_dir.read().clone();
        let right_entries: Vec<PathBuf> = tree_r.read().visible_rows().into_iter()
            .filter_map(|(n, _)| n.path.strip_prefix(&r_root2).ok().map(|r| r.to_path_buf()))
            .collect();
        for rel in right_entries {
            // Key is the right-side relative path; check if left counterpart exists.
            let right_key = DigestKey::RightOnly(rel.clone());
            if digest_map.read().contains_key(&right_key) { continue; }
            if !l_root2.join(&rel).exists() {
                digest_map.write().insert(right_key, DigestState::Unique);
            }
        }
    });

    // ── Picks ────────────────────────────────────────────────────────────────
    let left_pick  = store.left_pick.read().clone();
    let right_pick = store.right_pick.read().clone();
    let can_compare = left_pick.is_some() && right_pick.is_some();

    // ── Deep compare roots ───────────────────────────────────────────────────
    let deep_l = left_dir.read().clone();
    let deep_r = right_dir.read().clone();

    // ── Compute aligned rows ─────────────────────────────────────────────────
    let l_root_snap = left_dir.read().clone();
    let r_root_snap = right_dir.read().clone();
    let left_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_l.read().visible_rows().into_iter()
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d)).collect();
    let right_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_r.read().visible_rows().into_iter()
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d)).collect();
    let aligned = compute_aligned_rows(&left_flat, &right_flat, &l_root_snap, &r_root_snap);

    rsx! {
        div { class: "explorer",
            // ── Mode selector ────────────────────────────────────────────────
            div { class: "explorer-modes",
                button {
                    class: if *mode.read() == ExplorerMode::Browse { "mode-btn active" } else { "mode-btn" },
                    title: "Browse and navigate directories side by side",
                    onclick: move |_| mode.set(ExplorerMode::Browse),
                    "Browse"
                }
                button {
                    class: if *mode.read() == ExplorerMode::Deep { "mode-btn active" } else { "mode-btn" },
                    title: "Recursively compare all files in both directories and show a full status report",
                    onclick: move |_| mode.set(ExplorerMode::Deep),
                    "Directory Report"
                }
            }

            if *mode.read() == ExplorerMode::Browse {
                div { class: "explorer-browse",
                    // ── Path bars ────────────────────────────────────────────
                    div { class: "explorer-path-bars",
                        PathBar {
                            path: left_dir.read().clone(),
                            can_back:    left_hist.read().can_back(),
                            can_forward: left_hist.read().can_forward(),
                            on_back:    move |_| { let p = left_hist.write().back();    if let Some(p) = p { navigate_to(p, true,  store, left_hist,  left_dir); } },
                            on_forward: move |_| { let p = left_hist.write().forward(); if let Some(p) = p { navigate_to(p, true,  store, left_hist,  left_dir); } },
                            on_navigate: move |p| navigate_to(p, true, store, left_hist, left_dir),
                        }
                        PathBar {
                            path: right_dir.read().clone(),
                            can_back:    right_hist.read().can_back(),
                            can_forward: right_hist.read().can_forward(),
                            on_back:    move |_| { let p = right_hist.write().back();    if let Some(p) = p { navigate_to(p, false, store, right_hist, right_dir); } },
                            on_forward: move |_| { let p = right_hist.write().forward(); if let Some(p) = p { navigate_to(p, false, store, right_hist, right_dir); } },
                            on_navigate: move |p| navigate_to(p, false, store, right_hist, right_dir),
                        }
                    }

                    // ── Aligned tree body ────────────────────────────────────
                    div {
                        class: "aligned-tree",
                        tabindex: "0",
                        onkeydown: move |e: Event<KeyboardData>| {
                            use dioxus_swdir_tree::keyboard::{Modifiers as CM, TreeKey, handle_key};
                            if e.modifiers().contains(Modifiers::ALT) && e.key() == Key::ArrowUp {
                                let lp = left_dir.read().parent().map(|p| p.to_path_buf());
                                if let Some(p) = lp { navigate_to(p, true, store, left_hist, left_dir); }
                                let rp = right_dir.read().parent().map(|p| p.to_path_buf());
                                if let Some(p) = rp { navigate_to(p, false, store, right_hist, right_dir); }
                                return;
                            }
                            // Keyboard nav delegates to the left tree for focus.
                            let tk = match e.key() {
                                Key::ArrowUp => TreeKey::Up, Key::ArrowDown => TreeKey::Down,
                                Key::ArrowLeft => TreeKey::Left, Key::ArrowRight => TreeKey::Right,
                                Key::Enter => TreeKey::Enter, Key::Home => TreeKey::Home,
                                Key::End => TreeKey::End, Key::Escape => TreeKey::Escape,
                                Key::Character(ref s) if s == " " => TreeKey::Space,
                                _ => return,
                            };
                            let mods = CM { shift: e.modifiers().shift(), ctrl: e.modifiers().ctrl() };
                            let ev = handle_key(&tree_l.read(), tk, mods);
                            if let Some(ev) = ev {
                                e.prevent_default();
                                match ev {
                                    DirectoryTreeEvent::Toggled(p) => { if let Some(r) = tree_l.write().on_toggled(&p) { scans_l.send(r); } }
                                    DirectoryTreeEvent::Selected { path, is_dir, mode } => { tree_l.write().on_selected(&path, is_dir, mode); }
                                    DirectoryTreeEvent::Drag(_) => {}
                                }
                            }
                        },
                        for (left_row, right_row) in aligned.iter() {
                            {
                                let lr = left_row.clone();
                                let rr = right_row.clone();
                                rsx! {
                                    div { class: "aligned-row",
                                        // ── Left half ───────────────────────
                                        div { class: "pane-half",
                                            if let Some(ref row) = lr {
                                                {
                                                    let status = digest_map.read().get(&DigestKey::Common(row.rel_path.clone())).cloned();
                                                    let p_tgl = row.abs_path.clone();
                                                    let p_sel = row.abs_path.clone();
                                                    let p_dbl = row.abs_path.clone();
                                                    let p_nav = row.abs_path.clone();
                                                    let is_dir = row.is_dir;
                                                    let other_pick = store.right_pick.read().clone();
                                                    let mut lpick = store.left_pick;
                                                    rsx! {
                                                        TreeRow {
                                                            path: row.abs_path.clone(),
                                                            is_dir: row.is_dir, is_expanded: row.is_expanded,
                                                            is_selected: row.is_selected, depth: row.depth,
                                                            status,
                                                            on_toggle: move |_| {
                                                                if let Some(r) = tree_l.write().on_toggled(&p_tgl) { scans_l.send(r); }
                                                                digest_map.write().clear();
                                                            },
                                                            on_select: move |_| {
                                                                tree_l.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                                                if !is_dir { lpick.set(Some(p_sel.clone())); }
                                                            },
                                                            on_dblclick: move |_| {
                                                                if is_dir {
                                                                    navigate_to(p_nav.clone(), true, store, left_hist, left_dir);
                                                                } else if let Some(cp) = other_pick.as_ref().filter(|op| op.file_name() == p_dbl.file_name()) {
                                                                    open_compare(&mut store, p_dbl.clone(), cp.clone());
                                                                }
                                                            },
                                                        }
                                                    }
                                                }
                                            } else { div { class: "row-spacer" } }
                                        }
                                        // ── Right half ──────────────────────
                                        div { class: "pane-half",
                                            if let Some(ref row) = rr {
                                                {
                                                    // For right-side status: try Common first (present on both sides),
                                                    // then RightOnly (present on right side only).
                                                    let status = digest_map.read()
                                                        .get(&DigestKey::Common(row.rel_path.clone()))
                                                        .cloned()
                                                        .or_else(|| digest_map.read()
                                                            .get(&DigestKey::RightOnly(row.rel_path.clone()))
                                                            .cloned());
                                                    let p_tgl = row.abs_path.clone();
                                                    let p_sel = row.abs_path.clone();
                                                    let p_dbl = row.abs_path.clone();
                                                    let p_nav = row.abs_path.clone();
                                                    let is_dir = row.is_dir;
                                                    let other_pick = store.left_pick.read().clone();
                                                    let mut rpick = store.right_pick;
                                                    rsx! {
                                                        TreeRow {
                                                            path: row.abs_path.clone(),
                                                            is_dir: row.is_dir, is_expanded: row.is_expanded,
                                                            is_selected: row.is_selected, depth: row.depth,
                                                            status,
                                                            on_toggle: move |_| {
                                                                if let Some(r) = tree_r.write().on_toggled(&p_tgl) { scans_r.send(r); }
                                                                digest_map.write().clear();
                                                            },
                                                            on_select: move |_| {
                                                                tree_r.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                                                if !is_dir { rpick.set(Some(p_sel.clone())); }
                                                            },
                                                            on_dblclick: move |_| {
                                                                if is_dir {
                                                                    navigate_to(p_nav.clone(), false, store, right_hist, right_dir);
                                                                } else if let Some(cp) = other_pick.as_ref().filter(|op| op.file_name() == p_dbl.file_name()) {
                                                                    open_compare(&mut store, cp.clone(), p_dbl.clone());
                                                                }
                                                            },
                                                        }
                                                    }
                                                }
                                            } else { div { class: "row-spacer" } }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Footer ───────────────────────────────────────────────
                    div { class: "explorer-footer",
                        button {
                            disabled: !can_compare,
                            onclick: move |_| {
                                let l = store.left_pick.read().clone();
                                let r = store.right_pick.read().clone();
                                if let (Some(l), Some(r)) = (l, r) { open_compare(&mut store, l, r); }
                            },
                            {t(lang, "Compare")}
                        }
                        if let (Some(l), Some(r)) = (&left_pick, &right_pick) {
                            span { class: "compare-label",
                                {format!("{} ↔ {}", short_name(l), short_name(r))}
                            }
                        }
                    }
                }
            } else {
                DeepCompareView { left_root: deep_l, right_root: deep_r, lang }
            }
        }
    }
}
