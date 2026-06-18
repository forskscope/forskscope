//! Compare profile list and add-profile inline form (RFC-057, RFC-009).

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::Store;

/// Inline form for adding a new compare profile (revealed on demand, RFC-057).
#[component]
pub fn AddProfileInline(on_done: EventHandler<()>) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut name        = use_signal(String::new);
    #[allow(unused_mut)] let mut ignore_ws   = use_signal(|| false);
    #[allow(unused_mut)] let mut ignore_case = use_signal(|| false);
    #[allow(unused_mut)] let mut algorithm: Signal<crate::state::DiffAlgorithmSetting> =
        use_signal(Default::default);

    rsx! {
        div { class: "add-profile-form",
            input {
                placeholder: t(lang, "Profile name"),
                value: "{name}",
                oninput: move |e| name.set(e.value()),
                style: "flex:1;"
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_ws.read(),
                    onchange: move |e| ignore_ws.set(e.checked()) }
                span { {t(lang, "Ignore WS")} }
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_case.read(),
                    onchange: move |e| ignore_case.set(e.checked()) }
                span { {t(lang, "Ignore case")} }
            }
            select {
                onchange: move |e| {
                    algorithm.set(match e.value().as_str() {
                        "patience"  => crate::state::DiffAlgorithmSetting::Patience,
                        "histogram" => crate::state::DiffAlgorithmSetting::Histogram,
                        _           => crate::state::DiffAlgorithmSetting::Myers,
                    });
                },
                option { value: "myers",     "Myers"     }
                option { value: "patience",  "Patience"  }
                option { value: "histogram", "Histogram" }
            }
            button {
                disabled: name.read().trim().is_empty(),
                onclick: move |_| {
                    let n = name.read().trim().to_string();
                    if !n.is_empty() {
                        crate::state::add_profile(
                            &mut store, n,
                            *ignore_ws.read(), *ignore_case.read(),
                            *algorithm.read(),
                        );
                        on_done.call(());
                    }
                },
                {t(lang, "Add")}
            }
            button {
                onclick: move |_| on_done.call(()),
                {t(lang, "Cancel")}
            }
        }
    }
}
