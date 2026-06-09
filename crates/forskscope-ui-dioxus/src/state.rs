//! Application UI state and the core <-> UI glue (RFC-003 §state ownership).
//!
//! The UI owns presentation state (active tab, focused hunk, modal). Product
//! truth — documents, diff, merge transactions, dirty state — lives in
//! `forskscope-core` objects held inside each tab. The UI never recomputes
//! merge results from rendered content.

use std::path::PathBuf;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use forskscope_core::diff::DiffDocument;
use forskscope_core::document::{LoadOptions, LoadedDocument, load_path};
use forskscope_core::file_kind::FileKind;
use forskscope_core::{DiffOptions, MergeSession, compute_diff};

/// UI theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    Dark,
    Light,
    Night,
}

impl Theme {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Dark => "theme-dark",
            Self::Light => "theme-light",
            Self::Night => "theme-night",
        }
    }
}

/// UI language (RFC-009 localization).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lang {
    En,
    Ja,
}

/// Persisted user settings (RFC-009, RFC-011). Stored as JSON via
/// `app-json-settings` under the OS config directory for `forskscope`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub language: Lang,
    pub diff_font_size: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            language: Lang::En,
            diff_font_size: 14,
        }
    }
}

/// Modal overlay state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None,
    Settings,
    /// External-modification conflict awaiting the user's choice.
    ConfirmOverwrite(usize),
}

/// One open comparison tab. Holds the canonical core objects for the pair.
#[derive(Clone)]
pub struct CompareTab {
    pub title: String,
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    /// Retained for reload / swap-sides workflows (RFC-006); the live diff
    /// and merge sessions are what drive rendering.
    #[allow(dead_code)]
    pub left_doc: LoadedDocument,
    pub right_doc: LoadedDocument,
    pub diff: DiffDocument,
    pub merge: MergeSession,
    /// Both sides are real, mergeable text — only then may we save.
    pub can_save: bool,
    pub char_mode: bool,
    pub focused_change: usize,
}

impl CompareTab {
    pub fn right_label(&self) -> String {
        side_label(&self.right_doc)
    }
}

fn side_label(doc: &LoadedDocument) -> String {
    doc.text
        .as_ref()
        .map(|t| t.encoding.label.clone())
        .unwrap_or_else(|| "—".into())
}

/// Shared, copyable handle to all UI signals.
#[derive(Clone, Copy)]
pub struct Store {
    pub tabs: Signal<Vec<CompareTab>>,
    /// `None` = explorer workspace active; `Some(i)` = diff tab `i`.
    pub active: Signal<Option<usize>>,
    pub settings: Signal<AppSettings>,
    pub left_pick: Signal<Option<PathBuf>>,
    pub right_pick: Signal<Option<PathBuf>>,
    pub modal: Signal<Modal>,
    pub toast: Signal<Option<String>>,
}

impl Store {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            tabs: Signal::new(Vec::new()),
            active: Signal::new(None),
            settings: Signal::new(settings),
            left_pick: Signal::new(None),
            right_pick: Signal::new(None),
            modal: Signal::new(Modal::None),
            toast: Signal::new(None),
        }
    }

    pub fn lang(&self) -> Lang {
        self.settings.read().language
    }

    pub fn notify(&mut self, message: impl Into<String>) {
        self.toast.set(Some(message.into()));
    }
}

/// Build a comparison and push it as a new active tab.
pub fn open_compare(store: &mut Store, left: PathBuf, right: PathBuf) {
    let options = LoadOptions {
        allow_missing: true,
    };
    let mut left_doc = match load_path(&left, options) {
        Ok(d) => d,
        Err(e) => return store.notify(format!("Left: {e}")),
    };
    let mut right_doc = match load_path(&right, options) {
        Ok(d) => d,
        Err(e) => return store.notify(format!("Right: {e}")),
    };

    if left_doc.kind == FileKind::ExcelXlsx && right_doc.kind == FileKind::ExcelXlsx {
        let (lt, rt) = forskscope_core::xlsx::derive_pair_text(&left, &right);
        left_doc.text = Some(lt);
        right_doc.text = Some(rt);
    }

    let diff = compute_diff(left_doc.diff_text(), right_doc.diff_text(), DiffOptions::default());
    let merge = MergeSession::from_diff(&diff);
    let can_save = left_doc.kind.is_mergeable_text() && right_doc.kind.is_mergeable_text();

    let title = tab_title(&left, &right);    let tab = CompareTab {
        title,
        left_path: Some(left),
        right_path: Some(right),
        left_doc,
        right_doc,
        diff,
        merge,
        can_save,
        char_mode: false,
        focused_change: 0,
    };
    let index = store.tabs.read().len();
    store.tabs.write().push(tab);
    store.active.set(Some(index));
}

fn tab_title(left: &std::path::Path, right: &std::path::Path) -> String {
    let l = left.file_name().map(|n| n.to_string_lossy().into_owned());
    let r = right.file_name().map(|n| n.to_string_lossy().into_owned());
    match (l, r) {
        (Some(a), Some(b)) if a == b => a,
        (Some(a), Some(b)) => format!("{a} ↔ {b}"),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => "comparison".into(),
    }
}
