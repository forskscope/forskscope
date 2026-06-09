//! Explorer pane: directory tree view with breadcrumb navigation (RFC-054, RFC-055).
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTreeEvent, SelectionMode, ThreadExecutor, use_scan_driver};
use dioxus_swdir_tree::{DirectoryTree, LoadPayload, ScanExecutor, ScanFuture, ScanJob};

use forskscope_core::IgnoreRules;
use crate::state::Store;

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

// ── Public component ──────────────────────────────────────────────────────────

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
    let executor = Arc::new(FilteringExecutor { rules: ignore });
    let mut tree: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(initial_root.clone()));
    let scans = use_scan_driver(tree, executor);

    // Re-build tree whenever current_dir changes.
    use_effect(move || {
        let root = current_dir.read().clone();
        let mut new_tree = DirectoryTree::new(root.clone());
        if let Some(req) = new_tree.on_toggled(&root) {
            tree.set(new_tree);
            scans.send(req);
        } else {
            tree.set(new_tree);
        }
    });

    let mut pick: Signal<Option<PathBuf>> = if is_left { store.left_pick } else { store.right_pick };

    let other_pick: Option<PathBuf> = if is_left {
        store.right_pick.read().clone()
    } else {
        store.left_pick.read().clone()
    };

    let rows: Vec<(PathBuf, bool, bool, bool, u32)> = {
        let t = tree.read();
        t.visible_rows()
            .into_iter()
            .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d))
            .collect()
    };

    let side_label = if is_left { "Left / Old" } else { "Right / New" };

    rsx! {
        div { class: "dir-pane", aria_label: "{side_label}",

            // ── Breadcrumb (RFC-055) ────────────────────────────────
            BreadcrumbBar {
                path: current_dir.read().clone(),
                on_navigate: move |new_root: PathBuf| {
                    // Persist root and rebuild tree.
                    {
                        let mut s = store.settings.write();
                        if is_left { s.last_left_dir = Some(new_root.clone()); }
                        else       { s.last_right_dir = Some(new_root.clone()); }
                    }
                    current_dir.set(new_root);
                },
            }

            // ── Tree ────────────────────────────────────────────────
            div {
                class: "dir-tree", tabindex: "0",
                onkeydown: move |e| tree_keydown(e, tree, scans, current_dir, is_left, store),
                for (path, is_dir, is_expanded, is_selected, depth) in rows.iter() {
                    {
                        let p_toggle = path.clone();
                        let p_select = path.clone();
                        let p_dbl    = path.clone();
                        let p_nav    = path.clone();
                        let is_dir   = *is_dir;
                        let other    = other_pick.clone();
                        rsx! {
                            TreeRow {
                                path:        path.clone(),
                                is_dir,
                                is_expanded: *is_expanded,
                                is_selected: *is_selected,
                                depth:       *depth,
                                on_toggle: move |_| {
                                    if let Some(req) = tree.write().on_toggled(&p_toggle) {
                                        scans.send(req);
                                    }
                                },
                                on_select: move |_| {
                                    tree.write().on_selected(&p_select, is_dir, SelectionMode::Replace);
                                    if !is_dir { pick.set(Some(p_select.clone())); }
                                },
                                on_dblclick: move |_| {
                                    if is_dir {
                                        // Double-click directory → navigate into it.
                                        {
                                            let mut s = store.settings.write();
                                            if is_left { s.last_left_dir = Some(p_nav.clone()); }
                                            else       { s.last_right_dir = Some(p_nav.clone()); }
                                        }
                                        current_dir.set(p_nav.clone());
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

            // ── Footer ──────────────────────────────────────────────
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

// ── Breadcrumb bar (RFC-055) ──────────────────────────────────────────────────

#[component]
fn BreadcrumbBar(path: PathBuf, on_navigate: EventHandler<PathBuf>) -> Element {
    let segs = path_segs(&path);
    let n = segs.len();
    let visible: Vec<(PathBuf, String, bool)> = if n <= 4 {
        segs.iter().enumerate().map(|(i, (p, l))| (p.clone(), l.clone(), i == n-1)).collect()
    } else {
        let mut v = vec![
            (segs[0].0.clone(), segs[0].1.clone(), false),
            (PathBuf::new(), "…".into(), false),
        ];
        for (i, seg) in segs.iter().enumerate().skip(n - 2) {
            v.push((seg.0.clone(), seg.1.clone(), i == n-1));
        }
        v
    };

    rsx! {
        nav { class: "breadcrumb", aria_label: "Current directory",
            for (idx, (seg_path, label, is_cur)) in visible.iter().enumerate() {
                if idx > 0 { span { class: "bc-sep", " / " } }
                if label == "…" {
                    span { class: "bc-ellipsis", "…" }
                } else if *is_cur {
                    span { class: "bc-current", "{label}" }
                } else {
                    { let t = seg_path.clone();
                      rsx! { button { class: "bc-seg",
                          onclick: move |_| on_navigate.call(t.clone()),
                          "{label}" } } }
                }
            }
        }
    }
}

fn path_segs(path: &Path) -> Vec<(PathBuf, String)> {
    let mut acc = PathBuf::new();
    path.components().map(|c| {
        acc.push(c);
        let label = match &c {
            Component::RootDir      => "/".into(),
            Component::Prefix(p)    => p.as_os_str().to_string_lossy().into_owned(),
            Component::Normal(name) => name.to_string_lossy().into_owned(),
            Component::CurDir       => ".".into(),
            Component::ParentDir    => "..".into(),
        };
        (acc.clone(), label)
    }).collect()
}

// ── Tree row ──────────────────────────────────────────────────────────────────

#[component]
fn TreeRow(
    path: PathBuf, is_dir: bool, is_expanded: bool, is_selected: bool, depth: u32,
    on_toggle: EventHandler<()>, on_select: EventHandler<()>, on_dblclick: EventHandler<()>,
) -> Element {
    let indent = depth * 16;
    let caret  = if !is_dir { "\u{00A0}" } else if is_expanded { "▾" } else { "▸" };
    let icon   = if is_dir { "📁" } else { "📄" };
    let name   = path.file_name().unwrap_or(OsStr::new("")).to_string_lossy().into_owned();
    let row_class = if is_selected { "tree-row selected" } else { "tree-row" };
    rsx! {
        div {
            class: "{row_class}", role: "row", style: "padding-left: {indent}px",
            onclick:    move |_| on_select.call(()),
            ondoubleclick: move |_| on_dblclick.call(()),
            span {
                class: "tree-caret",
                onclick: move |e| { e.stop_propagation(); on_toggle.call(()); },
                "{caret}"
            }
            span { class: "tree-icon",  "{icon}" }
            span { class: "tree-label", "{name}" }
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

fn tree_keydown(
    e: Event<KeyboardData>,
    mut tree: Signal<DirectoryTree>,
    scans: Coroutine<dioxus_swdir_tree::ScanRequest>,
    mut current_dir: Signal<PathBuf>,
    is_left: bool,
    mut store: Store,
) {
    use dioxus_swdir_tree::keyboard::{Modifiers as CoreMods, TreeKey, handle_key};

    if e.modifiers().contains(Modifiers::ALT) && e.key() == Key::ArrowUp {
        // Alt+↑: navigate to parent directory.
        let parent = current_dir.read().parent().map(|p| p.to_path_buf());
        if let Some(p) = parent {
            { let mut s = store.settings.write();
              if is_left { s.last_left_dir = Some(p.clone()); }
              else       { s.last_right_dir = Some(p.clone()); } }
            current_dir.set(p);
        }
        return;
    }

    let key = match e.key() {
        Key::ArrowUp    => TreeKey::Up,
        Key::ArrowDown  => TreeKey::Down,
        Key::ArrowLeft  => TreeKey::Left,
        Key::ArrowRight => TreeKey::Right,
        Key::Enter      => TreeKey::Enter,
        Key::Home       => TreeKey::Home,
        Key::End        => TreeKey::End,
        Key::Escape     => TreeKey::Escape,
        Key::Character(ref s) if s == " " => TreeKey::Space,
        _ => return,
    };
    let mods = CoreMods { shift: e.modifiers().shift(), ctrl: e.modifiers().ctrl() };
    // Drop the read guard before any potential write.
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
}
