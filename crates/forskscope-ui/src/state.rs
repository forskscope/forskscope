//! Application UI state and the core <-> UI glue (RFC-003 §state ownership).
//!
//! Submodules:
//! - `settings`  — `AppSettings`, `DiffProfile`, theme/lang/font types
//! - `types`     — `BatchResultSpec`, `DirOp`
//! - `tab`       — `CompareTab`, `TabState`, `swap_sides`, `change_diff_options`, `set_diff_options`
//! - `compare`   — `open_compare`, `reload_tab`, `load_and_diff`, dir tabs
//! - `session`   — `save_session`, `restore_session`, `close_tab` (RFC-076 repository-backed)
//! - `profile`   — `add_profile`, `remove_profile`

pub mod compare;
pub mod profile;
pub mod session;
pub mod settings;
pub mod tab;
pub mod types;

pub use compare::{
    close_dir_tab, open_compare, open_compare_request, open_dir_compare, reload_tab,
};
pub(crate) use compare::{open_compare_request_with_options, reload_tab_with_options};
pub use profile::{add_profile, remove_profile};
pub use session::{close_tab, resolve_session, restore_tabs, save_session};
pub use settings::{AppSettings, BatchCopySpec, DiffAlgorithmSetting, DiffFontFamily, Lang, Theme};
pub use tab::{CompareTab, TabState, change_diff_options, set_diff_options, swap_sides};
pub use types::{BatchResultSpec, DirOp, LargeLoadPrompt, LargeLoadTarget};

use dioxus::prelude::*;
use forskscope_core::persist::schema::session::runtime::SessionRuntimeResolution;
use forskscope_core::persist::schema::settings::PersistedSettings;
use forskscope_core::persist::schema::settings::runtime::SettingsRuntimeResolution;
use forskscope_ui_logic::{CompareTabId, CompareTabIdAllocator, LoadIdentityError};
use std::path::PathBuf;

/// Resolves `settings.json`/`session.json`'s explicit path (RFC-076: the
/// repositories never resolve a platform config directory themselves — that
/// stays a UI/infrastructure concern). Falls back to the current directory
/// if the platform config directory cannot be determined, matching
/// `app-json-settings`'s previous behavior.
pub(crate) fn config_file_path(file_name: &str) -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("forskscope")
        .join(file_name)
}

// ── Modal variants ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum Modal {
    None,
    Settings,
    /// A save conflicted. `target` is the exact path that conflicting save
    /// attempted — the tab's own save target for an ordinary `save_tab`
    /// conflict, or the Save As destination for a Save As conflict.
    /// Confirming must overwrite this exact path, never silently fall back
    /// to whatever the tab's current save target happens to be (RFC-077,
    /// review 048 C1).
    ConfirmOverwrite(usize, std::path::PathBuf),
    SaveAs(usize, String),
    /// Save As chose a destination that already exists on disk (RFC-077
    /// test design: "select an existing Save As destination: overwrite
    /// confirmation is required"). Distinct from `ConfirmOverwrite`: this
    /// fires *before* any write is attempted, from a plain existence check
    /// in `SaveAsModal` — not from a `CoreError::Conflict` a save produced.
    /// Confirming proceeds to the real `save_as` call, which still runs its
    /// own fresh precondition check (a true race between this confirmation
    /// and the write still surfaces as `ConfirmOverwrite`).
    ConfirmSaveAsOverwrite(usize, std::path::PathBuf),
    ConfirmReload(usize),
    ConfirmSwap(usize),
    /// A diff-option toggle (ignore whitespace/case, algorithm) would
    /// discard applied merge work and the undo/redo stack — `recompute_diff`
    /// rebuilds `MergeSession` from scratch (F40). `options` is the new
    /// value to install on confirm, computed at click time so the toolbar
    /// doesn't need a second enum describing which control was used. Same
    /// hazard class as `ConfirmSwap`; this does not implement RFC-015 §8
    /// rule 4 (see `change_diff_options`'s doc comment and RFC-015's
    /// recorded gap).
    ConfirmDiffOptionChange(usize, forskscope_core::DiffOptions),
    ConfirmDirOp(DirOp),
    ConfirmClose(usize),
    ConfirmBatchCopy(BatchCopySpec),
    BatchResult(BatchResultSpec),
    About,
    KeyboardRef,
    /// RFC-076 patch 6: a blocking settings-recovery dialog (future-version,
    /// corrupt, or a failed migration commit) — the resolution the dialog
    /// renders and acts on. Never dismissible by Escape (see `app.rs`).
    SettingsRecovery(SettingsRuntimeResolution),
    /// Session mirror of [`Self::SettingsRecovery`].
    SessionRecovery(SessionRuntimeResolution),
    /// F84: the file pair is large enough to require confirmation before
    /// loading (`LoadGuard::ConfirmPrompt`) — nothing has been loaded yet.
    /// Confirming resumes `target` with `opts` already suppressed as the
    /// guard demands; cancelling discards the prompt and loads nothing.
    ConfirmLargeLoad(LargeLoadPrompt),
    /// F52: a non-conflict save failure (`diff_actions::handle_result`'s
    /// `Err(e)` arm, excluding `CoreError::Conflict` — that stays
    /// `ConfirmOverwrite`, RFC-077/review 048 C1). `target` is the exact
    /// path the failed save attempted; `view` is the recovery dialog to
    /// render.
    SaveError(
        usize,
        std::path::PathBuf,
        forskscope_ui_logic::SaveErrorView,
    ),
}

// ── Toast / notice ────────────────────────────────────────────────────────────

/// Severity of a user-facing notice / toast (RFC-063 C5).
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NoticeSeverity {
    Success,
    Info,
    Warning,
    Error,
}

/// A user-facing notice shown as a toast.
#[derive(Clone, PartialEq, Debug)]
pub struct Notice {
    pub message: String,
    pub severity: NoticeSeverity,
}

impl Notice {
    pub fn success(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            severity: NoticeSeverity::Success,
        }
    }
    #[allow(dead_code)]
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            severity: NoticeSeverity::Info,
        }
    }
    #[allow(dead_code)]
    pub fn warning(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            severity: NoticeSeverity::Warning,
        }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            severity: NoticeSeverity::Error,
        }
    }
    pub fn auto_dismiss_ms(&self) -> Option<u64> {
        match self.severity {
            NoticeSeverity::Success => Some(3500),
            NoticeSeverity::Info => Some(5000),
            NoticeSeverity::Warning | NoticeSeverity::Error => None,
        }
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Application-wide reactive state. All fields are `Signal<T>` (`Copy + Clone`),
/// so `Store` itself is `Clone + Copy` — required by `use_context::<Store>()`.
#[derive(Clone, Copy)]
pub struct Store {
    pub tabs: Signal<Vec<CompareTab>>,
    compare_tab_ids: Signal<CompareTabIdAllocator>,
    pub active: Signal<Option<usize>>,
    pub dir_tabs: Signal<Vec<(PathBuf, PathBuf)>>,
    pub active_dir: Signal<Option<usize>>,
    pub settings: Signal<AppSettings>,
    /// The last-resolved canonical settings (RFC-076), cached so `persist()`
    /// can merge UI-editable changes onto it without resetting fields the UI
    /// has no control over. See `AppSettings::merge_into_v2`.
    pub settings_v2_base: Signal<PersistedSettings>,
    /// `true` when the settings file is a future/corrupt/unwritable source
    /// this run could not establish is safe to overwrite — `persist()`
    /// becomes a no-op while this is set (RFC-076 "persistence_write_disabled").
    pub settings_write_disabled: Signal<bool>,
    /// Session mirror of [`Self::settings_write_disabled`].
    pub session_write_disabled: Signal<bool>,
    pub left_pick: Signal<Option<PathBuf>>,
    pub right_pick: Signal<Option<PathBuf>>,
    pub modal: Signal<Modal>,
    pub toast: Signal<Option<Notice>>,
    /// RFC-076 patch 6 / F28b: recovery dialogs still waiting to be shown.
    /// Settings and session resolve independently and either (or both) can
    /// need a blocking dialog on the same launch; queuing rather than
    /// dropping the second is what keeps both visible in sequence. See
    /// [`advance_recovery_queue`].
    pub pending_recovery: Signal<Vec<Modal>>,
}

impl Store {
    /// Create a new `Store` with all signals owned at `ScopeId::ROOT`.
    ///
    /// Signals must be rooted at the application root scope so that tasks
    /// spawned via `spawn_forever` (which runs at `ScopeId(0)`) can write to
    /// them without triggering the "copy value hoisted" warning.
    pub fn new(
        settings: AppSettings,
        settings_v2_base: PersistedSettings,
        settings_write_disabled: bool,
    ) -> Self {
        Self {
            tabs: Signal::new_in_scope(Vec::new(), ScopeId::ROOT),
            compare_tab_ids: Signal::new_in_scope(CompareTabIdAllocator::new(), ScopeId::ROOT),
            active: Signal::new_in_scope(None, ScopeId::ROOT),
            dir_tabs: Signal::new_in_scope(Vec::new(), ScopeId::ROOT),
            active_dir: Signal::new_in_scope(None, ScopeId::ROOT),
            settings: Signal::new_in_scope(settings, ScopeId::ROOT),
            settings_v2_base: Signal::new_in_scope(settings_v2_base, ScopeId::ROOT),
            settings_write_disabled: Signal::new_in_scope(settings_write_disabled, ScopeId::ROOT),
            session_write_disabled: Signal::new_in_scope(false, ScopeId::ROOT),
            left_pick: Signal::new_in_scope(None, ScopeId::ROOT),
            right_pick: Signal::new_in_scope(None, ScopeId::ROOT),
            modal: Signal::new_in_scope(Modal::None, ScopeId::ROOT),
            toast: Signal::new_in_scope(None, ScopeId::ROOT),
            pending_recovery: Signal::new_in_scope(Vec::new(), ScopeId::ROOT),
        }
    }
    pub fn lang(&self) -> Lang {
        self.settings.read().language
    }
    pub(crate) fn allocate_compare_tab_id(&mut self) -> Result<CompareTabId, LoadIdentityError> {
        self.compare_tab_ids.write().allocate()
    }
    pub fn notify(&mut self, msg: impl Into<String>) {
        self.toast.set(Some(Notice::error(msg)));
    }
    pub fn notify_success(&mut self, msg: impl Into<String>) {
        self.toast.set(Some(Notice::success(msg)));
    }
    #[allow(dead_code)]
    pub fn notify_info(&mut self, msg: impl Into<String>) {
        self.toast.set(Some(Notice::info(msg)));
    }
    pub fn notify_warning(&mut self, msg: impl Into<String>) {
        self.toast.set(Some(Notice::warning(msg)));
    }
}

/// Dismisses the current modal and shows the next queued recovery dialog, if
/// any (RFC-076 patch 6 / F28b). Every recovery-dialog action handler calls
/// this instead of `store.modal.set(Modal::None)` directly, so a session
/// dialog queued behind a settings one is never silently dropped.
pub fn advance_recovery_queue(store: &mut Store) {
    let next = if store.pending_recovery.read().is_empty() {
        None
    } else {
        Some(store.pending_recovery.write().remove(0))
    };
    store.modal.set(next.unwrap_or(Modal::None));
}

/// Test-only construction of a real, usable [`Store`] (F36). `Store::new`
/// requires a live Dioxus runtime — `Signal::new_in_scope` panics without
/// one — which a bare `#[test]` fn doesn't have. A headless
/// `dioxus_core::VirtualDom` provides that runtime with no renderer, no
/// WebView, no GTK: its root component runs inside a real scope, and the
/// `Store` constructed there stays fully readable/writable after
/// `rebuild_in_place()` returns, as long as `vdom` stays alive — which it
/// does for the duration of `f`.
///
/// This closes the gap for testing `Store`-mutating logic directly: call
/// the action function under test against a real `Store` and assert on the
/// resulting `Signal` values, the same as any other unit test. It does not
/// touch rendering, event dispatch, or visual correctness — those stay
/// outside this helper's scope (F34's territory, not F36's).
///
/// Runs `f` inside the `VirtualDom`'s own runtime context
/// (`VirtualDom::in_runtime`), not just during the initial
/// `rebuild_in_place()` — a function under test may itself need
/// `Runtime::current()` to succeed (e.g. anything that calls `spawn`/
/// `spawn_forever`, confirmed while testing `open_compare_request` for
/// F61: without this, task-spawning code panics with "Components run in
/// the Dioxus runtime" once outside the bare `rebuild_in_place()` scope).
#[cfg(test)]
pub(crate) fn with_test_store<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    use std::cell::RefCell;

    thread_local! {
        static CAPTURED: RefCell<Option<Store>> = const { RefCell::new(None) };
    }

    fn root() -> Element {
        let store = Store::new(AppSettings::default(), Default::default(), false);
        CAPTURED.with(|c| *c.borrow_mut() = Some(store));
        rsx! {}
    }

    let mut vdom = VirtualDom::new(root);
    vdom.rebuild_in_place();
    let mut store = CAPTURED
        .with(|c| c.borrow_mut().take())
        .expect("root() must have run synchronously during rebuild_in_place()");
    let result = vdom.in_runtime(|| f(&mut store));
    drop(vdom);
    result
}

#[cfg(test)]
mod tests {
    use super::with_test_store;
    use dioxus::prelude::ReadableExt;

    #[test]
    fn with_test_store_yields_a_real_usable_store() {
        with_test_store(|store| {
            store.notify_success("hi");
            assert_eq!(
                store.toast.read().as_ref().map(|n| n.message.clone()),
                Some("hi".to_string())
            );
        });
    }
}
