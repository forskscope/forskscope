//! Diff/merge workspace coordination and actions (RFC-006, RFC-007).
//! Rendering of individual hunks lives in [`crate::ui::hunk`].

use std::collections::HashSet;

use dioxus::prelude::*;

use forskscope_core::save::{BackupPolicy, SaveRequest, save_text};
use forskscope_core::CoreError;

use crate::i18n::t;
use crate::state::{Lang, Modal, Store, recompute_diff, reload_tab};
use crate::ui::hunk::HunkBlock;

#[component]
pub fn DiffWorkspace(index: usize) -> Element {
    let store = use_context::<Store>();
    let lang = store.lang();
    let font_size = store.settings.read().diff_font_size;
    let snap = {
        let tabs = store.tabs.read();
        match tabs.get(index) {
            Some(tab) => TabSnapshot::from_tab(tab, font_size),
            None => return rsx! { div { class: "notice", "No comparison." } },
        }
    };
    let mut expanded: Signal<HashSet<u64>> = use_signal(HashSet::new);
    rsx! {
        div { class: "diff-wrap",
            DiffHeader { index }
            Toolbar { index, snap: snap.clone(), lang }
            if !snap.can_save {
                div { class: "notice", {t(lang, "Merge/save unavailable for this file type.")} }
            }
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
                            char_mode:   snap.char_mode,
                            focused:     snap.focused_id == Some(hunk.hunk_id),
                            can_save:    snap.can_save,
                            is_expanded: expanded.read().contains(&hunk.hunk_id),
                            on_expand:   move |id: u64| { expanded.write().insert(id); },
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
struct TabSnapshot {
    hunks: Vec<forskscope_core::merge::MergeHunk>,
    identical: bool,
    char_mode: bool,
    can_save: bool,
    is_dirty: bool,
    can_undo: bool,
    can_redo: bool,
    font_size: u32,
    focused_id: Option<u64>,
    focused_change: usize,
    changes: usize,
    ignore_whitespace: bool,
}

impl TabSnapshot {
    fn from_tab(tab: &crate::state::CompareTab, font_size: u32) -> Self {
        let hunks = tab.merge.hunks().to_vec();
        let change_ids: Vec<u64> = hunks.iter()
            .filter(|h| h.kind.is_change()).map(|h| h.hunk_id).collect();
        Self {
            identical: tab.diff.is_identical(),
            char_mode: tab.char_mode,
            can_save: tab.can_save,
            is_dirty: tab.merge.is_dirty(),
            can_undo: tab.merge.can_undo(),
            can_redo: tab.merge.can_redo(),
            font_size,
            focused_id: change_ids.get(tab.focused_change).copied(),
            focused_change: tab.focused_change,
            changes: change_ids.len(),
            ignore_whitespace: tab.diff_options.ignore_whitespace,
            hunks,
        }
    }
}

#[component]
fn Toolbar(index: usize, snap: TabSnapshot, lang: Lang) -> Element {
    let mut store = use_context::<Store>();
    let mut advanced = use_signal(|| false);
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
                    onclick: move |_| { let _ = store.tabs.write()[index].merge.undo(); },
                    "Undo"
                }
                button {
                    disabled: !snap.is_dirty,
                    onclick: move |_| save_tab(&mut store, index, false),
                    {t(lang, "Save")}
                }
                button {
                    onclick: move |_| {
                        let path = store.tabs.read().get(index)
                            .and_then(|tab| tab.right_path.as_ref())
                            .map(|p| p.display().to_string()).unwrap_or_default();
                        store.modal.set(Modal::SaveAs(index, path));
                    },
                    "Save As"
                }
            }
            button {
                title: "Reload both files from disk",
                onclick: move |_| {
                    let dirty = store.tabs.read().get(index)
                        .map(|t| t.merge.is_dirty()).unwrap_or(false);
                    if dirty {
                        store.modal.set(Modal::ConfirmReload(index));
                    } else {
                        reload_tab(&mut store, index);
                        store.notify(t(store.lang(), "Reloaded."));
                    }
                },
                "↺"
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
                    onclick: move |_| { store.tabs.write()[index].char_mode ^= true; },
                    {format!("Inline diff: {}", if snap.char_mode { "on" } else { "off" })}
                }
                button {
                    disabled: !snap.can_redo,
                    onclick: move |_| { let _ = store.tabs.write()[index].merge.redo(); },
                    "Redo"
                }
                button {
                    onclick: move |_| {
                        let mut tabs = store.tabs.write();
                        if let Some(tab) = tabs.get_mut(index) {
                            tab.diff_options.ignore_whitespace ^= true;
                            recompute_diff(tab);
                        }
                    },
                    {format!("Ignore whitespace: {}", if snap.ignore_whitespace { "on" } else { "off" })}
                }
            }
        }
    }
}

/// Move the focused changed hunk and scroll it into view.
pub fn move_focus(store: &mut Store, index: usize, delta: i32) {
    let hunk_id = {
        let mut tabs = store.tabs.write();
        let Some(tab) = tabs.get_mut(index) else { return };
        let ids: Vec<u64> = tab.merge.hunks().iter()
            .filter(|h| h.kind.is_change()).map(|h| h.hunk_id).collect();
        if ids.is_empty() { return }
        let next = ((tab.focused_change as i32 + delta).rem_euclid(ids.len() as i32)) as usize;
        tab.focused_change = next;
        ids[next]
    };
    spawn(async move {
        let _ = dioxus::document::eval(
            &format!("document.getElementById('h-{hunk_id}')?.scrollIntoView({{block:'nearest',behavior:'smooth'}});")
        ).await;
    });
}

pub fn save_tab(store: &mut Store, index: usize, force: bool) {
    let req = build_save_request(store, index, force, None);
    let Some(request) = req else { return };
    handle_save(store, index, save_text(&request));
}

pub fn save_as(store: &mut Store, index: usize, path: String) {
    let target = std::path::PathBuf::from(&path);
    let req = build_save_request(store, index, true, Some(target));
    let Some(request) = req else { return };
    let is_new_path = true;
    match save_text(&request) {
        Ok(outcome) => {
            let mut tabs = store.tabs.write();
            if let Some(tab) = tabs.get_mut(index) {
                tab.merge.mark_saved();
                if is_new_path {
                    tab.right_path = Some(request.target.clone());
                }
                tab.right_doc.fingerprint_at_load = Some(outcome.new_fingerprint);
            }
            drop(tabs);
            store.modal.set(Modal::None);
            store.notify(t(store.lang(), "Saved."));
        }
        Err(e) => store.notify(e.to_string()),
    }
}

fn build_save_request(
    store: &Store, index: usize, force: bool, override_target: Option<std::path::PathBuf>,
) -> Option<SaveRequest> {
    let tabs = store.tabs.read();
    let tab = tabs.get(index)?;
    if !tab.can_save { return None; }
    let target = override_target.or_else(|| tab.right_path.clone())?;
    let enc = tab.right_doc.text.as_ref()
        .map(|t| t.encoding.label.clone()).unwrap_or_else(|| "UTF-8".into());
    let expected = if force {
        forskscope_core::document::FileFingerprint::capture(&target, None).ok()
    } else {
        tab.right_doc.fingerprint_at_load
    };
    Some(SaveRequest { target, content: tab.merge.result_text(),
                       encoding_label: enc, expected_fingerprint: expected,
                       backup: BackupPolicy::SiblingBak })
}

fn handle_save(store: &mut Store, index: usize, result: Result<forskscope_core::save::SaveOutcome, CoreError>) {
    match result {
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
        Err(CoreError::Conflict { .. }) => store.modal.set(Modal::ConfirmOverwrite(index)),
        Err(e) => store.notify(e.to_string()),
    }
}

fn trunc(s: &str) -> String {
    if let Some(idx) = s.rfind('/').or_else(|| s.rfind('\\')) {
        let (parent, name) = s.split_at(idx + 1);
        if parent.len() > 24 { format!("…/{name}") } else { s.to_string() }
    } else { s.to_string() }
}
