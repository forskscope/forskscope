//! About / diagnostics modal.
//!
//! The *Check for updates* button (F180, handoff 073): one HTTPS request,
//! only on click — never at startup, never retried automatically, never
//! stored across launches. The result lives in a `use_signal` local to
//! this component, so closing and reopening the dialog (which unmounts and
//! remounts `AboutModal`, per `ModalLayer`'s `Modal::About => rsx! {
//! AboutModal {} }`) starts over with nothing shown — satisfying "no
//! result stored across launches" by construction, not by remembering to
//! clear anything.
//!
//! The network call is a thin adapter (`crate::update_check::check_for_updates`,
//! off the UI thread via `spawn_blocking`); the decision about what it
//! means is `forskscope_ui_logic::update_check::decide` — the same
//! function its own unit tests call (F169, review 151).

use dioxus::prelude::*;
use forskscope_core::external_tool::open_url;
use forskscope_core::platform::PlatformInfo;
use forskscope_ui_logic::{
    AppChannel, CheckFailureReason, CheckOutcome, UpdateAction, UpdateCheckState, Version, decide,
    release_page_url, update_action,
};

use crate::i18n::t;
use crate::startup_mode::{self, ModeDisplay};
use crate::state::{Lang, Modal, Store};
use crate::update_check::check_for_updates;

/// RFC-091 §2/§3, acceptance criterion 7: the same mode text the status bar
/// shows, paired here with the data location itself (the status bar only
/// puts that in a tooltip) — `None` for `Installed`, where there is
/// neither a mode nor a location to report.
fn mode_and_location(lang: Lang) -> Option<(String, Option<String>)> {
    match startup_mode::mode_display() {
        ModeDisplay::Installed => None,
        ModeDisplay::Portable(dir) => Some((t(lang, "Portable"), Some(dir.display().to_string()))),
        ModeDisplay::PortableReadOnly(dir) => Some((
            t(lang, "Portable (read-only): changes are not saved"),
            Some(dir.display().to_string()),
        )),
        ModeDisplay::Private => Some((t(lang, "Private: nothing is saved"), None)),
    }
}

/// How this build was obtained, read once at compile time — never from
/// anything the network returned (§4).
fn app_channel() -> AppChannel {
    AppChannel::from_label(option_env!("FORSKSCOPE_CHANNEL"))
}

/// The message line for every state except `Checking`, which the button's
/// own label already covers (`button_label`, below) — showing it twice
/// would say the same thing in two places.
fn update_check_message(lang: Lang, state: &UpdateCheckState) -> String {
    match state {
        UpdateCheckState::Checking => t(lang, "Checking…"),
        UpdateCheckState::UpToDate { current } => {
            t(lang, "You have the latest version ({version}).")
                .replace("{version}", &current.to_string())
        }
        UpdateCheckState::NewerAvailable { latest, .. } => {
            t(lang, "{version} is available.").replace("{version}", &latest.to_string())
        }
        UpdateCheckState::AheadOfRelease { latest, .. } => t(
            lang,
            "This build is newer than the latest release ({version}).",
        )
        .replace("{version}", &latest.to_string()),
        UpdateCheckState::CouldNotCheck(reason) => match reason {
            CheckFailureReason::Unreachable => {
                t(lang, "Could not check for updates: no network connection.")
            }
            CheckFailureReason::RateLimited => t(
                lang,
                "Could not check for updates: rate-limited, try again later.",
            ),
            CheckFailureReason::HttpStatus(code) => t(
                lang,
                "Could not check for updates: unexpected response (HTTP {code}).",
            )
            .replace("{code}", &code.to_string()),
            CheckFailureReason::Unparseable => t(
                lang,
                "Could not check for updates: the reply could not be read.",
            ),
        },
    }
}

/// What a *Newer available* row offers, per §4: a copy installed from the
/// AUR or the Store is updated by that store, so only a `Direct` build
/// gets a button that opens anything — the other two are instructions, not
/// actions this app can perform on the user's behalf.
fn newer_available_action(lang: Lang, channel: AppChannel, latest: Version) -> Element {
    match update_action(channel) {
        UpdateAction::OpenReleasePage => {
            let url = release_page_url(&latest);
            rsx! {
                button {
                    onclick: move |_| {
                        let _ = open_url(&url);
                    },
                    {t(lang, "Open release page")}
                }
            }
        }
        UpdateAction::UseAurHelper => rsx! {
            span { {t(lang, "Update with your AUR helper")} }
        },
        UpdateAction::UseMicrosoftStore => rsx! {
            span { {t(lang, "Update through the Microsoft Store")} }
        },
    }
}

/// The plain-English line(s) appended to the copied diagnostics text —
/// matching `PlatformInfo::to_report`'s own un-localized style, so the
/// clipboard content a bug report gets stays in one language regardless of
/// the UI's. `None` for `Installed`, which has nothing extra to report.
fn diagnostics_mode_lines() -> Option<String> {
    match startup_mode::mode_display() {
        ModeDisplay::Installed => None,
        ModeDisplay::Portable(dir) => {
            Some(format!("Mode: Portable\nData location: {}", dir.display()))
        }
        ModeDisplay::PortableReadOnly(dir) => Some(format!(
            "Mode: Portable (read-only)\nData location: {}",
            dir.display()
        )),
        ModeDisplay::Private => Some("Mode: Private".to_string()),
    }
}

#[component]
pub fn AboutModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let info = PlatformInfo::collect();
    let mut diag = info.to_report();
    if let Some(mode_lines) = diagnostics_mode_lines() {
        diag.push('\n');
        diag.push_str(&mode_lines);
    }
    let d2 = diag.clone();
    let mode_and_location = mode_and_location(lang);

    let mut update_state = use_signal(|| None::<UpdateCheckState>);
    let checking = matches!(&*update_state.read(), Some(UpdateCheckState::Checking));
    let button_label = if checking {
        t(lang, "Checking…")
    } else {
        t(lang, "Check for updates")
    };
    let channel = app_channel();

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "About ForskScope"),
            div { class: "modal",
                h2 { "ForskScope v{info.app_version}" }
                div { class: "about-grid",
                    span { class: "about-key", {t(lang, "Version")} } span { "{info.app_version}" }
                    span { class: "about-key", {t(lang, "Rust")}    } span { "{info.rustc_version}" }
                    span { class: "about-key", {t(lang, "OS")}      } span { "{info.os}" }
                    span { class: "about-key", {t(lang, "Arch")}    } span { "{info.arch}" }
                    span { class: "about-key", {t(lang, "CPUs")}    } span { "{info.logical_cpus}" }
                    if let Some((mode, location)) = &mode_and_location {
                        span { class: "about-key", {t(lang, "Mode")} } span { "{mode}" }
                        if let Some(location) = location {
                            span { class: "about-key", {t(lang, "Data location")} } span { "{location}" }
                        }
                    }
                }
                div { class: "update-check",
                    button {
                        disabled: checking,
                        onclick: move |_| {
                            update_state.set(Some(UpdateCheckState::Checking));
                            spawn(async move {
                                let outcome = tokio::task::spawn_blocking(check_for_updates)
                                    .await
                                    .unwrap_or(CheckOutcome::Unreachable);
                                update_state.set(Some(decide(Version::current(), outcome)));
                            });
                        },
                        "{button_label}"
                    }
                    if let Some(state) = &*update_state.read() {
                        if !matches!(state, UpdateCheckState::Checking) {
                            p { class: "update-check-message", "{update_check_message(lang, state)}" }
                        }
                        if let UpdateCheckState::NewerAvailable { latest, .. } = state {
                            {newer_available_action(lang, channel, *latest)}
                        }
                    }
                }
                div { class: "actions",
                    button {
                        onclick: move |_| {
                            let d = d2.clone();
                            spawn(async move {
                                let _ = dioxus::document::eval(
                                    &format!("navigator.clipboard?.writeText({:?})", d)
                                ).await;
                            });
                        },
                        {t(lang, "Copy diagnostics")}
                    }
                    button {
                        autofocus: true,
                        onclick: move |_| store.modal.set(Modal::None),
                        {t(lang, "Close")}
                    }
                }
            }
        }
    }
}
