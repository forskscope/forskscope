//! Session persistence (RFC-035, RFC-076): save and restore open tabs across launches.

use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_core::persist::v2::session::runtime::{
    SessionRuntimeResolution, resolve_and_commit,
};
use forskscope_core::persist::v2::session::{
    PersistedComparePairV2, PersistedSessionV2, SessionRepository,
};
use forskscope_ui_logic::SessionRecoveryView;

use crate::state::compare::open_compare;
use crate::state::{Notice, Store, config_file_path};

fn repository() -> SessionRepository {
    SessionRepository::new(config_file_path("session.json"))
}

/// Persist the current open tabs for restoration on next launch. A no-op
/// when `store.session_write_disabled` is set — a future/corrupt/unwritable
/// source this run could not establish is safe to overwrite (RFC-076
/// "persistence_write_disabled").
pub fn save_session(store: &Store) {
    if *store.session_write_disabled.read() {
        return;
    }
    let pairs: Vec<(Option<PathBuf>, Option<PathBuf>)> = store
        .tabs
        .read()
        .iter()
        .map(|tab| (tab.left_path.clone(), tab.right_path.clone()))
        .collect();
    persist_session(&build_save_payload(&pairs), &repository());
}

/// The Store-independent half of [`save_session`]: what gets written. Split
/// out so a test can exercise it without needing a running Dioxus runtime or
/// constructing a full `CompareTab` — only the path pair matters here.
/// Drops any pair where either side is `None`.
pub fn build_save_payload(pairs: &[(Option<PathBuf>, Option<PathBuf>)]) -> PersistedSessionV2 {
    let saved: Vec<PersistedComparePairV2> = pairs
        .iter()
        .filter_map(|(l, r)| {
            let left = l.clone()?;
            let right = r.clone()?;
            Some(PersistedComparePairV2 { left, right })
        })
        .collect();
    PersistedSessionV2 {
        tabs: saved,
        active_tab: None,
        explorer_roots: None,
    }
}

/// Writes `payload` via `repo` — the exact repository call `save_session`
/// makes, exposed for direct testing (handoff §6: "targeted tests proving
/// the actual UI startup and save functions use the new repositories").
pub fn persist_session(payload: &PersistedSessionV2, repo: &SessionRepository) {
    let _ = repo.save(payload);
}

/// Loads the last-saved session via the RFC-076 repository, durably
/// committing any legacy migration, opens each tab whose paths still exist
/// (silently skipping pairs where both sides are gone), and sets
/// `store.session_write_disabled` from the resolution. Returns a one-time
/// startup notice, if any — see `crate::ui::view::settings::recovery_notice`.
pub fn restore_session(store: &mut Store) -> Option<Notice> {
    let resolution = load_session(&repository());
    store.session_write_disabled.set(resolution.write_disabled);
    for pair in &resolution.value.tabs {
        if pair.left.exists() || pair.right.exists() {
            open_compare(store, pair.left.clone(), pair.right.clone());
        }
    }
    recovery_notice(&resolution)
}

/// The repository-explicit half of [`restore_session`], exposed for direct
/// testing.
pub fn load_session(repo: &SessionRepository) -> SessionRuntimeResolution {
    resolve_and_commit(repo)
}

/// Session mirror of `crate::ui::view::settings::recovery_notice`.
pub fn recovery_notice(resolution: &SessionRuntimeResolution) -> Option<Notice> {
    let view = SessionRecoveryView::from_resolution(resolution);
    if let Some(notice) = view.migration_notice {
        return Some(Notice::success(notice.message));
    }
    if let Some(dialog) = view.dialog {
        return Some(Notice::error(dialog.body));
    }
    None
}

/// Close the tab at `index`, adjusting the active index so another tab
/// (or the Explorer) remains visible.
pub fn close_tab(store: &mut Store, index: usize) {
    store.tabs.write().remove(index);
    let len = store.tabs.read().len();
    let new_active = if len == 0 {
        None
    } else {
        Some(index.min(len - 1))
    };
    store.active.set(new_active);
    save_session(store);
}

#[cfg(test)]
mod tests;
