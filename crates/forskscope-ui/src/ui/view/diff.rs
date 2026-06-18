//! Diff/merge workspace: coordination, snapshot, and loading/error states.
//! Hunk rendering lives in [`crate::ui::view::hunk`].
//! Toolbar lives in [`diff::toolbar`].

pub mod toolbar;

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::Store;
use crate::ui::view::diff_actions::trunc;
pub use crate::ui::view::diff_actions::{apply_focused_hunk, move_focus, save_as, save_tab};
use crate::ui::view::hunk::HunkBlock;
use crate::ui::view::search::{SearchBar, SearchCtx, scroll_to_focused};
use crate::ui::bridge::search_index::MatchIndex;
use toolbar::Toolbar;

// ── Workspace component ───────────────────────────────────────────────────────

#[component]
pub fn DiffWorkspace(index: usize) -> Element {
    let store = use_context::<Store>();
    let lang = store.lang();
    let font_size     = store.settings.read().diff_font_size;
    let font_family   = store.settings.read().diff_font_family.css_value();
    let context_lines = store.settings.read().context_lines;

    // Loading / Error states (RFC-065).
    // Important: extract state and title *then drop the guard* before returning.
    // Holding the read guard across the return boundary prevents Dioxus from
    // registering this component as a subscriber, so the signal write from the
    // background task never triggers a re-render.
    {
        let (state, title) = {
            let tabs = store.tabs.read();
            let state = tabs.get(index).map(|t| t.state.clone());
            let title = tabs.get(index).map(|t| t.title.clone()).unwrap_or_default();
            (state, title)
        };
        match state {
            None => return rsx! { div { class: "notice", {t(lang, "No comparison.")} } },
            Some(crate::state::TabState::Loading) => {
                return rsx! {
                    div { class: "diff-loading",
                        span { class: "diff-loading-spinner", "⟳" }
                        span { {t(lang, "Loading")} " " {title} "…" }
                    }
                };
            }
            Some(crate::state::TabState::Error(msg)) => {
                return rsx! {
                    div { class: "diff-error",
                        p { class: "notice", "⚠ " {msg} }
                        p { class: "notice", {t(lang, "Check that the file exists and you have read permission.")} }
                    }
                };
            }
            Some(crate::state::TabState::Ready) => {}
        }
    }

    let snap = {
        let tabs = store.tabs.read();
        match tabs.get(index) {
            Some(tab) => TabSnapshot::from_tab(tab, font_size, font_family, context_lines, lang),
            None => return rsx! { div { class: "notice", {t(lang, "No comparison.")} } },
        }
    };

    let mut search_ctx: Signal<SearchCtx> = use_context_provider(|| Signal::new(SearchCtx::default()));
    let mut expanded:   Signal<HashSet<u64>> = use_signal(HashSet::new);

    // Rebuild match index on query change; auto-expand hunks containing matches.
    {
        let query  = search_ctx.read().query.clone();
        let active = search_ctx.read().active;
        if active && !query.is_empty() {
            let hunk_rows: Vec<(u64, Vec<(Option<&str>, Option<&str>)>)> = snap.hunks.iter()
                .map(|h| {
                    let rows = h.rows.iter()
                        .map(|r| (
                            r.left.as_ref().map(|l| l.content.as_str()),
                            r.right.as_ref().map(|r| r.content.as_str()),
                        ))
                        .collect();
                    (h.hunk_id, rows)
                })
                .collect();
            let new_index = MatchIndex::build(
                hunk_rows.iter().map(|(id, rows)| (*id, rows.as_slice())),
                &query,
            );
            for id in new_index.matching_hunk_ids() { expanded.write().insert(id); }
            let prev_len = search_ctx.read().index.len();
            if new_index.len() != prev_len || search_ctx.read().index.focused_number() == Some(1) {
                let ctx_snap = search_ctx.read();
                scroll_to_focused(&ctx_snap);
                drop(ctx_snap);
            }
            search_ctx.write().index = new_index;
        } else if !active {
            search_ctx.write().index = MatchIndex::default();
        }
    }

    let wrap_class = if snap.word_wrap { "diff-scroll wrap" } else { "diff-scroll" };

    rsx! {
        div {
            class: "diff-wrap",
            role: "region",
            aria_label: t(lang, "File comparison"),
            DiffHeader { index }
            Toolbar { index, snap: snap.clone(), lang }
            SearchBar {}
            for w in snap.warnings.iter() {
                div { class: "diff-warning-banner", role: "alert", "⚠ {w}" }
            }
            if !snap.can_save {
                div { class: "notice", {snap.readonly_notice.clone()} }
            }
            if snap.identical {
                div { class: "notice notice-ok", {t(lang, "Files are identical")} }
            }
            div { class: "diff-pane-labels", aria_hidden: "true",
                span { class: "pane-label-left",  {t(lang, "Left / Old")} }
                span { class: "pane-label-act" }
                span { class: "pane-label-right", {t(lang, "Right / New")} }
            }
            div {
                class: "{wrap_class}",
                style: "--diff-fs:{snap.font_size}px; --diff-ff:{snap.font_family};",
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

// ── Diff file header ──────────────────────────────────────────────────────────

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

// ── Tab snapshot ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub struct TabSnapshot {
    pub hunks: Vec<forskscope_core::merge::MergeHunk>,
    pub identical: bool,
    pub char_mode: bool,
    pub word_wrap: bool,
    pub can_save: bool,
    pub is_dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub font_size: u32,
    pub font_family: &'static str,
    pub focused_id: Option<u64>,
    pub focused_change: usize,
    pub changes: usize,
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    pub context_lines: usize,
    pub algorithm: forskscope_core::DiffAlgorithm,
    pub warnings: Vec<String>,
    pub readonly_notice: String,
}

impl TabSnapshot {
    pub fn from_tab(
        tab: &crate::state::CompareTab,
        font_size: u32,
        font_family: &'static str,
        context_lines: usize,
        lang: crate::state::Lang,
    ) -> Self {
        use forskscope_core::diff::DiffWarning;
        use forskscope_core::file_kind::FileKind;
        use crate::i18n::t;

        let hunks = tab.merge.hunks().to_vec();
        let ids: Vec<u64> = hunks.iter().filter(|h| h.kind.is_change()).map(|h| h.hunk_id).collect();
        let warnings = tab.diff.warnings.iter().map(|w| match w {
            DiffWarning::LargeFilePolicyApplied    => t(lang, "Large file — inline diff disabled and deadline shortened."),
            DiffWarning::DeadlineExpired           => t(lang, "Diff timed out — result may be approximate."),
            DiffWarning::InlineSkippedHunkTooLarge => t(lang, "Some hunks were too large for character-level diff."),
        }).collect();
        let both_missing = matches!(tab.left_doc.kind, FileKind::Missing)
            && matches!(tab.right_doc.kind, FileKind::Missing);
        let readonly_notice = if tab.can_save { String::new() } else {
            match (&tab.left_doc.kind, &tab.right_doc.kind) {
                (FileKind::Missing,  FileKind::Missing)  => t(lang, "Both files not found — read-only."),
                (FileKind::Binary,   _) | (_, FileKind::Binary)   => t(lang, "Binary file — read-only comparison (hex preview)."),
                (FileKind::ExcelXlsx,_) | (_, FileKind::ExcelXlsx)=> t(lang, "Spreadsheet — read-only comparison."),
                (FileKind::Missing,  _) | (_, FileKind::Missing)  => t(lang, "One side is missing — read-only."),
                (FileKind::Unsupported {..},_) | (_,FileKind::Unsupported {..}) =>
                    t(lang, "File type not supported for merge — read-only."),
                _ => t(lang, "Merge/save unavailable for this file type."),
            }
        };
        Self {
            identical: tab.diff.is_identical() && !both_missing,
            char_mode: tab.char_mode, word_wrap: tab.word_wrap, can_save: tab.can_save,
            is_dirty: tab.merge.is_dirty(), can_undo: tab.merge.can_undo(),
            can_redo: tab.merge.can_redo(), font_size, font_family,
            focused_id: ids.get(tab.focused_change).copied(),
            focused_change: tab.focused_change, changes: ids.len(),
            ignore_whitespace: tab.diff_options.ignore_whitespace,
            ignore_case:       tab.diff_options.ignore_case,
            algorithm: tab.diff_options.algorithm,
            context_lines, hunks, warnings, readonly_notice,
        }
    }
}
