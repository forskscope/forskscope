//! Diff workspace toolbar: primary navigation/save controls and the advanced
//! disclosure panel with diff options.

use dioxus::prelude::*;

use super::TabSnapshot;
use crate::i18n::t;
use crate::state::{
    Lang, Modal, Store, change_diff_options, change_encoding, reload_tab, swap_sides,
};
use crate::ui::component::select_field::{SelectField, SelectOption};
use crate::ui::view::diff_actions::{algo_val, export_patch, save_tab};
use crate::ui::view::search::SearchCtx;

#[component]
pub fn Toolbar(index: usize, snap: TabSnapshot, lang: Lang) -> Element {
    let mut store = use_context::<Store>();
    let mut advanced = use_signal(|| false);
    let mut search_ctx: Signal<SearchCtx> = use_context::<Signal<SearchCtx>>();

    let pos = if snap.changes == 0 {
        String::new()
    } else {
        format!("{} / {}", snap.focused_change + 1, snap.changes)
    };

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
                    onclick: move |_| save_tab(&mut store, index),
                    aria_label: t(lang, "Save merge result (Ctrl+S)"),
                    {t(lang, "Save")}
                }
                button {
                    onclick: move |_| {
                        // Default to the tab's actual save target
                        // (RFC-077) — for a mergetool tab that's the merged
                        // output, not right_path/the compared remote input.
                        // Falls back to right_path only if no save target
                        // was ever resolved (shouldn't happen once Ready).
                        let path = store.tabs.read().get(index)
                            .and_then(|t| t.save_target.as_ref().map(|st| st.path.clone())
                                .or_else(|| t.right_path.clone()))
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
            {
                // RFC-092 §1: unlike "Export patch" (in the `can_save`-gated
                // advanced row below), this control must stay reachable even
                // for a read-only spreadsheet/binary comparison — the RFC
                // asks for it *visible but disabled*, with a tooltip saying
                // why, not hidden the way the whole advanced row already is
                // for a non-saveable tab. Lives in the primary row for
                // that reason, not literally beside "Export patch".
                let (disabled, tooltip) = if !snap.right_is_text {
                    (true, t(lang, "Reports cover text comparisons for now."))
                } else if snap.is_dirty {
                    (true, t(lang, "Save or undo your changes first: a report describes the saved files."))
                } else {
                    (false, t(lang, "Export a comparison report"))
                };
                rsx! {
                    button {
                        disabled,
                        title: "{tooltip}",
                        aria_label: t(lang, "Export report"),
                        onclick: move |_| store.modal.set(Modal::ExportReport(index)),
                        {t(lang, "Export report…")}
                    }
                }
            }
        }

        // ── Advanced disclosure panel ─────────────────────────────────────────
        if *advanced.read() && snap.can_save {
            div { class: "diff-toolbar advanced",
                button {
                    aria_pressed: if snap.char_mode { "true" } else { "false" },
                    aria_label: t(lang, "Toggle character-level inline diff"),
                    disabled: !snap.inline_available,
                    title: if snap.inline_available { String::new() } else { t(lang, "Character-level highlighting is not available for this comparison.") },
                    onclick: move |_| {
                        if let Some(tab) = store.tabs.write().get_mut(index)
                            && crate::ui::view::diff::inline_available(&tab.diff_options)
                        {
                            tab.char_mode ^= true;
                        }
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
                        // F40: computed here and handed to change_diff_options,
                        // which guards against discarding applied merge work
                        // rather than mutating tab.diff_options directly.
                        let next = store.tabs.read().get(index).map(|t| {
                            let mut o = t.diff_options;
                            o.ignore_whitespace ^= true;
                            o
                        });
                        if let Some(next) = next {
                            change_diff_options(&mut store, index, next);
                        }
                    },
                    {format!("{}: {}", t(lang, "Ignore whitespace"), t(lang, if snap.ignore_whitespace { "on" } else { "off" }))}
                }
                button {
                    aria_pressed: if snap.ignore_case { "true" } else { "false" },
                    aria_label: t(lang, "Toggle ignore case"),
                    onclick: move |_| {
                        let next = store.tabs.read().get(index).map(|t| {
                            let mut o = t.diff_options;
                            o.ignore_case ^= true;
                            o
                        });
                        if let Some(next) = next {
                            change_diff_options(&mut store, index, next);
                        }
                    },
                    {format!("{}: {}", t(lang, "Ignore case"), t(lang, if snap.ignore_case { "on" } else { "off" }))}
                }
                SelectField {
                    title: t(lang, "Diff algorithm"),
                    value: algo_val(snap.algorithm).to_string(),
                    options: vec![
                        SelectOption::new("myers", "Myers"),
                        SelectOption::new("patience", "Patience"),
                        SelectOption::new("histogram", "Histogram"),
                    ],
                    onchange: move |v: String| {
                        use forskscope_core::DiffAlgorithm;
                        let algorithm = match v.as_str() {
                            "patience"  => DiffAlgorithm::Patience,
                            "histogram" => DiffAlgorithm::Histogram,
                            _           => DiffAlgorithm::Myers,
                        };
                        let next = store.tabs.read().get(index).map(|t| {
                            let mut o = t.diff_options;
                            o.algorithm = algorithm;
                            o
                        });
                        if let Some(next) = next {
                            change_diff_options(&mut store, index, next);
                        }
                    },
                }
                button {
                    title: t(lang, "Export unified-diff patch file"),
                    aria_label: t(lang, "Export patch"),
                    onclick: move |_| { export_patch(&mut store, index); },
                    {t(lang, "Export patch")}
                }
                if snap.right_is_text {
                    {
                        // The current label may not be one of the curated
                        // options (e.g. an unusual chardetng guess) — an
                        // extra option, passed through SelectField like any
                        // other, keeps the control showing the true current
                        // state rather than silently snapping to the list's
                        // first entry.
                        let mut options: Vec<SelectOption> = Vec::new();
                        if !forskscope_core::COMMON_ENCODING_LABELS.contains(&snap.right_encoding_label.as_str()) {
                            options.push(SelectOption::new(
                                snap.right_encoding_label.clone(),
                                snap.right_encoding_label.clone(),
                            ));
                        }
                        options.extend(
                            forskscope_core::COMMON_ENCODING_LABELS
                                .iter()
                                .map(|label| SelectOption::new(*label, *label)),
                        );
                        rsx! {
                            SelectField {
                                title: t(lang, "Encoding — re-decode if this looks wrong"),
                                aria_label: t(lang, "Right-side text encoding"),
                                value: snap.right_encoding_label.clone(),
                                options,
                                onchange: move |v: String| change_encoding(&mut store, index, v),
                            }
                        }
                    }
                }
            }
        }
    }
}
