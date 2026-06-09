//! Explorer workspace (RFC-005) — flat directory-comparison view.
//!
//! Clicking a common file (same name in both panes) opens a comparison
//! immediately. Keyboard navigation: ↑/↓ to move focus, Enter to
//! navigate into a folder or open a comparison, Tab to switch panes.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;

use forskscope_core::dir::{DirectoryListing, FileEntry, file_digest_equal, list_dir};

use crate::i18n::t;
use crate::state::{Store, open_compare};

#[derive(Debug, Clone, PartialEq, Eq)]
enum DigestState { Computing, Equal, Changed, Err }

/// Summary counts derived from both listings.
#[derive(Default, Clone)]
struct DirSummary {
    common: usize,
    changed: usize,
    left_only: usize,
    right_only: usize,
}

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let init_left  = store.settings.read().last_left_dir.clone().unwrap_or_else(|| cwd.clone());
    let init_right = store.settings.read().last_right_dir.clone().unwrap_or_else(|| cwd.clone());
    let left_dir  = use_signal(|| init_left);
    let right_dir = use_signal(|| init_right);
    let left_listing:  Signal<Option<DirectoryListing>> = use_signal(|| None);
    let right_listing: Signal<Option<DirectoryListing>> = use_signal(|| None);
    let mut digests: Signal<HashMap<String, DigestState>> = use_signal(HashMap::new);

    // Initial directory load.
    use_effect(move || {
        if left_listing.read().is_none()  { refresh(left_dir,  left_listing); }
        if right_listing.read().is_none() { refresh(right_dir, right_listing); }
    });

    // Background digest comparison when either listing changes.
    use_effect(move || {
        let lf = file_names(left_listing);
        let rf = file_names(right_listing);
        let rs: HashSet<&str> = rf.iter().map(String::as_str).collect();
        let common: Vec<String> = lf.iter().filter(|n| rs.contains(n.as_str())).cloned().collect();
        { let mut m = digests.write(); m.clear();
          for n in &common { m.insert(n.clone(), DigestState::Computing); } }
        let ld = left_dir.read().clone();
        let rd = right_dir.read().clone();
        for name in common {
            let lp = ld.join(&name); let rp = rd.join(&name); let n2 = name.clone();
            let mut dg = digests;
            spawn(async move {
                let state = match tokio::task::spawn_blocking(move || file_digest_equal(&lp, &rp)).await {
                    Ok(Ok(true)) => DigestState::Equal, Ok(Ok(false)) => DigestState::Changed,
                    _ => DigestState::Err,
                };
                dg.write().insert(n2, state);
            });
        }
    });

    let right_names: HashSet<String> = listing_names(right_listing);
    let left_names:  HashSet<String> = listing_names(left_listing);

    // Summary counts for the compare bar.
    let summary = compute_summary(&left_names, &right_names, &digests.read());
    let left  = store.left_pick.read().clone();
    let right = store.right_pick.read().clone();
    let can_compare = left.is_some() && right.is_some();

    rsx! {
        div { class: "explorer",
            DirPane {
                label: t(lang, "Left / Old"), dir: left_dir,
                listing: left_listing, other_names: right_names.clone(), digests,
                other_dir: right_dir.read().clone(), is_left: true,
                on_select: move |p: PathBuf| store.left_pick.set(Some(p)),
                on_auto_compare: move |(l, r): (PathBuf, PathBuf)| open_compare(&mut store, l, r),
                on_chdir: move |_| {
                    refresh(left_dir, left_listing);
                    digests.write().clear();
                    store.settings.write().last_left_dir = Some(left_dir.read().clone());
                    crate::ui::settings::persist(&store.settings.read());
                },
            }
            DirPane {
                label: t(lang, "Right / New"), dir: right_dir,
                listing: right_listing, other_names: left_names.clone(), digests,
                other_dir: left_dir.read().clone(), is_left: false,
                on_select: move |p: PathBuf| store.right_pick.set(Some(p)),
                on_auto_compare: move |(l, r): (PathBuf, PathBuf)| open_compare(&mut store, l, r),
                on_chdir: move |_| {
                    refresh(right_dir, right_listing);
                    digests.write().clear();
                    store.settings.write().last_right_dir = Some(right_dir.read().clone());
                    crate::ui::settings::persist(&store.settings.read());
                },
            }
            div { class: "compare-bar",
                if can_compare {
                    button {
                        onclick: move |_| {
                            let picks = (store.left_pick.read().clone(), store.right_pick.read().clone());
                            if let (Some(l), Some(r)) = picks { open_compare(&mut store, l, r); }
                        },
                        {t(lang, "Compare")}
                    }
                    span { class: "info", {format!("{} ↔ {}", fname(&left), fname(&right))} }
                } else {
                    span { class: "hint summary",
                        if summary.common > 0 {
                            {format!("{} common", summary.common)}
                        }
                        if summary.changed > 0  { " · {summary.changed} changed"  }
                        if summary.left_only > 0  { " · {summary.left_only} left-only"  }
                        if summary.right_only > 0 { " · {summary.right_only} right-only" }
                        if summary.common == 0 && summary.left_only == 0 && summary.right_only == 0 {
                            {t(lang, "Select left, then right, then Compare.")}
                        }
                    }
                }
            }
        }
    }
}

fn compute_summary(
    left: &HashSet<String>, right: &HashSet<String>,
    digests: &HashMap<String, DigestState>,
) -> DirSummary {
    let mut s = DirSummary::default();
    for name in left.iter().chain(right.iter()).collect::<HashSet<_>>() {
        let in_l = left.contains(name); let in_r = right.contains(name);
        match (in_l, in_r) {
            (true, true) => {
                s.common += 1;
                if matches!(digests.get(name), Some(DigestState::Changed) | None) {
                    // None = not yet computed; optimistically count as unknown, not changed
                }
                if matches!(digests.get(name), Some(DigestState::Changed)) { s.changed += 1; }
            }
            (true, false) => s.left_only  += 1,
            (false, true) => s.right_only += 1,
            _ => {}
        }
    }
    s
}

// ─── Per-pane component ───────────────────────────────────────────────────────

#[component]
fn DirPane(
    label: String, dir: Signal<PathBuf>, listing: Signal<Option<DirectoryListing>>,
    other_names: HashSet<String>, digests: Signal<HashMap<String, DigestState>>,
    other_dir: PathBuf, is_left: bool,
    on_select: EventHandler<PathBuf>,
    on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
    on_chdir: EventHandler<()>,
) -> Element {
    let mut path_input = use_signal(|| dir.read().display().to_string());
    let mut focused_row: Signal<i32> = use_signal(|| -1);

    // Precompute row data and focused index outside rsx! to avoid
    // let-binding issues inside macro for-loops.
    let dir_rows: Vec<(usize, String)> = listing.read().as_ref()
        .map(|l| l.dirs.iter().enumerate().map(|(i, d)| (i, d.clone())).collect())
        .unwrap_or_default();
    let dir_count = dir_rows.len();
    let file_rows: Vec<(usize, FileEntry)> = listing.read().as_ref()
        .map(|l| l.files.iter().enumerate().map(|(i, f)| (dir_count + i, f.clone())).collect())
        .unwrap_or_default();
    let row_count = (dir_rows.len() + file_rows.len()) as i32;
    let fr = *focused_row.read(); // snapshot for this render

    // A flat list of (is_dir, name) for keyboard Enter handling.
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
                button { onclick: move |_| { go(&PathBuf::from(path_input.read().clone()), dir, listing); on_chdir.call(()); }, "→" }
                button {
                    onclick: move |_| {
                        let parent = dir.read().parent().map(|p| p.to_path_buf());
                        if let Some(p) = parent {
                            path_input.set(p.display().to_string());
                            go(&p, dir, listing); on_chdir.call(());
                        }
                    }, "↑"
                }
            }
            div {
                class: "dir-table",
                tabindex: "0",
                onkeydown: move |e| {
                    match e.key() {
                        Key::ArrowDown => {
                            let n = *focused_row.read(); focused_row.set((n + 1).min(row_count - 1));
                        }
                        Key::ArrowUp => {
                            let n = *focused_row.read(); focused_row.set((n - 1).max(0));
                        }
                        Key::Enter => {
                            let fi = *focused_row.read();
                            if let Some((is_dir, name)) = kbrows.get(fi as usize) {
                                if *is_dir {
                                    let next = dir.read().join(name);
                                    path_input.set(next.display().to_string());
                                    go(&next, dir, listing); on_chdir.call(());
                                } else {
                                    activate_file(name, dir.read().clone(), &other_dir, &other_names, is_left,
                                        on_select, on_auto_compare);
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
                            is_left,
                            focused: fr == row_idx as i32,
                            other_dir: other_dir.clone(),
                            on_select,
                            on_auto_compare,
                            base_dir: dir.read().clone(),
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
    other_dir: PathBuf,
    on_select: EventHandler<PathBuf>,
    on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
    base_dir: PathBuf,
) -> Element {
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
    rsx! {
        div {
            class: "{row_class}",
            onclick: move |_| activate_file(&name, base_dir.clone(), &other_dir, &other_names,
                                             is_left, on_select, on_auto_compare),
            span { class: "dir-status {sc}", title: "{icon}", "{icon}" }
            span { class: "dir-name", "{file.name}" }
            span { class: "dir-size", "{file.human_size}" }
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Single activation handler: auto-compare if file exists on both sides,
/// otherwise record as a single-side pick.
fn activate_file(
    name: &str,
    base_dir: impl Into<PathBuf>,
    other_dir: &std::path::Path,
    other_names: &HashSet<String>,
    is_left: bool,
    on_select: EventHandler<PathBuf>,
    on_auto_compare: EventHandler<(PathBuf, PathBuf)>,
) {
    let base = base_dir.into();
    let path = base.join(name);
    if other_names.contains(name) {
        let (l, r) = if is_left {
            (path, other_dir.join(name))
        } else {
            (other_dir.join(name), path)
        };
        on_auto_compare.call((l, r));
    } else {
        on_select.call(path);
    }
}

fn go(path: &std::path::Path, mut dir: Signal<PathBuf>, listing: Signal<Option<DirectoryListing>>) {
    if path.is_dir() { dir.set(path.to_path_buf()); refresh(dir, listing); }
}

fn refresh(dir: Signal<PathBuf>, mut listing: Signal<Option<DirectoryListing>>) {
    let p = dir.read().clone();
    listing.set(Some(list_dir(Some(&p)).unwrap_or(DirectoryListing {
        current_dir: p, dirs: vec![], files: vec![]
    })));
}

fn file_names(listing: Signal<Option<DirectoryListing>>) -> Vec<String> {
    listing.read().as_ref().map(|l| l.files.iter().map(|f| f.name.clone()).collect()).unwrap_or_default()
}

fn listing_names(listing: Signal<Option<DirectoryListing>>) -> HashSet<String> {
    listing.read().as_ref().map(|l| l.files.iter().map(|f| f.name.clone()).collect()).unwrap_or_default()
}

fn fname(p: &Option<PathBuf>) -> String {
    p.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "—".into())
}
