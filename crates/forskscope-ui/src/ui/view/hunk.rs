//! Hunk and row rendering with colour-independent diff markers (RFC-006, RFC-019).
//!
//! Every changed row carries a visible glyph (− + ~) in addition to the
//! colour cue so that colour is never the sole indicator of change kind.
//!
//! Layout: DiffWorkspace renders each hunk into three parallel column
//! containers (.diff-col-left, .diff-col-act, .diff-col-right). Each column
//! container owns its own horizontal scrollbar so there is one scrollbar per
//! pane rather than one per row (RFC-064 Approach A).

use dioxus::prelude::*;

use forskscope_core::diff::{HunkKind, InlineKind, refine_pair};
use forskscope_core::merge::{HunkState, MergeHunk};

use crate::i18n::t;
use crate::state::{Lang, Store};
use crate::ui::view::search::{SearchCtx, line_matches};

/// Which column this `HunkBlock` should render.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HunkCol { Left, Act, Right }

#[component]
pub fn HunkBlock(
    index: usize, hunk: MergeHunk,
    col: HunkCol,
    char_mode: bool, context_lines: usize,
    focused: bool, can_save: bool,
    is_expanded: bool, on_expand: EventHandler<u64>,
) -> Element {
    let store = use_context::<Store>();
    let lang  = store.lang();
    let kind_class = match hunk.kind {
        HunkKind::Equal   => "hunk",
        HunkKind::Delete  => "hunk hunk-del",
        HunkKind::Insert  => "hunk hunk-ins",
        HunkKind::Replace => "hunk hunk-rep",
    };
    let class   = if focused { format!("{kind_class} focused") } else { kind_class.to_string() };
    let hunk_id = hunk.hunk_id;
    let applied = matches!(hunk.state, HunkState::AppliedLeftToRight);
    let pending = hunk.is_pending_change();
    let rows    = &hunk.rows;

    let collapse = hunk.kind == HunkKind::Equal && !is_expanded
        && context_lines > 0 && rows.len() > 2 * context_lines + 1;
    let hidden = if collapse { rows.len() - 2 * context_lines } else { 0 };

    let head_rows: Vec<(usize, _)> = if collapse { rows[..context_lines].iter().enumerate().collect() } else { vec![] };
    let tail_rows: Vec<(usize, _)> = if collapse {
        rows[rows.len() - context_lines..].iter().enumerate()
            .map(|(i, r)| (context_lines + 1 + i, r)).collect()
    } else { vec![] };
    let all_rows: Vec<(usize, _)> = if !collapse { rows.iter().enumerate().collect() } else { vec![] };

    match col {
        HunkCol::Left => rsx! {
            div { id: "h-{hunk_id}", class: "{class}",
                if collapse {
                    for (_, row) in head_rows { RowLeft {
                        left: row.left.as_ref().map(|l| l.content.clone()),
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left_no: row.left.as_ref().and_then(|l| l.original_line_number),
                        kind: hunk.kind, char_mode, lang,
                    } }
                    div { class: "collapse-divider", onclick: move |_| on_expand.call(hunk_id),
                        {t(lang, "··· {n} unchanged lines — click to expand ···").replace("{n}", &hidden.to_string())}
                    }
                    for (_, row) in tail_rows { RowLeft {
                        left: row.left.as_ref().map(|l| l.content.clone()),
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left_no: row.left.as_ref().and_then(|l| l.original_line_number),
                        kind: hunk.kind, char_mode, lang,
                    } }
                } else {
                    for (_, row) in all_rows { RowLeft {
                        left: row.left.as_ref().map(|l| l.content.clone()),
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left_no: row.left.as_ref().and_then(|l| l.original_line_number),
                        kind: hunk.kind, char_mode, lang,
                    } }
                }
            }
        },

        HunkCol::Act => rsx! {
            div { class: "{class}",
                if collapse {
                    for (i, _) in head_rows { ActCell { i, index, hunk_id, applied, can_save, pending, lang } }
                    div { class: "collapse-divider-spacer" }
                    for (i, _) in tail_rows { ActCell { i, index, hunk_id, applied, can_save, pending, lang } }
                } else {
                    for (i, _) in all_rows { ActCell { i, index, hunk_id, applied, can_save, pending, lang } }
                }
            }
        },

        HunkCol::Right => rsx! {
            div { class: "{class}",
                if collapse {
                    for (_, row) in head_rows { RowRight {
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left: row.left.as_ref().map(|l| l.content.clone()),
                        right_no: row.right.as_ref().and_then(|r| r.original_line_number),
                        kind: hunk.kind, char_mode, lang,
                    } }
                    div { class: "collapse-divider-spacer" }
                    for (_, row) in tail_rows { RowRight {
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left: row.left.as_ref().map(|l| l.content.clone()),
                        right_no: row.right.as_ref().and_then(|r| r.original_line_number),
                        kind: hunk.kind, char_mode, lang,
                    } }
                } else {
                    for (_, row) in all_rows { RowRight {
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left: row.left.as_ref().map(|l| l.content.clone()),
                        right_no: row.right.as_ref().and_then(|r| r.original_line_number),
                        kind: hunk.kind, char_mode, lang,
                    } }
                }
            }
        },
    }
}

// ── Row halves ────────────────────────────────────────────────────────────────

#[component]
fn RowLeft(
    left_no: Option<u32>,
    left: Option<String>, right: Option<String>,
    kind: HunkKind, char_mode: bool, lang: Lang,
) -> Element {
    let search: Signal<SearchCtx> = use_context::<Signal<SearchCtx>>();
    let ctx      = search.read();
    let is_match = left.as_deref().map(|c| line_matches(&ctx, c)).unwrap_or(false)
        || right.as_deref().map(|c| line_matches(&ctx, c)).unwrap_or(false);
    drop(ctx);

    let inline_left = if char_mode && kind == HunkKind::Replace {
        match (&left, &right) { (Some(l), Some(r)) => Some(refine_pair(l, r).left_spans), _ => None }
    } else { None };

    let gutter_class = match kind { HunkKind::Delete | HunkKind::Replace => "pane-gutter del", _ => "pane-gutter" };
    let mark         = match kind { HunkKind::Delete | HunkKind::Replace => "−", _ => " " };
    let sr_label: Option<String> = match kind {
        HunkKind::Delete  => Some(t(lang, "Deleted")),
        HunkKind::Replace => Some(t(lang, "Changed")),
        _                 => None,
    };
    let row_class = if is_match { "diff-row match" } else { "diff-row" };

    rsx! {
        div { class: "{row_class}", role: "row",
            if let Some(ref lbl) = sr_label { span { class: "sr-only", "{lbl}: " } }
            div { class: "{gutter_class}", {left_no.map(|n| n.to_string()).unwrap_or_default()} }
            span { class: "diff-mark", aria_hidden: "true", "{mark}" }
            div { class: "cell",
                if let Some(ref spans) = inline_left {
                    for s in spans.iter() { span { class: icls(s.kind), "{s.text}" } }
                } else if let Some(ref l) = left { "{l}" }
            }
        }
    }
}

#[component]
fn RowRight(
    right_no: Option<u32>,
    right: Option<String>, left: Option<String>,
    kind: HunkKind, char_mode: bool, lang: Lang,
) -> Element {
    let search: Signal<SearchCtx> = use_context::<Signal<SearchCtx>>();
    let ctx      = search.read();
    let is_match = left.as_deref().map(|c| line_matches(&ctx, c)).unwrap_or(false)
        || right.as_deref().map(|c| line_matches(&ctx, c)).unwrap_or(false);
    drop(ctx);

    let inline_right = if char_mode && kind == HunkKind::Replace {
        match (&left, &right) { (Some(l), Some(r)) => Some(refine_pair(l, r).right_spans), _ => None }
    } else { None };

    let gutter_class = match kind { HunkKind::Insert | HunkKind::Replace => "pane-gutter ins", _ => "pane-gutter" };
    let mark         = match kind { HunkKind::Insert | HunkKind::Replace => "+", _ => " " };
    let sr_label: Option<String> = match kind {
        HunkKind::Insert  => Some(t(lang, "Inserted")),
        HunkKind::Replace => Some(t(lang, "Changed")),
        _                 => None,
    };
    let row_class = if is_match { "diff-row match" } else { "diff-row" };

    rsx! {
        div { class: "{row_class}", role: "row",
            if let Some(ref lbl) = sr_label { span { class: "sr-only", "{lbl}: " } }
            div { class: "{gutter_class}", {right_no.map(|n| n.to_string()).unwrap_or_default()} }
            span { class: "diff-mark", aria_hidden: "true", "{mark}" }
            div { class: "cell",
                if let Some(ref spans) = inline_right {
                    for s in spans.iter() { span { class: icls(s.kind), "{s.text}" } }
                } else if let Some(ref r) = right { "{r}" }
            }
        }
    }
}

/// Act cell — uses use_context for Store to avoid E0369 (Store is not PartialEq).
#[component]
fn ActCell(
    i: usize, index: usize, hunk_id: u64,
    applied: bool, can_save: bool, pending: bool,
    lang: Lang,
) -> Element {
    let mut store = use_context::<Store>();
    rsx! {
        div { class: "diff-act",
            if i == 0 && pending && can_save {
                button {
                    class: "apply-btn",
                    onclick: move |_| {
                        if let Some(tab) = store.tabs.write().get_mut(index) {
                            let _ = tab.merge.apply_left_to_right(hunk_id);
                        }
                    },
                    title: t(lang, "Use this change (apply left to right)"),
                    aria_label: t(lang, "Use this change (apply left to right)"),
                    "▶"
                    span { class: "apply-btn-label", {t(lang, "Use")} }
                }
            } else if i == 0 && applied {
                span { class: "applied", aria_label: t(lang, "Applied"), "✓" }
            }
        }
    }
}

fn icls(k: InlineKind) -> &'static str {
    match k { InlineKind::Equal => "", InlineKind::Delete => "in-del", InlineKind::Insert => "in-ins" }
}
