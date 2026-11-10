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
            // RFC-097 stage 3 §1: a symbol alone wasn't enough — the words
            // stay beside the kept symbol, not in place of it.
            button {
                aria_label: t(lang, "F7 — Previous change"),
                onclick: move |_| crate::ui::view::diff_actions::move_focus(&mut store, index, -1),
                "◀ " {t(lang, "Previous change")}
            }
            button {
                aria_label: t(lang, "F8 — Next change"),
                onclick: move |_| crate::ui::view::diff_actions::move_focus(&mut store, index,  1),
                "▶ " {t(lang, "Next change")}
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
                // RFC-097 stage 3 §4: beside Undo now, not under More — same
                // enabled/disabled logic (`!snap.can_redo`), Ctrl+Y unchanged.
                button {
                    disabled: !snap.can_redo,
                    onclick: move |_| {
                        if let Some(tab) = store.tabs.write().get_mut(index) { let _ = tab.merge.redo(); }
                    },
                    aria_label: t(lang, "Redo last undone merge action (Ctrl+Y)"),
                    {t(lang, "Redo")}
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
                aria_label: t(lang, "Search within diff (Ctrl+F)"),
                onclick: move |_| { search_ctx.write().active ^= true; },
                "🔍 " {t(lang, "Search")}
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
                        use forskscope_compare::DiffAlgorithm;
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
                        if !forskscope_compare::COMMON_ENCODING_LABELS.contains(&snap.right_encoding_label.as_str()) {
                            options.push(SelectOption::new(
                                snap.right_encoding_label.clone(),
                                snap.right_encoding_label.clone(),
                            ));
                        }
                        options.extend(
                            forskscope_compare::COMMON_ENCODING_LABELS
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::state::AppSettings;

    fn snap(can_save: bool, can_undo: bool, can_redo: bool) -> TabSnapshot {
        TabSnapshot {
            hunks: Vec::new(),
            identical: false,
            char_mode: false,
            inline_available: false,
            word_wrap: false,
            can_save,
            is_dirty: false,
            can_undo,
            can_redo,
            font_size: 14,
            font_family: "monospace",
            focused_id: None,
            focused_change: 0,
            changes: 0,
            ignore_whitespace: false,
            ignore_case: false,
            context_lines: 3,
            algorithm: forskscope_compare::DiffAlgorithm::Myers,
            warnings: Vec::new(),
            readonly_notice: String::new(),
            right_is_text: true,
            right_encoding_label: "UTF-8".to_string(),
        }
    }

    fn root(snap: TabSnapshot) -> Element {
        use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
        use_context_provider(|| Signal::new_in_scope(SearchCtx::default(), ScopeId::ROOT));
        rsx! { Toolbar { index: 0, snap, lang: Lang::En } }
    }

    /// RFC-097 stage 3 §1: ◀/▶ keep their symbol and gain `aria_label`s
    /// naming the action in words ("Previous change"/"Next change" are the
    /// *visible* words; the existing F7/F8-prefixed keys stay as the
    /// accessible names, unchanged from before this handoff). Falsify by
    /// removing either `aria_label`: the corresponding assertion fails.
    #[test]
    fn previous_and_next_change_keep_their_f7_f8_accessible_names() {
        let mut vdom = VirtualDom::new_with_props(root, snap(true, true, true));
        let mutations = vdom.rebuild_to_vec();
        let labels: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute {
                    name: "aria-label",
                    value,
                    ..
                } => Some(format!("{value:?}")),
                _ => None,
            })
            .collect();
        assert!(
            labels.iter().any(|l| l.contains("F7 — Previous change")),
            "got {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("F8 — Next change")),
            "got {labels:?}"
        );
    }

    /// The Search button's `title`/duplicate `aria_label` ("Open search
    /// bar") are gone - it now reuses the same "Search within diff
    /// (Ctrl+F)" key that was already its `title`, as its one accessible
    /// name, and gains visible "Search" text. Falsify by restoring the old
    /// separate "Open search bar" aria_label: the second assertion then
    /// fails (both labels would be present).
    #[test]
    fn the_search_button_has_one_accessible_name_and_visible_text() {
        let mut vdom = VirtualDom::new_with_props(root, snap(true, true, true));
        let mutations = vdom.rebuild_to_vec();
        let labels: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute {
                    name: "aria-label",
                    value,
                    ..
                } => Some(format!("{value:?}")),
                _ => None,
            })
            .collect();
        assert!(
            labels.iter().any(|l| l.contains("Search within diff (Ctrl+F)")),
            "got {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("Open search bar")),
            "the old, now-duplicate aria_label must be gone, got {labels:?}"
        );

        let texts: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.contains("Search")),
            "the Search button must show the word, got {texts:?}"
        );
    }

    /// *Redo* beside *Undo*, both rendered whenever `can_save`: a plain
    /// mutation check of the initial render, same as the other two tests
    /// above.
    #[test]
    fn redo_renders_beside_undo_when_the_tab_can_save() {
        let mut vdom = VirtualDom::new_with_props(root, snap(true, true, true));
        let mutations = vdom.rebuild_to_vec();
        let texts: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.as_str() == "Redo"), "got {texts:?}");
        assert!(texts.iter().any(|t| t.as_str() == "Undo"), "got {texts:?}");
    }

    /// RFC-097 stage 3 §4: *Redo* moved out of the Advanced disclosure
    /// panel, beside *Undo* — its visible-text expression must now appear
    /// exactly once in the source, not twice. The Advanced panel only
    /// renders after a click `rebuild_to_vec()` never simulates (it starts
    /// collapsed, with no prop to open it), so a duplicate left behind
    /// there is invisible to a mutation-based test; a source scan is the
    /// only thing left to check, the same reasoning `select_field.rs`'s own
    /// source-scan test documents. Falsify by re-adding the old Redo button
    /// inside the advanced panel: the count then becomes 2.
    ///
    /// The needle is assembled from two literals, deliberately never
    /// spelling out the translation call as one contiguous token in this
    /// file's own source — `cargo xtask i18n` scans raw bytes for exactly
    /// that shape and would otherwise mistake this test's needle for a
    /// second, untranslatable usage site.
    #[test]
    fn redo_is_defined_exactly_once_in_the_toolbar_source() {
        let src = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/view/diff/toolbar.rs"),
        )
        .unwrap();
        // Only the `rsx!`/component source above this test module, not this
        // module's own test text - cut at the `#[cfg(test)]` marker.
        let component_src = src.split("#[cfg(test)]").next().unwrap();
        let needle = format!("{}{}", "{t", "(lang, \"Redo\")}");
        let count = component_src.matches(needle.as_str()).count();
        assert_eq!(
            count, 1,
            "exactly one Redo visible-text expression must exist in the \
             toolbar's component source, found {count}"
        );
    }
}
