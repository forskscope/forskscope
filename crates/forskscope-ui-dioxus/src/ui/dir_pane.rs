//! Directory pane: navigation history, filter, sort, copy buttons (RFC-005, RFC-021).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;

use forskscope_core::dir::FileEntry;

use crate::state::{DirOp, Modal, Store};
use crate::ui::explorer::{DigestState, ExplorerFilter, SortMode, refresh};

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
    filter: ExplorerFilter,
    sort: SortMode,
    show_hidden: bool,
    on_select: EventHandler<PathBuf>,
    on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
    on_chdir: EventHandler<()>,
) -> Element {
    let mut path_input = use_signal(|| dir.read().display().to_string());
    let mut focused_row: Signal<i32> = use_signal(|| -1);
    // Navigation history: a stack of directories.
    let history: Signal<Vec<PathBuf>> = use_signal(|| vec![dir.read().clone()]);
    // hist_pos.set() is called directly in onclick closures where the signal is captured
    // by Copy; `mut` is required for the &mut self method even though clippy disagrees.
    #[allow(unused_mut)]
    let mut hist_pos: Signal<usize> = use_signal(|| 0);

    // ── Build row data (apply filter, sort, show-hidden) ─────────────────────
    let raw_dirs: Vec<String> = listing.read().as_ref()
        .map(|l| l.dirs.iter()
            .filter(|d| show_hidden || !d.starts_with('.'))
            .cloned().collect())
        .unwrap_or_default();
    let raw_files: Vec<FileEntry> = listing.read().as_ref()
        .map(|l| l.files.iter()
            .filter(|f| show_hidden || !f.name.starts_with('.'))
            .filter(|f| match filter {
                ExplorerFilter::All       => true,
                ExplorerFilter::Different => !matches!(digests.read().get(&f.name), Some(DigestState::Equal)),
                ExplorerFilter::Equal     => matches!(digests.read().get(&f.name), Some(DigestState::Equal)),
            })
            .cloned().collect())
        .unwrap_or_default();
    let mut sorted_files = raw_files.clone();
    sorted_files.sort_by(|a, b| match sort {
        SortMode::ByName   => a.name.cmp(&b.name),
        SortMode::BySize   => a.len.cmp(&b.len),
        SortMode::ByStatus => {
            let ord = |f: &FileEntry| match digests.read().get(&f.name) {
                Some(DigestState::Changed) => 0, Some(DigestState::Err) => 1,
                None | Some(DigestState::Computing) => 2, Some(DigestState::Equal) => 3,
            };
            ord(a).cmp(&ord(b)).then(a.name.cmp(&b.name))
        }
    });
    let dir_rows: Vec<(usize, String)> = raw_dirs.iter().enumerate().map(|(i, d)| (i, d.clone())).collect();
    let dir_count = dir_rows.len();
    let file_rows: Vec<(usize, FileEntry)> = sorted_files.iter().enumerate()
        .map(|(i, f)| (dir_count + i, f.clone())).collect();
    let row_count = (dir_rows.len() + file_rows.len()) as i32;
    let fr = *focused_row.read();
    let kbrows: Vec<(bool, String)> = dir_rows.iter().map(|(_, d)| (true, d.clone()))
        .chain(file_rows.iter().map(|(_, f)| (false, f.name.clone()))).collect();
    let can_back    = *hist_pos.read() > 0;
    let can_forward = *hist_pos.read() + 1 < history.read().len();

    rsx! {
        div { class: "pane",
            h3 { "{label}" }
            div { class: "pathbar",
                button {
                    title: "Back",
                    disabled: !can_back,
                    onclick: move |_| {
                        let pos = hist_pos.read().saturating_sub(1);
                        hist_pos.set(pos);
                        let p = history.read()[pos].clone();
                        path_input.set(p.display().to_string());
                        dir.set(p.clone()); refresh(dir, listing); on_chdir.call(());
                    },
                    "◀"
                }
                button {
                    title: "Forward",
                    disabled: !can_forward,
                    onclick: move |_| {
                        let pos = *hist_pos.read() + 1;
                        hist_pos.set(pos);
                        let p = history.read()[pos].clone();
                        path_input.set(p.display().to_string());
                        dir.set(p.clone()); refresh(dir, listing); on_chdir.call(());
                    },
                    "▶"
                }
                input {
                    value: "{path_input}",
                    oninput: move |e| path_input.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            nav(PathBuf::from(path_input.read().clone()), dir, listing,
                                history, hist_pos, on_chdir);
                        }
                    },
                }
                button { title: "Go",
                    onclick: move |_| nav(PathBuf::from(path_input.read().clone()), dir,
                        listing, history, hist_pos, on_chdir),
                    "→"
                }
                button { title: "Up",
                    onclick: move |_| {
                        let parent = dir.read().parent().map(|p| p.to_path_buf());
                        if let Some(p) = parent {
                            path_input.set(p.display().to_string());
                            nav(p, dir, listing, history, hist_pos, on_chdir);
                        }
                    },
                    "↑"
                }
            }
            div {
                class: "dir-table", tabindex: "0",
                onkeydown: move |e| {
                    match e.key() {
                        Key::ArrowDown => { let n = fr; focused_row.set((n+1).min(row_count-1)); }
                        Key::ArrowUp   => { let n = fr; focused_row.set((n-1).max(0)); }
                        Key::Enter     => {
                            if let Some((is_dir, name)) = kbrows.get(fr as usize) {
                                if *is_dir {
                                    let next = dir.read().join(name);
                                    path_input.set(next.display().to_string());
                                    nav(next, dir, listing, history, hist_pos, on_chdir);
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
                                nav(next, dir, listing, history, hist_pos, on_chdir);
                            },
                            span { class: "dir-icon", "📁" }
                            span { class: "dir-name", "{d}" }
                        }
                    }
                    for (row_idx, file) in file_rows {
                        FileRow {
                            file: file.clone(), other_names: other_names.clone(),
                            digest: digests.read().get(&file.name).cloned(),
                            is_left, focused: fr == row_idx as i32,
                            base_dir: dir.read().clone(), other_dir: other_dir.clone(),
                            on_select, on_auto_compare,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FileRow(
    file: FileEntry, other_names: HashSet<String>, digest: Option<DigestState>,
    is_left: bool, focused: bool, base_dir: PathBuf, other_dir: PathBuf,
    on_select: EventHandler<PathBuf>, on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
) -> Element {
    let mut store = use_context::<Store>();
    let in_both = other_names.contains(&file.name);
    let (sc, icon) = match (in_both, &digest) {
        (false, _) => ("status-only", if is_left { "←" } else { "→" }),
        (true, Some(DigestState::Equal)) => ("status-equal", "✓"),
        (true, Some(DigestState::Changed)) => ("status-changed", "⚠"),
        (true, Some(DigestState::Err)) => ("status-err", "!"),
        _ => ("status-cmp", "⊙"),
    };
    let row_class = if focused { "dir-row file focused" } else { "dir-row file" };
    let name = file.name.clone();
    let copy_label = if is_left { format!("Copy {} → right ({})", file.name, other_dir.display()) }
                     else        { format!("Copy {} ← left ({})", file.name, other_dir.display()) };
    let op = DirOp { src: base_dir.join(&file.name), dst: other_dir.join(&file.name), label: copy_label };
    rsx! {
        div { class: "{row_class}",
            onclick: move |_| activate_file(&name, base_dir.clone(), &other_dir,
                                             &other_names, is_left, on_select, on_auto_compare),
            span { class: "dir-status {sc}", title: icon, "{icon}" }
            span { class: "dir-name", "{file.name}" }
            span { class: "dir-size", "{file.human_size}" }
            if !matches!(digest, Some(DigestState::Equal)) {
                button {
                    class: "dir-copy-btn", title: "{op.label}", aria_label: "{op.label}",
                    onclick: move |evt| { evt.stop_propagation(); store.modal.set(Modal::ConfirmDirOp(op.clone())); },
                    if is_left { "→" } else { "←" }
                }
            }
        }
    }
}

// ─── Navigation helpers ───────────────────────────────────────────────────────

fn nav(
    path: PathBuf, mut dir: Signal<PathBuf>,
    listing: Signal<Option<forskscope_core::dir::DirectoryListing>>,
    mut history: Signal<Vec<PathBuf>>, mut hist_pos: Signal<usize>,
    on_chdir: EventHandler<()>,
) {
    if path.is_dir() {
        let pos = *hist_pos.read();
        history.write().truncate(pos + 1);
        history.write().push(path.clone());
        hist_pos.set(pos + 1);
        dir.set(path); refresh(dir, listing); on_chdir.call(());
    }
}

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
    } else { on_select.call(path); }
}

