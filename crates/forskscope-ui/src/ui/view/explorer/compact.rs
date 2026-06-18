//! Compact (unaligned) Explorer view (RFC-068): each pane scrolls independently
//! with no spacer rows. Cross-pane row alignment is intentionally absent.

use std::collections::HashMap;
use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTree, SelectionMode, ScanRequest};

use crate::state::{Lang, Store, open_compare};
use crate::ui::view::dir_pane::{DigestState, NavHistory, TreeRow, navigate_to};
use super::{DigestKey, PickKind};

type FlatRow = (PathBuf, bool, bool, bool, u32);

#[allow(clippy::too_many_arguments)]
#[component]
pub fn CompactTree(
    lang:           Lang,
    mut store:      Store,
    left_flat:      Vec<FlatRow>,
    right_flat:     Vec<FlatRow>,
    l_root:         PathBuf,
    r_root:         PathBuf,
    mut tree_l:     Signal<DirectoryTree>,
    mut tree_r:     Signal<DirectoryTree>,
    scans_l:        Coroutine<ScanRequest>,
    scans_r:        Coroutine<ScanRequest>,
    left_dir:       Signal<PathBuf>,
    right_dir:      Signal<PathBuf>,
    left_hist:      Signal<NavHistory>,
    right_hist:     Signal<NavHistory>,
    mut left_pick:  Signal<Option<PickKind>>,
    mut right_pick: Signal<Option<PickKind>>,
    mut digest_map: Signal<HashMap<DigestKey, DigestState>>,
    mut binary_cache: Signal<HashMap<PathBuf, bool>>,
    binary_enabled: bool,
    filter_query:   Signal<String>,
) -> Element {
    rsx! {
        div { class: "compact-tree",
            // ── Left pane ────────────────────────────────────────────────────
            div { class: "compact-pane",
                for row in left_flat.iter().filter(|(p, _is_dir, ..)| {
                    if filter_query.read().is_empty() { true } else {
                        p.file_name().map(|n| n.to_string_lossy().to_lowercase()
                            .contains(&*filter_query.read())).unwrap_or(false)
                    }
                }) {
                    {
                        let (abs, is_dir, is_expanded, is_selected, depth) = row.clone();
                        let rel = abs.strip_prefix(&l_root).unwrap_or(&abs).to_path_buf();
                        let status = digest_map.read().get(&DigestKey::Common(rel)).cloned();
                        let p_tgl = abs.clone(); let p_sel = abs.clone();
                        let p_dbl = abs.clone(); let p_nav = abs.clone();
                        let p_bin = abs.clone();
                        let is_binary = if is_dir { false } else {
                            let cached = binary_cache.read().get(&abs).copied();
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
                                path: abs.clone(), is_dir, is_expanded, is_selected,
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
            // ── Right pane ───────────────────────────────────────────────────
            div { class: "compact-pane compact-pane-right",
                for row in right_flat.iter().filter(|(p, _is_dir, ..)| {
                    if filter_query.read().is_empty() { true } else {
                        p.file_name().map(|n| n.to_string_lossy().to_lowercase()
                            .contains(&*filter_query.read())).unwrap_or(false)
                    }
                }) {
                    {
                        let (abs, is_dir, is_expanded, is_selected, depth) = row.clone();
                        let rel = abs.strip_prefix(&r_root).unwrap_or(&abs).to_path_buf();
                        let common    = digest_map.read().get(&DigestKey::Common(rel.clone())).cloned();
                        let right_only = digest_map.read().get(&DigestKey::RightOnly(rel)).cloned();
                        let status = common.or(right_only);
                        let p_tgl = abs.clone(); let p_sel = abs.clone();
                        let p_dbl = abs.clone(); let p_nav = abs.clone();
                        let p_bin = abs.clone();
                        let is_binary = if is_dir { false } else {
                            let cached = binary_cache.read().get(&abs).copied();
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
                                path: abs.clone(), is_dir, is_expanded, is_selected,
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
}
