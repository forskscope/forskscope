//! Hunk and row rendering (RFC-006, RFC-024).

use dioxus::prelude::*;

use forskscope_core::diff::{HunkKind, InlineKind, refine_pair};
use forskscope_core::merge::{HunkState, MergeHunk};

use crate::state::Store;

/// Lines of context shown around changes inside a collapsed equal hunk.
pub const CONTEXT: usize = 3;

#[component]
pub fn HunkBlock(
    index: usize,
    hunk: MergeHunk,
    char_mode: bool,
    focused: bool,
    can_save: bool,
    is_expanded: bool,
    on_expand: EventHandler<u64>,
) -> Element {
    let mut store = use_context::<Store>();   // captured by copy in per-row closures
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

    // Collapse long equal hunks by default.
    let collapse = hunk.kind == HunkKind::Equal && !is_expanded && rows.len() > 2 * CONTEXT + 1;
    let hidden = if collapse { rows.len() - 2 * CONTEXT } else { 0 };

    rsx! {
        div { id: "h-{hunk_id}", class: "{class}",
            if collapse {
                for (i, row) in rows[..CONTEXT].iter().enumerate() {
                    Row {
                        left_no:     row.left.as_ref().and_then(|l| l.original_line_number),
                        right_no:    row.right.as_ref().and_then(|r| r.original_line_number),
                        left:        row.left.as_ref().map(|l| l.content.clone()),
                        right:       row.right.as_ref().map(|r| r.content.clone()),
                        kind:        hunk.kind,
                        char_mode:   false,
                        show_action: false,
                        applied:     i == 0 && applied,
                        on_apply:    EventHandler::new(|_| {}),
                    }
                }
                div {
                    class: "collapse-divider",
                    onclick: move |_| on_expand.call(hunk_id),
                    "··· {hidden} unchanged lines — click to expand ···"
                }
                for (i, row) in rows[rows.len() - CONTEXT..].iter().enumerate() {
                    Row {
                        left_no:     row.left.as_ref().and_then(|l| l.original_line_number),
                        right_no:    row.right.as_ref().and_then(|r| r.original_line_number),
                        left:        row.left.as_ref().map(|l| l.content.clone()),
                        right:       row.right.as_ref().map(|r| r.content.clone()),
                        kind:        hunk.kind,
                        char_mode:   false,
                        show_action: false,
                        applied:     i == 0 && applied,
                        on_apply:    EventHandler::new(|_| {}),
                    }
                }
            } else {
                for (i, row) in rows.iter().enumerate() {
                    Row {
                        left_no:     row.left.as_ref().and_then(|l| l.original_line_number),
                        right_no:    row.right.as_ref().and_then(|r| r.original_line_number),
                        left:        row.left.as_ref().map(|l| l.content.clone()),
                        right:       row.right.as_ref().map(|r| r.content.clone()),
                        kind:        hunk.kind,
                        char_mode,
                        show_action: i == 0 && hunk.is_pending_change() && can_save,
                        applied:     i == 0 && applied,
                        on_apply:    move |_| { let _ = store.tabs.write()[index].merge.apply_left_to_right(hunk_id); },
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
) -> Element {
    let inline = if char_mode && kind == HunkKind::Replace {
        match (&left, &right) { (Some(l), Some(r)) => Some(refine_pair(l, r)), _ => None }
    } else { None };
    let (lg, rg) = match kind {
        HunkKind::Delete => ("gutter del", "gutter"),
        HunkKind::Insert => ("gutter", "gutter ins"),
        _                => ("gutter",     "gutter"),
    };
    rsx! {
        div { class: "row",
            div { class: "{lg}", {left_no.map(|n| n.to_string()).unwrap_or_default()} }
            div { class: "cell",
                if let Some(ref spans) = inline {
                    for s in spans.left_spans.iter() { span { class: inline_cls(s.kind), "{s.text}" } }
                } else if let Some(ref l) = left { "{l}" }
            }
            div { class: "act",
                if show_action  { button { onclick: move |_| on_apply.call(()), "▶" } }
                else if applied { span { class: "applied", "✓" } }
            }
            div { class: "{rg}", {right_no.map(|n| n.to_string()).unwrap_or_default()} }
            div { class: "cell",
                if let Some(ref spans) = inline {
                    for s in spans.right_spans.iter() { span { class: inline_cls(s.kind), "{s.text}" } }
                } else if let Some(ref r) = right { "{r}" }
            }
        }
    }
}

fn inline_cls(kind: InlineKind) -> &'static str {
    match kind {
        InlineKind::Equal  => "",
        InlineKind::Delete => "in-del",
        InlineKind::Insert => "in-ins",
    }
}
