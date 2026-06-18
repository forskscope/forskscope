//! Application UI state and the core <-> UI glue (RFC-003 §state ownership).
//!
//! Submodules:
//! - `settings`  — `AppSettings`, `DiffProfile`, theme/lang/font types
//! - `types`     — `BatchResultSpec`, `DirOp`
//! - `tab`       — `CompareTab`, `TabState`, `recompute_diff`, `swap_sides`
//! - `compare`   — `open_compare`, `reload_tab`, `load_and_diff`, dir tabs
//! - `session`   — `SessionState`, `save_session`, `restore_session`, `close_tab`
//! - `profile`   — `add_profile`, `remove_profile`

pub mod settings;
pub mod types;
pub mod tab;
pub mod compare;
pub mod session;
pub mod profile;

pub use settings::{AppSettings, BatchCopySpec, DiffAlgorithmSetting, DiffFontFamily, Lang, Theme};
pub use types::{BatchResultSpec, DirOp};
pub use tab::{CompareTab, TabState, recompute_diff, swap_sides};
pub use compare::{open_compare, reload_tab, open_dir_compare, close_dir_tab};
pub use session::{SessionState, save_session, restore_session, close_tab};
pub use profile::{add_profile, remove_profile};

use dioxus::prelude::*;
use std::path::PathBuf;

// ── Modal variants ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum Modal {
    None, Settings,
    ConfirmOverwrite(usize), SaveAs(usize, String),
    ConfirmReload(usize),    ConfirmSwap(usize),
    ConfirmDirOp(DirOp),     ConfirmClose(usize),
    ConfirmBatchCopy(BatchCopySpec),
    BatchResult(BatchResultSpec),
    About, KeyboardRef,
}

// ── Toast / notice ────────────────────────────────────────────────────────────

/// Severity of a user-facing notice / toast (RFC-063 C5).
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NoticeSeverity { Success, Info, Warning, Error }

/// A user-facing notice shown as a toast.
#[derive(Clone, PartialEq, Debug)]
pub struct Notice {
    pub message:  String,
    pub severity: NoticeSeverity,
}

impl Notice {
    pub fn success(msg: impl Into<String>) -> Self { Self { message: msg.into(), severity: NoticeSeverity::Success } }
    #[allow(dead_code)]
    pub fn info(msg: impl Into<String>)    -> Self { Self { message: msg.into(), severity: NoticeSeverity::Info } }
    pub fn warning(msg: impl Into<String>) -> Self { Self { message: msg.into(), severity: NoticeSeverity::Warning } }
    pub fn error(msg: impl Into<String>)   -> Self { Self { message: msg.into(), severity: NoticeSeverity::Error } }
    pub fn auto_dismiss_ms(&self) -> Option<u64> {
        match self.severity {
            NoticeSeverity::Success => Some(3500),
            NoticeSeverity::Info    => Some(5000),
            NoticeSeverity::Warning | NoticeSeverity::Error => None,
        }
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Application-wide reactive state. All fields are `Signal<T>` (`Copy + Clone`),
/// so `Store` itself is `Clone + Copy` — required by `use_context::<Store>()`.
#[derive(Clone, Copy)]
pub struct Store {
    pub tabs:       Signal<Vec<CompareTab>>,
    pub active:     Signal<Option<usize>>,
    pub dir_tabs:   Signal<Vec<(PathBuf, PathBuf)>>,
    pub active_dir: Signal<Option<usize>>,
    pub settings:   Signal<AppSettings>,
    pub left_pick:  Signal<Option<PathBuf>>,
    pub right_pick: Signal<Option<PathBuf>>,
    pub modal:      Signal<Modal>,
    pub toast:      Signal<Option<Notice>>,
}

impl Store {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            tabs: Signal::new(Vec::new()), active: Signal::new(None),
            dir_tabs: Signal::new(Vec::new()), active_dir: Signal::new(None),
            settings: Signal::new(settings),
            left_pick: Signal::new(None), right_pick: Signal::new(None),
            modal: Signal::new(Modal::None), toast: Signal::new(None),
        }
    }
    pub fn lang(&self) -> Lang { self.settings.read().language }
    pub fn notify(&mut self, msg: impl Into<String>)         { self.toast.set(Some(Notice::error(msg))); }
    pub fn notify_success(&mut self, msg: impl Into<String>) { self.toast.set(Some(Notice::success(msg))); }
    #[allow(dead_code)]
    pub fn notify_info(&mut self, msg: impl Into<String>)    { self.toast.set(Some(Notice::info(msg))); }
    pub fn notify_warning(&mut self, msg: impl Into<String>) { self.toast.set(Some(Notice::warning(msg))); }
}
