//! Application UI state and the core <-> UI glue (RFC-003 §state ownership).

use std::path::PathBuf;

use app_json_settings::ConfigManager;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use forskscope_core::diff::DiffDocument;
use forskscope_core::document::{LoadOptions, LoadedDocument, load_path};
use forskscope_core::file_kind::FileKind;
use forskscope_core::{DiffOptions, MergeSession, compute_diff};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme { Dark, Light, Night }

impl Theme {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Dark  => "theme-dark",
            Self::Light => "theme-light",
            Self::Night => "theme-night",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lang { En, Ja }

// Re-export for UI use without depending on the core type directly.
pub use forskscope_core::DiffAlgorithm;

/// A named preset for diff options — stored in settings, applied when
/// opening new comparisons (RFC-009 compare profiles).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffProfile {
    pub name: String,
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    pub algorithm: DiffAlgorithmSetting,
    /// Built-in profiles ship with the app and cannot be deleted.
    #[serde(default)]
    pub built_in: bool,
}

/// Serialisable wrapper around `DiffAlgorithm` for profile persistence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DiffAlgorithmSetting { #[default] Myers, Patience, Histogram }

impl DiffProfile {
    pub fn to_diff_options(&self) -> DiffOptions {
        let algo = match self.algorithm {
            DiffAlgorithmSetting::Myers     => DiffAlgorithm::Myers,
            DiffAlgorithmSetting::Patience  => DiffAlgorithm::Patience,
            DiffAlgorithmSetting::Histogram => DiffAlgorithm::Histogram,
        };
        DiffOptions {
            ignore_whitespace: self.ignore_whitespace,
            ignore_case:       self.ignore_case,
            algorithm:         algo,
            ..DiffOptions::default()
        }
    }
}

fn default_profiles() -> Vec<DiffProfile> {
    vec![
        DiffProfile { name: "Exact (default)".into(),   ignore_whitespace: false, ignore_case: false, algorithm: DiffAlgorithmSetting::Myers,     built_in: true },
        DiffProfile { name: "Ignore whitespace".into(), ignore_whitespace: true,  ignore_case: false, algorithm: DiffAlgorithmSetting::Myers,     built_in: true },
        DiffProfile { name: "Ignore case".into(),       ignore_whitespace: false, ignore_case: true,  algorithm: DiffAlgorithmSetting::Myers,     built_in: true },
        DiffProfile { name: "Histogram".into(),         ignore_whitespace: false, ignore_case: false, algorithm: DiffAlgorithmSetting::Histogram, built_in: true },
    ]
}

/// Add a user-defined profile and persist settings.
pub fn add_profile(store: &mut Store, name: String, ignore_whitespace: bool, ignore_case: bool, algorithm: DiffAlgorithmSetting) {
    store.settings.write().profiles.push(DiffProfile {
        name, ignore_whitespace, ignore_case, algorithm, built_in: false,
    });
    crate::ui::settings::persist(&store.settings.read());
}

/// Remove the profile at `index` if it is not built-in.
pub fn remove_profile(store: &mut Store, index: usize) {
    let is_builtin = store.settings.read().profiles.get(index).map(|p| p.built_in).unwrap_or(true);
    if is_builtin { return; }
    let mut s = store.settings.write();
    s.profiles.remove(index);
    if s.active_profile >= s.profiles.len() {
        s.active_profile = s.profiles.len().saturating_sub(1);
    }
    drop(s);
    crate::ui::settings::persist(&store.settings.read());
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub language: Lang,
    pub diff_font_size: u32,
    #[serde(default = "default_ctx")]
    pub context_lines: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_left_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_right_dir: Option<PathBuf>,
    #[serde(default = "default_profiles")]
    pub profiles: Vec<DiffProfile>,
    #[serde(default)]
    pub active_profile: usize,
    /// Comma-separated file extensions to ignore (e.g. `"o, class, tmp"`).
    #[serde(default)]
    pub ignore_extensions: String,
    /// Comma-separated directory-name patterns to ignore (e.g. `"target, node_modules, *.cache"`).
    #[serde(default)]
    pub ignore_dirs: String,
}

fn default_ctx() -> usize { 3 }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark, language: Lang::En, diff_font_size: 14,
            context_lines: 3, last_left_dir: None, last_right_dir: None,
            profiles: default_profiles(), active_profile: 0,
            ignore_extensions: String::new(), ignore_dirs: String::new(),
        }
    }
}

impl AppSettings {
    /// Build an [`IgnoreRules`] snapshot from the current settings.
    pub fn ignore_rules(&self) -> forskscope_core::IgnoreRules {
        forskscope_core::IgnoreRules::from_settings(&self.ignore_extensions, &self.ignore_dirs)
    }
}

/// Specification for a batch file-copy operation (deep compare "Copy all").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCopySpec {
    pub items: Vec<(PathBuf, PathBuf)>,   // (src, dst)
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None, Settings,
    ConfirmOverwrite(usize), SaveAs(usize, String),
    ConfirmReload(usize), ConfirmSwap(usize),
    #[allow(dead_code)] ConfirmDirOp(DirOp), ConfirmClose(usize),
    ConfirmBatchCopy(BatchCopySpec),
    About, KeyboardRef,
}

/// A pending directory file operation awaiting user confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirOp {
    pub src: std::path::PathBuf,
    pub dst: std::path::PathBuf,
    pub label: String,          // human-readable description for the modal
}

#[derive(Clone)]
pub struct CompareTab {
    pub title: String,
    pub left_path:  Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub left_doc:  LoadedDocument,
    pub right_doc: LoadedDocument,
    pub diff:  DiffDocument,
    pub merge: MergeSession,
    pub diff_options: DiffOptions,
    pub can_save:   bool,
    pub char_mode:  bool,
    pub word_wrap:  bool,
    pub focused_change: usize,
}

impl CompareTab {
    pub fn right_label(&self) -> String {
        self.right_doc.text.as_ref()
            .map(|t| t.encoding.label.clone())
            .unwrap_or_else(|| "—".into())
    }
}

pub fn recompute_diff(tab: &mut CompareTab) {
    let diff = compute_diff(tab.left_doc.diff_text(), tab.right_doc.diff_text(), tab.diff_options);
    tab.merge = MergeSession::from_diff(&diff);
    tab.diff = diff;
    tab.focused_change = 0;
    tab.char_mode = false;
}

pub fn reload_tab(store: &mut Store, index: usize) {
    let (lp, rp) = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else { return };
        (tab.left_path.clone(), tab.right_path.clone())
    };
    let opt = LoadOptions { allow_missing: true };
    let mut ld = match lp.as_deref().map(|p| load_path(p, opt)) {
        Some(Ok(d)) => d, Some(Err(e)) => return store.notify(format!("Reload L: {e}")),
        None => LoadedDocument::empty(),
    };
    let mut rd = match rp.as_deref().map(|p| load_path(p, opt)) {
        Some(Ok(d)) => d, Some(Err(e)) => return store.notify(format!("Reload R: {e}")),
        None => LoadedDocument::empty(),
    };
    if ld.kind == FileKind::ExcelXlsx && rd.kind == FileKind::ExcelXlsx {
        if let (Some(l), Some(r)) = (&lp, &rp) {
            let (lt, rt) = forskscope_core::xlsx::derive_pair_text(l, r);
            ld.text = Some(lt); rd.text = Some(rt);
        }
    }
    let mut tabs = store.tabs.write();
    let Some(tab) = tabs.get_mut(index) else { return };
    tab.left_doc = ld; tab.right_doc = rd;
    tab.can_save = tab.left_doc.kind.is_mergeable_text() && tab.right_doc.kind.is_mergeable_text();
    recompute_diff(tab);
}

pub fn swap_sides(store: &mut Store, index: usize) {
    let mut tabs = store.tabs.write();
    let Some(tab) = tabs.get_mut(index) else { return };
    std::mem::swap(&mut tab.left_doc,  &mut tab.right_doc);
    std::mem::swap(&mut tab.left_path, &mut tab.right_path);
    tab.can_save = tab.left_doc.kind.is_mergeable_text() && tab.right_doc.kind.is_mergeable_text();
    recompute_diff(tab);
}

#[derive(Clone, Copy)]
pub struct Store {
    pub tabs:      Signal<Vec<CompareTab>>,
    pub active:    Signal<Option<usize>>,
    pub settings:  Signal<AppSettings>,
    pub left_pick:  Signal<Option<PathBuf>>,
    pub right_pick: Signal<Option<PathBuf>>,
    pub modal: Signal<Modal>,
    pub toast: Signal<Option<String>>,
}

impl Store {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            tabs: Signal::new(Vec::new()), active: Signal::new(None),
            settings: Signal::new(settings),
            left_pick: Signal::new(None), right_pick: Signal::new(None),
            modal: Signal::new(Modal::None), toast: Signal::new(None),
        }
    }
    pub fn lang(&self) -> Lang { self.settings.read().language }
    pub fn notify(&mut self, msg: impl Into<String>) { self.toast.set(Some(msg.into())); }
}

pub fn open_compare(store: &mut Store, left: PathBuf, right: PathBuf) {
    let options = LoadOptions { allow_missing: true };
    let mut ld = match load_path(&left,  options) { Ok(d) => d, Err(e) => return store.notify(format!("L: {e}")) };
    let mut rd = match load_path(&right, options) { Ok(d) => d, Err(e) => return store.notify(format!("R: {e}")) };
    if ld.kind == FileKind::ExcelXlsx && rd.kind == FileKind::ExcelXlsx {
        let (lt, rt) = forskscope_core::xlsx::derive_pair_text(&left, &right);
        ld.text = Some(lt); rd.text = Some(rt);
    }
    // Use the active compare profile's options (RFC-009).
    let settings = store.settings.read();
    let opts = settings.profiles
        .get(settings.active_profile)
        .map(|p| p.to_diff_options())
        .unwrap_or_default();
    drop(settings);
    let diff = compute_diff(ld.diff_text(), rd.diff_text(), opts);
    let merge = MergeSession::from_diff(&diff);
    let can_save = ld.kind.is_mergeable_text() && rd.kind.is_mergeable_text();
    let title = tab_title(&left, &right);
    let tab = CompareTab {
        title, left_path: Some(left), right_path: Some(right),
        left_doc: ld, right_doc: rd, diff, merge, diff_options: opts,
        can_save, char_mode: false, word_wrap: false, focused_change: 0,
    };
    let idx = store.tabs.read().len();
    store.tabs.write().push(tab);
    store.active.set(Some(idx));
}

fn tab_title(l: &std::path::Path, r: &std::path::Path) -> String {
    let ln = l.file_name().map(|n| n.to_string_lossy().into_owned());
    let rn = r.file_name().map(|n| n.to_string_lossy().into_owned());
    match (ln, rn) {
        (Some(a), Some(b)) if a == b => a,
        (Some(a), Some(b)) => format!("{a} ↔ {b}"),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => "comparison".into(),
    }
}

// ─── Session persistence (RFC-035) ───────────────────────────────────────────

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    /// Pairs of (left_path, right_path) for each open comparison tab.
    pub tabs: Vec<(String, String)>,
}

fn session_manager() -> ConfigManager<SessionState> {
    ConfigManager::new().with_filename("session.json")
}

/// Persist the current open tabs so they can be restored next launch.
pub fn save_session(store: &Store) {
    let tabs = store.tabs.read();
    let saved: Vec<(String, String)> = tabs.iter()
        .filter_map(|tab| {
            let l = tab.left_path.as_ref()?.display().to_string();
            let r = tab.right_path.as_ref()?.display().to_string();
            Some((l, r))
        })
        .collect();
    let _ = session_manager().save(&SessionState { tabs: saved });
}

/// Load the last-saved session, opening each tab whose paths still exist.
/// Silently skips pairs where both sides are gone.
pub fn restore_session(store: &mut Store) {
    let state = session_manager().load_or_default().unwrap_or_default();
    for (left, right) in state.tabs {
        let lp = PathBuf::from(&left);
        let rp = PathBuf::from(&right);
        if lp.exists() || rp.exists() {
            open_compare(store, lp, rp);
        }
    }
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
    // Persist the updated session immediately.
    save_session(store);
}
