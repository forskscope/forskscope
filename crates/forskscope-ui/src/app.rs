//! Application root with global keyboard shortcuts and accessibility (RFC-003, RFC-019, RFC-046).

use dioxus::prelude::*;
use forskscope_ui_logic::StartupRequest;

use crate::state::{
    Store, advance_recovery_queue, open_compare_request, resolve_session, restore_tabs,
};
use crate::ui::layout::header::Header;
use crate::ui::layout::statusbar::StatusBar;
use crate::ui::layout::tabs::TabBar;
use crate::ui::view::diff::{DiffWorkspace, apply_focused_hunk, move_focus, save_tab};
use crate::ui::view::explorer::Explorer;
use crate::ui::view::settings::{ModalLayer, load};

// Assembled at generation time from assets/css/ (see assets/css/ORDER.txt).
// Regenerate with: cargo xtask css
const MAIN_CSS: &str = include_str!("../assets/main.css");

/// Set once by `main()` from parsed CLI arguments (RFC-077), before the
/// Dioxus event loop starts. `App()` reads it exactly once, in its startup
/// `use_hook`. Falls back to `StartupRequest::Explorer` if never set (e.g. a
/// future test harness that constructs `App` without going through `main`).
pub static STARTUP_REQUEST: std::sync::OnceLock<StartupRequest> = std::sync::OnceLock::new();

#[component]
pub fn App() -> Element {
    let mut store = use_context_provider(|| {
        let (settings, resolution) = load();
        let mut store = Store::new(
            settings,
            resolution.value.clone(),
            resolution.write_disabled,
        );
        if let Some(notice) = crate::ui::view::settings::recovery_notice(&resolution) {
            store.toast.set(Some(notice));
        }
        if let Some(modal) = crate::ui::view::settings::recovery_modal(&resolution) {
            store.pending_recovery.write().push(modal);
        }
        store
    });

    use_hook(|| {
        // Resolving the session file (and setting session_write_disabled)
        // must happen unconditionally, before branching on a CLI startup
        // pair — review 041 C1: a CLI launch never restores tabs from the
        // session file, but still opens a tab and triggers a save, which
        // must not silently overwrite a future/corrupt session.json.
        let (session_resolution, session_notice) = resolve_session(&mut store);
        if store.toast.read().is_none()
            && let Some(notice) = session_notice
        {
            store.toast.set(Some(notice));
        }
        if let Some(modal) = crate::state::session::recovery_modal(&session_resolution) {
            store.pending_recovery.write().push(modal);
        }
        // F28b: settings and session resolve independently and either (or
        // both) can need a blocking dialog on the same launch — both pushes
        // above have landed by now, so this shows the first without dropping
        // a second queued behind it.
        advance_recovery_queue(&mut store);

        // RFC-077: a single typed request replaces the STARTUP_PAIR/
        // STARTUP_MERGED pair. `into_compare_request()` is where
        // normal-vs-mergetool save destination is decided; `open_compare_request`
        // installs it atomically with the tab, so there is no window where
        // `right_path` means one thing and the save target means another.
        let startup_request = STARTUP_REQUEST
            .get()
            .cloned()
            .unwrap_or(StartupRequest::Explorer);
        match startup_request.into_compare_request() {
            Some(request) => open_compare_request(&mut store, request),
            None => {
                // No explicit startup pair — restore the previous session (RFC-035).
                restore_tabs(&mut store, &session_resolution);
            }
        }
    });

    // Update the OS window title to reflect the active comparison.
    use_effect(move || {
        let title = match *store.active.read() {
            Some(i) => store
                .tabs
                .read()
                .get(i)
                .map(|t| format!("ForskScope — {}", t.title))
                .unwrap_or_else(|| "ForskScope".into()),
            None => "ForskScope".into(),
        };
        spawn(async move {
            let _ = dioxus::document::eval(&format!("document.title = {:?}", title)).await;
        });
    });

    let theme_class = store.settings.read().theme.css_class();
    let active = *store.active.read();
    let toast = store.toast.read().cloned();

    rsx! {
        style { {MAIN_CSS} }
        div {
            class: "app {theme_class}",
            id: "app-root",
            tabindex: "-1",
            onmounted: move |_| {
                spawn(async move {
                    let _ = dioxus::document::eval(
                        "document.getElementById('app-root')?.focus();"
                    ).await;
                });
            },
            // RFC-060/handoff 020: the decision of what a global keypress
            // should do lives in `crate::keyboard::global_key_action`, a
            // pure function this closure calls into — this stays a thin
            // dispatcher over its result so the decision itself (including
            // the modal-swallow and RFC-076 patch 6's recovery exclusion)
            // is testable without a Dioxus event closure in the loop.
            onkeydown: move |e: Event<KeyboardData>| {
                use crate::keyboard::{GlobalKeyAction, ModalState, global_key_action};

                let modal = ModalState::from_modal(&store.modal.read());
                let mods = e.modifiers();
                let active = *store.active.read();
                let action = global_key_action(&e.key(), mods, modal, active.is_some());

                match action {
                    GlobalKeyAction::Ignore => {}
                    GlobalKeyAction::CloseModal => store.modal.set(crate::state::Modal::None),
                    GlobalKeyAction::MoveFocus(delta) => {
                        if let Some(index) = active { move_focus(&mut store, index, delta); }
                    }
                    GlobalKeyAction::SearchNext => {
                        spawn(async move {
                            let _ = dioxus::document::eval(
                                "document.getElementById('search-next-btn')?.click();"
                            ).await;
                        });
                    }
                    GlobalKeyAction::SearchPrev => {
                        spawn(async move {
                            let _ = dioxus::document::eval(
                                "document.getElementById('search-prev-btn')?.click();"
                            ).await;
                        });
                    }
                    GlobalKeyAction::ApplyFocusedHunk => {
                        if let Some(index) = active { apply_focused_hunk(&mut store, index); }
                    }
                    GlobalKeyAction::Save => {
                        if let Some(index) = active { save_tab(&mut store, index); }
                    }
                    GlobalKeyAction::Undo => {
                        if let Some(index) = active {
                            let _ = store.tabs.write().get_mut(index).map(|t| t.merge.undo());
                        }
                    }
                    GlobalKeyAction::Redo => {
                        if let Some(index) = active {
                            let _ = store.tabs.write().get_mut(index).map(|t| t.merge.redo());
                        }
                    }
                    GlobalKeyAction::RequestCloseTab => {
                        if let Some(index) = active {
                            // Ctrl+W: close the active tab, with dirty-state guard.
                            let dirty = store.tabs.read().get(index)
                                .map(|t| t.can_save && t.merge.is_dirty())
                                .unwrap_or(false);
                            if dirty {
                                store.modal.set(crate::state::Modal::ConfirmClose(index));
                            } else {
                                crate::state::close_tab(&mut store, index);
                            }
                        }
                    }
                    GlobalKeyAction::OpenKeyboardRef => store.modal.set(crate::state::Modal::KeyboardRef),
                    GlobalKeyAction::OpenSearch => {
                        // Ctrl+F: the search bar inside DiffWorkspace handles its own
                        // context; we use document::eval to click the search button.
                        spawn(async move {
                            let _ = dioxus::document::eval(
                                "document.getElementById('search-open-btn')?.click();"
                            ).await;
                        });
                    }
                }
            },
            Header {}
            TabBar {}
            div { class: "body",
                match (active, *store.active_dir.read()) {
                    (_, Some(dir_idx)) => {
                        let dir_tabs = store.dir_tabs.read();
                        if let Some((l, r)) = dir_tabs.get(dir_idx).cloned() {
                            let lang = store.lang();
                            drop(dir_tabs);
                            rsx! { crate::ui::view::deep_compare::DeepCompareView { left_root: l, right_root: r, lang } }
                        } else {
                            rsx! { Explorer {} }
                        }
                    }
                    (None, None)        => rsx! { Explorer {} },
                    (Some(index), None) => rsx! { DiffWorkspace { index } },
                }
            }
            StatusBar {}
            ModalLayer {}
            if let Some(notice) = toast {
                {
                    let severity_class = match notice.severity {
                        crate::state::NoticeSeverity::Success => "toast toast-success",
                        crate::state::NoticeSeverity::Info    => "toast toast-info",
                        crate::state::NoticeSeverity::Warning => "toast toast-warning",
                        crate::state::NoticeSeverity::Error   => "toast toast-error",
                    };
                    let message = notice.message.clone();
                    // Auto-dismiss for Success and Info notices.
                    if let Some(ms) = notice.auto_dismiss_ms() {
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                            store.toast.set(None);
                        });
                    }
                    rsx! {
                        div {
                            class: "{severity_class}",
                            role: "status",
                            aria_live: "polite",
                            onclick: move |_| store.toast.set(None),
                            "{message}"
                        }
                    }
                }
            }
        }
    }
}
