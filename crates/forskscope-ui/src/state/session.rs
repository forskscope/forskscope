//! Session persistence (RFC-035, RFC-076): save and restore open tabs across launches.

use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_core::persist::schema::session::runtime::{
    SessionRuntimeResolution, resolve_and_commit,
};
use forskscope_core::persist::schema::session::{
    PersistedComparePair, PersistedSession, SessionRepository,
};
use forskscope_core::persist::schema::{PersistenceCommitError, PersistenceIoError};
use forskscope_ui_logic::SessionRecoveryView;

use crate::state::compare::open_compare;
use crate::state::{Modal, Notice, Store, config_file_path};

fn repository() -> SessionRepository {
    SessionRepository::new(config_file_path("session.json"))
}

/// Persist the current open tabs for restoration on next launch. A no-op
/// when `store.session_write_disabled` is set — a future/corrupt/unwritable
/// source this run could not establish is safe to overwrite (RFC-076
/// "persistence_write_disabled"). F62: a write failure is shown to the user
/// as an error toast rather than discarded — the same treatment whether the
/// call came from an explicit action (`close_tab`) or the startup/tab-change
/// reactive effect, since either way the user's session may now not survive
/// a restart and silence about that is exactly F62's defect.
pub fn save_session(store: &Store) {
    let pairs: Vec<(Option<PathBuf>, Option<PathBuf>)> = store
        .tabs
        .read()
        .iter()
        .map(|tab| (tab.left_path.clone(), tab.right_path.clone()))
        .collect();
    let result = save_session_if_allowed(
        *store.session_write_disabled.read(),
        &build_save_payload(&pairs),
        &repository(),
    );
    if let Err(e) = result {
        let mut store = *store;
        store.notify(format!("Could not save session: {e}"));
    }
}

/// The write-disable gate itself, exposed for direct testing (review 041
/// C1): asserts that a `write_disabled` source is never written to, without
/// needing a `Store`/Dioxus runtime to exercise `save_session`'s call site.
pub fn save_session_if_allowed(
    write_disabled: bool,
    payload: &PersistedSession,
    repo: &SessionRepository,
) -> Result<(), PersistenceIoError> {
    if write_disabled {
        return Ok(());
    }
    persist_session(payload, repo)
}

/// The Store-independent half of [`save_session`]: what gets written. Split
/// out so a test can exercise it without needing a running Dioxus runtime or
/// constructing a full `CompareTab` — only the path pair matters here.
/// Drops any pair where either side is `None`.
pub fn build_save_payload(pairs: &[(Option<PathBuf>, Option<PathBuf>)]) -> PersistedSession {
    let saved: Vec<PersistedComparePair> = pairs
        .iter()
        .filter_map(|(l, r)| {
            let left = l.clone()?;
            let right = r.clone()?;
            Some(PersistedComparePair { left, right })
        })
        .collect();
    PersistedSession {
        tabs: saved,
        active_tab: None,
        explorer_roots: None,
    }
}

/// Writes `payload` via `repo` — the exact repository call `save_session`
/// makes, exposed for direct testing (handoff §6: "targeted tests proving
/// the actual UI startup and save functions use the new repositories").
/// F62: returns the write's `Result` instead of discarding it — `Result`
/// is `#[must_use]` in `std`, so a caller that ignores this now gets a
/// compiler warning, not silence.
pub fn persist_session(
    payload: &PersistedSession,
    repo: &SessionRepository,
) -> Result<(), PersistenceIoError> {
    repo.save(payload)
}

/// Loads the last-saved session via the RFC-076 repository, durably
/// committing any legacy migration, and sets `store.session_write_disabled`
/// from the resolution. Returns the resolution (for [`restore_tabs`])
/// alongside a one-time startup notice, if any — see
/// `crate::ui::view::settings::recovery_notice`.
///
/// Review 041 C1: this must run unconditionally at startup, independent of
/// whether tabs actually get restored from it — a CLI-mode launch
/// (`forskscope left right`) never restores tabs, but still needs
/// `session_write_disabled` set before its own `open_compare` triggers a
/// `save_session`, or a future/corrupt session file is silently overwritten.
pub fn resolve_session(store: &mut Store) -> (SessionRuntimeResolution, Option<Notice>) {
    let resolution = load_session(&repository());
    store.session_write_disabled.set(resolution.write_disabled);
    let notice = recovery_notice(&resolution);
    (resolution, notice)
}

/// Opens each tab in `resolution.value.tabs` whose paths still exist
/// (silently skipping pairs where both sides are gone). Only called when no
/// CLI startup pair was given — see [`resolve_session`]'s doc for why
/// resolving and restoring are two separate steps.
pub fn restore_tabs(store: &mut Store, resolution: &SessionRuntimeResolution) {
    for pair in &resolution.value.tabs {
        if pair.left.exists() || pair.right.exists() {
            open_compare(store, pair.left.clone(), pair.right.clone());
        }
    }
}

/// The repository-explicit half of [`restore_session`], exposed for direct
/// testing.
pub fn load_session(repo: &SessionRepository) -> SessionRuntimeResolution {
    resolve_and_commit(repo)
}

/// Session mirror of `crate::ui::view::settings::recovery_notice`.
pub fn recovery_notice(resolution: &SessionRuntimeResolution) -> Option<Notice> {
    let view = SessionRecoveryView::from_resolution(resolution);
    view.migration_notice.map(|n| Notice::success(n.message))
}

/// Session mirror of `crate::ui::view::settings::recovery_modal`.
pub fn recovery_modal(resolution: &SessionRuntimeResolution) -> Option<Modal> {
    let view = SessionRecoveryView::from_resolution(resolution);
    view.dialog
        .is_some()
        .then(|| Modal::SessionRecovery(resolution.clone()))
}

/// Session mirror of `crate::ui::view::settings::reset_settings_with_backup`.
pub fn reset_session_with_backup(
    value: &PersistedSession,
    original_bytes: &[u8],
) -> Result<(), PersistenceCommitError> {
    reset_session(value, original_bytes, &repository())
}

/// The repository-explicit half of [`reset_session_with_backup`], exposed
/// for direct testing (same split as [`load_session`]).
pub fn reset_session(
    value: &PersistedSession,
    original_bytes: &[u8],
    repo: &SessionRepository,
) -> Result<(), PersistenceCommitError> {
    repo.reset_with_backup(value, original_bytes).map(|_| ())
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
