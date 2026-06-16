//! Hunk and row rendering with colour-independent diff markers (RFC-006, RFC-019).
//!
//! Every changed row carries a visible glyph (− + ~) in addition to the
//! colour cue so that colour is never the sole indicator of change kind.
//!
//! Layout: each Row renders as a flex row with two independent scroll panes
//! (.diff-pane.left and .diff-pane.right) separated by a fixed .diff-act
//! column. Each pane overflows and scrolls independently so a long line in
//! one pane never shrinks or shifts the other pane.

use dioxus::prelude::*;

use forskscope_core::diff::{HunkKind, InlineKind, refine_pair};
use forskscope_core::merge::{HunkState, MergeHunk};

use crate::i18n::t;
use crate::state::{Lang, Store};
use crate::ui::search::{SearchCtx, line_matches};

#[component]
pub fn HunkBlock(
    index: usize, hunk: MergeHunk,
    char_mode: bool, context_lines: usize,
    focused: bool, can_save: bool,
    is_expanded: bool, on_expand: EventHandler<u64>,
) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let kind_class = match hunk.kind {
        HunkKind::Equal   => "hunk",
        HunkKind::Delete  => "hunk hunk-del",
        HunkKind::Insert  => "hunk hunk-ins",
        HunkKind::Replace => "hunk hunk-rep",
    };
    let class = if focused { "hunk focused" } else { kind_class };
    let hunk_id = hunk.hunk_id;
    let applied = matches!(hunk.state, HunkState::AppliedLeftToRight);
    let rows = &hunk.rows;

    // Collapse long equal hunks unless expanded.
    let collapse = hunk.kind == HunkKind::Equal && !is_expanded
        && context_lines > 0 && rows.len() > 2 * context_lines + 1;
    let hidden = if collapse { rows.len() - 2 * context_lines } else { 0 };

    let head_rows: Vec<(usize, _)> = if collapse { rows[..context_lines].iter().enumerate().collect() } else { vec![] };
    let tail_rows: Vec<(usize, _)> = if collapse { rows[rows.len() - context_lines..].iter().enumerate().collect() } else { vec![] };
    let all_rows:  Vec<(usize, _)> = if !collapse { rows.iter().enumerate().collect() } else { vec![] };

    rsx! {
        div { id: "h-{hunk_id}", class: "{class}",
            if collapse {
                for (i, row) in head_rows {
                    Row {
                        left:  row.left.as_ref().map(|l| l.content.clone()),
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left_no:  row.left.as_ref().and_then(|l| l.original_line_number),
                        right_no: row.right.as_ref().and_then(|r| r.original_line_number),
                        kind: hunk.kind, char_mode: false,
                        show_action: false, applied: i == 0 && applied,
                        on_apply: EventHandler::new(|_| {}), lang,
                    }
                }
                div {
                    class: "collapse-divider",
                    onclick: move |_| on_expand.call(hunk_id),
                    {t(lang, "··· {n} unchanged lines — click to expand ···").replace("{n}", &hidden.to_string())}
                }
                for (i, row) in tail_rows {
                    Row {
                        left:  row.left.as_ref().map(|l| l.content.clone()),
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left_no:  row.left.as_ref().and_then(|l| l.original_line_number),
                        right_no: row.right.as_ref().and_then(|r| r.original_line_number),
                        kind: hunk.kind, char_mode: false,
                        show_action: false, applied: i == 0 && applied,
                        on_apply: EventHandler::new(|_| {}), lang,
                    }
                }
            } else {
                for (i, row) in all_rows {
                    Row {
                        left:  row.left.as_ref().map(|l| l.content.clone()),
                        right: row.right.as_ref().map(|r| r.content.clone()),
                        left_no:  row.left.as_ref().and_then(|l| l.original_line_number),
                        right_no: row.right.as_ref().and_then(|r| r.original_line_number),
                        kind: hunk.kind, char_mode,
                        show_action: i == 0 && hunk.is_pending_change() && can_save,
                        applied: i == 0 && applied,
                        on_apply: move |_| { if let Some(tab) = store.tabs.write().get_mut(index) { let _ = tab.merge.apply_left_to_right(hunk_id); } },
                        lang,
                    }
                }
            }
        }
    }
}

#[component]
fn Row(
    left_no: Option<u32>, right_no: Option<u32>,
    left: Option<String>, right: Option<String>,
    kind: HunkKind, char_mode: bool,
    show_action: bool, applied: bool,
    on_apply: EventHandler<()>,
    lang: Lang,
) -> Element {
    let search: Signal<SearchCtx> = use_context::<Signal<SearchCtx>>();
    let ctx = search.read();
    let is_match = left.as_deref().map(|c| line_matches(&ctx, c)).unwrap_or(false)
        || right.as_deref().map(|c| line_matches(&ctx, c)).unwrap_or(false);
    drop(ctx);

    let inline = if char_mode && kind == HunkKind::Replace {
        match (&left, &right) { (Some(l), Some(r)) => Some(refine_pair(l, r)), _ => None }
    } else { None };

    // Colour-independent gutter marks (RFC-019 §19.3).
    let left_mark  = match kind { HunkKind::Delete | HunkKind::Replace => "−", _ => " " };
    let right_mark = match kind { HunkKind::Insert | HunkKind::Replace => "+", _ => " " };

    let left_gutter_class  = match kind { HunkKind::Delete  => "pane-gutter del", _ => "pane-gutter" };
    let right_gutter_class = match kind { HunkKind::Insert  => "pane-gutter ins", _ => "pane-gutter" };

    let row_class = if is_match { "diff-row match" } else { "diff-row" };

    // Screen-reader label for non-equal rows.
    let sr_label: Option<String> = match kind {
        HunkKind::Delete  => Some(t(lang, "Deleted")),
        HunkKind::Insert  => Some(t(lang, "Inserted")),
        HunkKind::Replace => Some(t(lang, "Changed")),
        HunkKind::Equal   => None,
    };

    rsx! {
        div { class: "{row_class}", role: "row",
            if let Some(ref lbl) = sr_label {
                span { class: "sr-only", "{lbl}: " }
            }

            // ── Left pane (independent scroll) ───────────────────
            div { class: "diff-pane left",
                div { class: "{left_gutter_class}",
                    {left_no.map(|n| n.to_string()).unwrap_or_default()}
                }
                span { class: "diff-mark", aria_hidden: "true", "{left_mark}" }
                div { class: "cell",
                    if let Some(ref spans) = inline {
                        for s in spans.left_spans.iter() { span { class: icls(s.kind), "{s.text}" } }
                    } else if let Some(ref l) = left { "{l}" }
                }
            }

            // ── Act column (fixed, centred) ───────────────────────
            div { class: "diff-act",
                if show_action  { button { onclick: move |_| on_apply.call(()), aria_label: t(lang, "Apply change left to right"), "▶" } }
                else if applied { span { class: "applied", aria_label: t(lang, "Applied"), "✓" } }
            }

            // ── Right pane (independent scroll) ──────────────────
            div { class: "diff-pane right",
                div { class: "{right_gutter_class}",
                    {right_no.map(|n| n.to_string()).unwrap_or_default()}
                }
                span { class: "diff-mark", aria_hidden: "true", "{right_mark}" }
                div { class: "cell",
                    if let Some(ref spans) = inline {
                        for s in spans.right_spans.iter() { span { class: icls(s.kind), "{s.text}" } }
                    } else if let Some(ref r) = right { "{r}" }
                }
            }
        }
    }
}

fn icls(k: InlineKind) -> &'static str {
    match k { InlineKind::Equal => "", InlineKind::Delete => "in-del", InlineKind::Insert => "in-ins" }
}
