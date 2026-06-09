//! Directory pane and file row rendering for the explorer workspace (RFC-005).
//!
//! Contains the keyboard-navigable `DirPane` component and the `FileRow`
//! component with copy-to-other-side buttons for directory merge operations.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;

use forskscope_core::dir::FileEntry;

use crate::state::{DirOp, Modal, Store};
use crate::ui::explorer::{DigestState, refresh};

/// A single directory pane.
///
/// Props:
/// - `dir` / `listing` / `other_names` / `digests`: shared state from Explorer.
/// - `other_dir`: the directory on the opposite side, used to build copy targets.
/// - `is_left`: whether this is the left (old) pane.
/// - `on_select`: called when a non-common file is clicked (single-side pick).
/// - `on_auto_compare`: called with `(left, right)` when a common file is clicked.
/// - `on_chdir`: called after navigation to signal the parent to refresh digests.
#[allow(clippy::too_many_arguments)]
#[component]
pub fn DirPane(
    label: String,
    dir: Signal<PathBuf>,
    listing: Signal<Option<forskscope_core::dir::DirectoryListing>>,
    other_names: HashSet<String>,
    digests: Signal<HashMap<String, DigestState>>,
    other_dir: PathBuf,
    is_left: bool,
    on_select: EventHandler<PathBuf>,
    on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
    on_chdir: EventHandler<()>,
) -> Element {
    let mut path_input = use_signal(|| dir.read().display().to_string());
    let mut focused_row: Signal<i32> = use_signal(|| -1);

    let dir_rows: Vec<(usize, String)> = listing.read().as_ref()
        .map(|l| l.dirs.iter().enumerate().map(|(i, d)| (i, d.clone())).collect())
        .unwrap_or_default();
    let dir_count = dir_rows.len();
    let file_rows: Vec<(usize, FileEntry)> = listing.read().as_ref()
        .map(|l| l.files.iter().enumerate().map(|(i, f)| (dir_count + i, f.clone())).collect())
        .unwrap_or_default();
    let row_count = (dir_rows.len() + file_rows.len()) as i32;
    let fr = *focused_row.read();
    let kbrows: Vec<(bool, String)> = dir_rows.iter().map(|(_, d)| (true, d.clone()))
        .chain(file_rows.iter().map(|(_, f)| (false, f.name.clone())))
        .collect();

    rsx! {
        div { class: "pane",
            h3 { "{label}" }
            div { class: "pathbar",
                input {
                    value: "{path_input}",
                    oninput: move |e| path_input.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            let p = PathBuf::from(path_input.read().clone());
                            go(&p, dir, listing); on_chdir.call(());
                        }
                    },
                }
                button { title: "Go",
                    onclick: move |_| { go(&PathBuf::from(path_input.read().clone()), dir, listing); on_chdir.call(()); },
                    "→"
                }
                button { title: "Up",
                    onclick: move |_| {
                        let parent = dir.read().parent().map(|p| p.to_path_buf());
                        if let Some(p) = parent {
                            path_input.set(p.display().to_string());
                            go(&p, dir, listing); on_chdir.call(());
                        }
                    },
                    "↑"
                }
            }
            div {
                class: "dir-table",
                tabindex: "0",
                onkeydown: move |e| {
                    match e.key() {
                        Key::ArrowDown => { let n = fr; focused_row.set((n + 1).min(row_count - 1)); }
                        Key::ArrowUp   => { let n = fr; focused_row.set((n - 1).max(0)); }
                        Key::Enter => {
                            if let Some((is_dir, name)) = kbrows.get(fr as usize) {
                                if *is_dir {
                                    let next = dir.read().join(name);
                                    path_input.set(next.display().to_string());
                                    go(&next, dir, listing); on_chdir.call(());
                                } else {
                                    activate_file(name, dir.read().clone(), &other_dir,
                                        &other_names, is_left, on_select, on_auto_compare);
                                }
                            }
                        }
                        _ => {}
                    }
                },
                if listing.read().is_none() {
                    div { class: "dir-loading", "…" }
                } else {
                    for (row_idx, d) in dir_rows {
                        div {
                            class: if fr == row_idx as i32 { "dir-row folder focused" } else { "dir-row folder" },
                            onclick: move |_| {
                                let next = dir.read().join(&d);
                                path_input.set(next.display().to_string());
                                go(&next, dir, listing); on_chdir.call(());
                            },
                            span { class: "dir-icon", "📁" }
                            span { class: "dir-name", "{d}" }
                        }
                    }
                    for (row_idx, file) in file_rows {
                        FileRow {
                            file: file.clone(),
                            other_names: other_names.clone(),
                            digest: digests.read().get(&file.name).cloned(),
                            is_left, focused: fr == row_idx as i32,
                            base_dir: dir.read().clone(),
                            other_dir: other_dir.clone(),
                            on_select, on_auto_compare,
                        }
                    }
                }
            }
        }
    }
}

// ─── File row ─────────────────────────────────────────────────────────────────

#[component]
fn FileRow(
    file: FileEntry,
    other_names: HashSet<String>,
    digest: Option<DigestState>,
    is_left: bool, focused: bool,
    base_dir: PathBuf, other_dir: PathBuf,
    on_select: EventHandler<PathBuf>,
    on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
) -> Element {
    let mut store = use_context::<Store>();
    let in_both = other_names.contains(&file.name);
    let (sc, icon) = match (in_both, &digest) {
        (false, _)                              => ("status-only",    if is_left { "←" } else { "→" }),
        (true, Some(DigestState::Equal))        => ("status-equal",   "✓"),
        (true, Some(DigestState::Changed))      => ("status-changed", "⚠"),
        (true, Some(DigestState::Err))          => ("status-err",     "!"),
        _                                       => ("status-cmp",     "⊙"),
    };
    let row_class = if focused { "dir-row file focused" } else { "dir-row file" };
    let name = file.name.clone();
    let src = base_dir.join(&file.name);
    let dst = other_dir.join(&file.name);

    // Build copy-op label for the modal.
    let copy_label = if is_left {
        format!("Copy {} → right ({})", file.name, other_dir.display())
    } else {
        format!("Copy {} ← left ({})", file.name, other_dir.display())
    };
    let op = DirOp { src: src.clone(), dst: dst.clone(), label: copy_label };

    rsx! {
        div {
            class: "{row_class}",
            onclick: move |_| activate_file(&name, base_dir.clone(), &other_dir, &other_names,
                                             is_left, on_select, on_auto_compare),
            span { class: "dir-status {sc}", title: icon, "{icon}" }
            span { class: "dir-name", "{file.name}" }
            span { class: "dir-size", "{file.human_size}" }
            // Copy button: shown for left-only, right-only, or changed files.
            if !matches!(digest, Some(DigestState::Equal)) {
                button {
                    class: "dir-copy-btn",
                    title: "{op.label}",
                    aria_label: "{op.label}",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        store.modal.set(Modal::ConfirmDirOp(op.clone()));
                    },
                    if is_left { "→" } else { "←" }
                }
            }
        }
    }
}

// ─── Helpers re-used by Explorer ─────────────────────────────────────────────

/// Single activation handler: auto-compare common files, or select as pick.
pub fn activate_file(
    name: &str, base_dir: PathBuf, other_dir: &std::path::Path,
    other_names: &HashSet<String>, is_left: bool,
    on_select: EventHandler<PathBuf>,
    on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
) {
    let path = base_dir.join(name);
    if other_names.contains(name) {
        let (l, r) = if is_left { (path, other_dir.join(name)) }
                     else { (other_dir.join(name), path) };
        on_auto_compare.call((l, r));
    } else {
        on_select.call(path);
    }
}

pub fn go(path: &std::path::Path, mut dir: Signal<PathBuf>,
          listing: Signal<Option<forskscope_core::dir::DirectoryListing>>) {
    if path.is_dir() { dir.set(path.to_path_buf()); refresh(dir, listing); }
}
