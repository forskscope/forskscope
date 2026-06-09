//! Explorer pane: directory tree view with full path navigation (RFC-054, RFC-055).
//!
//! Path bar: back/forward history, home, folder picker (rfd), editable
//! path input with error feedback, breadcrumb with per-segment max-length.
//!
//! Tree rows show digest status (✓/⚠/·/⟳) for files compared against the
//! opposite pane\'s root.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTreeEvent, SelectionMode, ThreadExecutor, use_scan_driver};
use dioxus_swdir_tree::{DirectoryTree, LoadPayload, ScanExecutor, ScanFuture, ScanJob};

use forskscope_core::{IgnoreRules, dir::file_digest_equal};

use crate::state::Store;

// ── Digest state ──────────────────────────────────────────────────────────────

/// Per-file comparison status shown in tree rows.
#[derive(Clone, PartialEq, Debug)]
pub enum DigestState { Computing, Equal, Different, Unique }

// ── Navigation history ────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct NavHistory { entries: Vec<PathBuf>, idx: usize }

impl NavHistory {
    fn push(&mut self, path: PathBuf) {
        if self.entries.last().map(|p| p == &path).unwrap_or(false) { return; }
        self.entries.truncate(self.idx + 1);
        self.entries.push(path);
        self.idx = self.entries.len() - 1;
    }
    fn can_back(&self) -> bool { self.idx > 0 }
    fn can_forward(&self) -> bool { self.idx + 1 < self.entries.len() }
    fn back(&mut self) -> Option<PathBuf> {
        if self.can_back() { self.idx -= 1; Some(self.entries[self.idx].clone()) } else { None }
    }
    fn forward(&mut self) -> Option<PathBuf> {
        if self.can_forward() { self.idx += 1; Some(self.entries[self.idx].clone()) } else { None }
    }
}

// ── Filtering executor ────────────────────────────────────────────────────────

struct FilteringExecutor { rules: IgnoreRules }
unsafe impl Send for FilteringExecutor {}
unsafe impl Sync for FilteringExecutor {}

impl ScanExecutor for FilteringExecutor {
    fn spawn_blocking(&self, job: ScanJob) -> ScanFuture {
        let rules = self.rules.clone();
        let filtered: ScanJob = Box::new(move || {
            let mut payload: LoadPayload = job();
            if !rules.is_empty() {
                if let Ok(ref mut entries) = payload.result {
                    entries.retain(|e| {
                        let name = e.file_name().to_str().unwrap_or("");
                        if e.is_dir { !rules.is_dir_ignored(name) }
                        else        { !rules.is_file_ignored(name) }
                    });
                }
            }
            payload
        });
        ThreadExecutor.spawn_blocking(filtered)
    }
}

// ── Public DirPane ────────────────────────────────────────────────────────────

#[component]
pub fn DirPane(
    is_left: bool,
    ignore: IgnoreRules,
    on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
) -> Element {
    let mut store = use_context::<Store>();

    let initial_root = {
        let s = store.settings.read();
        if is_left { s.last_left_dir.clone() } else { s.last_right_dir.clone() }
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
    };

    let mut current_dir: Signal<PathBuf> = use_signal(|| initial_root.clone());
    let mut history: Signal<NavHistory>  = use_signal(NavHistory::default);
    let mut digest_map: Signal<HashMap<PathBuf, DigestState>> = use_signal(HashMap::new);

    use_hook(|| { history.write().push(initial_root.clone()); });

    let executor = Arc::new(FilteringExecutor { rules: ignore });
    let mut tree: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(initial_root.clone()));
    let scans = use_scan_driver(tree, executor);

    use_effect(move || {
        let root = current_dir.read().clone();
        let mut nt = DirectoryTree::new(root.clone());
        if let Some(req) = nt.on_toggled(&root) { tree.set(nt); scans.send(req); }
        else { tree.set(nt); }
        digest_map.write().clear();
    });

    // Background digest status for visible files vs the other pane.
    use_effect(move || {
        let root = current_dir.read().clone();
        let other = {
            let s = store.settings.read();
            if is_left { s.last_right_dir.clone() } else { s.last_left_dir.clone() }
                .unwrap_or_default()
        };
        if other.as_os_str().is_empty() { return; }
        let visible: Vec<PathBuf> = tree.read().visible_rows().into_iter()
            .filter(|(n, _)| !n.is_dir)
            .map(|(n, _)| n.path.clone()).collect();
        for abs in visible {
            let Ok(rel) = abs.strip_prefix(&root) else { continue };
            let rel = rel.to_path_buf();
            if digest_map.read().contains_key(&rel) { continue; }
            let cp = other.join(&rel);
            if !cp.exists() { digest_map.write().insert(rel, DigestState::Unique); continue; }
            let (lp, rp) = if is_left { (abs.clone(), cp) } else { (cp, abs.clone()) };
            let key = rel.clone();
            let mut dmap = digest_map;
            dmap.write().insert(key.clone(), DigestState::Computing);
            spawn(async move {
                let eq = tokio::task::spawn_blocking(move || {
                    file_digest_equal(&lp, &rp).unwrap_or(false)
                }).await.unwrap_or(false);
                dmap.write().insert(key, if eq { DigestState::Equal } else { DigestState::Different });
            });
        }
    });

    let mut pick = if is_left { store.left_pick } else { store.right_pick };
    let other_pick: Option<PathBuf> = if is_left {
        store.right_pick.read().clone()
    } else {
        store.left_pick.read().clone()
    };

    let rows: Vec<(PathBuf, bool, bool, bool, u32)> = {
        let t = tree.read();
        t.visible_rows().into_iter()
            .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d))
            .collect()
    };

    let cur_root   = current_dir.read().clone();
    let can_back   = history.read().can_back();
    let can_fwd    = history.read().can_forward();
    let side_label = if is_left { "Left / Old" } else { "Right / New" };

    rsx! {
        div { class: "dir-pane", aria_label: "{side_label}",
            PathBar {
                path: cur_root.clone(),
                can_back, can_forward: can_fwd,
                on_back: move |_| {
                    if let Some(p) = history.write().back() {
                        let mut s = store.settings.write();
                        if is_left { s.last_left_dir = Some(p.clone()); } else { s.last_right_dir = Some(p.clone()); }
                        drop(s);
                        current_dir.set(p);
                    }
                },
                on_forward: move |_| {
                    if let Some(p) = history.write().forward() {
                        let mut s = store.settings.write();
                        if is_left { s.last_left_dir = Some(p.clone()); } else { s.last_right_dir = Some(p.clone()); }
                        drop(s);
                        current_dir.set(p);
                    }
                },
                on_navigate: move |p| navigate_to(p, is_left, store, history, current_dir),
            }
            div {
                class: "dir-tree", tabindex: "0",
                onkeydown: move |e: Event<KeyboardData>| {
                    use dioxus_swdir_tree::keyboard::{Modifiers as CM, TreeKey, handle_key};
                    if e.modifiers().contains(Modifiers::ALT) && e.key() == Key::ArrowUp {
                        let parent = current_dir.read().parent().map(|p| p.to_path_buf());
                        if let Some(p) = parent { navigate_to(p, is_left, store, history, current_dir); }
                        return;
                    }
                    let key = match e.key() {
                        Key::ArrowUp => TreeKey::Up, Key::ArrowDown => TreeKey::Down,
                        Key::ArrowLeft => TreeKey::Left, Key::ArrowRight => TreeKey::Right,
                        Key::Enter => TreeKey::Enter, Key::Home => TreeKey::Home,
                        Key::End => TreeKey::End, Key::Escape => TreeKey::Escape,
                        Key::Character(ref s) if s == " " => TreeKey::Space,
                        _ => return,
                    };
                    let mods = CM { shift: e.modifiers().shift(), ctrl: e.modifiers().ctrl() };
                    let ev = handle_key(&tree.read(), key, mods);
                    if let Some(ev) = ev {
                        e.prevent_default();
                        match ev {
                            DirectoryTreeEvent::Toggled(p) => {
                                if let Some(r) = tree.write().on_toggled(&p) { scans.send(r); }
                            }
                            DirectoryTreeEvent::Selected { path, is_dir, mode } => {
                                tree.write().on_selected(&path, is_dir, mode);
                            }
                            DirectoryTreeEvent::Drag(_) => {}
                        }
                    }
                },
                for (path, is_dir, is_expanded, is_selected, depth) in rows.iter() {
                    {
                        let p_tgl = path.clone(); let p_sel = path.clone();
                        let p_dbl = path.clone(); let p_nav = path.clone();
                        let is_dir = *is_dir; let other = other_pick.clone();
                        let status = path.strip_prefix(&cur_root).ok()
                            .and_then(|k| digest_map.read().get(k).cloned());
                        rsx! {
                            TreeRow {
                                path: path.clone(), is_dir,
                                is_expanded: *is_expanded, is_selected: *is_selected,
                                depth: *depth, status,
                                on_toggle: move |_| {
                                    if let Some(r) = tree.write().on_toggled(&p_tgl) { scans.send(r); }
                                },
                                on_select: move |_| {
                                    tree.write().on_selected(&p_sel, is_dir, SelectionMode::Replace);
                                    if !is_dir { pick.set(Some(p_sel.clone())); }
                                },
                                on_dblclick: move |_| {
                                    if is_dir {
                                        navigate_to(p_nav.clone(), is_left, store, history, current_dir);
                                    } else if let Some(cp) = counterpart(&p_dbl, &other) {
                                        let pair = if is_left { (p_dbl.clone(), cp) } else { (cp, p_dbl.clone()) };
                                        on_auto_compare.call(pair);
                                    }
                                },
                            }
                        }
                    }
                }
                if rows.is_empty() { div { class: "dir-empty", "…" } }
            }
            div { class: "dir-pane-footer",
                if let Some(ref p) = *pick.read() {
                    span { class: "pick-label", title: "{p.display()}", {short_name(p)} }
                } else {
                    span { class: "pick-hint", "click · dblclick to compare" }
                }
            }
        }
    }
}

// ── Navigation helper ─────────────────────────────────────────────────────────

/// Persist the new root in settings, push history, and update current_dir.
/// All parameters are `Copy` signal types, safe to pass by value to multiple callers.
fn navigate_to(
    path: PathBuf, is_left: bool,
    mut store: Store,
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

// ── Path bar ──────────────────────────────────────────────────────────────────

const MAX_SEG: usize = 18;

#[component]
fn PathBar(
    path: PathBuf, can_back: bool, can_forward: bool,
    on_back: EventHandler<()>, on_forward: EventHandler<()>,
    on_navigate: EventHandler<PathBuf>,
) -> Element {
    // Pre-compute before closures consume `path`.
    let segs = path_segs(&path);
    let n    = segs.len();
    let path_str = path.display().to_string();  // for use_effect
    let path_str_reset = path_str.clone();       // for escape / invalid blur
    let path_str_blur  = path_str.clone();       // for onblur (separate move)

    let mut edit_mode: Signal<bool>   = use_signal(|| false);
    let mut input_val: Signal<String> = use_signal(|| path_str.clone());
    let mut input_err: Signal<bool>   = use_signal(|| false);

    // Keep input_val synced with the current directory when not editing.
    use_effect(move || {
        if !*edit_mode.read() { input_val.set(path_str.clone()); }
    });

    rsx! {
        div { class: "path-bar",
            button { class: "path-btn", title: "Back",    disabled: !can_back,    onclick: move |_| on_back.call(()),    "←" }
            button { class: "path-btn", title: "Forward", disabled: !can_forward, onclick: move |_| on_forward.call(()), "→" }
            button { class: "path-btn", title: "Home directory",
                onclick: move |_| on_navigate.call(home_dir()), "⌂" }
            button { class: "path-btn", title: "Open folder…",
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
                        oninput: move |e| { input_val.set(e.value()); input_err.set(false); },
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
                            if v.is_dir() {
                                edit_mode.set(false); on_navigate.call(v);
                            } else {
                                input_val.set(path_str_blur.clone());
                                edit_mode.set(false); input_err.set(false);
                            }
                        },
                    }
                } else {
                    for (idx, (seg_path, label)) in segs.iter().enumerate() {
                        if idx > 0 { span { class: "bc-sep", " / " } }
                        if idx == n - 1 {
                            span { class: "bc-current", title: "{label}",
                                onclick: move |_| edit_mode.set(true),
                                {trunc_label(label, MAX_SEG)} }
                        } else {
                            { let t = seg_path.clone(); let full = label.clone();
                              rsx! { button { class: "bc-seg", title: "{full}",
                                  onclick: move |_| on_navigate.call(t.clone()),
                                  {trunc_label(&full, MAX_SEG)} } } }
                        }
                    }
                    button { class: "path-btn path-edit-btn", title: "Edit path",
                        onclick: move |_| edit_mode.set(true), "✎" }
                }
            }
        }
    }
}

// ── Tree row ──────────────────────────────────────────────────────────────────

#[component]
fn TreeRow(
    path: PathBuf, is_dir: bool, is_expanded: bool, is_selected: bool, depth: u32,
    status: Option<DigestState>,
    on_toggle: EventHandler<()>, on_select: EventHandler<()>, on_dblclick: EventHandler<()>,
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
            span { class: "tree-icon", "{icon}" }
            span { class: "tree-label", "{name}" }
            if !st_icon.is_empty() {
                span { class: "tree-status {st_cls}", "{st_icon}" }
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn counterpart(path: &Path, other_pick: &Option<PathBuf>) -> Option<PathBuf> {
    other_pick.as_ref().filter(|op| op.file_name() == path.file_name()).cloned()
}

fn short_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

fn trunc_label(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.into() }
    else { format!("{}…", s.chars().take(max - 1).collect::<String>()) }
}

fn path_segs(path: &Path) -> Vec<(PathBuf, String)> {
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

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/"))
}
