//! The Explorer's two-pane tree (RFC-054, RFC-061, RFC-068; handoff 071).
//!
//! One component for both layouts. It renders a sequence of row pairs, one
//! `(left, right)` per visual row, and does not know how they were paired:
//! aligned pairs same-name entries with spacers (`compute_aligned_rows`), compact
//! packs each side and pairs by index (`pair_by_index`). Keyboard events go to the
//! focused pane in both layouts.

use std::collections::HashMap;
use std::path::PathBuf;

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTree, DirectoryTreeEvent, ScanRequest, SelectionMode};
use forskscope_core::IgnoreRules;
use forskscope_core::dir::EqualityEvidence;
use forskscope_ui_logic::AlignedRow;

use super::{DigestKey, FocusedPane, PickKind, Tier1Map, row_evidence, start_tier2_verify};
use crate::i18n::t;
use crate::state::{Lang, Store, open_compare};
use crate::ui::view::digest_epoch::DigestEpoch;
use crate::ui::view::dir_pane::{
    NavHistory, TreeRow, home_dir, navigate_to, navigate_to_from_history,
};

#[allow(clippy::too_many_arguments)]
#[component]
pub fn ExplorerTree(
    lang: Lang,
    rows: Vec<AlignedRow>,
    mut tree_l: Signal<DirectoryTree>,
    mut tree_r: Signal<DirectoryTree>,
    scans_l: Coroutine<ScanRequest>,
    scans_r: Coroutine<ScanRequest>,
    left_dir: Signal<PathBuf>,
    right_dir: Signal<PathBuf>,
    left_hist: Signal<NavHistory>,
    right_hist: Signal<NavHistory>,
    mut left_pick: Signal<Option<PickKind>>,
    mut right_pick: Signal<Option<PickKind>>,
    mut focused_pane: Signal<FocusedPane>,
    mut digest_map: Signal<HashMap<DigestKey, EqualityEvidence>>,
    tier1_map: Signal<Tier1Map>,
    mut binary_cache: Signal<HashMap<PathBuf, bool>>,
    binary_enabled: bool,
    /// RFC-080 tier 2 (handoff 060): the "Verify" control's dependencies —
    /// shared with tier 1 rather than given their own (handoff 060 §1).
    tier1_epoch: Signal<DigestEpoch>,
    tier1_announcement: Signal<String>,
    rules: IgnoreRules,
) -> Element {
    let mut store = use_context::<Store>();
    let l_root = left_dir.read().cloned();
    let r_root = right_dir.read().cloned();

    rsx! {
        div {
            id: "explorer-tree",
            class: "explorer-tree",
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
                if dispatch_go_up(
                    &e, focused_pane, store, left_hist, left_dir, right_hist, right_dir,
                ) {
                    return;
                }
                if dispatch_pathbar_shortcut(
                    &e, focused_pane, store, left_hist, left_dir, right_hist, right_dir,
                ) {
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

            if rows.is_empty() {
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

            for (left_row, right_row) in rows.iter() {
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
                                        let status = row_evidence(
                                            digest_map.read()
                                                .get(&DigestKey::Common(row.rel_path.clone()))
                                                .cloned(),
                                            &tier1_map.read(), &l_root, &r_root, &row.rel_path,
                                        );
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
                                        let verify_rel = row.rel_path.clone();
                                        let verify_l_root = l_root.clone();
                                        let verify_r_root = r_root.clone();
                                        let verify_rules = rules.clone();
                                        rsx! {
                                            TreeRow {
                                                lang,
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
                                                        let other_root = right_dir.read().cloned();
                                                        let other_pick = store.right_pick.read().cloned();
                                                        if let Some((l, r)) = dblclick_counterpart(
                                                            true, &p_dbl, other_pick.as_ref(), &l_root_c, &other_root,
                                                        ) {
                                                            open_compare(&mut store, l, r);
                                                        }
                                                    }
                                                },
                                                on_verify: move |_| {
                                                    start_tier2_verify(
                                                        verify_rel.clone(), is_dir,
                                                        verify_l_root.clone(), verify_r_root.clone(),
                                                        verify_rules.clone(), lang,
                                                        tier1_map, digest_map, tier1_epoch, tier1_announcement,
                                                    );
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
                                        let status = row_evidence(
                                            common.or(right_only),
                                            &tier1_map.read(), &l_root, &r_root, &row.rel_path,
                                        );
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
                                        let verify_rel = row.rel_path.clone();
                                        let verify_l_root = l_root.clone();
                                        let verify_r_root = r_root.clone();
                                        let verify_rules = rules.clone();
                                        rsx! {
                                            TreeRow {
                                                lang,
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
                                                        let other_root = left_dir.read().cloned();
                                                        let other_pick = store.left_pick.read().cloned();
                                                        if let Some((l, r)) = dblclick_counterpart(
                                                            false, &p_dbl, other_pick.as_ref(), &r_root_c, &other_root,
                                                        ) {
                                                            open_compare(&mut store, l, r);
                                                        }
                                                    }
                                                },
                                                on_verify: move |_| {
                                                    start_tier2_verify(
                                                        verify_rel.clone(), is_dir,
                                                        verify_l_root.clone(), verify_r_root.clone(),
                                                        verify_rules.clone(), lang,
                                                        tier1_map, digest_map, tier1_epoch, tier1_announcement,
                                                    );
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

/// What a double-click on a file compares it with, in both layouts (handoff 071 §1):
/// the file picked on the other side, if there is one; otherwise the same-named file
/// on the other side, if that exists. The result is always `(left, right)`. `None`
/// means there is nothing to compare, and the double-click does nothing.
fn dblclick_counterpart(
    own_is_left: bool,
    own_path: &std::path::Path,
    other_pick: Option<&PathBuf>,
    own_root: &std::path::Path,
    other_root: &std::path::Path,
) -> Option<(PathBuf, PathBuf)> {
    let other = match other_pick.filter(|p| p.is_file()) {
        Some(picked) => picked.clone(),
        None => {
            let rel = own_path.strip_prefix(own_root).ok()?;
            let same_named = other_root.join(rel);
            if !same_named.is_file() {
                return None;
            }
            same_named
        }
    };
    Some(if own_is_left {
        (own_path.to_path_buf(), other)
    } else {
        (other, own_path.to_path_buf())
    })
}

#[cfg(test)]
mod dblclick_tests {
    use super::*;

    /// A fresh pair of roots, each with one file of the same relative name. Per-process
    /// names, so parallel test runs do not collide.
    fn roots(tag: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("fsk-dblclick-{tag}-{}", std::process::id()));
        let (l, r) = (base.join("l"), base.join("r"));
        std::fs::create_dir_all(&l).unwrap();
        std::fs::create_dir_all(&r).unwrap();
        std::fs::write(l.join("same.txt"), "a").unwrap();
        std::fs::write(r.join("same.txt"), "b").unwrap();
        (l, r)
    }

    /// Handoff 071 §7(4): the fallback is shared by both layouts, and is tested here
    /// through the one helper both layouts call. Falsify by deleting the fallback arm:
    /// the second test fails.
    #[test]
    fn with_nothing_picked_the_same_named_file_on_the_other_side_is_compared() {
        let (l, r) = roots("fallback");
        let got = dblclick_counterpart(true, &l.join("same.txt"), None, &l, &r);
        assert_eq!(got, Some((l.join("same.txt"), r.join("same.txt"))));
    }

    #[test]
    fn with_nothing_picked_and_no_same_named_file_nothing_happens() {
        let (l, r) = roots("missing");
        std::fs::write(l.join("only_left.txt"), "a").unwrap();
        let got = dblclick_counterpart(true, &l.join("only_left.txt"), None, &l, &r);
        assert_eq!(got, None);
    }

    /// The right pane's double-click pairs the same way, with the order kept `(left, right)`.
    #[test]
    fn a_double_click_on_the_right_pane_keeps_left_then_right() {
        let (l, r) = roots("right");
        let got = dblclick_counterpart(false, &r.join("same.txt"), None, &r, &l);
        assert_eq!(got, Some((l.join("same.txt"), r.join("same.txt"))));
    }

    /// A file picked on the other side wins over the same-named file.
    #[test]
    fn a_picked_file_on_the_other_side_wins_over_the_same_named_one() {
        let (l, r) = roots("picked");
        std::fs::write(r.join("picked.txt"), "c").unwrap();
        let pick = r.join("picked.txt");
        let got = dblclick_counterpart(true, &l.join("same.txt"), Some(&pick), &l, &r);
        assert_eq!(got, Some((l.join("same.txt"), r.join("picked.txt"))));
    }
}

/// Alt+↑: go up one directory in the focused pane. Returns `true` when `e` was
/// this binding.
///
/// The parent is read in a `let` of its own, so the signal's read guard is
/// dropped before `navigate_to` writes that signal. Reading it in an `if let`
/// scrutinee holds the guard for the whole body, and the write then panics
/// with `AlreadyBorrowed`. The app aborts on that panic, because it is raised
/// inside a webview callback (F169). The unit test below calls this function
/// directly and does reproduce the panic on the old shape, so it guards the
/// borrow as well as the navigation.
fn dispatch_go_up(
    e: &Event<KeyboardData>,
    focused_pane: Signal<FocusedPane>,
    store: Store,
    left_hist: Signal<NavHistory>,
    left_dir: Signal<PathBuf>,
    right_hist: Signal<NavHistory>,
    right_dir: Signal<PathBuf>,
) -> bool {
    if !(e.modifiers().contains(Modifiers::ALT) && e.key() == Key::ArrowUp) {
        return false;
    }
    e.prevent_default();
    let is_left = focused_pane.read().is_left();
    let (hist, dir) = if is_left {
        (left_hist, left_dir)
    } else {
        (right_hist, right_dir)
    };
    let parent = dir.read().parent().map(|p| p.to_path_buf());
    if let Some(p) = parent {
        navigate_to(p, is_left, store, hist, dir);
    }
    true
}

/// F100: Home directory / Open folder, on `AlignedTree`'s focused pane —
/// extracted from `onkeydown`'s body for direct testing, the same shape
/// `path_input_keydown` (`dir_pane.rs`) established. Returns `true` when
/// `e` was one of these two bindings (already consumed via
/// `e.prevent_default()`), so the caller's `TreeKey` dispatch is not
/// reached for them. Bare `Home` (jump to the first row) is `TreeKey::Home`,
/// handled by that later dispatch, and must not be shadowed here.
///
/// The Open-folder binding's native picker cannot run headlessly — the
/// same limit `export_patch`'s save dialog has (RFC-084) — so this only
/// captures which pane was focused and dispatches the async task; what a
/// completed pick *does* is [`apply_picked_folder`], tested directly.
fn dispatch_pathbar_shortcut(
    e: &Event<KeyboardData>,
    focused_pane: Signal<FocusedPane>,
    store: Store,
    left_hist: Signal<NavHistory>,
    left_dir: Signal<PathBuf>,
    right_hist: Signal<NavHistory>,
    right_dir: Signal<PathBuf>,
) -> bool {
    if e.modifiers().contains(Modifiers::ALT) && e.key() == Key::Home {
        e.prevent_default();
        let is_left = focused_pane.read().is_left();
        apply_picked_folder(
            home_dir(),
            is_left,
            store,
            left_hist,
            left_dir,
            right_hist,
            right_dir,
        );
        return true;
    }
    // F161: Back and Forward. `navigate_to_from_history`, not `navigate_to`:
    // `navigate_to` pushes onto the history, which after `back()` would
    // truncate the forward entry (F72). `prevent_default` keeps the webview's
    // own Alt+← (Back) from also firing.
    if e.modifiers().contains(Modifiers::ALT) && matches!(e.key(), Key::ArrowLeft | Key::ArrowRight)
    {
        e.prevent_default();
        let is_left = focused_pane.read().is_left();
        let (mut hist, dir) = if is_left {
            (left_hist, left_dir)
        } else {
            (right_hist, right_dir)
        };
        let step = if e.key() == Key::ArrowLeft {
            hist.write().back()
        } else {
            hist.write().forward()
        };
        if let Some(p) = step {
            navigate_to_from_history(p, is_left, store, dir);
        }
        return true;
    }
    if e.modifiers().contains(Modifiers::CONTROL)
        && matches!(&e.key(), Key::Character(s) if s.eq_ignore_ascii_case("o"))
    {
        e.prevent_default();
        let is_left = focused_pane.read().is_left();
        spawn(async move {
            let picked = tokio::task::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
                .await
                .ok()
                .flatten();
            if let Some(p) = picked {
                apply_picked_folder(
                    p, is_left, store, left_hist, left_dir, right_hist, right_dir,
                );
            }
        });
        return true;
    }
    false
}

/// Navigates whichever pane `is_left` selects to `picked` — shared by the
/// Home binding (synchronous) and the Open-folder binding's completed
/// async pick.
fn apply_picked_folder(
    picked: PathBuf,
    is_left: bool,
    store: Store,
    left_hist: Signal<NavHistory>,
    left_dir: Signal<PathBuf>,
    right_hist: Signal<NavHistory>,
    right_dir: Signal<PathBuf>,
) {
    if is_left {
        navigate_to(picked, true, store, left_hist, left_dir);
    } else {
        navigate_to(picked, false, store, right_hist, right_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::with_test_store;

    fn key_event(key: Key, modifiers: Modifiers) -> Event<KeyboardData> {
        crate::keyboard::test_support::key_event(key, modifiers)
    }

    // F100 falsification 1: removing the Home binding (or hardcoding it to
    // a no-op) fails this — it asserts the *action* (the focused pane's
    // directory becomes the home directory), not merely that the key was
    // consumed.
    #[test]
    fn alt_home_navigates_the_focused_pane_to_the_home_directory() {
        with_test_store(|store| {
            let focused_pane = Signal::new_in_scope(FocusedPane::Left, ScopeId::ROOT);
            let left_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let left_dir = Signal::new_in_scope(PathBuf::from("/somewhere/deep"), ScopeId::ROOT);
            let right_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let right_dir = Signal::new_in_scope(PathBuf::from("/other"), ScopeId::ROOT);

            let e = key_event(Key::Home, Modifiers::ALT);
            let handled = dispatch_pathbar_shortcut(
                &e,
                focused_pane,
                *store,
                left_hist,
                left_dir,
                right_hist,
                right_dir,
            );

            assert!(
                handled,
                "Alt+Home must be recognized as this view's binding"
            );
            assert_eq!(*left_dir.read(), home_dir());
            assert_eq!(
                *right_dir.read(),
                PathBuf::from("/other"),
                "the unfocused pane must not move"
            );
        });
    }

    // F100 falsification 2 — the one that matters: with the *right* pane
    // focused, Alt+Home must navigate the right path bar, not the left
    // one. Falsify by hardcoding `is_left = true` in
    // `dispatch_pathbar_shortcut` and this fails, the same regression
    // `Alt+↑` already guards against.
    #[test]
    fn alt_home_targets_the_right_pane_when_the_right_pane_is_focused() {
        with_test_store(|store| {
            let focused_pane = Signal::new_in_scope(FocusedPane::Right, ScopeId::ROOT);
            let left_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let left_dir = Signal::new_in_scope(PathBuf::from("/left/unmoved"), ScopeId::ROOT);
            let right_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let right_dir = Signal::new_in_scope(PathBuf::from("/somewhere/deep"), ScopeId::ROOT);

            let e = key_event(Key::Home, Modifiers::ALT);
            dispatch_pathbar_shortcut(
                &e,
                focused_pane,
                *store,
                left_hist,
                left_dir,
                right_hist,
                right_dir,
            );

            assert_eq!(*right_dir.read(), home_dir());
            assert_eq!(
                *left_dir.read(),
                PathBuf::from("/left/unmoved"),
                "the unfocused left pane must not move"
            );
        });
    }

    /// F161: Alt+← goes Back and Alt+→ goes Forward in the focused pane, through
    /// the same history the ◀ ▶ buttons use, and the second step returns to the
    /// page the first one left. Falsify by calling `navigate_to` in the new
    /// branch: `navigate_to` pushes, which truncates the forward entry, so the
    /// Forward step below finds nothing and the assertion fails (F72's bug).
    #[test]
    fn alt_arrows_go_back_then_forward_in_the_focused_pane() {
        with_test_store(|store| {
            let focused_pane = Signal::new_in_scope(FocusedPane::Left, ScopeId::ROOT);
            let mut left_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let left_dir = Signal::new_in_scope(PathBuf::from("/b"), ScopeId::ROOT);
            let right_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let right_dir = Signal::new_in_scope(PathBuf::from("/right/unmoved"), ScopeId::ROOT);
            left_hist.write().push(PathBuf::from("/a"));
            left_hist.write().push(PathBuf::from("/b"));

            let back = key_event(Key::ArrowLeft, Modifiers::ALT);
            let handled = dispatch_pathbar_shortcut(
                &back,
                focused_pane,
                *store,
                left_hist,
                left_dir,
                right_hist,
                right_dir,
            );
            assert!(handled, "Alt+← must be recognized as this view's binding");
            assert_eq!(*left_dir.read(), PathBuf::from("/a"), "Alt+← is Back");

            let forward = key_event(Key::ArrowRight, Modifiers::ALT);
            dispatch_pathbar_shortcut(
                &forward,
                focused_pane,
                *store,
                left_hist,
                left_dir,
                right_hist,
                right_dir,
            );
            assert_eq!(
                *left_dir.read(),
                PathBuf::from("/b"),
                "Alt+→ after Back must return to the page Back left"
            );
            assert_eq!(
                *right_dir.read(),
                PathBuf::from("/right/unmoved"),
                "the unfocused pane must not move"
            );
        });
    }

    /// F161: with the right pane focused, Alt+← moves the right pane's history,
    /// not the left one. Falsify by hardcoding `is_left = true` in the new branch.
    #[test]
    fn alt_left_targets_the_right_pane_when_the_right_pane_is_focused() {
        with_test_store(|store| {
            let focused_pane = Signal::new_in_scope(FocusedPane::Right, ScopeId::ROOT);
            let left_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let left_dir = Signal::new_in_scope(PathBuf::from("/left/unmoved"), ScopeId::ROOT);
            let mut right_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let right_dir = Signal::new_in_scope(PathBuf::from("/y"), ScopeId::ROOT);
            right_hist.write().push(PathBuf::from("/x"));
            right_hist.write().push(PathBuf::from("/y"));

            let back = key_event(Key::ArrowLeft, Modifiers::ALT);
            dispatch_pathbar_shortcut(
                &back,
                focused_pane,
                *store,
                left_hist,
                left_dir,
                right_hist,
                right_dir,
            );

            assert_eq!(*right_dir.read(), PathBuf::from("/x"));
            assert_eq!(
                *left_dir.read(),
                PathBuf::from("/left/unmoved"),
                "the unfocused left pane must not move"
            );
        });
    }

    /// F169: Alt+↑ reaches the navigation for the focused pane. It also guards
    /// the `AlreadyBorrowed` abort, because it calls `dispatch_go_up` directly:
    /// on the old `if let` shape this test panics with that error (reproduced).
    /// It does not cover the real webview dispatch, which is checked in the app.
    #[test]
    fn alt_up_goes_to_the_parent_of_the_focused_pane() {
        with_test_store(|store| {
            let focused_pane = Signal::new_in_scope(FocusedPane::Left, ScopeId::ROOT);
            let left_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let left_dir = Signal::new_in_scope(PathBuf::from("/a/b"), ScopeId::ROOT);
            let right_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let right_dir = Signal::new_in_scope(PathBuf::from("/r/s"), ScopeId::ROOT);

            let e = key_event(Key::ArrowUp, Modifiers::ALT);
            let handled = dispatch_go_up(
                &e,
                focused_pane,
                *store,
                left_hist,
                left_dir,
                right_hist,
                right_dir,
            );

            assert!(handled, "Alt+↑ must be recognized as this view's binding");
            assert_eq!(*left_dir.read(), PathBuf::from("/a"));
        });
    }

    /// F169: the right-pane branch has the same shape and the same abort, so it
    /// is tested on its own. Falsify by calling `navigate_to` on the left pane
    /// in that branch: the right pane stays put and this fails.
    #[test]
    fn alt_up_goes_to_the_parent_of_the_right_pane_when_it_is_focused() {
        with_test_store(|store| {
            let focused_pane = Signal::new_in_scope(FocusedPane::Right, ScopeId::ROOT);
            let left_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let left_dir = Signal::new_in_scope(PathBuf::from("/l/m"), ScopeId::ROOT);
            let right_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let right_dir = Signal::new_in_scope(PathBuf::from("/r/s"), ScopeId::ROOT);

            let e = key_event(Key::ArrowUp, Modifiers::ALT);
            dispatch_go_up(
                &e,
                focused_pane,
                *store,
                left_hist,
                left_dir,
                right_hist,
                right_dir,
            );

            assert_eq!(*right_dir.read(), PathBuf::from("/r"));
            assert_eq!(
                *left_dir.read(),
                PathBuf::from("/l/m"),
                "the unfocused pane must not move"
            );
        });
    }

    #[test]
    fn ctrl_o_is_recognized_as_this_views_binding() {
        with_test_store(|store| {
            let focused_pane = Signal::new_in_scope(FocusedPane::Left, ScopeId::ROOT);
            let left_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let left_dir = Signal::new_in_scope(PathBuf::from("/a"), ScopeId::ROOT);
            let right_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let right_dir = Signal::new_in_scope(PathBuf::from("/b"), ScopeId::ROOT);

            let e = key_event(Key::Character("o".into()), Modifiers::CONTROL);
            // `spawn()` inside `dispatch_pathbar_shortcut`'s Ctrl+O branch
            // needs an active scope on the stack, which `with_test_store`'s
            // `in_runtime` alone does not push — same reason
            // `path_input_keydown`'s tests use `Runtime::in_scope` for
            // `EventHandler::new`.
            let handled = dioxus_core::Runtime::current().in_scope(ScopeId::ROOT, || {
                dispatch_pathbar_shortcut(
                    &e,
                    focused_pane,
                    *store,
                    left_hist,
                    left_dir,
                    right_hist,
                    right_dir,
                )
            });

            assert!(handled, "Ctrl+O must be recognized as this view's binding");
        });
    }

    // F100 falsification 1/2 for Open-folder, on the half that can run
    // without a real display: `apply_picked_folder` is what a completed
    // pick calls — the native `rfd` dialog itself cannot run headlessly
    // (same limit `export_patch`'s save dialog has, RFC-084), so this
    // assumes a folder was already picked and asserts the action targets
    // the focused pane, exactly as `alt_home_targets_the_right_pane_*`
    // does for Home.
    #[test]
    fn apply_picked_folder_targets_the_focused_pane() {
        with_test_store(|store| {
            let left_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let left_dir = Signal::new_in_scope(PathBuf::from("/left/unmoved"), ScopeId::ROOT);
            let right_hist = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let right_dir = Signal::new_in_scope(PathBuf::from("/right/unmoved"), ScopeId::ROOT);

            apply_picked_folder(
                PathBuf::from("/picked"),
                false,
                *store,
                left_hist,
                left_dir,
                right_hist,
                right_dir,
            );

            assert_eq!(*right_dir.read(), PathBuf::from("/picked"));
            assert_eq!(*left_dir.read(), PathBuf::from("/left/unmoved"));
        });
    }
}
