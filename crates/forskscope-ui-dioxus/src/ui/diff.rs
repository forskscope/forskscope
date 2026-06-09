//! Diff/merge workspace: coordination, toolbar, search, and actions.
//! Hunk rendering lives in [`crate::ui::hunk`].

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Lang, Modal, Store, recompute_diff, reload_tab, swap_sides};
pub use crate::ui::diff_actions::{apply_focused_hunk, move_focus, save_as, save_tab};
use crate::ui::diff_actions::{algo_val, trunc};
use crate::ui::hunk::HunkBlock;
use crate::ui::search::{SearchBar, SearchCtx, line_matches};

#[component]
pub fn DiffWorkspace(index: usize) -> Element {
    let store = use_context::<Store>();
    let lang = store.lang();
    let font_size = store.settings.read().diff_font_size;
    let context_lines = store.settings.read().context_lines;

    let snap = {
        let tabs = store.tabs.read();
        match tabs.get(index) {
            Some(tab) => TabSnapshot::from_tab(tab, font_size, context_lines),
            None => return rsx! { div { class: "notice", "No comparison." } },
        }
    };

    // Search context — scoped to this workspace instance.
    let search_ctx: Signal<SearchCtx> = use_context_provider(|| Signal::new(SearchCtx::default()));
    let mut expanded: Signal<HashSet<u64>> = use_signal(HashSet::new);

    // Count matching hunks for the search bar label.
    let match_count: usize = if search_ctx.read().active && !search_ctx.read().query.is_empty() {
        snap.hunks.iter().map(|h| {
            h.rows.iter().filter(|r| {
                let ctx = search_ctx.read();
                r.left.as_ref().map(|l| line_matches(&ctx, &l.content)).unwrap_or(false)
                    || r.right.as_ref().map(|r| line_matches(&ctx, &r.content)).unwrap_or(false)
            }).count()
        }).sum()
    } else { 0 };

    let wrap_class = if snap.word_wrap { "diff-scroll wrap" } else { "diff-scroll" };

    rsx! {
        div {
            class: "diff-wrap",
            role: "region",
            aria_label: "File comparison",
            DiffHeader { index }
            Toolbar { index, snap: snap.clone(), lang }
            SearchBar { match_count }
            for w in snap.warnings.iter() {
                div { class: "diff-warning-banner", role: "alert", "⚠ {t(lang, w)}" }
            }
            if !snap.can_save {
                div { class: "notice", {t(lang, &snap.readonly_notice)} }
            }
            if snap.identical {
                div { class: "identical", {t(lang, "Files are identical")} }
            } else {
                div {
                    class: "{wrap_class}",
                    style: "--diff-fs:{snap.font_size}px;",
                    for hunk in snap.hunks.iter() {
                        HunkBlock {
                            index,
                            hunk: hunk.clone(),
                            char_mode: snap.char_mode,
                            context_lines: snap.context_lines,
                            focused: snap.focused_id == Some(hunk.hunk_id),
                            can_save: snap.can_save,
                            is_expanded: expanded.read().contains(&hunk.hunk_id),
                            on_expand: move |id: u64| { expanded.write().insert(id); },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DiffHeader(index: usize) -> Element {
    let store = use_context::<Store>();
    let (left, right) = {
        let tabs = store.tabs.read();
        let tab = match tabs.get(index) { Some(t) => t, None => return rsx!{} };
        (
            tab.left_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "—".into()),
            tab.right_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "—".into()),
        )
    };
    rsx! {
        div { class: "diff-file-header",
            span { class: "path-old", title: "{left}",  {trunc(&left)} }
            span { class: "arrow", "↔" }
            span { class: "path-new", title: "{right}", {trunc(&right)} }
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct TabSnapshot {
    hunks: Vec<forskscope_core::merge::MergeHunk>,
    identical: bool, char_mode: bool, word_wrap: bool, can_save: bool,
    is_dirty: bool, can_undo: bool, can_redo: bool, font_size: u32,
    focused_id: Option<u64>, focused_change: usize, changes: usize,
    ignore_whitespace: bool, ignore_case: bool, context_lines: usize,
    algorithm: forskscope_core::DiffAlgorithm,
    /// Human-readable messages from the diff engine (large file, deadline, …).
    warnings: Vec<String>,
    /// Shown instead of the generic "unavailable" text when `!can_save`.
    readonly_notice: String,
}

impl TabSnapshot {
    fn from_tab(tab: &crate::state::CompareTab, font_size: u32, context_lines: usize) -> Self {
        use forskscope_core::diff::DiffWarning;
        use forskscope_core::file_kind::FileKind;
        let hunks = tab.merge.hunks().to_vec();
        let ids: Vec<u64> = hunks.iter().filter(|h| h.kind.is_change()).map(|h| h.hunk_id).collect();
        let warnings = tab.diff.warnings.iter().map(|w| match w {
            DiffWarning::LargeFilePolicyApplied => "Large file — inline diff disabled and deadline shortened.",
            DiffWarning::DeadlineExpired        => "Diff timed out — result may be approximate.",
            DiffWarning::InlineSkippedHunkTooLarge => "Some hunks were too large for character-level diff.",
        }).map(str::to_string).collect();
        let readonly_notice = if tab.can_save { String::new() } else {
            match (&tab.left_doc.kind, &tab.right_doc.kind) {
                (FileKind::Binary, _) | (_, FileKind::Binary) =>
                    "Binary file — read-only comparison (hex preview).".into(),
                (FileKind::ExcelXlsx, _) | (_, FileKind::ExcelXlsx) =>
                    "Spreadsheet — read-only comparison.".into(),
                (FileKind::Missing, _) | (_, FileKind::Missing) =>
                    "One side is missing — read-only.".into(),
                (FileKind::Unsupported { .. }, _) | (_, FileKind::Unsupported { .. }) =>
                    "File type not supported for merge — read-only.".into(),
                _ => "Merge/save unavailable for this file type.".into(),
            }
        };
        Self {
            identical: tab.diff.is_identical(), char_mode: tab.char_mode,
            word_wrap: tab.word_wrap, can_save: tab.can_save,
            is_dirty: tab.merge.is_dirty(), can_undo: tab.merge.can_undo(),
            can_redo: tab.merge.can_redo(), font_size,
            focused_id: ids.get(tab.focused_change).copied(),
            focused_change: tab.focused_change, changes: ids.len(),
            ignore_whitespace: tab.diff_options.ignore_whitespace,
            ignore_case:       tab.diff_options.ignore_case,
            algorithm: tab.diff_options.algorithm,
            context_lines, hunks, warnings, readonly_notice,
        }
    }
}

#[component]
fn Toolbar(index: usize, snap: TabSnapshot, lang: Lang) -> Element {
    let mut store = use_context::<Store>();
    let mut advanced = use_signal(|| false);
    let mut search_ctx: Signal<SearchCtx> = use_context::<Signal<SearchCtx>>();
    let pos = if snap.changes == 0 { String::new() }
              else { format!("{} / {}", snap.focused_change + 1, snap.changes) };
    rsx! {
        div { class: "diff-toolbar",
            button { onclick: move |_| move_focus(&mut store, index, -1), title: "F7", "◀" }
            button { onclick: move |_| move_focus(&mut store, index,  1), title: "F8", "▶" }
            span { class: "info", "{pos}" }
            span { class: "spacer" }
            if snap.can_save {
                button {
                    disabled: !snap.can_undo,
                    onclick: move |_| { if let Some(tab) = store.tabs.write().get_mut(index) { let _ = tab.merge.undo(); } },
                    aria_label: "Undo last merge action (Ctrl+Z)",
                    "Undo"
                }
                button {
                    disabled: !snap.is_dirty,
                    onclick: move |_| save_tab(&mut store, index, false),
                    aria_label: "Save merge result (Ctrl+S)",
                    {t(lang, "Save")}
                }
                button {
                    onclick: move |_| {
                        let path = store.tabs.read().get(index)
                            .and_then(|t| t.right_path.as_ref())
                            .map(|p| p.display().to_string()).unwrap_or_default();
                        store.modal.set(Modal::SaveAs(index, path));
                    },
                    "Save As"
                }
            }
            button {
                title: "Reload both files from disk",
                aria_label: "Reload files from disk",
                onclick: move |_| {
                    let dirty = store.tabs.read().get(index).map(|t| t.merge.is_dirty()).unwrap_or(false);
                    if dirty { store.modal.set(Modal::ConfirmReload(index)); }
                    else { reload_tab(&mut store, index); store.notify(t(store.lang(), "Reloaded.")); }
                },
                "↺"
            }
            button {
                title: "Search within diff (Ctrl+F)",
                aria_label: "Open search bar",
                onclick: move |_| { search_ctx.write().active ^= true; },
                "🔍"
            }
            if snap.can_save {
                button {
                    onclick: move |_| { let v = *advanced.read(); advanced.set(!v); },
                    if *advanced.read() { "Less ▲" } else { "More ▼" }
                }
            }
        }
        if *advanced.read() && snap.can_save {
            div { class: "diff-toolbar advanced",
                button {
                    aria_pressed: if snap.char_mode { "true" } else { "false" },
                    aria_label: "Toggle character-level inline diff",
                    onclick: move |_| { if let Some(tab) = store.tabs.write().get_mut(index) { tab.char_mode ^= true; } },
                    {format!("Inline: {}", if snap.char_mode { "on" } else { "off" })}
                }
                button {
                    aria_pressed: if snap.word_wrap { "true" } else { "false" },
                    aria_label: "Toggle word wrap",
                    onclick: move |_| { if let Some(tab) = store.tabs.write().get_mut(index) { tab.word_wrap ^= true; } },
                    {format!("Wrap: {}", if snap.word_wrap { "on" } else { "off" })}
                }
                button {
                    disabled: !snap.can_redo,
                    onclick: move |_| { if let Some(tab) = store.tabs.write().get_mut(index) { let _ = tab.merge.redo(); } },
                    "Redo"
                }
                button {
                    onclick: move |_| {
                        let dirty = store.tabs.read().get(index).map(|t| t.merge.is_dirty()).unwrap_or(false);
                        if dirty { store.modal.set(Modal::ConfirmSwap(index)); }
                        else { swap_sides(&mut store, index); }
                    },
                    "⇄ Swap sides"
                }
                button {
                    aria_pressed: if snap.ignore_whitespace { "true" } else { "false" },
                    aria_label: "Toggle ignore whitespace",
                    onclick: move |_| {
                        let mut tabs = store.tabs.write();
                        if let Some(tab) = tabs.get_mut(index) {
                            tab.diff_options.ignore_whitespace ^= true;
                            recompute_diff(tab);
                        }
                    },
                    {format!("Ignore WS: {}", if snap.ignore_whitespace { "on" } else { "off" })}
                }
                button {
                    aria_pressed: if snap.ignore_case { "true" } else { "false" },
                    aria_label: "Toggle ignore case",
                    onclick: move |_| {
                        let mut tabs = store.tabs.write();
                        if let Some(tab) = tabs.get_mut(index) {
                            tab.diff_options.ignore_case ^= true;
                            recompute_diff(tab);
                        }
                    },
                    {format!("Ignore case: {}", if snap.ignore_case { "on" } else { "off" })}
                }
                select {
                    title: "Diff algorithm",
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
            }
        }
    }
}
