//! Dir-pane building blocks used by the Explorer aligned view (RFC-054).
//!
//! This module provides the types, helper functions, and leaf components
//! (`PathBar`, `TreeRow`) used by `explorer.rs`.  The `DirPane` monolith that
//! managed its own tree state is gone; tree ownership now lives in Explorer so
//! both panes can be rendered in an aligned structure.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::Lang;
use dioxus_swdir_tree::{ThreadExecutor};
use dioxus_swdir_tree::{LoadPayload, ScanExecutor, ScanFuture, ScanJob};

use forskscope_core::IgnoreRules;

// ── Public types ──────────────────────────────────────────────────────────────

/// Per-entry comparison status shown in tree rows.
#[derive(Clone, PartialEq, Debug)]
pub enum DigestState { Computing, Equal, Different, Unique }

/// Per-pane navigation history (back/forward).
#[derive(Clone, Default)]
pub struct NavHistory { pub entries: Vec<PathBuf>, pub idx: usize }

impl NavHistory {
    pub fn push(&mut self, path: PathBuf) {
        if self.entries.last().map(|p| p == &path).unwrap_or(false) { return; }
        self.entries.truncate(self.idx + 1);
        self.entries.push(path);
        self.idx = self.entries.len() - 1;
    }
    pub fn can_back(&self) -> bool { self.idx > 0 }
    pub fn can_forward(&self) -> bool { self.idx + 1 < self.entries.len() }
    pub fn back(&mut self) -> Option<PathBuf> {
        if self.can_back() { self.idx -= 1; Some(self.entries[self.idx].clone()) } else { None }
    }
    pub fn forward(&mut self) -> Option<PathBuf> {
        if self.can_forward() { self.idx += 1; Some(self.entries[self.idx].clone()) } else { None }
    }
}

// ── Filtering executor ────────────────────────────────────────────────────────

pub struct FilteringExecutor { pub rules: IgnoreRules }
// IgnoreRules is plain Vec<String>; Send + Sync derive automatically.

impl ScanExecutor for FilteringExecutor {
    fn spawn_blocking(&self, job: ScanJob) -> ScanFuture {
        let rules = self.rules.clone();
        let f: ScanJob = Box::new(move || {
            let mut p: LoadPayload = job();
            if !rules.is_empty() {
                if let Ok(ref mut entries) = p.result {
                    entries.retain(|e| {
                        let name = e.file_name().to_str().unwrap_or("");
                        if e.is_dir { !rules.is_dir_ignored(name) }
                        else        { !rules.is_file_ignored(name) }
                    });
                }
            }
            p
        });
        ThreadExecutor.spawn_blocking(f)
    }
}

// ── PathBar component ─────────────────────────────────────────────────────────

/// Full-featured path navigation bar.
///
/// Layout (single row, never wraps):
/// `← → ⌂ 📁 │ /path/segments/current ✎`
///
/// The breadcrumb uses `direction: rtl` via CSS so that when the path is too
/// long for the available space, the LEADING segments (ancestors) overflow
/// invisibly to the left while the CURRENT directory stays visible on the right.
#[component]
pub fn PathBar(
    path: PathBuf,
    can_back: bool, can_forward: bool,
    on_back:    EventHandler<()>,
    on_forward: EventHandler<()>,
    on_navigate: EventHandler<PathBuf>,
    lang: Lang,
) -> Element {
    // Pre-compute everything before closures consume values.
    let segs = path_segs(&path);
    let n    = segs.len();
    let path_str        = path.display().to_string();
    let path_str_reset  = path_str.clone();
    let path_str_blur   = path_str.clone();

    let mut edit_mode: Signal<bool>   = use_signal(|| false);
    let mut input_val: Signal<String> = use_signal(|| path_str.clone());
    let mut input_err: Signal<bool>   = use_signal(|| false);

    use_effect(move || {
        if !*edit_mode.read() { input_val.set(path_str.clone()); }
    });

    rsx! {
        div { class: "path-bar",
            button { class: "path-btn", t(lang, "Back"),    disabled: !can_back,    onclick: move |_| on_back.call(()),    "←" }
            button { class: "path-btn", title: t(lang, "Forward"), disabled: !can_forward, onclick: move |_| on_forward.call(()), "→" }
            button { class: "path-btn", title: t(lang, "Home directory"),
                onclick: move |_| on_navigate.call(home_dir()), "⌂" }
            button { class: "path-btn", title: t(lang, "Open folder…"),
                onclick: move |_| {
                    let nav = on_navigate;
                    spawn(async move {
                        let r = tokio::task::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
                            .await.ok().flatten();
                        if let Some(p) = r { nav.call(p); }
                    });
                }, "📁" }

            div { class: "path-segments",
                if *edit_mode.read() {
                    input {
                        class: if *input_err.read() { "path-input error" } else { "path-input" },
                        r#type: "text", value: "{input_val}", autofocus: true,
                        oninput:  move |e| { input_val.set(e.value()); input_err.set(false); },
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                let v = PathBuf::from(input_val.read().clone());
                                if v.is_dir() { edit_mode.set(false); on_navigate.call(v); }
                                else { input_err.set(true); }
                            }
                            if e.key() == Key::Escape {
                                input_val.set(path_str_reset.clone());
                                edit_mode.set(false); input_err.set(false);
                            }
                        },
                        onblur: move |_| {
                            let v = PathBuf::from(input_val.read().clone());
                            if v.is_dir() { edit_mode.set(false); on_navigate.call(v); }
                            else { input_val.set(path_str_blur.clone()); edit_mode.set(false); input_err.set(false); }
                        },
                    }
                } else {
                    for (idx, (seg_path, label)) in segs.iter().enumerate() {
                        if idx > 0 { span { class: "bc-sep", " / " } }
                        if idx == n - 1 {
                            span { class: "bc-current", title: "{label}",
                                onclick: move |_| edit_mode.set(true),
                                {trunc_label(label, 20)} }
                        } else {
                            { let t = seg_path.clone(); let full = label.clone();
                              rsx! { button { class: "bc-seg", title: "{full}",
                                  onclick: move |_| on_navigate.call(t.clone()),
                                  {trunc_label(&full, 20)} } } }
                        }
                    }
                    button { class: "path-btn path-edit-btn", title: t(lang, "Edit path"),
                        onclick: move |_| edit_mode.set(true), "✎" }
                }
            }
        }
    }
}

// ── TreeRow component ─────────────────────────────────────────────────────────

#[component]
pub fn TreeRow(
    path: PathBuf, is_dir: bool, is_expanded: bool, is_selected: bool, depth: u32,
    status: Option<DigestState>,
    on_toggle:    EventHandler<()>,
    on_select:    EventHandler<()>,
    on_dblclick:  EventHandler<()>,
) -> Element {
    let indent = depth * 16;
    let caret  = if !is_dir { "\u{00A0}" } else if is_expanded { "▾" } else { "▸" };
    let icon   = if is_dir { "📁" } else { "📄" };
    let name   = path.file_name().unwrap_or(OsStr::new("")).to_string_lossy().into_owned();
    let rc     = if is_selected { "tree-row selected" } else { "tree-row" };

    let (st_icon, st_cls) = match &status {
        None                         => ("",  ""),
        Some(DigestState::Computing) => ("⟳", "st-computing"),
        Some(DigestState::Equal)     => ("✓", "st-equal"),
        Some(DigestState::Different) => ("⚠", "st-diff"),
        Some(DigestState::Unique)    => ("·", "st-unique"),
    };
    rsx! {
        div {
            class: "{rc}", role: "row", style: "padding-left: {indent}px",
            onclick:      move |_| on_select.call(()),
            ondoubleclick: move |_| on_dblclick.call(()),
            span { class: "tree-caret",
                onclick: move |e| { e.stop_propagation(); on_toggle.call(()); }, "{caret}" }
            span { class: "tree-icon",  "{icon}" }
            span { class: "tree-label", "{name}" }
            if !st_icon.is_empty() {
                span { class: "tree-status {st_cls}", "{st_icon}" }
            }
        }
    }
}

// ── Helper functions (pub for explorer.rs) ────────────────────────────────────

pub fn path_segs(path: &Path) -> Vec<(PathBuf, String)> {
    let mut acc = PathBuf::new();
    path.components().map(|c| {
        acc.push(c);
        let lbl = match &c {
            Component::RootDir      => "/".into(),
            Component::Prefix(p)    => p.as_os_str().to_string_lossy().into_owned(),
            Component::Normal(name) => name.to_string_lossy().into_owned(),
            Component::CurDir       => ".".into(),
            Component::ParentDir    => "..".into(),
        };
        (acc.clone(), lbl)
    }).collect()
}

pub fn trunc_label(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.into() }
    else { format!("{}…", s.chars().take(max - 1).collect::<String>()) }
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn short_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

/// Persist the new root in settings, push history, and update current_dir.
pub fn navigate_to(
    path: PathBuf, is_left: bool,
    mut store: crate::state::Store,
    mut history: Signal<NavHistory>,
    mut current_dir: Signal<PathBuf>,
) {
    {
        let mut s = store.settings.write();
        if is_left { s.last_left_dir = Some(path.clone()); }
        else       { s.last_right_dir = Some(path.clone()); }
    }
    history.write().push(path.clone());
    current_dir.set(path);
}
