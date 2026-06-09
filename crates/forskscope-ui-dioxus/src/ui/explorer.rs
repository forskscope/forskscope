//! Explorer workspace (RFC-005). Two directory panes backed by
//! `dioxus-swdir-tree`; selecting a file on each side enables Compare.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_swdir_tree::{
    DirectoryTree, DirectoryTreeEvent, DirectoryTreeView, SelectionMode, ThreadExecutor,
    use_scan_driver,
};

use crate::i18n::t;
use crate::state::{Store, open_compare};

/// Which side a pane feeds.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let left = store.left_pick.read().clone();
    let right = store.right_pick.read().clone();
    let can_compare = left.is_some() && right.is_some();

    rsx! {
        div { class: "explorer",
            ExplorerPane { side: Side::Left }
            ExplorerPane { side: Side::Right }
            div { class: "compare-bar",
                button {
                    disabled: !can_compare,
                    onclick: move |_| {
                        let l = store.left_pick.read().clone();
                        let r = store.right_pick.read().clone();
                        if let (Some(l), Some(r)) = (l, r) {
                            open_compare(&mut store, l, r);
                        }
                    },
                    {t(lang, "Compare")}
                }
                span { class: "info",
                    if can_compare {
                        {format!("{} ↔ {}", short(&left), short(&right))}
                    } else {
                        {t(lang, "Select left, then right, then Compare.")}
                    }
                }
            }
        }
    }
}

#[component]
fn ExplorerPane(side: Side) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut tree = use_signal(|| {
        let mut t = DirectoryTree::new(start.clone());
        t.expand_blocking(&start);
        t
    });
    let scans = use_scan_driver(tree, Arc::new(ThreadExecutor));
    let mut path_input = use_signal(|| start.display().to_string());

    let title = match side {
        Side::Left => t(lang, "Left / Old"),
        Side::Right => t(lang, "Right / New"),
    };
    let pick = match side {
        Side::Left => store.left_pick.read().clone(),
        Side::Right => store.right_pick.read().clone(),
    };

    let on_event = move |ev: DirectoryTreeEvent| match ev {
        DirectoryTreeEvent::Toggled(path) => {
            if let Some(req) = tree.write().on_toggled(&path) {
                scans.send(req);
            }
        }
        DirectoryTreeEvent::Selected { path, is_dir, mode } => {
            tree.write().on_selected(&path, is_dir, mode);
            if !is_dir {
                set_pick(&mut store, side, path);
            }
        }
        DirectoryTreeEvent::Drag(msg) => {
            let _ = tree.write().on_drag_msg(msg);
        }
    };

    rsx! {
        div { class: "pane",
            h3 { "{title}" }
            div { class: "pathbar",
                input {
                    value: "{path_input}",
                    oninput: move |e| path_input.set(e.value()),
                }
                button {
                    onclick: move |_| {
                        let p = PathBuf::from(path_input.read().clone());
                        if p.is_dir() {
                            let mut t = DirectoryTree::new(p.clone());
                            t.expand_blocking(&p);
                            tree.set(t);
                        }
                    },
                    {t(lang, "List")}
                }
            }
            div { class: "tree",
                DirectoryTreeView { tree, on_event }
            }
            div { class: "pick",
                if let Some(p) = pick {
                    span { {short(&Some(p))} }
                } else {
                    span { "—" }
                }
            }
        }
    }
}

fn set_pick(store: &mut Store, side: Side, path: PathBuf) {
    match side {
        Side::Left => store.left_pick.set(Some(path)),
        Side::Right => store.right_pick.set(Some(path)),
    }
}

fn short(p: &Option<PathBuf>) -> String {
    p.as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "—".into())
}

#[allow(dead_code)]
fn is_dir(p: &Path) -> bool {
    p.is_dir()
}

// Bind SelectionMode so the re-export is exercised and stays in scope.
const _REPLACE_MODE: SelectionMode = SelectionMode::Replace;
