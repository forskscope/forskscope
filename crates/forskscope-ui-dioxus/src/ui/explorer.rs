//! Explorer workspace coordination (RFC-005).
//!
//! This module owns the two-pane state (directories, listings, digests)
//! and delegates all rendering to [`crate::ui::dir_pane`].

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use dioxus::prelude::*;

use forskscope_core::dir::{DirectoryListing, file_digest_equal, list_dir};

use crate::i18n::t;
use crate::state::{Store, open_compare};
use crate::ui::dir_pane::DirPane;

/// Digest comparison state for a same-name file pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestState { Computing, Equal, Changed, Err }

/// Summary counts shown in the compare bar.
#[derive(Default, Clone)]
pub struct DirSummary { pub common: usize, pub changed: usize,
                         pub left_only: usize, pub right_only: usize }

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let init_l = store.settings.read().last_left_dir.clone().unwrap_or_else(|| cwd.clone());
    let init_r = store.settings.read().last_right_dir.clone().unwrap_or_else(|| cwd.clone());
    let left_dir  = use_signal(|| init_l);
    let right_dir = use_signal(|| init_r);
    let left_listing:  Signal<Option<DirectoryListing>> = use_signal(|| None);
    let right_listing: Signal<Option<DirectoryListing>> = use_signal(|| None);
    let mut digests: Signal<HashMap<String, DigestState>> = use_signal(HashMap::new);

    use_effect(move || {
        if left_listing.read().is_none()  { refresh(left_dir,  left_listing); }
        if right_listing.read().is_none() { refresh(right_dir, right_listing); }
    });

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
                let s = match tokio::task::spawn_blocking(move || file_digest_equal(&lp, &rp)).await {
                    Ok(Ok(true)) => DigestState::Equal, Ok(Ok(false)) => DigestState::Changed,
                    _ => DigestState::Err,
                };
                dg.write().insert(n2, s);
            });
        }
    });

    let right_names: HashSet<String> = listing_names(right_listing);
    let left_names:  HashSet<String> = listing_names(left_listing);
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
                    refresh(left_dir, left_listing); digests.write().clear();
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
                    refresh(right_dir, right_listing); digests.write().clear();
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
                        if summary.common > 0 || summary.left_only > 0 || summary.right_only > 0 {
                            {format!("{} common · {} changed · {} left-only · {} right-only",
                                summary.common, summary.changed, summary.left_only, summary.right_only)}
                        } else {
                            {t(lang, "Select left, then right, then Compare.")}
                        }
                    }
                }
            }
        }
    }
}

// ─── Helpers exported for dir_pane.rs ────────────────────────────────────────

pub fn refresh(dir: Signal<PathBuf>, mut listing: Signal<Option<DirectoryListing>>) {
    let p = dir.read().clone();
    listing.set(Some(list_dir(Some(&p)).unwrap_or(DirectoryListing {
        current_dir: p, dirs: vec![], files: vec![]
    })));
}

fn file_names(l: Signal<Option<DirectoryListing>>) -> Vec<String> {
    l.read().as_ref().map(|l| l.files.iter().map(|f| f.name.clone()).collect()).unwrap_or_default()
}

fn listing_names(l: Signal<Option<DirectoryListing>>) -> HashSet<String> {
    l.read().as_ref().map(|l| l.files.iter().map(|f| f.name.clone()).collect()).unwrap_or_default()
}

fn fname(p: &Option<PathBuf>) -> String {
    p.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "—".into())
}

fn compute_summary(l: &HashSet<String>, r: &HashSet<String>,
                   d: &HashMap<String, DigestState>) -> DirSummary {
    let mut s = DirSummary::default();
    let all: HashSet<&String> = l.iter().chain(r.iter()).collect();
    for name in all {
        match (l.contains(name), r.contains(name)) {
            (true, true) => {
                s.common += 1;
                if matches!(d.get(name), Some(DigestState::Changed)) { s.changed += 1; }
            }
            (true, false) => s.left_only  += 1,
            (false, true) => s.right_only += 1,
            _ => {}
        }
    }
    s
}
