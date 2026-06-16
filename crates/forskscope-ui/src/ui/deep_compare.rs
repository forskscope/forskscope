//! Recursive directory comparison with incremental digest progress (RFC-037, RFC-038, RFC-040).
//!
//! Phase 1 (fast): `list_recursive_for_display` walks both trees; common files
//! get `RecStatus::Computing`.  Phase 2: per-file digest tasks update entries
//! in-place so the table refreshes as results arrive.

use std::path::PathBuf;

use dioxus::prelude::*;

use forskscope_core::dir::{RecEntry, RecStatus, file_digest_equal, list_recursive_for_display};

use crate::i18n::t;
use crate::state::{Lang, Store, open_compare};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DeepFilter { #[default] Different, All, Equal }

#[component]
pub fn DeepCompareView(left_root: PathBuf, right_root: PathBuf, lang: Lang) -> Element {
    // Clone once outside all closures so the props aren't moved piecemeal.
    let lr = left_root.clone();
    let rr = right_root.clone();

    let entries:      Signal<Vec<RecEntry>> = use_signal(Vec::new);
    #[allow(unused_mut)] let mut scanning:     Signal<bool>          = use_signal(|| true);
    #[allow(unused_mut)] let mut computed:     Signal<usize>         = use_signal(|| 0);
    #[allow(unused_mut)] let mut total_common: Signal<usize>         = use_signal(|| 0);
    let mut filter: Signal<DeepFilter>  = use_signal(DeepFilter::default);

    use_effect(move || {
        // Phase 1: fast listing (no I/O-heavy digests).
        let lr1 = lr.clone();
        let rr1 = rr.clone();
        let lr2 = lr.clone();   // for phase-2 absolute-path construction
        let rr2 = rr.clone();
        let mut ent = entries;
        let mut scan = scanning;
        let mut tc = total_common;
        let comp = computed;

        spawn(async move {
            let initial = tokio::task::spawn_blocking(move || list_recursive_for_display(&lr1, &rr1))
                .await.unwrap_or_default();

            // Build the list of (rel, abs_left, abs_right) for common pairs.
            let pairs: Vec<(PathBuf, PathBuf, PathBuf)> = initial.iter()
                .filter(|e| e.status == RecStatus::Computing)
                .map(|e| (e.rel_path.clone(), lr2.join(&e.rel_path), rr2.join(&e.rel_path)))
                .collect();

            tc.set(pairs.len());
            ent.set(initial);
            scan.set(false);

            // Phase 2: one digest task per common pair.
            for (rel, lp, rp) in pairs {
                let mut e2 = ent;
                let mut cp2 = comp;
                spawn(async move {
                    let equal = tokio::task::spawn_blocking(move || file_digest_equal(&lp, &rp))
                        .await.ok().and_then(|r| r.ok()).unwrap_or(false);
                    let status = if equal { RecStatus::Equal } else { RecStatus::Changed };
                    if let Some(entry) = e2.write().iter_mut().find(|e| e.rel_path == rel) {
                        entry.status = status;
                    }
                    let next = *cp2.read() + 1;
                    cp2.set(next);
                });
            }
        });
    });

    let f = *filter.read();
    let snap = entries.read();
    let total     = snap.len();
    let diff_cnt  = snap.iter().filter(|e| !matches!(e.status, RecStatus::Equal | RecStatus::Computing)).count();
    let done      = *computed.read();
    let tc        = *total_common.read();
    let is_scan   = *scanning.read();
    let in_flight = !is_scan && tc > 0 && done < tc;
    let visible: Vec<RecEntry> = snap.iter()
        .filter(|e| match f {
            DeepFilter::Different => e.status != RecStatus::Equal,
            DeepFilter::All       => true,
            DeepFilter::Equal     => e.status == RecStatus::Equal,
        })
        .cloned().collect();
    drop(snap);

    rsx! {
        div { class: "deep-compare",
            div { class: "deep-compare-toolbar",
                span { class: "deep-label", {t(lang, "Deep compare")} }
                button { class: if f==DeepFilter::Different {"filter-btn active"} else {"filter-btn"},
                    onclick: move |_| filter.set(DeepFilter::Different), {t(lang, "Different")} }
                button { class: if f==DeepFilter::All {"filter-btn active"} else {"filter-btn"},
                    onclick: move |_| filter.set(DeepFilter::All), {t(lang, "All")} }
                button { class: if f==DeepFilter::Equal {"filter-btn active"} else {"filter-btn"},
                    onclick: move |_| filter.set(DeepFilter::Equal), {t(lang, "Equal only")} }
                span { class: "spacer" }
                BatchCopyButtons { entries, left_root: left_root.clone(), right_root: right_root.clone() }
            }
            if is_scan {
                div { class: "deep-scanning", {t(lang, "Scanning…")} }
            } else {
                div { class: "deep-stats",
                    {format!("{} {} · {} {} · {} {}",
                        diff_cnt, t(lang, "different"),
                        total_common_eq(total, diff_cnt), t(lang, "equal"),
                        total, t(lang, "total"))}
                    if in_flight {
                        span { class: "deep-progress", {format!(" · {} {}/{}…", t(lang, "checking"), done, tc)} }
                    }
                }
                div { class: "deep-table",
                    for entry in visible { DeepRow { entry, lang } }
                }
            }
        }
    }
}

fn total_common_eq(total: usize, diff: usize) -> usize { total.saturating_sub(diff) }

#[component]
fn DeepRow(entry: RecEntry, lang: Lang) -> Element {
    let mut store = use_context::<Store>();
    let (icon, cls) = match entry.status {
        RecStatus::Changed   => ("⚠", "status-changed"),
        RecStatus::LeftOnly  => ("←", "status-only"),
        RecStatus::RightOnly => ("→", "status-only"),
        RecStatus::Equal     => ("✓", "status-equal"),
        RecStatus::Computing => ("⊙", "status-cmp"),
        RecStatus::Symlink   => ("↗", "status-symlink"),
    };
    let path_str   = entry.rel_path.display().to_string();
    let can_cmp    = !matches!(entry.status, RecStatus::Equal | RecStatus::Computing | RecStatus::Symlink);
    let e2 = entry.clone();
    rsx! {
        div { class: "deep-row",
            span { class: "dir-status {cls}", "{icon}" }
            span { class: "deep-path", "{path_str}" }
            span { class: "dir-size", {size_label(&entry)} }
            if can_cmp {
                button { class: "deep-compare-btn",
                    onclick: move |_| {
                        let s = store.settings.read();
                        if let (Some(lr), Some(rr)) = (&s.last_left_dir, &s.last_right_dir) {
                            let lp = lr.join(&e2.rel_path);
                            let rp = rr.join(&e2.rel_path);
                            drop(s);
                            open_compare(&mut store, lp, rp);
                        }
                    },
                    {t(lang, "Compare")}
                }
            }
        }
    }
}

fn size_label(e: &RecEntry) -> String {
    match (e.left_size, e.right_size) {
        (Some(l), Some(r)) if l != r => format!("{} → {}", fmt(l), fmt(r)),
        (Some(s), _) | (_, Some(s))  => fmt(s),
        _                             => String::new(),
    }
}
fn fmt(n: u64) -> String {
    if n < 1024 { format!("{n}B") }
    else if n < 1_048_576 { format!("{:.1}KB", n as f64 / 1024.0) }
    else { format!("{:.1}MB", n as f64 / 1_048_576.0) }
}

// ─── Batch copy buttons ───────────────────────────────────────────────────────

#[component]
fn BatchCopyButtons(entries: Signal<Vec<RecEntry>>, left_root: PathBuf, right_root: PathBuf) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let snap = entries.read();
    let has_changes = snap.iter().any(|e| !matches!(e.status, RecStatus::Equal | RecStatus::Computing));
    if !has_changes { return rsx! {}; }

    let lr = left_root.clone();
    let rr = right_root.clone();
    let lr2 = left_root;
    let rr2 = right_root;

    // "Copy all →" = copy left-only and changed files to the right tree
    let to_right: Vec<(PathBuf, PathBuf)> = snap.iter()
        .filter(|e| matches!(e.status, RecStatus::Changed | RecStatus::LeftOnly))
        .map(|e| (lr.join(&e.rel_path), rr.join(&e.rel_path)))
        .collect();
    // "Copy all ←" = copy right-only and changed files to the left tree
    let to_left: Vec<(PathBuf, PathBuf)> = snap.iter()
        .filter(|e| matches!(e.status, RecStatus::Changed | RecStatus::RightOnly))
        .map(|e| (rr2.join(&e.rel_path), lr2.join(&e.rel_path)))
        .collect();
    drop(snap);

    let tr_count = to_right.len();
    let tl_count = to_left.len();
    rsx! {
        if tr_count > 0 {
            button {
                class: "filter-btn",
                title: format!("{} {tr_count} {} →", t(lang, "Copy"), t(lang, "files to the right directory")),
                onclick: move |_| {
                    use crate::state::{BatchCopySpec, Modal};
                    store.modal.set(Modal::ConfirmBatchCopy(BatchCopySpec {
                        items: to_right.clone(),
                        label: format!("{} {tr_count} {} →",
                            t(lang, "Copy"), t(lang, "files to the right directory")),
                    }));
                },
                {format!("{} {tr_count} →", t(lang, "Copy"))}
            }
        }
        if tl_count > 0 {
            button {
                class: "filter-btn",
                title: format!("← {} {tl_count} {}", t(lang, "Copy"), t(lang, "files to the left directory")),
                onclick: move |_| {
                    use crate::state::{BatchCopySpec, Modal};
                    store.modal.set(Modal::ConfirmBatchCopy(BatchCopySpec {
                        items: to_left.clone(),
                        label: format!("← {} {tl_count} {}",
                            t(lang, "Copy"), t(lang, "files to the left directory")),
                    }));
                },
                {format!("← {} {tl_count}", t(lang, "Copy"))}
            }
        }
    }
}
