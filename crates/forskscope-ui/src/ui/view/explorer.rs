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
use crate::state::{Store, open_compare, open_dir_compare};
use crate::ui::bridge::explorer_align::compute_aligned_rows;
// ── Digest map key ────────────────────────────────────────────────────────────

/// Typed key for the shared digest map (RFC-059 §M2).
#[derive(Clone, PartialEq, Eq, Hash)]
enum DigestKey {
    Common(PathBuf),
    RightOnly(PathBuf),
}
use crate::ui::view::dir_pane::{
    DigestState, FilteringExecutor, NavHistory, PathBar, TreeRow,
    navigate_to, short_name,
};

// ── Focused pane (RFC-061) ────────────────────────────────────────────────────

/// Which pane currently receives keyboard events in the Explorer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusedPane { Left, Right }

impl FocusedPane {
    fn toggle(self) -> Self { match self { Self::Left => Self::Right, Self::Right => Self::Left } }
    fn is_left(self)  -> bool { self == Self::Left }
    fn is_right(self) -> bool { self == Self::Right }
}

#[derive(Clone, PartialEq, Eq)]
enum PickKind { File(PathBuf), Dir(PathBuf) }

impl PickKind {
    fn path(&self) -> &PathBuf { match self { Self::File(p) | Self::Dir(p) => p } }
    fn is_file(&self) -> bool { matches!(self, Self::File(_)) }
}

// ── Compare action derived from picks ────────────────────────────────────────

#[derive(Clone, PartialEq, Eq)]
enum CompareAction {
    /// Both picks are files — open a file diff tab.
    Files(PathBuf, PathBuf),
    /// Both picks are directories — open Directory Report.
    Dirs(PathBuf, PathBuf),
    /// Picks are mismatched or missing — disabled.
    None,
}

fn compare_action(lp: &Option<PickKind>, rp: &Option<PickKind>) -> CompareAction {
    match (lp, rp) {
        (Some(PickKind::File(l)), Some(PickKind::File(r))) =>
            CompareAction::Files(l.clone(), r.clone()),
        (Some(PickKind::Dir(l)), Some(PickKind::Dir(r))) =>
            CompareAction::Dirs(l.clone(), r.clone()),
        _ => CompareAction::None,
    }
}

// ── Explorer component ────────────────────────────────────────────────────────

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    let ignore = store.settings.read().ignore_rules();
    let binary_enabled = store.settings.read().enable_binary_comparison;
    let compact_mode   = store.settings.read().explorer_compact;

    // Cache of paths known to be binary; populated lazily at render time (RFC-066).
    // Cleared when either directory changes so stale results don't linger.
    let mut binary_cache: Signal<std::collections::HashMap<PathBuf, bool>> = use_signal(Default::default);

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
        let root = left_dir.read().cloned();
        let mut nt = DirectoryTree::new(root.clone());
        binary_cache.write().clear(); // stale results invalid when dir changes
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
        let root = right_dir.read().cloned();
        let mut nt = DirectoryTree::new(root.clone());
        binary_cache.write().clear(); // stale results invalid when dir changes
        if let Some(req) = nt.on_toggled(&root) { tree_r.set(nt); scans_r.send(req); }
        else { tree_r.set(nt); }
    });

    // ── Shared digest map ────────────────────────────────────────────────────
    let mut digest_map: Signal<HashMap<DigestKey, DigestState>> = use_signal(HashMap::new);
    let mut digest_roots: Signal<(PathBuf, PathBuf)> = use_signal(|| (PathBuf::new(), PathBuf::new()));

    use_effect(move || {
        let l_root = left_dir.read().cloned();
        let r_root = right_dir.read().cloned();
        if r_root.as_os_str().is_empty() || l_root.as_os_str().is_empty() { return; }

        {
            let roots = digest_roots.read();
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
                if rel.as_os_str().is_empty() { return None; } // skip root itself
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
            let right_key = DigestKey::RightOnly(rel.clone());
            if digest_map.read().contains_key(&right_key) { continue; }
            if !l_root2.join(&rel).exists() {
                digest_map.write().insert(right_key, DigestState::Unique);
            }
        }
    });

    // ── Filter bar state (RFC-067) ────────────────────────────────────────────
    let mut filter_open:    Signal<bool>   = use_signal(|| false);
    let mut filter_query:   Signal<String> = use_signal(String::new);
    let mut filter_hide_bin:Signal<bool>   = use_signal(|| false);
    let mut filter_hide_eq: Signal<bool>   = use_signal(|| false);

    // ── Picks (file or directory) ─────────────────────────────────────────────
    let mut left_pick:  Signal<Option<PickKind>> = use_signal(|| None);
    let mut right_pick: Signal<Option<PickKind>> = use_signal(|| None);

    // ── Focused pane (RFC-061) ────────────────────────────────────────────────
    // F6 switches focus; keyboard tree events dispatch to the focused tree.
    let mut focused_pane: Signal<FocusedPane> = use_signal(|| FocusedPane::Left);

    // Also sync file picks into Store so dblclick priority logic can read them.
    use_effect(move || {
        let lp = left_pick.read();
        store.left_pick.set(lp.as_ref().filter(|p| p.is_file()).map(|p| p.path().clone()));
    });
    use_effect(move || {
        let rp = right_pick.read();
        store.right_pick.set(rp.as_ref().filter(|p| p.is_file()).map(|p| p.path().clone()));
    });

    // ── Compute aligned rows ─────────────────────────────────────────────────
    let l_root_snap = left_dir.read().cloned();
    let r_root_snap = right_dir.read().cloned();
    let left_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_l.read().visible_rows().into_iter()
        .filter(|(n, _)| n.path != l_root_snap)   // skip the root node itself
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d)).collect();
    let right_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_r.read().visible_rows().into_iter()
        .filter(|(n, _)| n.path != r_root_snap)   // skip the root node itself
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d)).collect();
    let aligned = compute_aligned_rows(&left_flat, &right_flat, &l_root_snap, &r_root_snap);

    // ── Apply filter bar (RFC-067) ────────────────────────────────────────────
    let q      = filter_query.read().to_lowercase();
    let h_bin  = *filter_hide_bin.read();
    let h_eq   = *filter_hide_eq.read();
    let bin_en = binary_enabled;
    let aligned: Vec<_> = aligned.into_iter().filter(|(lr, rr)| {
        // Name filter: match if either side's filename contains query.
        let name_ok = if q.is_empty() { true } else {
            let l_match = lr.as_ref().and_then(|r| r.rel_path.file_name())
                .map(|n| n.to_string_lossy().to_lowercase().contains(&q))
                .unwrap_or(false);
            let r_match = rr.as_ref().and_then(|r| r.rel_path.file_name())
                .map(|n| n.to_string_lossy().to_lowercase().contains(&q))
                .unwrap_or(false);
            l_match || r_match
        };
        // Hide-binary filter: hide when all present file sides are binary
        // (and binary comparison is off, so "bin" badge is shown).
        // Uses binary_cache to avoid redundant file I/O on every render.
        let bin_ok = if h_bin && !bin_en {
            let l_bin = lr.as_ref().map(|r| {
                if r.is_dir { return false; }
                let cached = binary_cache.read().get(&r.abs_path).copied();
                cached.unwrap_or_else(|| {
                    let b = matches!(forskscope_core::file_kind::classify(&r.abs_path),
                        Ok(forskscope_core::file_kind::FileKind::Binary));
                    binary_cache.write().insert(r.abs_path.clone(), b);
                    b
                })
            }).unwrap_or(false);
            let r_bin = rr.as_ref().map(|r| {
                if r.is_dir { return false; }
                let cached = binary_cache.read().get(&r.abs_path).copied();
                cached.unwrap_or_else(|| {
                    let b = matches!(forskscope_core::file_kind::classify(&r.abs_path),
                        Ok(forskscope_core::file_kind::FileKind::Binary));
                    binary_cache.write().insert(r.abs_path.clone(), b);
                    b
                })
            }).unwrap_or(false);
            let l_present = lr.is_some();
            let r_present = rr.is_some();
            // Show if at least one present side is NOT binary.
            match (l_present, r_present) {
                (true,  true)  => !l_bin || !r_bin,
                (true,  false) => !l_bin,
                (false, true)  => !r_bin,
                (false, false) => true,
            }
        } else { true };
        // Hide-identical filter: hide when digest is Equal.
        let eq_ok = if h_eq {
            let rel = lr.as_ref().or(rr.as_ref()).map(|r| r.rel_path.clone());
            if let Some(rel) = rel {
                !matches!(digest_map.read().get(&DigestKey::Common(rel)), Some(DigestState::Equal))
            } else { true }
        } else { true };
        name_ok && bin_ok && eq_ok
    }).collect();

    // ── Compare button label and state ────────────────────────────────────────
    let lp = left_pick.read().clone();
    let rp = right_pick.read().clone();
    let action = compare_action(&lp, &rp);
    let can_compare = action != CompareAction::None;
    let compare_tooltip = match &action {
        CompareAction::Files(..) => t(lang, "Compare selected files"),
        CompareAction::Dirs(..)  => t(lang, "Compare selected directories"),
        CompareAction::None => t(lang, "Select a file or directory on each side to compare"),
    };

    rsx! {
        div { class: "explorer",

            // ── Browse view ───────────────────────────────────────────────
            div { class: "explorer-browse",
                    div { class: "explorer-path-bars",
                        PathBar {
                            path: left_dir.read().cloned(),
                            can_back:    left_hist.read().can_back(),
                            can_forward: left_hist.read().can_forward(),
                            on_back:    move |_| { let p = left_hist.write().back();    if let Some(p) = p { navigate_to(p, true,  store, left_hist,  left_dir); } },
                            on_forward: move |_| { let p = left_hist.write().forward(); if let Some(p) = p { navigate_to(p, true,  store, left_hist,  left_dir); } },
                            on_navigate: move |p| navigate_to(p, true, store, left_hist, left_dir),
                            lang,
                        }
                        PathBar {
                            path: right_dir.read().cloned(),
                            can_back:    right_hist.read().can_back(),
                            can_forward: right_hist.read().can_forward(),
                            on_back:    move |_| { let p = right_hist.write().back();    if let Some(p) = p { navigate_to(p, false, store, right_hist, right_dir); } },
                            on_forward: move |_| { let p = right_hist.write().forward(); if let Some(p) = p { navigate_to(p, false, store, right_hist, right_dir); } },
                            on_navigate: move |p| navigate_to(p, false, store, right_hist, right_dir),
                            lang,
                        }
                    }

                    // ── Filter bar (RFC-067) ──────────────────────────────────
                    div { class: "filter-bar-row",
                        button {
                            class: if *filter_open.read() { "filter-toggle active" } else { "filter-toggle" },
                            title: t(lang, "Filter items"),
                            aria_label: t(lang, "Filter items"),
                            onclick: move |_| { let v = *filter_open.read(); filter_open.set(!v); },
                            "⊞"
                        }
                        if *filter_open.read() {
                            input {
                                class: "filter-input",
                                r#type: "search",
                                placeholder: t(lang, "Filter by name…"),
                                value: "{filter_query}",
                                oninput: move |e| filter_query.set(e.value()),
                                onkeydown: move |e| { e.stop_propagation(); },
                            }
                            label { class: "filter-check",
                                input { r#type: "checkbox", checked: *filter_hide_bin.read(),
                                    onchange: move |e| filter_hide_bin.set(e.checked()) }
                                span { {t(lang, "Hide binary")} }
                            }
                            label { class: "filter-check",
                                input { r#type: "checkbox", checked: *filter_hide_eq.read(),
                                    onchange: move |e| filter_hide_eq.set(e.checked()) }
                                span { {t(lang, "Hide identical")} }
                            }
                            if !filter_query.read().is_empty() || *filter_hide_bin.read() || *filter_hide_eq.read() {
                                button {
                                    class: "filter-clear",
                                    title: t(lang, "Clear filter"),
                                    onclick: move |_| {
                                        filter_query.set(String::new());
                                        filter_hide_bin.set(false);
                                        filter_hide_eq.set(false);
                                    },
                                    "✕"
                                }
                            }
                        }
                    }

                    // ── Per-pane root labels (pinned between path bar and scroll area) ─
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
                    // ── Aligned OR compact tree ───────────────────────────────
                    if !compact_mode {
                        div {
                        id: "aligned-tree",
                        class: "aligned-tree",
                        tabindex: "0",
                        onkeydown: move |e: Event<KeyboardData>| {
                            use dioxus_swdir_tree::keyboard::{Modifiers as CM, TreeKey, handle_key};

                            // F6: switch focused pane (RFC-061).
                            if e.key() == Key::F6 {
                                e.prevent_default();
                                let next = focused_pane.read().toggle();
                                focused_pane.set(next);
                                return;
                            }

                            // Alt+↑: navigate the focused pane up (RFC-061 — per-pane, not both).
                            if e.modifiers().contains(Modifiers::ALT) && e.key() == Key::ArrowUp {
                                e.prevent_default();
                                if focused_pane.read().is_left() {
                                    if let Some(p) = left_dir.read().parent().map(|p| p.to_path_buf()) {
                                        navigate_to(p, true, store, left_hist, left_dir);
                                    }
                                } else {
                                    if let Some(p) = right_dir.read().parent().map(|p| p.to_path_buf()) {
                                        navigate_to(p, false, store, right_hist, right_dir);
                                    }
                                }
                                return;
                            }

                            let (tk, is_select_key) = match e.key() {
                                Key::ArrowUp    => (TreeKey::Up,    false),
                                Key::ArrowDown  => (TreeKey::Down,  false),
                                Key::ArrowLeft  => (TreeKey::Left,  false),
                                Key::ArrowRight => (TreeKey::Right, false),
                                Key::Enter      => (TreeKey::Enter, true),
                                Key::Home       => (TreeKey::Home,  false),
                                Key::End        => (TreeKey::End,   false),
                                Key::Escape     => (TreeKey::Escape,false),
                                Key::Character(ref s) if s == " " => (TreeKey::Space, true),
                                _ => return,
                            };
                            let mods = CM { shift: e.modifiers().shift(), ctrl: e.modifiers().ctrl() };

                            // Dispatch to the focused tree (RFC-061).
                            if focused_pane.read().is_left() {
                                let ev = handle_key(&tree_l.read(), tk, mods);
                                if let Some(ev) = ev {
                                    e.prevent_default();
                                    match ev {
                                        DirectoryTreeEvent::Toggled(p) => {
                                            if let Some(r) = tree_l.write().on_toggled(&p) { scans_l.send(r); }
                                        }
                                        DirectoryTreeEvent::Selected { path, is_dir, mode } => {
                                            tree_l.write().on_selected(&path, is_dir, mode);
                                            // Space / Enter: set as left pick.
                                            if is_select_key {
                                                left_pick.set(Some(if is_dir {
                                                    PickKind::Dir(path)
                                                } else {
                                                    PickKind::File(path)
                                                }));
                                            }
                                        }
                                        DirectoryTreeEvent::Drag(_) => {}
                                    }
                                }
                            } else {
                                let ev = handle_key(&tree_r.read(), tk, mods);
                                if let Some(ev) = ev {
                                    e.prevent_default();
                                    match ev {
                                        DirectoryTreeEvent::Toggled(p) => {
                                            if let Some(r) = tree_r.write().on_toggled(&p) { scans_r.send(r); }
                                        }
                                        DirectoryTreeEvent::Selected { path, is_dir, mode } => {
                                            tree_r.write().on_selected(&path, is_dir, mode);
                                            // Space / Enter: set as right pick.
                                            if is_select_key {
                                                right_pick.set(Some(if is_dir {
                                                    PickKind::Dir(path)
                                                } else {
                                                    PickKind::File(path)
                                                }));
                                            }
                                        }
                                        DirectoryTreeEvent::Drag(_) => {}
                                    }
                                }
                            }
                        },
                    // ── Aligned entries (children of each root) ────────────────
                    if aligned.is_empty() {
                        div { class: "explorer-empty",
                            div { class: "explorer-empty-icon", "📂" }
                            p { class: "explorer-empty-title", {t(lang, "Compare files or folders")} }
                            p { class: "explorer-empty-hint",
                                {t(lang, "Choose a folder for each side, then select items to compare.")}
                            }
                            p { class: "explorer-empty-local",
                                "🔒 " {t(lang, "Nothing leaves this computer.")}
                            }
                        }
                    }
                    for (left_row, right_row) in aligned.iter() {
                            {
                                let lr = left_row.clone();
                                let rr = right_row.clone();
                                rsx! {
                                    div { class: "aligned-row",
                                        div { class: "pane-half",
                                            if let Some(ref row) = lr {
                                                {
                                                    let status = digest_map.read().get(&DigestKey::Common(row.rel_path.clone())).cloned();
                                                    let p_tgl = row.abs_path.clone();
                                                    let p_sel = row.abs_path.clone();
                                                    let p_dbl = row.abs_path.clone();
                                                    let p_nav = row.abs_path.clone();
                                                    let p_bin = row.abs_path.clone();
                                                    let is_dir = row.is_dir;
                                                    // Binary detection (RFC-066).
                                                    let is_binary = if is_dir { false } else {
                                                        let cached = binary_cache.read().get(&row.abs_path).copied();
                                                        cached.unwrap_or_else(|| {
                                                            let b = matches!(forskscope_core::file_kind::classify(&p_bin), Ok(forskscope_core::file_kind::FileKind::Binary));
                                                            binary_cache.write().insert(p_bin, b);
                                                            b
                                                        })
                                                    };
                                                    rsx! {
                                                        TreeRow {
                                                            path: row.abs_path.clone(),
                                                            is_dir: row.is_dir, is_expanded: row.is_expanded,
                                                            is_selected: row.is_selected, depth: row.depth,
                                                            status,
                                                            is_binary,
                                                            binary_enabled,
                                                            on_toggle: move |_| {
                                                                if let Some(r) = tree_l.write().on_toggled(&p_tgl) { scans_l.send(r); }
                                                                digest_map.write().clear();
                                                            },
                                                            on_select: move |_| {
                                                                tree_l.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                                                left_pick.set(Some(if is_dir {
                                                                    PickKind::Dir(p_sel.clone())
                                                                } else {
                                                                    PickKind::File(p_sel.clone())
                                                                }));
                                                            },
                                                            on_dblclick: move |_| {
                                                                if is_dir {
                                                                    navigate_to(p_nav.clone(), true, store, left_hist, left_dir);
                                                                } else {
                                                                    let rp = store.right_pick.read().cloned();
                                                                    if let Some(cp) = rp.filter(|p| p.is_file()) {
                                                                        open_compare(&mut store, p_dbl.clone(), cp);
                                                                        return;
                                                                    }
                                                                    let l_root = left_dir.read().cloned();
                                                                    let r_root = right_dir.read().cloned();
                                                                    if let Ok(rel) = p_dbl.strip_prefix(&l_root) {
                                                                        let cp = r_root.join(rel);
                                                                        if cp.is_file() {
                                                                            open_compare(&mut store, p_dbl.clone(), cp);
                                                                        }
                                                                    }
                                                                }
                                                            },
                                                        }
                                                    }
                                                }
                                            } else { div { class: "row-spacer" } }
                                        }
                                        div { class: "pane-half",
                                            if let Some(ref row) = rr {
                                                {
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
                                                    let p_bin = row.abs_path.clone();
                                                    let is_dir = row.is_dir;
                                                    // Binary detection (RFC-066).
                                                    let is_binary = if is_dir { false } else {
                                                        let cached = binary_cache.read().get(&row.abs_path).copied();
                                                        cached.unwrap_or_else(|| {
                                                            let b = matches!(forskscope_core::file_kind::classify(&p_bin), Ok(forskscope_core::file_kind::FileKind::Binary));
                                                            binary_cache.write().insert(p_bin, b);
                                                            b
                                                        })
                                                    };
                                                    rsx! {
                                                        TreeRow {
                                                            path: row.abs_path.clone(),
                                                            is_dir: row.is_dir, is_expanded: row.is_expanded,
                                                            is_selected: row.is_selected, depth: row.depth,
                                                            status,
                                                            is_binary,
                                                            binary_enabled,
                                                            on_toggle: move |_| {
                                                                if let Some(r) = tree_r.write().on_toggled(&p_tgl) { scans_r.send(r); }
                                                                digest_map.write().clear();
                                                            },
                                                            on_select: move |_| {
                                                                tree_r.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                                                right_pick.set(Some(if is_dir {
                                                                    PickKind::Dir(p_sel.clone())
                                                                } else {
                                                                    PickKind::File(p_sel.clone())
                                                                }));
                                                            },
                                                            on_dblclick: move |_| {
                                                                if is_dir {
                                                                    navigate_to(p_nav.clone(), false, store, right_hist, right_dir);
                                                                } else {
                                                                    let lp = store.left_pick.read().cloned();
                                                                    if let Some(cp) = lp.filter(|p| p.is_file()) {
                                                                        open_compare(&mut store, cp, p_dbl.clone());
                                                                        return;
                                                                    }
                                                                    let l_root = left_dir.read().cloned();
                                                                    let r_root = right_dir.read().cloned();
                                                                    if let Ok(rel) = p_dbl.strip_prefix(&r_root) {
                                                                        let cp = l_root.join(rel);
                                                                        if cp.is_file() {
                                                                            open_compare(&mut store, cp, p_dbl.clone());
                                                                        }
                                                                    }
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
                    } // end if !compact_mode

                    if compact_mode {
                        // ── Compact (unaligned) view — RFC-068 ───────────────
                        // Each pane scrolls independently; no spacer rows;
                        // cross-pane row alignment is intentionally absent.
                        div { class: "compact-tree",
                            // Left pane
                            div { class: "compact-pane",
                                for row in left_flat.iter().filter(|(p, _is_dir, ..)| {
                                    if filter_query.read().is_empty() { true } else {
                                        p.file_name().map(|n| n.to_string_lossy().to_lowercase()
                                            .contains(&*filter_query.read())).unwrap_or(false)
                                    }
                                }) {
                                    {
                                        // row is &(PathBuf, bool, bool, bool, u32) — clone gives owned values
                                        let (abs, is_dir, is_expanded, is_selected, depth) = row.clone();
                                        let rel = abs.strip_prefix(&l_root_snap).unwrap_or(&abs).to_path_buf();
                                        // Left-only files have no DigestKey entry; Common covers same-name pairs.
                                        let status = digest_map.read().get(&DigestKey::Common(rel.clone())).cloned();
                                        let p_tgl = abs.clone(); let p_sel = abs.clone();
                                        let p_dbl = abs.clone(); let p_nav = abs.clone();
                                        let p_bin = abs.clone();
                                        let is_binary = if is_dir { false } else {
                                            let cached = binary_cache.read().get(&abs).copied();
                                            cached.unwrap_or_else(|| {
                                                let b = matches!(forskscope_core::file_kind::classify(&p_bin),
                                                    Ok(forskscope_core::file_kind::FileKind::Binary));
                                                binary_cache.write().insert(p_bin, b);
                                                b
                                            })
                                        };
                                        rsx! {
                                            TreeRow {
                                                path: abs.clone(), is_dir,
                                                is_expanded, is_selected,
                                                depth, status, is_binary, binary_enabled,
                                                on_toggle: move |_| {
                                                    if let Some(r) = tree_l.write().on_toggled(&p_tgl) { scans_l.send(r); }
                                                    digest_map.write().clear();
                                                },
                                                on_select: move |_| {
                                                    tree_l.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                                    left_pick.set(Some(if is_dir { PickKind::Dir(p_sel.clone()) } else { PickKind::File(p_sel.clone()) }));
                                                },
                                                on_dblclick: move |_| {
                                                    if is_dir { navigate_to(p_nav.clone(), true, store, left_hist, left_dir); }
                                                    else {
                                                        let rp = store.right_pick.read().cloned();
                                                        if let Some(cp) = rp.filter(|p| p.is_file()) {
                                                            open_compare(&mut store, p_dbl.clone(), cp);
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                            // Right pane
                            div { class: "compact-pane compact-pane-right",
                                for row in right_flat.iter().filter(|(p, _is_dir, ..)| {
                                    if filter_query.read().is_empty() { true } else {
                                        p.file_name().map(|n| n.to_string_lossy().to_lowercase()
                                            .contains(&*filter_query.read())).unwrap_or(false)
                                    }
                                }) {
                                    {
                                        let (abs, is_dir, is_expanded, is_selected, depth) = row.clone();
                                        let rel = abs.strip_prefix(&r_root_snap).unwrap_or(&abs).to_path_buf();
                                        // Two separate reads avoid holding two guards simultaneously (E0515).
                                        let status = {
                                            let common   = digest_map.read().get(&DigestKey::Common(rel.clone())).cloned();
                                            let right_only = digest_map.read().get(&DigestKey::RightOnly(rel.clone())).cloned();
                                            common.or(right_only)
                                        };
                                        let p_tgl = abs.clone(); let p_sel = abs.clone();
                                        let p_dbl = abs.clone(); let p_nav = abs.clone();
                                        let p_bin = abs.clone();
                                        let is_binary = if is_dir { false } else {
                                            let cached = binary_cache.read().get(&abs).copied();
                                            cached.unwrap_or_else(|| {
                                                let b = matches!(forskscope_core::file_kind::classify(&p_bin),
                                                    Ok(forskscope_core::file_kind::FileKind::Binary));
                                                binary_cache.write().insert(p_bin, b);
                                                b
                                            })
                                        };
                                        rsx! {
                                            TreeRow {
                                                path: abs.clone(), is_dir,
                                                is_expanded, is_selected,
                                                depth, status, is_binary, binary_enabled,
                                                on_toggle: move |_| {
                                                    if let Some(r) = tree_r.write().on_toggled(&p_tgl) { scans_r.send(r); }
                                                    digest_map.write().clear();
                                                },
                                                on_select: move |_| {
                                                    tree_r.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                                    right_pick.set(Some(if is_dir { PickKind::Dir(p_sel.clone()) } else { PickKind::File(p_sel.clone()) }));
                                                },
                                                on_dblclick: move |_| {
                                                    if is_dir { navigate_to(p_nav.clone(), false, store, right_hist, right_dir); }
                                                    else {
                                                        let lp = store.left_pick.read().cloned();
                                                        if let Some(cp) = lp.filter(|p| p.is_file()) {
                                                            open_compare(&mut store, cp, p_dbl.clone());
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Footer: targets label + compare action (RFC-069) ─────
                    div { class: "explorer-footer",
                        div { class: "targets-label",
                            {
                                let lp = left_pick.read();
                                let rp = right_pick.read();
                                match (&*lp, &*rp) {
                                    (None, None) => rsx! {
                                        span { class: "targets-hint",
                                            {t(lang, "Choose a file or folder on each side to compare")}
                                        }
                                    },
                                    (Some(l), None) => rsx! {
                                        span { class: "targets-pick", {short_name(l.path())} }
                                        span { class: "targets-sep", " ↔ " }
                                        span { class: "targets-hint", {t(lang, "Choose a file or folder on the right")} }
                                    },
                                    (None, Some(r)) => rsx! {
                                        span { class: "targets-hint", {t(lang, "Choose a file or folder on the left")} }
                                        span { class: "targets-sep", " ↔ " }
                                        span { class: "targets-pick", {short_name(r.path())} }
                                    },
                                    (Some(l), Some(r)) => rsx! {
                                        span { class: "targets-pick", {short_name(l.path())} }
                                        span { class: "targets-sep", " ↔ " }
                                        span { class: "targets-pick", {short_name(r.path())} }
                                    },
                                }
                            }
                        }
                        button {
                            class: "compare-btn",
                            disabled: !can_compare,
                            title: compare_tooltip.clone(),
                            aria_label: compare_tooltip.clone(),
                            onclick: move |_| {
                                let lp = left_pick.read().clone();
                                let rp = right_pick.read().clone();
                                match compare_action(&lp, &rp) {
                                    CompareAction::Files(l, r) => open_compare(&mut store, l, r),
                                    CompareAction::Dirs(l, r)  => open_dir_compare(&mut store, l, r),
                                    CompareAction::None => {}
                                }
                            },
                            {t(lang, "Compare")} " ▶"
                        }
                    }
            }
        }
    }
}
