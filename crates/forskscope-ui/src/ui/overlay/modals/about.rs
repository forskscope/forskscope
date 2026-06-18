//! About / diagnostics modal.

use dioxus::prelude::*;
use forskscope_core::platform::PlatformInfo;

use crate::i18n::t;
use crate::state::{Modal, Store};

#[component]
pub fn AboutModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let info = PlatformInfo::collect();
    let diag = info.to_report();
    let d2   = diag.clone();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "About ForskScope",
            div { class: "modal",
                h2 { "ForskScope v{info.app_version}" }
                div { class: "about-grid",
                    span { class: "about-key", {t(lang, "Version")} } span { "{info.app_version}" }
                    span { class: "about-key", {t(lang, "Rust")}    } span { "{info.rustc_version}" }
                    span { class: "about-key", {t(lang, "OS")}      } span { "{info.os}" }
                    span { class: "about-key", {t(lang, "Arch")}    } span { "{info.arch}" }
                    span { class: "about-key", {t(lang, "CPUs")}    } span { "{info.logical_cpus}" }
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
