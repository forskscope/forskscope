//! Aligned two-pane tree rendering with keyboard navigation (RFC-054, RFC-061).
//!
//! Displays same-name entries on the same row with spacers where one side is
//! missing. Keyboard events are dispatched to the focused pane.

use std::collections::HashMap;
use std::path::PathBuf;

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTree, DirectoryTreeEvent, SelectionMode, ScanRequest};
use forskscope_ui_logic::AlignedRow;

use crate::i18n::t;
use crate::state::{Lang, Store, open_compare};
use crate::ui::view::dir_pane::{DigestState, NavHistory, TreeRow, navigate_to};
use super::{DigestKey, FocusedPane, PickKind};

#[allow(clippy::too_many_arguments)]
#[component]
pub fn AlignedTree(
    lang:             Lang,
    aligned:          Vec<AlignedRow>,
    mut tree_l:       Signal<DirectoryTree>,
    mut tree_r:       Signal<DirectoryTree>,
    scans_l:          Coroutine<ScanRequest>,
    scans_r:          Coroutine<ScanRequest>,
    left_dir:         Signal<PathBuf>,
    right_dir:        Signal<PathBuf>,
    left_hist:        Signal<NavHistory>,
    right_hist:       Signal<NavHistory>,
    mut left_pick:    Signal<Option<PickKind>>,
    mut right_pick:   Signal<Option<PickKind>>,
    mut focused_pane: Signal<FocusedPane>,
    mut digest_map:   Signal<HashMap<DigestKey, DigestState>>,
    mut binary_cache: Signal<HashMap<PathBuf, bool>>,
    binary_enabled:   bool,
) -> Element {
    let mut store = use_context::<Store>();
    let l_root = left_dir.read().cloned();
    let r_root = right_dir.read().cloned();

    rsx! {
        div {
            id: "aligned-tree",
            class: "aligned-tree",
            tabindex: "0",
            onkeydown: move |e: Event<KeyboardData>| {
                use dioxus_swdir_tree::keyboard::{Modifiers as CM, TreeKey, handle_key};

                if e.key() == Key::F6 {
                    e.prevent_default();
                    // Drop read guard before calling set (E0502).
                    let next = focused_pane.read().toggle();
                    focused_pane.set(next);
                    return;
                }
                if e.modifiers().contains(Modifiers::ALT) && e.key() == Key::ArrowUp {
                    e.prevent_default();
                    if focused_pane.read().is_left() {
                        if let Some(p) = left_dir.read().parent().map(|p| p.to_path_buf()) {
                            navigate_to(p, true, store, left_hist, left_dir);
                        }
                    } else if let Some(p) = right_dir.read().parent().map(|p| p.to_path_buf()) {
                        navigate_to(p, false, store, right_hist, right_dir);
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
                if focused_pane.read().is_left() {
                    // Evaluate the event while holding the read guard, then drop
                    // before calling write (E0502).
                    let ev = handle_key(&tree_l.read(), tk, mods);
                    if let Some(ev) = ev {
                        e.prevent_default();
                        match ev {
                            DirectoryTreeEvent::Toggled(p) => {
                                if let Some(r) = tree_l.write().on_toggled(&p) { scans_l.send(r); }
                            }
                            DirectoryTreeEvent::Selected { path, is_dir, mode } => {
                                tree_l.write().on_selected(&path, is_dir, mode);
                                if is_select_key {
                                    left_pick.set(Some(if is_dir { PickKind::Dir(path) } else { PickKind::File(path) }));
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
                                if is_select_key {
                                    right_pick.set(Some(if is_dir { PickKind::Dir(path) } else { PickKind::File(path) }));
                                }
                            }
                            DirectoryTreeEvent::Drag(_) => {}
                        }
                    }
                }
            },

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
                    // Clone roots so closures can capture them repeatedly (E0507).
                    let l_root_c = l_root.clone();
                    let r_root_c = r_root.clone();
                    rsx! {
                        div { class: "aligned-row",
                            // ── Left half ────────────────────────────────
                            div { class: "pane-half",
                                if let Some(ref row) = lr {
                                    {
                                        let status = digest_map.read()
                                            .get(&DigestKey::Common(row.rel_path.clone()))
                                            .cloned();
                                        let p_tgl = row.abs_path.clone();
                                        let p_sel = row.abs_path.clone();
                                        let p_dbl = row.abs_path.clone();
                                        let p_nav = row.abs_path.clone();
                                        let p_bin = row.abs_path.clone();
                                        let is_dir = row.is_dir;
                                        let is_binary = if is_dir { false } else {
                                            let cached = binary_cache.read().get(&row.abs_path).copied();
                                            cached.unwrap_or_else(|| {
                                                let b = matches!(
                                                    forskscope_core::file_kind::classify(&p_bin),
                                                    Ok(forskscope_core::file_kind::FileKind::Binary)
                                                );
                                                binary_cache.write().insert(p_bin, b);
                                                b
                                            })
                                        };
                                        rsx! {
                                            TreeRow {
                                                path: row.abs_path.clone(),
                                                is_dir, is_expanded: row.is_expanded,
                                                is_selected: row.is_selected, depth: row.depth,
                                                status, is_binary, binary_enabled,
                                                on_toggle: move |_| {
                                                    if let Some(r) = tree_l.write().on_toggled(&p_tgl) { scans_l.send(r); }
                                                    digest_map.write().clear();
                                                },
                                                on_select: move |_| {
                                                    tree_l.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                                    left_pick.set(Some(if is_dir { PickKind::Dir(p_sel.clone()) } else { PickKind::File(p_sel.clone()) }));
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
                                                        let r_root = right_dir.read().cloned();
                                                        if let Ok(rel) = p_dbl.strip_prefix(&l_root_c) {
                                                            let cp = r_root.join(rel);
                                                            if cp.is_file() { open_compare(&mut store, p_dbl.clone(), cp); }
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                    }
                                } else { div { class: "row-spacer" } }
                            }
                            // ── Right half ───────────────────────────────
                            div { class: "pane-half",
                                if let Some(ref row) = rr {
                                    {
                                        let common     = digest_map.read().get(&DigestKey::Common(row.rel_path.clone())).cloned();
                                        let right_only = digest_map.read().get(&DigestKey::RightOnly(row.rel_path.clone())).cloned();
                                        let status = common.or(right_only);
                                        let p_tgl = row.abs_path.clone();
                                        let p_sel = row.abs_path.clone();
                                        let p_dbl = row.abs_path.clone();
                                        let p_nav = row.abs_path.clone();
                                        let p_bin = row.abs_path.clone();
                                        let is_dir = row.is_dir;
                                        let is_binary = if is_dir { false } else {
                                            let cached = binary_cache.read().get(&row.abs_path).copied();
                                            cached.unwrap_or_else(|| {
                                                let b = matches!(
                                                    forskscope_core::file_kind::classify(&p_bin),
                                                    Ok(forskscope_core::file_kind::FileKind::Binary)
                                                );
                                                binary_cache.write().insert(p_bin, b);
                                                b
                                            })
                                        };
                                        rsx! {
                                            TreeRow {
                                                path: row.abs_path.clone(),
                                                is_dir, is_expanded: row.is_expanded,
                                                is_selected: row.is_selected, depth: row.depth,
                                                status, is_binary, binary_enabled,
                                                on_toggle: move |_| {
                                                    if let Some(r) = tree_r.write().on_toggled(&p_tgl) { scans_r.send(r); }
                                                    digest_map.write().clear();
                                                },
                                                on_select: move |_| {
                                                    tree_r.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                                    right_pick.set(Some(if is_dir { PickKind::Dir(p_sel.clone()) } else { PickKind::File(p_sel.clone()) }));
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
                                                        if let Ok(rel) = p_dbl.strip_prefix(&r_root_c) {
                                                            let cp = l_root.join(rel);
                                                            if cp.is_file() { open_compare(&mut store, cp, p_dbl.clone()); }
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
    }
}
