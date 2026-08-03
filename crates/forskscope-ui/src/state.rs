//! Application UI state and the core <-> UI glue (RFC-003 §state ownership).
//!
//! Submodules:
//! - `settings`  — `AppSettings`, `DiffProfile`, theme/lang/font types
//! - `types`     — `BatchResultSpec`, `DirOp`
//! - `tab`       — `CompareTab`, `TabState`, `recompute_diff`, `swap_sides`
//! - `compare`   — `open_compare`, `reload_tab`, `load_and_diff`, dir tabs
//! - `session`   — `save_session`, `restore_session`, `close_tab` (RFC-076 repository-backed)
//! - `profile`   — `add_profile`, `remove_profile`

pub mod compare;
pub mod profile;
pub mod session;
pub mod settings;
pub mod tab;
pub mod types;

pub use compare::{close_dir_tab, open_compare, open_dir_compare, reload_tab};
pub use profile::{add_profile, remove_profile};
pub use session::{close_tab, restore_session, save_session};
pub use settings::{AppSettings, BatchCopySpec, DiffAlgorithmSetting, DiffFontFamily, Lang, Theme};
pub use tab::{CompareTab, TabState, recompute_diff, swap_sides};
pub use types::{BatchResultSpec, DirOp};

use dioxus::prelude::*;
use forskscope_core::persist::v2::settings::PersistedSettingsV2;
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
    ConfirmOverwrite(usize),
    SaveAs(usize, String),
    ConfirmReload(usize),
    ConfirmSwap(usize),
    ConfirmDirOp(DirOp),
    ConfirmClose(usize),
    ConfirmBatchCopy(BatchCopySpec),
    BatchResult(BatchResultSpec),
    About,
    KeyboardRef,
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
    pub settings_v2_base: Signal<PersistedSettingsV2>,
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
}

impl Store {
    /// Create a new `Store` with all signals owned at `ScopeId::ROOT`.
    ///
    /// Signals must be rooted at the application root scope so that tasks
    /// spawned via `spawn_forever` (which runs at `ScopeId(0)`) can write to
    /// them without triggering the "copy value hoisted" warning.
    pub fn new(
        settings: AppSettings,
        settings_v2_base: PersistedSettingsV2,
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
    #[allow(dead_code)]
    pub fn notify_warning(&mut self, msg: impl Into<String>) {
        self.toast.set(Some(Notice::warning(msg)));
    }
}
