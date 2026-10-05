//! Settings modal dialog: appearance, advanced options, and compare profiles
//! (RFC-009, RFC-057, RFC-063 C6).

use dioxus::prelude::*;
use forskscope_ui_logic::{FieldDebounce, clamp_font_size, theme_choices};

use super::profile::AddProfileInline;
use super::{lf, lv, tf, tv};
use crate::i18n::t;
use crate::state::{Modal, Store};

/// F136: starts (or restarts) the rest for one ignore-pattern field. `draft`
/// is set immediately, so the input stays responsive to every keystroke;
/// `debounce` and the spawned check decide when (if ever, before the next
/// keystroke) `commit` actually runs — the `store.settings` write that
/// `AppSettings::ignore_rules()` and the Explorer's tree-rebuild effects
/// react to, which is the expensive step this exists to debounce, not the
/// field's own display.
fn debounce_ignore_field_input(
    value: String,
    mut draft: Signal<String>,
    mut debounce: Signal<FieldDebounce<String>>,
    clock: std::time::Instant,
    mut commit: impl FnMut(String) + 'static,
) {
    draft.set(value.clone());
    let now = clock.elapsed();
    debounce.write().changed(now, value);
    spawn(async move {
        let delay = debounce
            .peek()
            .due_at()
            .map(|due| due.saturating_sub(now))
            .unwrap_or_default();
        tokio::time::sleep(delay + std::time::Duration::from_millis(5)).await;
        if let Some(committed) = debounce.write().tick(clock.elapsed()) {
            commit(committed);
        }
    });
}

#[component]
pub fn SettingsModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let cur = store.settings.read().cloned();

    let mut show_new_profile = use_signal(|| false);
    // Progressive disclosure: Advanced hidden by default (RFC-063 C6).
    let mut show_advanced = use_signal(|| false);

    // F136: a fresh `SettingsModal` mounts each time the dialog opens
    // (`Modal::Settings => rsx! { SettingsModal {} }`), so initializing from
    // `cur` here is always the current committed value - there is no reset
    // action reachable from inside this open dialog that this would need to
    // react to mid-edit.
    let ext_draft: Signal<String> = use_signal(|| cur.ignore_extensions.clone());
    let ext_debounce: Signal<FieldDebounce<String>> = use_signal(FieldDebounce::default);
    let dirs_draft: Signal<String> = use_signal(|| cur.ignore_dirs.clone());
    let dirs_debounce: Signal<FieldDebounce<String>> = use_signal(FieldDebounce::default);
    let clock = use_hook(std::time::Instant::now);

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Settings"),
            tabindex: "-1",
            onclick: move |_| store.modal.set(Modal::None),
            onkeydown: move |e: Event<KeyboardData>| {
                // RFC-060/handoff 020 §5: swallow every key, not just Escape. This
                // wrapper's own Escape-close is not needed for correctness even
                // without this call: app.rs's own modal-open guard also closes any
                // open, non-recovery modal on Escape. (Review 092 §5: the two
                // mechanisms produce the same outcome — this is not "app.rs already
                // saw the keypress," since the old code's own stop_propagation()
                // meant it never did.) Kept anyway, for construction-based
                // uniformity with the other three surfaces and as a guard against a
                // future key handled here without its own.
                crate::keyboard::swallow_when_typing(&e);
                if e.key() == dioxus::html::input_data::keyboard_types::Key::Escape {
                    store.modal.set(Modal::None);
                }
            },
            div { class: "modal", onclick: move |e| e.stop_propagation(),
                // Header row: title + About button (RFC-057).
                div { class: "modal-header-row",
                    h2 { id: "settings-title", {t(lang, "Settings")} }
                    // The control inherits the shared button style on purpose: F150
                    // had cancelled it, so the control was invisible as a button.
                    button {
                        title: t(lang, "About ForskScope"),
                        aria_label: t(lang, "About ForskScope"),
                        onclick: move |_| store.modal.set(Modal::About),
                        "ℹ"
                    }
                }

                // ── Appearance ────────────────────────────────────────────────
                div { class: "field",
                    span { {t(lang, "Theme")} }
                    select {
                        value: tv(cur.theme),
                        onchange: move |e| {
                            store.settings.write().theme = tf(&e.value());
                            super::persist(store);
                        },
                        for choice in theme_choices() {
                            option { value: choice.value, {choice.label} }
                        }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Language")} }
                    select {
                        value: lv(cur.language),
                        onchange: move |e| {
                            store.settings.write().language = lf(&e.value());
                            super::persist(store);
                        },
                        option { value: "en", "English" }
                        option { value: "ja", "日本語"   }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Diff font size")} }
                    input {
                        r#type: "number", min: "8", max: "32",
                        value: "{cur.diff_font_size}",
                        onchange: move |e| {
                            if let Ok(n) = e.value().parse::<u32>() {
                                store.settings.write().diff_font_size = u32::from(clamp_font_size(n));
                                super::persist(store);
                            }
                        }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Diff font family")} }
                    select {
                        value: match cur.diff_font_family {
                            crate::state::DiffFontFamily::Monospace  => "monospace",
                            crate::state::DiffFontFamily::SansSerif  => "sans-serif",
                            crate::state::DiffFontFamily::Serif      => "serif",
                            crate::state::DiffFontFamily::CourierNew => "courier-new",
                            crate::state::DiffFontFamily::Consolas   => "consolas",
                        },
                        onchange: move |e| {
                            use crate::state::DiffFontFamily;
                            let ff = match e.value().as_str() {
                                "sans-serif"  => DiffFontFamily::SansSerif,
                                "serif"       => DiffFontFamily::Serif,
                                "courier-new" => DiffFontFamily::CourierNew,
                                "consolas"    => DiffFontFamily::Consolas,
                                _             => DiffFontFamily::Monospace,
                            };
                            store.settings.write().diff_font_family = ff;
                            super::persist(store);
                        },
                        option { value: "monospace",  {t(lang, "Monospace (default)")} }
                        option { value: "sans-serif",  {t(lang, "Sans-serif")} }
                        option { value: "serif",       {t(lang, "Serif")} }
                        option { value: "courier-new", "Courier New" }
                        option { value: "consolas",    "Consolas / Menlo" }
                    }
                }

                // ── Advanced disclosure toggle ─────────────────────────────────
                button {
                    class: "advanced-toggle",
                    onclick: move |_| { let v = *show_advanced.read(); show_advanced.set(!v); },
                    if *show_advanced.read() { "▾ " {t(lang, "Hide advanced")} }
                    else                     { "▸ " {t(lang, "Advanced")} }
                }

                if *show_advanced.read() {
                    div { class: "field",
                        span { {t(lang, "Enable binary comparison")} }
                        input {
                            r#type: "checkbox",
                            checked: cur.enable_binary_comparison,
                            title: t(lang, "When off, binary files cannot be compared and are shown as non-actionable in the Explorer."),
                            onchange: move |e| {
                                store.settings.write().enable_binary_comparison = e.checked();
                                super::persist(store);
                            }
                        }
                    }
                    div { class: "field",
                        span { {t(lang, "Explorer layout")} }
                        select {
                            value: if cur.explorer_compact { "compact" } else { "aligned" },
                            onchange: move |e| {
                                store.settings.write().explorer_compact = e.value() == "compact";
                                super::persist(store);
                            },
                            option { value: "aligned", {t(lang, "Aligned (default)")} }
                            option { value: "compact", {t(lang, "Compact (independent panes)")} }
                        }
                    }
                    div { class: "field",
                        span { {t(lang, "Remember Explorer directories")} }
                        input {
                            r#type: "checkbox",
                            checked: cur.remember_explorer_dirs,
                            title: t(lang, "When on, the Explorer reopens the last directory shown in each pane. When off, it always starts at your home directory."),
                            onchange: move |e| {
                                let on = e.checked();
                                {
                                    let mut s = store.settings.write();
                                    s.remember_explorer_dirs = on;
                                    // Turning the feature off clears any stored locations so
                                    // they are not silently retained on disk.
                                    if !on {
                                        s.last_left_dir = None;
                                        s.last_right_dir = None;
                                    }
                                }
                                super::persist(store);
                            }
                        }
                    }
                    div { class: "field",
                        span { {t(lang, "Context lines")} }
                        select {
                            value: "{cur.context_lines}",
                            onchange: move |e| {
                                if let Ok(n) = e.value().parse::<usize>() {
                                    store.settings.write().context_lines = n;
                                    super::persist(store);
                                }
                            },
                            option { value: "0",  {t(lang, "0 (show all)")} }
                            option { value: "3",  {t(lang, "3 (default)")} }
                            option { value: "5",  "5"  }
                            option { value: "10", "10" }
                        }
                    }

                    // ── Ignore patterns (RFC-056) ─────────────────────────────
                    // F136: the displayed value is the draft, updated on
                    // every keystroke; the `store.settings` write (and the
                    // Explorer rescan it triggers) is debounced - see
                    // `debounce_ignore_field_input`'s own doc comment.
                    div { class: "field",
                        span { {t(lang, "Ignore file extensions")} }
                        input {
                            r#type: "text",
                            placeholder: t(lang, "o, class, tmp  (comma separated, no dot needed)"),
                            value: "{ext_draft}",
                            oninput: move |e| {
                                debounce_ignore_field_input(
                                    e.value(), ext_draft, ext_debounce, clock,
                                    move |value| {
                                        store.settings.write().ignore_extensions = value;
                                        super::persist(store);
                                    },
                                );
                            }
                        }
                    }
                    div { class: "field",
                        span { {t(lang, "Ignore directory names")} }
                        input {
                            r#type: "text",
                            placeholder: t(lang, "target, node_modules, *.cache  (* wildcard allowed)"),
                            value: "{dirs_draft}",
                            oninput: move |e| {
                                debounce_ignore_field_input(
                                    e.value(), dirs_draft, dirs_debounce, clock,
                                    move |value| {
                                        store.settings.write().ignore_dirs = value;
                                        super::persist(store);
                                    },
                                );
                            }
                        }
                    }
                    // F149: hidden entries are shown by default; this hides them —
                    // in the Explorer tree and in every comparison that reads a
                    // directory (recursive compare, Deep Compare, RFC-080 tier 1),
                    // not only the tree. Title text says so, per the handoff: this
                    // is not a display-only setting.
                    div { class: "field",
                        span { {t(lang, "Hide hidden files")} }
                        input {
                            r#type: "checkbox",
                            checked: cur.hide_dotfiles,
                            title: t(lang, "Also excludes hidden files and folders from comparisons — recursive compare, Deep Compare, and the Explorer's quick folder check. Off by default, so .gitignore, .env, and similar files are included."),
                            onchange: move |e| {
                                store.settings.write().hide_dotfiles = e.checked();
                                super::persist(store);
                            }
                        }
                    }

                    // ── Compare profiles ──────────────────────────────────────
                    div { class: "field",
                        span { {t(lang, "Compare profiles")} }
                        div { class: "profile-list",
                            for (i, p) in cur.profiles.iter().enumerate() {
                                div {
                                    class: if cur.active_profile == i { "profile-row active" } else { "profile-row" },
                                    span {
                                        class: "profile-name",
                                        onclick: move |_| {
                                            store.settings.write().active_profile = i;
                                            super::persist(store);
                                        },
                                        if cur.active_profile == i { "▸ " } else { "  " }
                                        "{p.name}"
                                    }
                                    if !p.built_in {
                                        button {
                                            class: "profile-delete",
                                            title: t(lang, "Delete profile"),
                                            onclick: move |_| crate::state::remove_profile(&mut store, i),
                                            "×"
                                        }
                                    }
                                }
                            }
                            if !*show_new_profile.read() {
                                button {
                                    class: "new-profile-btn",
                                    onclick: move |_| show_new_profile.set(true),
                                    {t(lang, "+ New profile")}
                                }
                            } else {
                                AddProfileInline { on_done: move |_| show_new_profile.set(false) }
                            }
                        }
                    }
                } // end show_advanced

                div { class: "actions",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppSettings;

    /// F150, reversed: the About control keeps its accessible name and title,
    /// which are dynamic attributes this harness can see. Falsify by removing
    /// `aria_label`: the assertion fails. It does NOT cover the absence of a
    /// class, glyph or static style: those are template content, which the
    /// mutations here do not carry (checked by reading the source and in the app).
    #[test]
    fn the_about_control_keeps_its_accessible_name() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                SettingsModal {}
            }
        }

        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();

        // `title` carries the same text, so only the `aria-label` attribute
        // itself is checked, by name.
        let labels: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute { name, value, .. }
                    if name.contains("label") =>
                {
                    Some(format!("{value:?}"))
                }
                _ => None,
            })
            .collect();

        assert!(
            labels.iter().any(|a| a.contains("About ForskScope")),
            "the About control must keep its accessible name, got {labels:?}"
        );
    }
}
