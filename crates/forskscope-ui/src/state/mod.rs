//! Application UI state and the core <-> UI glue (RFC-003 §state ownership).

pub mod settings;
pub use settings::{
    AppSettings, BatchCopySpec, DiffAlgorithmSetting, Lang, Theme,
};

use std::path::PathBuf;
use app_json_settings::ConfigManager;
use dioxus::prelude::*;
use forskscope_core::diff::DiffDocument;
use forskscope_core::document::{LoadOptions, LoadedDocument, load_path};
use forskscope_core::file_kind::FileKind;
use forskscope_core::{DiffOptions, MergeSession, compute_diff};
use crate::i18n::t;

#[derive(Clone)]
pub enum Modal {
    None, Settings,
    ConfirmOverwrite(usize), SaveAs(usize, String),
    ConfirmReload(usize), ConfirmSwap(usize),
    ConfirmDirOp(DirOp), ConfirmClose(usize),
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
        Some(Ok(d)) => d, Some(Err(e)) => return store.notify(format!("{}: {e}", t(store.lang(), "Left file read error"))),
        None => LoadedDocument::empty(),
    };
    let mut rd = match rp.as_deref().map(|p| load_path(p, opt)) {
        Some(Ok(d)) => d, Some(Err(e)) => return store.notify(format!("{}: {e}", t(store.lang(), "Right file read error"))),
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
    pub dir_tabs:  Signal<Vec<(PathBuf, PathBuf)>>,
    pub active_dir: Signal<Option<usize>>,
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
            dir_tabs: Signal::new(Vec::new()), active_dir: Signal::new(None),
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
    let mut ld = match load_path(&left,  options) { Ok(d) => d, Err(e) => return store.notify(format!("{}: {e}", t(store.lang(), "Left file read error"))) };
    let mut rd = match load_path(&right, options) { Ok(d) => d, Err(e) => return store.notify(format!("{}: {e}", t(store.lang(), "Right file read error"))) };

    // Guard: warn when comparing text against binary (meaningless hex diff).
    // Excel is always allowed (sheets-diff handles it). Missing files are allowed.
    let l_bin = matches!(ld.kind, FileKind::Binary);
    let r_bin = matches!(rd.kind, FileKind::Binary);
    let l_text = matches!(ld.kind, FileKind::Text);
    let r_text = matches!(rd.kind, FileKind::Text);
    if (l_bin && r_text) || (l_text && r_bin) {
        return store.notify(t(store.lang(), "Cannot compare: one file is binary and the other is text. Compare text with text, or binary with binary."));
    }
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
    let title = tab_title(&left, &right, store.lang());
    let tab = CompareTab {
        title, left_path: Some(left), right_path: Some(right),
        left_doc: ld, right_doc: rd, diff, merge, diff_options: opts,
        can_save, char_mode: false, word_wrap: false, focused_change: 0,
    };
    let idx = store.tabs.read().len();
    store.tabs.write().push(tab);
    store.active.set(Some(idx));
}

/// Open a directory compare tab for `left` vs `right`.
pub fn open_dir_compare(store: &mut Store, left: PathBuf, right: PathBuf) {
    store.dir_tabs.write().push((left, right));
    let idx = store.dir_tabs.read().len() - 1;
    store.active.set(None);
    store.active_dir.set(Some(idx));
}

/// Close a directory compare tab at `index`.
pub fn close_dir_tab(store: &mut Store, index: usize) {
    store.dir_tabs.write().remove(index);
    let len = store.dir_tabs.read().len();
    let cur = *store.active_dir.read();
    if len == 0 {
        store.active_dir.set(None);
    } else if cur == Some(index) {
        store.active_dir.set(Some(index.saturating_sub(1).min(len - 1)));
    } else if cur > Some(index) {
        store.active_dir.set(cur.map(|i| i - 1));
    }
}

fn tab_title(l: &std::path::Path, r: &std::path::Path, lang: Lang) -> String {
    let ln = l.file_name().map(|n| n.to_string_lossy().into_owned());
    let rn = r.file_name().map(|n| n.to_string_lossy().into_owned());
    match (ln, rn) {
        (Some(a), Some(b)) if a == b => a,
        (Some(a), Some(b)) => format!("{a} ↔ {b}"),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => t(lang, "comparison"),
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

// ─── GTK-free unit tests ──────────────────────────────────────────────────────
//
// These tests run under `cargo test --lib -p forskscope-ui` without requiring
// GTK or a display server (RFC-020 §7 "Unit Tests").  They cover pure
// functions in this module that contain no Dioxus signal or component code.

/// Add a user-defined diff profile and persist settings.
pub fn add_profile(store: &mut Store, name: String, ignore_whitespace: bool, ignore_case: bool, algorithm: DiffAlgorithmSetting) {
    store.settings.write().profiles.push(settings::DiffProfile {
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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::tab_title;

    // ── tab_title ─────────────────────────────────────────────────────────────

    #[test]
    fn same_filename_both_sides_shows_single_name() {
        let title = tab_title(
            Path::new("/old/src/main.rs"),
            Path::new("/new/src/main.rs"),
            super::Lang::En,
        );
        assert_eq!(title, "main.rs");
    }

    #[test]
    fn different_filenames_shows_both_with_arrow() {
        let title = tab_title(
            Path::new("/old/foo.txt"),
            Path::new("/new/bar.txt"),
            super::Lang::En,
        );
        assert_eq!(title, "foo.txt ↔ bar.txt");
    }

    #[test]
    fn left_only_filename_shows_left() {
        // Right path has no filename component (e.g. directory root "/")
        let title = tab_title(
            Path::new("/project/README.md"),
            Path::new("/"),
            super::Lang::En,
        );
        assert_eq!(title, "README.md");
    }

    #[test]
    fn both_missing_filenames_shows_fallback() {
        let title = tab_title(Path::new("/"), Path::new("/"), super::Lang::En);
        assert_eq!(title, "comparison");
    }

    #[test]
    fn hidden_dotfile_names_match_correctly() {
        let title = tab_title(
            Path::new("/a/.gitignore"),
            Path::new("/b/.gitignore"),
            super::Lang::En,
        );
        assert_eq!(title, ".gitignore");
    }

    #[test]
    fn deeply_nested_same_filename_shows_single_name() {
        let title = tab_title(
            Path::new("/home/alice/projectA/src/lib/core/mod.rs"),
            Path::new("/home/bob/projectB/src/lib/core/mod.rs"),
            super::Lang::En,
        );
        assert_eq!(title, "mod.rs");
    }

    // ── SessionState round-trip (pure serde, no I/O) ──────────────────────────

    #[test]
    fn session_state_serialises_and_deserialises() {
        use super::SessionState;
        let state = SessionState {
            tabs: vec![
                ("/old/a.rs".into(), "/new/a.rs".into()),
                ("/old/b.rs".into(), "/new/b.rs".into()),
            ],
        };
        let json = serde_json::to_string(&state).expect("serialise");
        let back: SessionState = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back.tabs.len(), 2);
        assert_eq!(back.tabs[0].0, "/old/a.rs");
        assert_eq!(back.tabs[1].1, "/new/b.rs");
    }

    #[test]
    fn empty_session_state_round_trips() {
        use super::SessionState;
        let state = SessionState::default();
        let json = serde_json::to_string(&state).unwrap();
        let back: SessionState = serde_json::from_str(&json).unwrap();
        assert!(back.tabs.is_empty());
    }
}
