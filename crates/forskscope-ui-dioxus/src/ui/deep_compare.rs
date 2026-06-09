//! Recursive directory comparison view (RFC-037, RFC-038).
//!
//! Triggered from the Explorer workspace; shows a flat, sorted list of
//! every file that differs across the two directory trees. The scan runs
//! in `spawn_blocking` so the UI stays responsive. Clicking any row opens
//! a file comparison.

use std::path::PathBuf;

use dioxus::prelude::*;

use forskscope_core::dir::{RecEntry, RecStatus, recursive_diff};

use crate::i18n::t;
use crate::state::{Lang, Store, open_compare};

/// Filter applied to the deep-compare results table.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DeepFilter {
    #[default]
    Different,   // changed + left-only + right-only (default: show what's interesting)
    All,
    Equal,
}

#[component]
pub fn DeepCompareView(left_root: PathBuf, right_root: PathBuf, lang: Lang) -> Element {
    let results: Signal<Option<Vec<RecEntry>>> = use_signal(|| None);
    let mut filter: Signal<DeepFilter> = use_signal(DeepFilter::default);

    // Launch the background scan once on mount.
    use_effect(move || {
        let lr = left_root.clone();
        let rr = right_root.clone();
        let mut res = results;
        spawn(async move {
            let entries = tokio::task::spawn_blocking(move || recursive_diff(&lr, &rr))
                .await
                .unwrap_or_default();
            res.set(Some(entries));
        });
    });

    let f = *filter.read();
    rsx! {
        div { class: "deep-compare",
            div { class: "deep-compare-toolbar",
                span { class: "deep-label", "Deep compare" }
                button {
                    class: if f == DeepFilter::Different { "filter-btn active" } else { "filter-btn" },
                    onclick: move |_| filter.set(DeepFilter::Different),
                    "Different"
                }
                button {
                    class: if f == DeepFilter::All { "filter-btn active" } else { "filter-btn" },
                    onclick: move |_| filter.set(DeepFilter::All),
                    "All"
                }
                button {
                    class: if f == DeepFilter::Equal { "filter-btn active" } else { "filter-btn" },
                    onclick: move |_| filter.set(DeepFilter::Equal),
                    "Equal only"
                }
            }
            match results.read().as_ref() {
                None => rsx! { div { class: "deep-scanning", "Scanning…" } },
                Some(entries) => {
                    let visible: Vec<RecEntry> = entries.iter()
                        .filter(|e| match f {
                            DeepFilter::Different => e.status != RecStatus::Equal,
                            DeepFilter::All       => true,
                            DeepFilter::Equal     => e.status == RecStatus::Equal,
                        })
                        .cloned()
                        .collect();
                    let total_diff = entries.iter().filter(|e| e.status != RecStatus::Equal).count();
                    rsx! {
                        div { class: "deep-stats",
                            {format!("{} different · {} equal · {} total",
                                total_diff,
                                entries.len() - total_diff,
                                entries.len()
                            )}
                        }
                        div { class: "deep-table",
                            for entry in visible {
                                DeepRow { entry: entry.clone(), lang }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DeepRow(entry: RecEntry, lang: Lang) -> Element {
    let mut store = use_context::<Store>();
    let (icon, icon_class) = match entry.status {
        RecStatus::Changed  => ("⚠", "status-changed"),
        RecStatus::LeftOnly  => ("←", "status-only"),
        RecStatus::RightOnly => ("→", "status-only"),
        RecStatus::Equal     => ("✓", "status-equal"),
    };
    let path_str = entry.rel_path.display().to_string();
    let can_compare = matches!(entry.status, RecStatus::Changed | RecStatus::LeftOnly | RecStatus::RightOnly);
    let e2 = entry.clone();
    rsx! {
        div { class: "deep-row",
            span { class: "dir-status {icon_class}", "{icon}" }
            span { class: "deep-path", "{path_str}" }
            span { class: "dir-size", {size_label(&entry)} }
            if can_compare {
                button {
                    class: "deep-compare-btn",
                    onclick: move |_| {
                        // Construct full paths from the stored left/right roots
                        // via the store's settings (last_left_dir / last_right_dir).
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
        (Some(l), Some(r)) if l != r => format!("{l} → {r}"),
        (Some(l), Some(_)) => fmt_size(l),
        (Some(l), None) => fmt_size(l),
        (None, Some(r)) => fmt_size(r),
        _ => String::new(),
    }
}

fn fmt_size(n: u64) -> String {
    if n < 1024 { format!("{n} B") }
    else if n < 1024 * 1024 { format!("{:.1} KB", n as f64 / 1024.0) }
    else { format!("{:.1} MB", n as f64 / (1024.0 * 1024.0)) }
}
