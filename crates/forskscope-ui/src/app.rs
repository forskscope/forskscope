//! Application root with global keyboard shortcuts and accessibility (RFC-003, RFC-019, RFC-046).

use dioxus::prelude::*;
use forskscope_ui_logic::StartupRequest;

use crate::state::{
    Store, advance_recovery_queue, open_compare_request, resolve_session, restore_tabs,
};
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

    // RFC-089 §2: the desktop window's title follows the selected tab, its
    // title text, and its unsaved state.
    //
    // Set directly in the render body, not from a `use_effect` — live
    // testing found that this dioxus-desktop integration's effect queue
    // lags renders by several ticks in this environment (confirmed by
    // instrumenting both: a render's own `eprintln!` fires on every real
    // state change, but a `use_effect` queued by that same change sat
    // unflushed through multiple further actions and 8+ idle seconds,
    // only running once *another* unrelated action arrived). A stale
    // window title for an unpredictable number of actions is a real,
    // visible defect, not a cosmetic one, so this follows render's own
    // prompt timing instead of depending on effect flushing — the same
    // reasoning this codebase already applies to session saves (F194:
    // `set_active_tab` calls `save_session` directly, not via an effect).
    // `use_window()` is a real hook (`use_hook` underneath) and must be
    // called unconditionally at the top level, every render, in the same
    // order — never from inside a conditional or a closure.
    let desktop = dioxus_desktop::use_window();
    // `Rc<RefCell<_>>`, not a bare `RefCell<_>`: `use_hook` clones its
    // stored value on every call (its `State: Clone` bound), and cloning a
    // bare `RefCell` would hand back an independent copy each render —
    // mutations through it would never reach the next render's clone. `Rc`
    // clones cheaply (a refcount bump) while every clone still shares the
    // one underlying cell.
    let last_window_title = use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(String::new())));
    {
        let (selected_title, dirty): (Option<String>, bool) =
            match (*store.active.read(), *store.active_dir.read()) {
                (Some(i), _) => {
                    let tabs = store.tabs.read();
                    match tabs.get(i) {
                        Some(t) => (Some(t.title.clone()), t.can_save && t.merge.is_dirty()),
                        None => (None, false),
                    }
                }
                (None, Some(j)) => {
                    let dir_tabs = store.dir_tabs.read();
                    let title = dir_tabs
                        .get(j)
                        .map(|(l, r)| crate::state::dir_tab_title(l, r));
                    (title, false)
                }
                (None, None) => (None, false),
            };
        let title = window_title(selected_title.as_deref(), dirty);
        // A plain `RefCell`, not a `Signal` — this is a side-effect guard
        // read and written in the same render pass, never read by `rsx!`,
        // so it must never participate in Dioxus's own reactivity (writing
        // a `Signal` during render, rather than in an event handler or
        // effect, schedules a further render on every call).
        if *last_window_title.borrow() != title {
            desktop.window.set_title(&title);
            *last_window_title.borrow_mut() = title;
        }
    }

    let theme_class = store.settings.read().theme.css_class();
    // RFC-088: a second, independent root class — "" for Standard, so the
    // root carries no extra class and only the theme's own tokens apply.
    let cv_class =
        crate::state::settings::colour_vision_css_class(store.settings.read().colour_vision);
    let active = *store.active.read();
    let toast = store.toast.read().cloned();

    rsx! {
        style { {MAIN_CSS} }
        div {
            class: "app {theme_class} {cv_class}",
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
                        // Ctrl+W: close the active tab, through the same path
                        // the tab's × button and Delete on a focused tab use.
                        if let Some(index) = active {
                            crate::state::request_close_file_tab(&mut store, index);
                        }
                    }
                    GlobalKeyAction::OpenKeyboardRef => store.modal.set(crate::state::Modal::KeyboardRef),
                    GlobalKeyAction::NextTab => crate::state::activate_adjacent_tab(&mut store, true),
                    GlobalKeyAction::PrevTab => crate::state::activate_adjacent_tab(&mut store, false),
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
            TabBar {}
            crate::keyboard::ControlZone {
                class: "body",
                role: "tabpanel",
                // RFC-087 §1: the content area is labelled by the tab that
                // is actually selected, by the same flat position its own
                // `id="tab-{position}"` (tabs.rs) is rendered with.
                aria_labelledby: {
                    let file_count = store.tabs.read().len();
                    let position = crate::state::tab_nav::position_of(
                        active,
                        *store.active_dir.read(),
                        file_count,
                    );
                    format!("tab-{position}")
                },
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
                    let action = notice.action.clone();
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
                            if let Some(action) = action {
                                button {
                                    class: "toast-action",
                                    onclick: move |e| {
                                        // RFC-092 §2: stays on the toast's own
                                        // click-anywhere-to-dismiss, but this
                                        // click must act (open the file), not
                                        // merely dismiss - stop it reaching the
                                        // outer div's onclick.
                                        e.stop_propagation();
                                        if let Err(err) = forskscope_core::external_tool::open_path(&action.path) {
                                            eprintln!("open_path: {err} (path: {})", action.path);
                                        }
                                        store.toast.set(None);
                                    },
                                    "{action.label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// RFC-089 §2: the desktop window's title for the given selection state —
/// pure, so the mapping itself is unit-testable independent of the
/// `set_title` call; the effect above is `window_title`'s only caller.
/// `selected` is `None` for the Explorer, `Some(title)` for a file or
/// folder comparison; `dirty` is only ever true for a file comparison.
fn window_title(selected: Option<&str>, dirty: bool) -> String {
    match selected {
        None => "ForskScope".to_string(),
        Some(title) if dirty => format!("● {title} — ForskScope"),
        Some(title) => format!("{title} — ForskScope"),
    }
}

#[cfg(test)]
mod tests {
    use super::window_title;

    /// RFC-089 §2's table, one row per case. Falsify by deleting any one
    /// arm's distinguishing behaviour (the dirty-bullet prefix, or the
    /// em-dash suffix): the matching assertion fails.
    #[test]
    fn window_title_follows_rfc_089_table() {
        assert_eq!(window_title(None, false), "ForskScope", "Explorer");
        assert_eq!(
            window_title(Some("app.js ↔ app.js"), false),
            "app.js ↔ app.js — ForskScope",
            "a file comparison"
        );
        assert_eq!(
            window_title(Some("app.js ↔ app.js"), true),
            "● app.js ↔ app.js — ForskScope",
            "a file comparison with unsaved changes"
        );
        assert_eq!(
            window_title(Some("src ↔ src"), false),
            "src ↔ src — ForskScope",
            "a folder comparison"
        );
    }
}
