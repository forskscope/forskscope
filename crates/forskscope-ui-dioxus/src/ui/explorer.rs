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

    let ignore = store.settings.read().ignore_rules();

    let left_pick  = store.left_pick.read().clone();
    let right_pick = store.right_pick.read().clone();
    let can_compare = left_pick.is_some() && right_pick.is_some();

    let on_auto_compare = move |(l, r): (PathBuf, PathBuf)| {
        open_compare(&mut store, l, r);
    };

    let left_root  = store.settings.read().last_left_dir.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
    let right_root = store.settings.read().last_right_dir.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));

    rsx! {
        div { class: "explorer",
            // Mode selector toolbar (explains what each mode does).
            div { class: "explorer-modes",
                button {
                    class: if *mode.read() == ExplorerMode::Browse { "mode-btn active" } else { "mode-btn" },
                    title: "Browse and navigate directories side by side",
                    onclick: move |_| mode.set(ExplorerMode::Browse),
                    "Browse"
                }
                button {
                    class: if *mode.read() == ExplorerMode::Deep { "mode-btn active" } else { "mode-btn" },
                    title: "Recursively compare all files in both directories and show a full status report",
                    onclick: move |_| mode.set(ExplorerMode::Deep),
                    "Directory Report"
                }
            }

            // ── Active workspace ─────────────────────────────────────────────
            if *mode.read() == ExplorerMode::Browse {
                div { class: "explorer-panes",
                    DirPane { is_left: true,  ignore: ignore.clone(), on_auto_compare }
                    DirPane { is_left: false, ignore: ignore.clone(), on_auto_compare }
                }
            } else {
                DeepCompareView { left_root, right_root, lang }
            }

            // ── Footer (Browse mode only) ────────────────────────────────────
            if *mode.read() == ExplorerMode::Browse {
                div { class: "explorer-footer",
                    button {
                        disabled: !can_compare,
                        onclick: move |_| {
                            let l = store.left_pick.read().clone();
                            let r = store.right_pick.read().clone();
                            if let (Some(l), Some(r)) = (l, r) { open_compare(&mut store, l, r); }
                        },
                        {t(lang, "Compare")}
                    }
                    if let (Some(l), Some(r)) = (&left_pick, &right_pick) {
                        span { class: "compare-label",
                            {format!("{} ↔ {}", short_name(l), short_name(r))}
                        }
                    }
                }
            }
        }
    }
}

fn short_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}
