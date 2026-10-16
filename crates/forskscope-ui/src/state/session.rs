//! Session persistence (RFC-035, RFC-076): save and restore open tabs across launches.

use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_core::persist::schema::session::runtime::{
    SessionRuntimeOutcome, SessionRuntimeResolution, resolve_and_commit,
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
    let active = *store.active.read();
    let result = save_session_if_allowed(
        *store.session_write_disabled.read(),
        &build_save_payload(&pairs, active),
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
///
/// F67: `active` is `store.active`'s value, an index into `pairs` — the same
/// index space `store.tabs` and `pairs` share, since `pairs` is built from
/// `store.tabs` in order. A dropped (incomplete) pair before the active one
/// shifts every later index, so `active_tab` in the payload is computed
/// against `saved`'s own position, not copied from `active` verbatim: the
/// file's index must mean "this many pairs into what's actually saved",
/// never "this many pairs into what was open".
pub fn build_save_payload(
    pairs: &[(Option<PathBuf>, Option<PathBuf>)],
    active: Option<usize>,
) -> PersistedSession {
    let mut saved: Vec<PersistedComparePair> = Vec::new();
    let mut active_tab = None;
    for (i, (l, r)) in pairs.iter().enumerate() {
        let (Some(left), Some(right)) = (l.clone(), r.clone()) else {
            continue;
        };
        if active == Some(i) {
            active_tab = Some(saved.len());
        }
        saved.push(PersistedComparePair { left, right });
    }
    PersistedSession {
        tabs: saved,
        active_tab,
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
///
/// RFC-091 §3: private mode reads nothing, so this never constructs a
/// repository when [`crate::startup_mode::is_private`] — a CLI-mode
/// launch under `--private` still opens its explicit pair (that is the
/// user's files, not ForskScope's own records), but nothing is restored
/// from a session file and nothing is read from one either.
pub fn resolve_session(store: &mut Store) -> (SessionRuntimeResolution, Option<Notice>) {
    if crate::startup_mode::is_private() {
        let resolution = SessionRuntimeResolution {
            value: PersistedSession::default(),
            write_disabled: true,
            outcome: SessionRuntimeOutcome::Fresh,
            raw_bytes: None,
        };
        store.session_write_disabled.set(true);
        return (resolution, None);
    }
    let resolution = load_session(&repository());
    store.session_write_disabled.set(resolution.write_disabled);
    let notice = recovery_notice(&resolution);
    (resolution, notice)
}

/// Opens each tab in `resolution.value.tabs` whose paths still exist
/// (silently skipping pairs where both sides are gone). Only called when no
/// CLI startup pair was given — see [`resolve_session`]'s doc for why
/// resolving and restoring are two separate steps.
///
/// F67: also restores which tab was active. `active_tab` is an index into
/// the *file's* `tabs` list, but a pair whose files are both gone is skipped
/// above, so the index actually opened can differ from the file's — tracked
/// here as `opened` increments only for a pair that was actually opened.
/// Falls back to the first restored tab (`Some(0)`) when `active_tab` is
/// absent, out of range, or names a pair that was skipped; `open_compare`
/// already leaves `store.active` on the *last* opened tab by its own side
/// effect, so this still has to run, even to confirm that fallback is right.
pub fn restore_tabs(store: &mut Store, resolution: &SessionRuntimeResolution) {
    let mut opened = 0usize;
    let mut active_idx = None;
    for (i, pair) in resolution.value.tabs.iter().enumerate() {
        if pair.left.exists() || pair.right.exists() {
            open_compare(store, pair.left.clone(), pair.right.clone());
            if resolution.value.active_tab == Some(i) {
                active_idx = Some(opened);
            }
            opened += 1;
        }
    }
    if opened > 0 {
        set_active_tab(store, Some(active_idx.unwrap_or(0)));
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
    set_active_tab(store, new_active);
}

/// F194 (RFC-087 §3a): the one place `store.active` — the file-tab index
/// `session.json`'s `active_tab` is derived from — is set. Every route that
/// changes it goes through here: a tab-bar click, `Ctrl+Tab` and the tab
/// list's own arrow keys (this release), and a close that selects a
/// neighbour (already a caller, above). Before this, the session was saved
/// only when a tab opened, closed or swapped sides — switching which tab
/// was active, with none of those, left the restored tab wrong on next
/// launch. `store.active_dir` is not routed through here: it is never read
/// by `build_save_payload`, and it is always set in the same breath as
/// `store.active` by every caller that changes it (never on its own, so
/// there is nothing it alone could leave unsaved) — see `activate_tab_at`.
pub(crate) fn set_active_tab(store: &mut Store, index: Option<usize>) {
    store.active.set(index);
    save_session(store);
}

/// Request closing a file comparison tab — the one path the × button,
/// `Ctrl+W` and `Delete` on a focused tab (RFC-087 §1) all go through, so
/// "the same path" is true by construction rather than by three call sites
/// agreeing to copy each other. A loading tab closes without asking
/// (RFC-065: it has no merge state yet to lose); a dirty one asks first via
/// `Modal::ConfirmClose`; otherwise it closes at once.
pub(crate) fn request_close_file_tab(store: &mut Store, index: usize) {
    let Some((is_loading, dirty)) = store.tabs.read().get(index).map(|tab| {
        (
            tab.state == crate::state::TabState::Loading,
            tab.can_save && tab.merge.is_dirty(),
        )
    }) else {
        return;
    };
    if is_loading {
        close_tab(store, index);
    } else if dirty {
        store.modal.set(Modal::ConfirmClose(index));
    } else {
        close_tab(store, index);
    }
}

#[cfg(test)]
mod tests;
