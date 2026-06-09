//! Explorer workspace: two tree-view panes + compare controls (RFC-054).

use std::path::{Path, PathBuf};

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Store, open_compare};
use crate::ui::deep_compare::DeepCompareView;
use crate::ui::dir_pane::DirPane;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ExplorerMode { #[default] Browse, Deep }

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut mode: Signal<ExplorerMode> = use_signal(ExplorerMode::default);

    // Derive ignore rules from settings for both panes.
    let ignore = store.settings.read().ignore_rules();

    let left_pick  = store.left_pick.read().clone();
    let right_pick = store.right_pick.read().clone();
    let can_compare = left_pick.is_some() && right_pick.is_some();

    let on_auto_compare = move |(l, r): (PathBuf, PathBuf)| {
        open_compare(&mut store, l, r);
    };

    // For the deep compare view, we need the current roots (from settings).
    let left_root  = store.settings.read().last_left_dir.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
    let right_root = store.settings.read().last_right_dir.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));

    rsx! {
        div { class: "explorer",
            // ── Panes ──────────────────────────────────────────────
            if *mode.read() == ExplorerMode::Browse {
                div { class: "explorer-panes",
                    DirPane { is_left: true,  ignore: ignore.clone(), on_auto_compare }
                    DirPane { is_left: false, ignore: ignore.clone(), on_auto_compare }
                }
            } else {
                DeepCompareView { left_root, right_root, lang }
            }
            // ── Footer toolbar ─────────────────────────────────────
            div { class: "explorer-footer",
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
                // Show selected pair label.
                if let (Some(l), Some(r)) = (&left_pick, &right_pick) {
                    span { class: "compare-label",
                        {format!("{} ↔ {}", short_name(l), short_name(r))}
                    }
                }
                span { class: "spacer" }
                button {
                    onclick: move |_| {
                        let next = if *mode.read() == ExplorerMode::Browse {
                            ExplorerMode::Deep
                        } else {
                            ExplorerMode::Browse
                        };
                        mode.set(next);
                    },
                    if *mode.read() == ExplorerMode::Browse { "⟳ Deep compare" } else { "← Browse" }
                }
            }
        }
    }
}

fn short_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}
