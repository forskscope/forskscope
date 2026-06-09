//! Diff workspace action functions (pure state mutations, RFC-003 §state ownership).
//! These are free functions used by `diff.rs` components and `app.rs` keyboard handlers.

use dioxus::prelude::*;

use forskscope_core::save::{BackupPolicy, SaveRequest, save_text};
use forskscope_core::CoreError;

use crate::i18n::t;
use crate::state::{Modal, Store};

// ─── Public action functions ──────────────────────────────────────────────────

/// Apply the focused changed hunk and auto-advance to the next one.
pub fn apply_focused_hunk(store: &mut Store, index: usize) {
    let hunk_id = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else { return };
        if !tab.can_save { return }
        let ids: Vec<u64> = tab.merge.hunks().iter()
            .filter(|h| h.is_pending_change())
            .map(|h| h.hunk_id)
            .collect();
        ids.get(tab.focused_change).copied()
    };
    if let Some(id) = hunk_id {
        let _ = store.tabs.write()
            .get_mut(index).map(|t| t.merge.apply_left_to_right(id));
        // Advance to the next pending change so the user can keep pressing Enter.
        move_focus(store, index, 1);
    }
}

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
    let req = build_request(store, index, force, None);
    let Some(request) = req else { return };
    handle_result(store, index, save_text(&request));
}

pub fn save_as(store: &mut Store, index: usize, path: String) {
    let target = std::path::PathBuf::from(&path);
    let req = build_request(store, index, true, Some(target.clone()));
    let Some(request) = req else { return };
    match save_text(&request) {
        Ok(outcome) => {
            let mut tabs = store.tabs.write();
            if let Some(tab) = tabs.get_mut(index) {
                tab.merge.mark_saved();
                tab.right_path = Some(target);
                tab.right_doc.fingerprint_at_load = Some(outcome.new_fingerprint);
            }
            drop(tabs);
            store.modal.set(Modal::None);
            store.notify(t(store.lang(), "Saved."));
        }
        Err(e) => store.notify(e.to_string()),
    }
}

fn build_request(
    store: &Store, index: usize, force: bool, target: Option<std::path::PathBuf>,
) -> Option<SaveRequest> {
    let tabs = store.tabs.read();
    let tab = tabs.get(index)?;
    if !tab.can_save { return None; }
    let tgt = target.or_else(|| tab.right_path.clone())?;
    let enc = tab.right_doc.text.as_ref()
        .map(|t| t.encoding.label.clone()).unwrap_or_else(|| "UTF-8".into());
    let expected = if force {
        forskscope_core::document::FileFingerprint::capture(&tgt, None).ok()
    } else {
        tab.right_doc.fingerprint_at_load
    };
    Some(SaveRequest { target: tgt, content: tab.merge.result_text(),
                       encoding_label: enc, expected_fingerprint: expected,
                       backup: BackupPolicy::SiblingBak })
}

fn handle_result(store: &mut Store, index: usize, result: Result<forskscope_core::save::SaveOutcome, CoreError>) {
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

pub(crate) fn trunc(s: &str) -> String {
    if let Some(i) = s.rfind('/').or_else(|| s.rfind('\\')) {
        let (parent, name) = s.split_at(i + 1);
        if parent.len() > 24 { return format!("…/{name}"); }
    }
    s.to_string()
}

pub(crate) fn algo_val(a: forskscope_core::DiffAlgorithm) -> &'static str {
    use forskscope_core::DiffAlgorithm;
    match a { DiffAlgorithm::Patience => "patience", DiffAlgorithm::Histogram => "histogram", _ => "myers" }
}
