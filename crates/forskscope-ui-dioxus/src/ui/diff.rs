//! Diff/merge workspace (RFC-006, RFC-024, RFC-035).
//!
//! Rows are rendered from the tab's [`MergeSession`] working hunks so that
//! applied merges are reflected immediately. A single scroll container holds
//! both sides, which keeps the panes synchronized by construction.

use dioxus::prelude::*;

use forskscope_core::diff::{HunkKind, InlineKind, refine_pair};
use forskscope_core::merge::{HunkState, MergeHunk};
use forskscope_core::save::{BackupPolicy, SaveRequest, save_text};

use crate::i18n::t;
use crate::state::{CompareTab, Lang, Modal, Store};

#[component]
pub fn DiffWorkspace(index: usize) -> Element {
    let store = use_context::<Store>();
    let lang = store.lang();

    // Snapshot what we need from the tab for this render.
    let font_size = store.settings.read().diff_font_size;
    let snapshot = {
        let tabs = store.tabs.read();
        tabs.get(index)
            .map(|tab| TabSnapshot::from_tab(tab, font_size))
    };
    let Some(snap) = snapshot else {
        return rsx! { div { class: "notice", "No comparison." } };
    };

    rsx! {
        div { style: "display:flex;flex-direction:column;height:100%;",
            Toolbar { index, snap: snap.clone(), lang }
            if !snap.can_save {
                div { class: "notice", {t(lang, "Merge/save unavailable for this file type.")} }
            }
            div { class: "body",
                if snap.identical {
                    div { class: "identical", {t(lang, "Files are identical")} }
                } else {
                    div {
                        class: "diff-scroll",
                        style: "--diff-fs:{snap.font_size}px;",
                        for hunk in snap.hunks.iter() {
                            HunkBlock {
                                index,
                                hunk: hunk.clone(),
                                char_mode: snap.char_mode,
                                focused: snap.focused_id == Some(hunk.hunk_id),
                                can_save: snap.can_save,
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Lightweight, cloneable snapshot used for rendering.
#[derive(Clone, PartialEq)]
struct TabSnapshot {
    hunks: Vec<MergeHunk>,
    identical: bool,
    char_mode: bool,
    can_save: bool,
    is_dirty: bool,
    can_undo: bool,
    can_redo: bool,
    font_size: u32,
    focused_id: Option<u64>,
    /// 0-based position among changed hunks, and total changed hunks.
    focused_change: usize,
    changes: usize,
}

impl TabSnapshot {
    fn from_tab(tab: &CompareTab, font_size: u32) -> Self {
        let hunks = tab.merge.hunks().to_vec();
        let change_ids: Vec<u64> = hunks
            .iter()
            .filter(|h| h.kind.is_change())
            .map(|h| h.hunk_id)
            .collect();
        let focused_id = change_ids.get(tab.focused_change).copied();
        Self {
            identical: tab.diff.is_identical(),
            char_mode: tab.char_mode,
            can_save: tab.can_save,
            is_dirty: tab.merge.is_dirty(),
            can_undo: tab.merge.can_undo(),
            can_redo: tab.merge.can_redo(),
            font_size,
            focused_id,
            focused_change: tab.focused_change,
            changes: change_ids.len(),
            hunks,
        }
    }
}

#[component]
fn Toolbar(index: usize, snap: TabSnapshot, lang: Lang) -> Element {
    let mut store = use_context::<Store>();
    // Advanced controls stay hidden until the user asks for them.
    let mut advanced = use_signal(|| false);

    // A single quiet position indicator: "2 / 5" while navigating changes.
    let position = if snap.changes == 0 {
        String::new()
    } else {
        format!("{} / {}", snap.focused_change + 1, snap.changes)
    };
    let more_label = if *advanced.read() { "Less ▲" } else { "More ▼" };

    rsx! {
        div { class: "diff-toolbar",
            button { onclick: move |_| move_focus(&mut store, index, -1), title: "{t(lang, \"Previous change\")}", "◀" }
            button { onclick: move |_| move_focus(&mut store, index, 1), title: "{t(lang, \"Next change\")}", "▶" }
            span { class: "info", "{position}" }
            span { class: "spacer" }
            if snap.can_save {
                button {
                    disabled: !snap.can_undo,
                    onclick: move |_| { let _ = store.tabs.write()[index].merge.undo(); },
                    {t(lang, "Undo")}
                }
                button {
                    disabled: !(snap.is_dirty),
                    onclick: move |_| save_tab(&mut store, index, false),
                    {t(lang, "Save")}
                }
                button { onclick: move |_| { let v = *advanced.read(); advanced.set(!v); }, "{more_label}" }
            }
        }
        if *advanced.read() && snap.can_save {
            div { class: "diff-toolbar advanced",
                button {
                    onclick: move |_| { store.tabs.write()[index].char_mode ^= true; },
                    {format!("{}: {}", t(lang, "Inline diff"), if snap.char_mode { "on" } else { "off" })}
                }
                button {
                    disabled: !snap.can_redo,
                    onclick: move |_| { let _ = store.tabs.write()[index].merge.redo(); },
                    {t(lang, "Redo")}
                }
            }
        }
    }
}

#[component]
fn HunkBlock(index: usize, hunk: MergeHunk, char_mode: bool, focused: bool, can_save: bool) -> Element {
    let mut store = use_context::<Store>();
    let kind_class = match hunk.kind {
        HunkKind::Equal => "hunk",
        HunkKind::Delete => "hunk hunk-del",
        HunkKind::Insert => "hunk hunk-ins",
        HunkKind::Replace => "hunk hunk-rep",
    };
    let focused_class = if focused { "hunk focused" } else { kind_class };
    let class = if focused { focused_class } else { kind_class };
    let hunk_id = hunk.hunk_id;
    let pending = hunk.is_pending_change();
    let applied = matches!(hunk.state, HunkState::AppliedLeftToRight);

    rsx! {
        div { class: "{class}",
            for (i, row) in hunk.rows.iter().enumerate() {
                Row {
                    left_no: row.left.as_ref().and_then(|l| l.original_line_number),
                    right_no: row.right.as_ref().and_then(|r| r.original_line_number),
                    left: row.left.as_ref().map(|l| l.content.clone()),
                    right: row.right.as_ref().map(|r| r.content.clone()),
                    kind: hunk.kind,
                    char_mode,
                    show_action: i == 0 && pending && can_save,
                    applied: i == 0 && applied,
                    on_apply: move |_| { let _ = store.tabs.write()[index].merge.apply_left_to_right(hunk_id); },
                }
            }
        }
    }
}

#[component]
fn Row(
    left_no: Option<u32>,
    right_no: Option<u32>,
    left: Option<String>,
    right: Option<String>,
    kind: HunkKind,
    char_mode: bool,
    show_action: bool,
    applied: bool,
    on_apply: EventHandler<()>,
) -> Element {
    let inline = if char_mode && kind == HunkKind::Replace {
        match (&left, &right) {
            (Some(l), Some(r)) => Some(refine_pair(l, r)),
            _ => None,
        }
    } else {
        None
    };
    let (left_g, right_g) = match kind {
        HunkKind::Delete => ("gutter del", "gutter"),
        HunkKind::Insert => ("gutter", "gutter ins"),
        _ => ("gutter", "gutter"),
    };

    rsx! {
        div { class: "row",
            div { class: "{left_g}", {left_no.map(|n| n.to_string()).unwrap_or_default()} }
            div { class: "cell",
                if let Some(inline) = &inline {
                    for span in inline.left_spans.iter() {
                        span { class: inline_class(span.kind), "{span.text}" }
                    }
                } else if let Some(l) = &left {
                    "{l}"
                }
            }
            div { class: "act",
                if show_action {
                    button { onclick: move |_| on_apply.call(()), "▶" }
                } else if applied {
                    span { class: "applied", "✓" }
                }
            }
            div { class: "{right_g}", {right_no.map(|n| n.to_string()).unwrap_or_default()} }
            div { class: "cell",
                if let Some(inline) = &inline {
                    for span in inline.right_spans.iter() {
                        span { class: inline_class(span.kind), "{span.text}" }
                    }
                } else if let Some(r) = &right {
                    "{r}"
                }
            }
        }
    }
}

fn inline_class(kind: InlineKind) -> &'static str {
    match kind {
        InlineKind::Equal => "",
        InlineKind::Delete => "in-del",
        InlineKind::Insert => "in-ins",
    }
}

fn move_focus(store: &mut Store, index: usize, delta: i32) {
    let mut tabs = store.tabs.write();
    let Some(tab) = tabs.get_mut(index) else {
        return;
    };
    let changes = tab.merge.hunks().iter().filter(|h| h.kind.is_change()).count();
    if changes == 0 {
        return;
    }
    let current = tab.focused_change as i32;
    let next = (current + delta).rem_euclid(changes as i32) as usize;
    tab.focused_change = next;
}

/// Run the save flow for a tab. `force` re-captures the on-disk fingerprint
/// to overwrite after the user confirmed an external-change conflict.
pub fn save_tab(store: &mut Store, index: usize, force: bool) {
    let request = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else { return };
        if !tab.can_save {
            return;
        }
        let Some(target) = tab.right_path.clone() else {
            return;
        };
        let encoding_label = tab
            .right_doc
            .text
            .as_ref()
            .map(|t| t.encoding.label.clone())
            .unwrap_or_else(|| "UTF-8".into());
        let expected = if force {
            forskscope_core::document::FileFingerprint::capture(&target, None).ok()
        } else {
            tab.right_doc.fingerprint_at_load
        };
        SaveRequest {
            target,
            content: tab.merge.result_text(),
            encoding_label,
            expected_fingerprint: expected,
            backup: BackupPolicy::SiblingBak,
        }
    };

    match save_text(&request) {
        Ok(outcome) => {
            let mut tabs = store.tabs.write();
            if let Some(tab) = tabs.get_mut(index) {
                tab.merge.mark_saved();
                tab.right_doc.fingerprint_at_load = Some(outcome.new_fingerprint);
            }
            drop(tabs);
            store.modal.set(Modal::None);
            store.notify(t(store.lang(), "Saved."));
        }
        Err(forskscope_core::CoreError::Conflict { .. }) => {
            store.modal.set(Modal::ConfirmOverwrite(index));
        }
        Err(e) => store.notify(e.to_string()),
    }
}
