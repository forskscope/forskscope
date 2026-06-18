//! Diff workspace toolbar: primary navigation/save controls and the advanced
//! disclosure panel with diff options.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Lang, Modal, Store, recompute_diff, reload_tab, swap_sides};
use crate::ui::view::diff_actions::{algo_val, export_patch, save_tab};
use crate::ui::view::search::SearchCtx;
use super::TabSnapshot;

#[component]
pub fn Toolbar(index: usize, snap: TabSnapshot, lang: Lang) -> Element {
    let mut store    = use_context::<Store>();
    let mut advanced = use_signal(|| false);
    let mut search_ctx: Signal<SearchCtx> = use_context::<Signal<SearchCtx>>();

    let pos = if snap.changes == 0 { String::new() }
              else { format!("{} / {}", snap.focused_change + 1, snap.changes) };

    rsx! {
        // ── Primary toolbar ───────────────────────────────────────────────────
        div { class: "diff-toolbar",
            button {
                onclick: move |_| crate::ui::view::diff_actions::move_focus(&mut store, index, -1),
                title: t(lang, "F7 — Previous change"), "◀"
            }
            button {
                onclick: move |_| crate::ui::view::diff_actions::move_focus(&mut store, index,  1),
                title: t(lang, "F8 — Next change"), "▶"
            }
            span { class: "info", "{pos}" }
            span { class: "spacer" }
            if snap.can_save {
                button {
                    disabled: !snap.can_undo,
                    onclick: move |_| {
                        if let Some(tab) = store.tabs.write().get_mut(index) { let _ = tab.merge.undo(); }
                    },
                    aria_label: t(lang, "Undo last merge action (Ctrl+Z)"),
                    {t(lang, "Undo")}
                }
                button {
                    disabled: !snap.is_dirty,
                    onclick: move |_| save_tab(&mut store, index, false),
                    aria_label: t(lang, "Save merge result (Ctrl+S)"),
                    {t(lang, "Save")}
                }
                button {
                    onclick: move |_| {
                        let path = store.tabs.read().get(index)
                            .and_then(|t| t.right_path.as_ref())
                            .map(|p| p.display().to_string()).unwrap_or_default();
                        store.modal.set(Modal::SaveAs(index, path));
                    },
                    {t(lang, "Save As")}
                }
            }
            button {
                title: t(lang, "Reload both files from disk"),
                aria_label: t(lang, "Reload files from disk"),
                onclick: move |_| {
                    let dirty = store.tabs.read().get(index).map(|t| t.merge.is_dirty()).unwrap_or(false);
                    if dirty { store.modal.set(Modal::ConfirmReload(index)); }
                    else { reload_tab(&mut store, index); store.notify_success(t(store.lang(), "Reloaded.")); }
                },
                "↺"
            }
            button {
                id: "search-open-btn",
                title: t(lang, "Search within diff (Ctrl+F)"),
                aria_label: t(lang, "Open search bar"),
                onclick: move |_| { search_ctx.write().active ^= true; },
                "🔍"
            }
            if snap.can_save {
                button {
                    onclick: move |_| { let v = *advanced.read(); advanced.set(!v); },
                    if *advanced.read() { {t(lang, "Less ▲")} } else { {t(lang, "More ▼")} }
                }
            }
        }

        // ── Advanced disclosure panel ─────────────────────────────────────────
        if *advanced.read() && snap.can_save {
            div { class: "diff-toolbar advanced",
                button {
                    aria_pressed: if snap.char_mode { "true" } else { "false" },
                    aria_label: t(lang, "Toggle character-level inline diff"),
                    onclick: move |_| {
                        if let Some(tab) = store.tabs.write().get_mut(index) { tab.char_mode ^= true; }
                    },
                    {format!("{}: {}", t(lang, "Inline diff"), t(lang, if snap.char_mode { "on" } else { "off" }))}
                }
                button {
                    aria_pressed: if snap.word_wrap { "true" } else { "false" },
                    aria_label: t(lang, "Toggle word wrap"),
                    onclick: move |_| {
                        if let Some(tab) = store.tabs.write().get_mut(index) { tab.word_wrap ^= true; }
                    },
                    {format!("{}: {}", t(lang, "Wrap"), t(lang, if snap.word_wrap { "on" } else { "off" }))}
                }
                button {
                    disabled: !snap.can_redo,
                    onclick: move |_| {
                        if let Some(tab) = store.tabs.write().get_mut(index) { let _ = tab.merge.redo(); }
                    },
                    {t(lang, "Redo")}
                }
                button {
                    onclick: move |_| {
                        let dirty = store.tabs.read().get(index).map(|t| t.merge.is_dirty()).unwrap_or(false);
                        if dirty { store.modal.set(Modal::ConfirmSwap(index)); }
                        else { swap_sides(&mut store, index); }
                    },
                    {format!("⇄ {}", t(lang, "Swap sides"))}
                }
                button {
                    aria_pressed: if snap.ignore_whitespace { "true" } else { "false" },
                    aria_label: t(lang, "Toggle ignore whitespace"),
                    onclick: move |_| {
                        let mut tabs = store.tabs.write();
                        if let Some(tab) = tabs.get_mut(index) {
                            tab.diff_options.ignore_whitespace ^= true;
                            recompute_diff(tab);
                        }
                    },
                    {format!("{}: {}", t(lang, "Ignore WS"), t(lang, if snap.ignore_whitespace { "on" } else { "off" }))}
                }
                button {
                    aria_pressed: if snap.ignore_case { "true" } else { "false" },
                    aria_label: t(lang, "Toggle ignore case"),
                    onclick: move |_| {
                        let mut tabs = store.tabs.write();
                        if let Some(tab) = tabs.get_mut(index) {
                            tab.diff_options.ignore_case ^= true;
                            recompute_diff(tab);
                        }
                    },
                    {format!("{}: {}", t(lang, "Ignore case"), t(lang, if snap.ignore_case { "on" } else { "off" }))}
                }
                select {
                    title: t(lang, "Diff algorithm"),
                    value: algo_val(snap.algorithm),
                    onchange: move |e| {
                        let mut tabs = store.tabs.write();
                        if let Some(tab) = tabs.get_mut(index) {
                            use forskscope_core::DiffAlgorithm;
                            tab.diff_options.algorithm = match e.value().as_str() {
                                "patience"  => DiffAlgorithm::Patience,
                                "histogram" => DiffAlgorithm::Histogram,
                                _           => DiffAlgorithm::Myers,
                            };
                            recompute_diff(tab);
                        }
                    },
                    option { value: "myers",     "Myers"     }
                    option { value: "patience",  "Patience"  }
                    option { value: "histogram", "Histogram" }
                }
                button {
                    title: t(lang, "Export unified-diff patch file"),
                    aria_label: t(lang, "Export patch"),
                    onclick: move |_| { export_patch(&store, index); },
                    {t(lang, "Export patch")}
                }
            }
        }
    }
}
