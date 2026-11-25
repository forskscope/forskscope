//! Application root with global keyboard shortcuts and accessibility (RFC-003, RFC-019, RFC-046).

use dioxus::prelude::*;
use forskscope_ui_logic::StartupRequest;

use crate::i18n::t;
use crate::state::{
    StartupAction, Store, advance_recovery_queue, classify_startup_request, fs_probe,
    open_compare_request, open_dir_compare, open_startup_mismatch_error, resolve_session,
    restore_tabs,
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

/// RFC-102 §2: on a focus-taking (error/warning) toast's mount, save
/// `document.activeElement` — but only if nothing is already saved,
/// since a second error replacing the first (`Notice.id`'s own doc
/// comment) must not overwrite the *original* previous focus with the
/// first toast's own Dismiss button — then move focus to the toast's
/// one focusable element (its Dismiss button, or its action button
/// where it has one — `rsx!` below renders only one of the two).
const SAVE_FOCUS_AND_FOCUS_NOTICE_JS: &str = "\
    if (!window.__fskNoticeReturnFocus) { \
        window.__fskNoticeReturnFocus = document.activeElement; \
    } \
    const el = document.querySelector('.toast [autofocus]'); \
    if (el) { el.focus(); }";

/// The other half: Escape or *Dismiss* returns focus to whatever had it
/// before the notice appeared, falling back to the app's own starting
/// point (`#app-root`) if that element is gone (RFC-102 §2: "If that
/// element is gone, focus goes to the main view's own starting point").
const RETURN_FOCUS_JS: &str = "\
    const el = window.__fskNoticeReturnFocus; \
    window.__fskNoticeReturnFocus = null; \
    if (el && document.body.contains(el) && typeof el.focus === 'function') { \
        el.focus(); \
    } else { \
        document.getElementById('app-root')?.focus(); \
    }";

/// RFC-102 §2: dismiss a focus-taking toast and return focus — the one
/// path both Escape (`GlobalKeyAction::CloseToast`) and the toast's own
/// Dismiss/action button go through, so the two can never drift apart.
fn dismiss_focus_taking_toast(store: &mut Store) {
    store.toast.set(None);
    spawn(async move {
        let _ = dioxus::document::eval(RETURN_FOCUS_JS).await;
    });
}

#[component]
pub fn App() -> Element {
    let mut store = use_context_provider(|| {
        let (settings, resolution, initial_system_color_scheme) = load();
        let mut store = Store::new(
            settings,
            resolution.value.clone(),
            resolution.write_disabled,
        );
        store.seed_system_color_scheme(initial_system_color_scheme);
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
        //
        // F209: before that conversion, `classify_startup_request` tells
        // two folders (open Deep Compare, never a file tab) and a
        // file/folder mismatch (refused in-window, never a raw I/O error)
        // apart from an ordinary file+file launch, which proceeds exactly
        // as before.
        let startup_request = STARTUP_REQUEST
            .get()
            .cloned()
            .unwrap_or(StartupRequest::Explorer);
        match classify_startup_request(startup_request, fs_probe) {
            StartupAction::Explorer => {
                // No explicit startup pair — restore the previous session (RFC-035).
                restore_tabs(&mut store, &session_resolution);
            }
            StartupAction::AsRequested(request) => match request.into_compare_request() {
                Some(compare_request) => open_compare_request(&mut store, compare_request),
                None => restore_tabs(&mut store, &session_resolution),
            },
            StartupAction::CompareFolders { left, right } => {
                open_dir_compare(&mut store, left, right);
            }
            StartupAction::Refused {
                left,
                right,
                message,
            } => open_startup_mismatch_error(&mut store, left, right, message),
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
    // Review 212 §2: Windows' own light/dark preference is read from
    // tao's window frame, not the web view's `matchMedia` - measured live
    // that `WM_SETTINGCHANGE`'s broadcast repaints tao's own titlebar but
    // never reaches WebView2's query (`state::settings::
    // spawn_system_preference_bridges`'s own doc comment has the full
    // story). `use_window`/`use_wry_event_handler` are real hooks and
    // must be called unconditionally, every render, in the same order -
    // `#[cfg(target_os = "windows")]` on the whole block is fine (that's
    // resolved at compile time, so the call order is still identical
    // across every render on whichever platform actually built it).
    #[cfg(target_os = "windows")]
    {
        use_hook(|| {
            store.system_color_scheme.set(Some(
                crate::state::settings::tao_theme_to_system_color_scheme(desktop.theme()),
            ));
        });
        dioxus_desktop::use_wry_event_handler({
            let window_id = desktop.window.id();
            let mut store = store;
            move |event, _| {
                if let dioxus_desktop::tao::event::Event::WindowEvent {
                    window_id: event_window_id,
                    event: dioxus_desktop::tao::event::WindowEvent::ThemeChanged(theme),
                    ..
                } = event
                    && *event_window_id == window_id
                {
                    store.system_color_scheme.set(Some(
                        crate::state::settings::tao_theme_to_system_color_scheme(*theme),
                    ));
                }
            }
        });
    }
    // RFC-098 stage A (handoff 101): a no-op in every ordinary build
    // (feature off) and a no-op even with the feature on unless
    // `layout_probe::PLAN_ENV_VAR` is set - `use_hook` itself must still
    // run unconditionally, every render, in the same order as every other
    // hook above (resolved at compile time by the `#[cfg]`, same as the
    // Windows-only block above it).
    #[cfg(feature = "layout-probe")]
    use_hook(|| {
        crate::layout_probe::maybe_start(store, desktop.clone());
    });
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

    // RFC-097 stage 1 (handoff 095 §2): the whole web view zooms by the
    // Text size setting — `wry::WebView::zoom`, reached here as
    // `set_zoom_level` on the same `desktop` context used for the window
    // title above. Same reason, same shape as that guard: called directly
    // in the render body rather than from a `use_effect`, so this applies
    // on the very first render (as early as "before the first paint" can
    // mean in a webview-hosted app) as well as on every later change, with
    // no dependency on this integration's effect queue actually flushing.
    // `0u32` as the sentinel guarantees the first render always differs
    // from a real step (100-200) and so always applies once at startup.
    let last_text_size = use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(0u32)));
    {
        let text_size = store.settings.read().text_size;
        if *last_text_size.borrow() != text_size {
            desktop.set_zoom_level(f64::from(text_size) / 100.0);
            *last_text_size.borrow_mut() = text_size;
        }
    }

    // RFC-099 §2: the window's own title bar (and on Linux, any other
    // GTK-drawn chrome) follows ForskScope's own resolved theme, not only
    // the desktop's — same guarded-write shape as the zoom level above,
    // for the same reason (applies on the very first render as well as
    // every later change, with no dependency on an effect ever flushing).
    // `None` as the sentinel, not a `Theme` value: tao's own `set_theme`
    // takes `Option<tao::window::Theme>`, and passing `None` to *it* means
    // "follow the OS" (and is read as light on Linux regardless of the
    // desktop's own dark setting — the RFC's own explicit warning) — a
    // different thing entirely from this guard's own "never applied yet".
    let last_window_theme = use_hook(|| {
        std::rc::Rc::new(std::cell::RefCell::new(
            None::<dioxus_desktop::tao::window::Theme>,
        ))
    });
    {
        let resolved = crate::state::settings::resolved_tao_theme(
            store.settings.read().theme,
            *store.system_color_scheme.read(),
        );
        if *last_window_theme.borrow() != Some(resolved) {
            desktop.window.set_theme(Some(resolved));
            *last_window_theme.borrow_mut() = Some(resolved);
        }
    }

    // RFC-097 stage 2 §2.1: Theme::System resolves through the live OS
    // reading (Windows/Linux only — see the onmounted tasks below), never
    // through a context-free class.
    let theme_class = crate::state::settings::resolved_theme_css_class(
        store.settings.read().theme,
        *store.system_color_scheme.read(),
    );
    // RFC-088: a second, independent root class — "" for Standard, so the
    // root carries no extra class and only the theme's own tokens apply.
    let cv_class =
        crate::state::settings::colour_vision_css_class(store.settings.read().colour_vision);
    // RFC-097 stage 2 §2.2: Contrast::System resolves through the same
    // live `prefers-contrast: more` reading every platform measured
    // (reply 184/review 210) — sets the identical `.contrast-high` class
    // the explicit choice uses, never a second copy of its tokens.
    let contrast_class = crate::state::settings::resolved_contrast_css_class(
        store.settings.read().contrast,
        *store.system_prefers_contrast_more.read(),
    );
    let active = *store.active.read();
    let toast = store.toast.read().cloned();
    let lang = store.lang();

    // RFC-102 §3 (amended 2026-11-25, review 239 — the Windows
    // measurement, reply 203): every passive (info/success) notice is
    // announced through this one permanent, visually hidden live
    // region, mounted once below, never through the toast itself any
    // more ("the visible toast stays as it is, but no longer carries
    // the live region itself, so nothing is announced twice"). Cleared,
    // then set, for each one — the one shape the Windows measurement
    // found `LiveRegionChanged` fires for reliably (3/3); a region
    // inserted with its text already there, the toast's own shape,
    // fired 0-1 times in 3.
    let mut passive_announcement = use_signal::<Option<crate::i18n::TranslatableMessage>>(|| None);
    use_effect(move || {
        let notice = store.toast.read().clone();
        if let Some(notice) = notice
            && !notice.severity.takes_focus()
        {
            passive_announcement.set(None);
            let message = notice.message.clone();
            spawn(async move {
                // A real tick, not just a second signal write in the same
                // render - the DOM text node must actually clear before
                // it is set again (shape C), which a same-frame
                // clear-then-set would not reliably give it.
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                passive_announcement.set(Some(message));
            });
        }
    });

    rsx! {
        style { {MAIN_CSS} }
        div {
            class: "app {theme_class} {cv_class} {contrast_class}",
            id: "app-root",
            tabindex: "-1",
            onmounted: move |_| {
                spawn(async move {
                    let _ = dioxus::document::eval(
                        "document.getElementById('app-root')?.focus();"
                    ).await;
                });
                crate::state::settings::spawn_system_preference_bridges(store);
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
                let toast_takes_focus = store
                    .toast
                    .read()
                    .as_ref()
                    .is_some_and(|n| n.severity.takes_focus());
                let action =
                    global_key_action(&e.key(), mods, modal, active.is_some(), toast_takes_focus);

                match action {
                    GlobalKeyAction::Ignore => {}
                    GlobalKeyAction::CloseToast => dismiss_focus_taking_toast(&mut store),
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
                    GlobalKeyAction::TextSizeUp => {
                        let next = forskscope_ui_logic::step_text_size(
                            store.settings.read().text_size,
                            1,
                        );
                        store.settings.write().text_size = next;
                        crate::ui::view::settings::persist(store);
                    }
                    GlobalKeyAction::TextSizeDown => {
                        let next = forskscope_ui_logic::step_text_size(
                            store.settings.read().text_size,
                            -1,
                        );
                        store.settings.write().text_size = next;
                        crate::ui::view::settings::persist(store);
                    }
                    GlobalKeyAction::TextSizeReset => {
                        store.settings.write().text_size =
                            forskscope_desktop::settings::display::TEXT_SIZE_DEFAULT;
                        crate::ui::view::settings::persist(store);
                    }
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
            // RFC-102 §3: the one permanent, visually hidden live region,
            // mounted unconditionally (not inside `if let Some(notice) =
            // toast`) — a passive notice clears it, then sets it (the
            // `use_effect` above); it never moves focus, and the toast
            // itself no longer carries `role="status"`, so nothing is
            // announced twice.
            PassiveLiveRegion { lang, message: passive_announcement.read().clone() }
            if let Some(notice) = toast {
                Toast { lang, notice }
            }
        }
    }
}

/// RFC-102 §3's one permanent, visually hidden live region. A separate
/// component (not inline in `App`) so its markup is directly testable
/// through a `VirtualDom`, the same way `TabError`/`SaveErrorModal` are.
#[component]
fn PassiveLiveRegion(
    lang: crate::state::Lang,
    message: Option<crate::i18n::TranslatableMessage>,
) -> Element {
    rsx! {
        span {
            class: "visually-hidden",
            role: "status",
            aria_live: "polite",
            if let Some(message) = message {
                crate::ui::view::TranslatedText {
                    lang,
                    id: message.id,
                    args: message.args.clone(),
                    isolate: message.isolate.clone(),
                }
            }
        }
    }
}

/// The toast itself (RFC-102 §2) — a separate component for the same
/// testability reason as [`PassiveLiveRegion`]. An error/warning takes
/// focus (`role="alertdialog"`, labelled by the message, autofocus on
/// its Dismiss or action button); success/info stay passive, announced
/// only through [`PassiveLiveRegion`], never by moving focus.
#[component]
fn Toast(lang: crate::state::Lang, notice: crate::state::Notice) -> Element {
    let mut store = use_context::<Store>();
    let severity_class = match notice.severity {
        crate::state::NoticeSeverity::Success => "toast toast-success",
        crate::state::NoticeSeverity::Info => "toast toast-info",
        crate::state::NoticeSeverity::Warning => "toast toast-warning",
        crate::state::NoticeSeverity::Error => "toast toast-error",
    };
    let message = notice.message.clone();
    let action = notice.action.clone();
    let takes_focus = notice.severity.takes_focus();
    let msg_id = format!("notice-{}-msg", notice.id);
    // Auto-dismiss for Success and Info notices only — RFC-102 §2: an
    // error/warning "stays until clicked" (or Escape/Dismiss), same as
    // today.
    if let Some(ms) = notice.auto_dismiss_ms() {
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            store.toast.set(None);
        });
    }
    rsx! {
        div {
            key: "notice-{notice.id}",
            class: "{severity_class}",
            role: if takes_focus { "alertdialog" },
            aria_labelledby: if takes_focus { "{msg_id}" },
            // RFC-102 §2: "typing isn't swallowed ... the text typed so
            // far is kept" - moving DOM focus away from a text input
            // never touches its own value signal, so this needs no
            // special handling beyond not clearing one anywhere in this
            // path (verified by a real test, not just asserted here).
            onmounted: move |_| {
                if takes_focus {
                    spawn(async move {
                        let _ = dioxus::document::eval(SAVE_FOCUS_AND_FOCUS_NOTICE_JS).await;
                    });
                }
            },
            onclick: move |_| {
                if takes_focus {
                    dismiss_focus_taking_toast(&mut store);
                } else {
                    store.toast.set(None);
                }
            },
            span {
                id: "{msg_id}",
                crate::ui::view::TranslatedText {
                    lang,
                    id: message.id,
                    args: message.args.clone(),
                    isolate: message.isolate.clone(),
                }
            }
            // RFC-102 §2: focus moves to *either* the Dismiss button,
            // named "Dismiss" and described by the message, *or* the
            // action button where the notice has one, described the
            // same way - never both, so `action` takes priority on a
            // focus-taking notice that happens to carry one (no call
            // site does today; this stays correct if one ever does).
            if takes_focus && action.is_none() {
                button {
                    autofocus: true,
                    "aria-describedby": "{msg_id}",
                    onclick: move |e| {
                        e.stop_propagation();
                        dismiss_focus_taking_toast(&mut store);
                    },
                    {t(lang, "recovery-action-dismiss")}
                }
            }
            if let Some(action) = action {
                button {
                    class: "toast-action",
                    autofocus: takes_focus,
                    "aria-describedby": if takes_focus { "{msg_id}" },
                    onclick: move |e| {
                        // RFC-092 §2: stays on the toast's own
                        // click-anywhere-to-dismiss, but this click must
                        // act (open the file), not merely dismiss - stop
                        // it reaching the outer div's onclick.
                        e.stop_propagation();
                        if let Err(err) = forskscope_desktop::external_tool::open_path(&action.path) {
                            eprintln!("open_path: {err} (path: {})", action.path);
                        }
                        if takes_focus {
                            dismiss_focus_taking_toast(&mut store);
                        } else {
                            store.toast.set(None);
                        }
                    },
                    "{action.label}"
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
///
/// F235: `selected`'s own name(s) are escaped with `<U+XXXX>` (never the
/// raw character) before they reach the OS's own window-title API — this
/// is the one place in the app that can't use `BidiText`'s marker markup
/// at all, since an OS window title (and, through it, the taskbar and
/// window-switcher) is a plain string with no markup support.
fn window_title(selected: Option<&str>, dirty: bool) -> String {
    match selected {
        None => "ForskScope".to_string(),
        Some(title) => {
            let title = forskscope_compare::escape_to_plain_marker(title);
            if dirty {
                format!("● {title} — ForskScope")
            } else {
                format!("{title} — ForskScope")
            }
        }
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

    /// F235: a name carrying a bidi-control character (the classic
    /// spoofing shape) never reaches the OS window title as the raw
    /// character — only its `<U+XXXX>` escape does. Falsify by removing
    /// the `escape_to_plain_marker` call: this assertion then fails,
    /// comparing an escaped expectation against a title holding the raw
    /// U+202E still in it.
    #[test]
    fn a_bidi_control_character_in_the_title_is_escaped_not_raw() {
        let spoofed = "invoice\u{202E}fdp.exe ↔ invoice.pdf";
        assert_eq!(
            window_title(Some(spoofed), false),
            "invoice<U+202E>fdp.exe ↔ invoice.pdf — ForskScope"
        );
    }
}

#[cfg(test)]
mod toast_markup_tests {
    use dioxus::prelude::*;

    use super::{PassiveLiveRegion, Toast};
    use crate::i18n::TranslatableMessage;
    use crate::state::{AppSettings, Lang, Notice, Store};

    fn attribute_values(mutations: &dioxus_core::Mutations, attr_name: &str) -> Vec<String> {
        mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute { name, value, .. } if *name == attr_name => {
                    Some(format!("{value:?}"))
                }
                _ => None,
            })
            .collect()
    }

    /// RFC-102 §2, verification point 2: "an error notice renders
    /// `role=\"alertdialog\"`". Falsify by hardcoding `role: "status"`
    /// (the old, always-passive markup) on `Toast`'s outer div: this
    /// assertion then finds no `"alertdialog"` value at all.
    #[test]
    fn an_error_notice_renders_role_alertdialog() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                Toast {
                    lang: Lang::En,
                    notice: Notice::error(TranslatableMessage::new("test-fixture-plain")),
                }
            }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        let roles = attribute_values(&mutations, "role");
        assert!(
            roles.iter().any(|r| r.contains("alertdialog")),
            "an error notice must render role=\"alertdialog\": {roles:?}"
        );
    }

    /// RFC-102 §2, verification point 2 (the other half): "a success
    /// notice renders neither" — no `role` attribute at all (the
    /// permanent live region, not the toast, carries `role="status"`
    /// for a passive notice — §3). Falsify by setting `role:
    /// "alertdialog"` unconditionally on `Toast`'s outer div: this
    /// assertion then finds one.
    #[test]
    fn a_success_notice_renders_no_role() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                Toast {
                    lang: Lang::En,
                    notice: Notice::success(TranslatableMessage::new("test-fixture-plain")),
                }
            }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        let roles = attribute_values(&mutations, "role");
        assert!(
            roles.is_empty() || roles.iter().all(|r| r.contains("None")),
            "a success notice must render no role attribute: {roles:?}"
        );
    }

    /// RFC-102 §2: "its Dismiss button, named \"Dismiss\" ... so a
    /// screen reader speaks the message as focus arrives" - the button
    /// must carry `autofocus` when the notice takes focus and has no
    /// action. Falsify by removing `autofocus: true` from the Dismiss
    /// button: this assertion then finds no `true` value.
    #[test]
    fn an_error_notices_dismiss_button_is_autofocused() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                Toast {
                    lang: Lang::En,
                    notice: Notice::warning(TranslatableMessage::new("test-fixture-plain")),
                }
            }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        let autofocus = attribute_values(&mutations, "autofocus");
        assert!(
            autofocus.iter().any(|v| v.contains("true")),
            "a warning notice's Dismiss button must be autofocused: {autofocus:?}"
        );
    }

    /// RFC-102 §3: the permanent live region renders no message text
    /// when passed `None` (its own "cleared" state, between one notice
    /// and the next). `role="status"` itself is a static literal with
    /// no dynamic part (never conditional, unlike the toast's `role`) —
    /// confirmed by reading `PassiveLiveRegion`'s own source, not by a
    /// mutation this harness can see.
    #[test]
    fn the_passive_live_region_with_no_message_renders_no_text() {
        fn text_nodes(mutations: &dioxus_core::Mutations) -> Vec<String> {
            mutations
                .edits
                .iter()
                .filter_map(|m| match m {
                    dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                    _ => None,
                })
                .collect()
        }
        fn root() -> Element {
            rsx! { PassiveLiveRegion { lang: Lang::En, message: None } }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        assert!(
            text_nodes(&mutations).is_empty(),
            "with no message, the live region must render no text node"
        );
    }
}
