//! Settings modal dialog: appearance, advanced options, and compare profiles
//! (RFC-009, RFC-057, RFC-063 C6).

use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;
use forskscope_ui_logic::{
    IgnoreListKind, add_entries, clamp_font_size, contrast_choices, current_entries, remove_entry,
    theme_choices,
};

use super::profile::AddProfileInline;
use super::{lf, lv, tf, tv};
use crate::i18n::t;
use crate::state::settings::{host_os, system_theme_offered};
use crate::state::{Contrast, Lang, Modal, Store};
use crate::ui::component::select_field::{SelectField, SelectOption};

/// RFC-099 §3: one ignore-list field (*Ignore file extensions* or *Ignore
/// directory names*), shown as badges with an input after them. `csv` is
/// this render's own current, committed value (read from `store.settings`
/// by the caller, not held locally) — `on_change` is called with the new
/// text to store and persist on every add or remove; there is nothing
/// left to debounce (F136's own module doc; `forskscope-ui-logic::
/// settings::ignore_list`'s doc comment has the full reasoning), since
/// nothing here writes on a keystroke that isn't itself an add or a
/// remove.
#[component]
fn IgnoreListField(
    lang: Lang,
    label: String,
    label_id: String,
    placeholder: String,
    kind: IgnoreListKind,
    csv: String,
    on_change: EventHandler<String>,
) -> Element {
    let mut typed = use_signal(String::new);
    // RFC-099 §3, round 2 (review 218 §3): two separate signals, not one.
    // `live_message` backs the screen-reader-only region below, which
    // stays mounted (empty) from the very first render — a region
    // inserted together with its own first message is reliably announced
    // as a *change* only the second time onward, found live in review
    // 218's own harness, so the live region itself must already exist
    // before the first add. `duplicate_notice` is sighted-only: the
    // *visible* line is "Already in the list" alone — the badge itself
    // already shows an add or a remove, so repeating that in text is
    // noise a sighted user doesn't need.
    let mut live_message = use_signal(String::new);
    let mut duplicate_notice = use_signal(|| false);
    let entries = current_entries(&csv, kind);

    let mut try_add = move |raw: String, csv: String| {
        let outcome = add_entries(&csv, &raw, kind);
        if !outcome.added.is_empty() {
            on_change.call(outcome.csv);
            live_message.set(format!(
                "{}: {}",
                t(lang, "Added"),
                outcome.added.join(", ")
            ));
            duplicate_notice.set(false);
        } else if !outcome.duplicates.is_empty() {
            live_message.set(t(lang, "Already in the list"));
            duplicate_notice.set(true);
        }
        typed.set(String::new());
    };

    rsx! {
        div { class: "field ignore-list-field",
            span { id: "{label_id}", {label} }
            div { class: "ignore-badges",
                ul { class: "ignore-badge-list",
                    for entry in entries {
                        li { class: "ignore-badge", key: "{entry}",
                            span { "{entry}" }
                            button {
                                class: "icon-btn",
                                aria_label: format!("{} {entry}", t(lang, "Remove")),
                                onclick: {
                                    let entry = entry.clone();
                                    let csv = csv.clone();
                                    move |_| {
                                        on_change.call(remove_entry(&csv, &entry, kind));
                                        live_message.set(format!("{}: {entry}", t(lang, "Removed")));
                                        duplicate_notice.set(false);
                                    }
                                },
                                "×"
                            }
                        }
                    }
                }
                input {
                    r#type: "text",
                    "aria-labelledby": "{label_id}",
                    placeholder: "{placeholder}",
                    value: "{typed}",
                    oninput: {
                        let csv = csv.clone();
                        move |e: Event<FormData>| {
                            // Review 218 §3 / review 219 §2: cleared on
                            // every keystroke, not only a successful add —
                            // "Already in the list" describes the text as
                            // typed a moment ago, not the text now being
                            // changed. `live_message` goes with it:
                            // without this, typing the same duplicate
                            // twice in a row (with at least one keystroke
                            // of its own between them, as retyping the
                            // same text always has) left the live
                            // region's text unchanged the second time,
                            // so there was nothing to announce as a
                            // change.
                            duplicate_notice.set(false);
                            live_message.set(String::new());
                            let value = e.value();
                            if value.contains(',') {
                                try_add(value, csv.clone());
                            } else {
                                typed.set(value);
                            }
                        }
                    },
                    onkeydown: {
                        let csv = csv.clone();
                        move |e: Event<KeyboardData>| {
                            // `value` must be its own statement: `peek()`'s
                            // borrow guard is a temporary of the full
                            // enclosing statement, otherwise, so it would
                            // still be held (a live `Ref`) while `try_add`
                            // tries its own `.set()` inside the same call -
                            // found live, a real `AlreadyBorrowed` panic
                            // the type system alone never catches.
                            let value = typed.peek().clone();
                            if e.key() == Key::Enter && !value.trim().is_empty() {
                                e.prevent_default();
                                try_add(value, csv.clone());
                            }
                        }
                    },
                    onblur: {
                        let csv = csv.clone();
                        move |_| {
                            let value = typed.peek().clone();
                            if !value.trim().is_empty() {
                                try_add(value, csv.clone());
                            }
                        }
                    },
                }
            }
            // Always mounted, even with nothing to say yet (review 218
            // §3) — a region a screen reader already knows about is
            // reliably announced on a text *change*; one inserted
            // together with its first message is not.
            p { class: "sr-only", role: "status", "aria-live": "polite", "{live_message}" }
            if *duplicate_notice.read() {
                p { class: "ignore-list-notice", "aria-hidden": "true", {t(lang, "Already in the list")} }
            }
        }
    }
}

#[component]
pub fn SettingsModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let cur = store.settings.read().cloned();

    let mut show_new_profile = use_signal(|| false);
    // Progressive disclosure: Advanced hidden by default (RFC-063 C6).
    let mut show_advanced = use_signal(|| false);

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
                // RFC-091 §3: "A Settings change says the same next to it" -
                // the status bar's own "Private: nothing is saved" repeated
                // here, so a change made from inside this dialog is not
                // mistaken for one that will survive the run.
                if crate::startup_mode::is_private() {
                    p { class: "notice", {t(lang, "Private: nothing is saved")} }
                }

                // ── Appearance ────────────────────────────────────────────────
                // RFC-097 stage 1 (handoff 095 §2): first item of the group.
                // Handoff 095's own measurement (reply 177, review 199)
                // found no *System* choice belongs here — the platform
                // already applies its own text scale automatically, so this
                // is a plain, relative scale on top of whatever that is.
                div { class: "field",
                    span { {t(lang, "Text size")} }
                    SelectField {
                        value: cur.text_size.to_string(),
                        aria_describedby: "text-size-help",
                        options: forskscope_desktop::settings::display::TEXT_SIZE_STEPS
                            .iter()
                            .map(|s| SelectOption::new(s.to_string(), format!("{s}%")))
                            .collect::<Vec<_>>(),
                        onchange: move |v: String| {
                            if let Ok(n) = v.parse::<u32>() {
                                store.settings.write().text_size =
                                    forskscope_desktop::settings::display::nearest_text_size_step(n);
                                super::persist(store);
                            }
                        },
                    }
                }
                // RFC-097 stage 3 §6: a visible line, not a `title` tooltip a
                // sighted user who never hovers would never see (review 203).
                p { class: "field-help", id: "text-size-help",
                    {t(lang, "Enlarges everything in ForskScope, on top of your system's text size. Diff font size (below) sets the code panes relative to it.")}
                }
                div { class: "field",
                    span { {t(lang, "Theme")} }
                    SelectField {
                        value: tv(cur.theme).to_string(),
                        options: {
                            let offered = system_theme_offered(
                                host_os(),
                                store.system_color_scheme.read().is_some(),
                            );
                            // RFC-097 stage 2 §2.1 / review 211 §3: a saved
                            // System that isn't offered *this session* must
                            // still be listed, selected, with a label saying
                            // what actually renders - never silently dropped,
                            // which would show Dark as selected while the
                            // persisted value still says system.
                            let saved_unavailable_system =
                                !offered && cur.theme == crate::state::Theme::System;
                            theme_choices(offered, saved_unavailable_system)
                                .into_iter()
                                .map(|c| SelectOption::new(c.value, t(lang, c.label)))
                                .collect::<Vec<_>>()
                        },
                        onchange: move |v: String| {
                            store.settings.write().theme = tf(&v);
                            super::persist(store);
                        },
                    }
                }
                // RFC-097 stage 2 §2.2, directly under Theme, the same
                // placement RFC-088 gave Colour vision ("a user who needs
                // it must find it without knowing it exists").
                div { class: "field",
                    span { {t(lang, "Contrast")} }
                    SelectField {
                        value: cur.contrast.as_str().to_string(),
                        options: contrast_choices().into_iter()
                            .map(|c| SelectOption::new(c.value, t(lang, c.label)))
                            .collect::<Vec<_>>(),
                        onchange: move |v: String| {
                            store.settings.write().contrast =
                                Contrast::from_id(&v).unwrap_or_default();
                            super::persist(store);
                        },
                    }
                }
                div { class: "field",
                    span { {t(lang, "Colour vision")} }
                    SelectField {
                        value: cur.colour_vision.as_str().to_string(),
                        options: vec![
                            SelectOption::new("standard", t(lang, "Standard")),
                            SelectOption::new("red-green-safe", t(lang, "Red–green safe (blue and orange)")),
                        ],
                        onchange: move |v: String| {
                            store.settings.write().colour_vision =
                                crate::state::ColourVision::from_id(&v).unwrap_or_default();
                            super::persist(store);
                        },
                    }
                }
                div { class: "field",
                    span { {t(lang, "Language")} }
                    SelectField {
                        value: lv(cur.language).to_string(),
                        options: vec![
                            SelectOption::new("en", "English"),
                            SelectOption::new("ja", "日本語"),
                        ],
                        onchange: move |v: String| {
                            store.settings.write().language = lf(&v);
                            super::persist(store);
                        },
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
                    {
                        let ff_value = match cur.diff_font_family {
                            crate::state::DiffFontFamily::Monospace  => "monospace",
                            crate::state::DiffFontFamily::SansSerif  => "sans-serif",
                            crate::state::DiffFontFamily::Serif      => "serif",
                            crate::state::DiffFontFamily::CourierNew => "courier-new",
                            crate::state::DiffFontFamily::Consolas   => "consolas",
                        };
                        rsx! {
                            SelectField {
                                value: ff_value.to_string(),
                                options: vec![
                                    SelectOption::new("monospace", t(lang, "Monospace (default)")),
                                    SelectOption::new("sans-serif", t(lang, "Sans-serif")),
                                    SelectOption::new("serif", t(lang, "Serif")),
                                    SelectOption::new("courier-new", "Courier New"),
                                    SelectOption::new("consolas", "Consolas / Menlo"),
                                ],
                                onchange: move |v: String| {
                                    use crate::state::DiffFontFamily;
                                    let ff = match v.as_str() {
                                        "sans-serif"  => DiffFontFamily::SansSerif,
                                        "serif"       => DiffFontFamily::Serif,
                                        "courier-new" => DiffFontFamily::CourierNew,
                                        "consolas"    => DiffFontFamily::Consolas,
                                        _             => DiffFontFamily::Monospace,
                                    };
                                    store.settings.write().diff_font_family = ff;
                                    super::persist(store);
                                },
                            }
                        }
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
                            aria_describedby: "binary-cmp-help",
                            onchange: move |e| {
                                store.settings.write().enable_binary_comparison = e.checked();
                                super::persist(store);
                            }
                        }
                    }
                    p { class: "field-help", id: "binary-cmp-help",
                        {t(lang, "When off, binary files cannot be compared and are shown as non-actionable in the Explorer.")}
                    }
                    div { class: "field",
                        span { {t(lang, "Explorer layout")} }
                        {
                            let layout_value = if cur.explorer_compact { "compact" } else { "aligned" };
                            rsx! {
                                SelectField {
                                    value: layout_value.to_string(),
                                    options: vec![
                                        SelectOption::new("aligned", t(lang, "Aligned")),
                                        SelectOption::new("compact", t(lang, "Compact (no spacer rows, default)")),
                                    ],
                                    onchange: move |v: String| {
                                        store.settings.write().explorer_compact = v == "compact";
                                        super::persist(store);
                                    },
                                }
                            }
                        }
                    }
                    div { class: "field",
                        span { {t(lang, "Remember Explorer directories")} }
                        input {
                            r#type: "checkbox",
                            checked: cur.remember_explorer_dirs,
                            aria_describedby: "remember-dirs-help",
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
                    p { class: "field-help", id: "remember-dirs-help",
                        {t(lang, "When on, the Explorer reopens the last directory shown in each pane. When off, it always starts at your home directory.")}
                    }
                    div { class: "field",
                        span { {t(lang, "Context lines")} }
                        {
                            let context_lines = cur.context_lines;
                            rsx! {
                                SelectField {
                                    value: context_lines.to_string(),
                                    options: vec![
                                        SelectOption::new("0", t(lang, "0 (show all)")),
                                        SelectOption::new("3", t(lang, "3 (default)")),
                                        SelectOption::new("5", "5"),
                                        SelectOption::new("10", "10"),
                                    ],
                                    onchange: move |v: String| {
                                        if let Ok(n) = v.parse::<usize>() {
                                            store.settings.write().context_lines = n;
                                            super::persist(store);
                                        }
                                    },
                                }
                            }
                        }
                    }

                    // ── Ignore patterns (RFC-056; badges, RFC-099 §3) ──────────
                    IgnoreListField {
                        lang,
                        label: t(lang, "Ignore file extensions"),
                        label_id: "ignore-extensions-label",
                        placeholder: t(lang, "Add an extension (no dot needed)"),
                        kind: IgnoreListKind::Extension,
                        csv: cur.ignore_extensions.clone(),
                        on_change: move |value| {
                            store.settings.write().ignore_extensions = value;
                            super::persist(store);
                        },
                    }
                    IgnoreListField {
                        lang,
                        label: t(lang, "Ignore directory names"),
                        label_id: "ignore-dirs-label",
                        placeholder: t(lang, "Add a folder name or pattern"),
                        kind: IgnoreListKind::DirPattern,
                        csv: cur.ignore_dirs.clone(),
                        on_change: move |value| {
                            store.settings.write().ignore_dirs = value;
                            super::persist(store);
                        },
                    }
                    // F149: hidden entries are shown by default; this hides them —
                    // in the Explorer tree and in every comparison that reads a
                    // directory (recursive compare, Deep Compare, RFC-080 tier 1),
                    // not only the tree. The help line below says so, per the
                    // handoff: this is not a display-only setting.
                    div { class: "field",
                        span { {t(lang, "Hide hidden files")} }
                        input {
                            r#type: "checkbox",
                            checked: cur.hide_dotfiles,
                            aria_describedby: "hide-dotfiles-help",
                            onchange: move |e| {
                                store.settings.write().hide_dotfiles = e.checked();
                                super::persist(store);
                            }
                        }
                    }
                    p { class: "field-help", id: "hide-dotfiles-help",
                        {t(lang, "Also excludes hidden files and folders from comparisons — recursive compare, Deep Compare, and the Explorer's quick folder check. Off by default, so .gitignore, .env, and similar files are included.")}
                    }

                    // ── Compare profiles (RFC-087 §3: a radio group) ───────────
                    div { class: "field",
                        span { {t(lang, "Compare profiles")} }
                        div {
                            class: "profile-list",
                            role: "radiogroup",
                            aria_label: t(lang, "Active compare profile"),
                            for (i, p) in cur.profiles.iter().enumerate() {
                                div {
                                    class: if cur.active_profile == i { "profile-row active" } else { "profile-row" },
                                    input {
                                        r#type: "radio",
                                        class: "profile-radio",
                                        id: "profile-radio-{i}",
                                        name: "active-compare-profile",
                                        checked: cur.active_profile == i,
                                        onchange: move |_| crate::state::select_profile(&mut store, i),
                                    }
                                    label {
                                        class: "profile-name",
                                        r#for: "profile-radio-{i}",
                                        if cur.active_profile == i { "▸ " } else { "  " }
                                        "{p.name}"
                                    }
                                    if !p.built_in {
                                        button {
                                            class: "profile-delete icon-btn",
                                            title: t(lang, "Delete profile"),
                                            aria_label: t(lang, "Delete profile"),
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

    /// Review 200 §4: the *Text size* field must show whatever
    /// `store.settings.text_size` holds — including a value a keyboard
    /// shortcut just wrote, not the modal's own default — since Ctrl+=/
    /// Ctrl+-/Ctrl+0 now work while Settings is open. This checks the half
    /// a unit test can see: the `<select>`'s own `value` attribute (a
    /// dynamic, interpolated string, so it is a real `SetAttribute`
    /// mutation here, the same as `aria-label` above) reflects a
    /// non-default `AppSettings.text_size` at mount, rather than a
    /// hardcoded 100 — a stand-in for "the dialog re-renders after the
    /// shortcut mutates the store", which this harness has no established
    /// way to simulate (no test in this crate re-renders after a later
    /// signal write; every one constructs the initial state it checks, the
    /// same shape `move_together_aria_pressed_is_true_when_sync_is_on` and
    /// its `_off` counterpart use in `explorer/filter.rs`).
    #[test]
    fn the_text_size_field_reflects_a_non_default_setting() {
        fn root() -> Element {
            let settings = AppSettings {
                text_size: 125,
                ..AppSettings::default()
            };
            use_context_provider(|| Store::new(settings, Default::default(), false));
            rsx! {
                SettingsModal {}
            }
        }

        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();

        let values: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute { name, value, .. } if *name == "value" => {
                    Some(format!("{value:?}"))
                }
                _ => None,
            })
            .collect();

        assert!(
            values.iter().any(|v| v.contains("125")),
            "the Text size select's value must reflect text_size=125, got {values:?}"
        );
    }

    /// RFC-097 stage 3 §6: the *Text size* help sentence is a visible line
    /// linked by `aria-describedby`, not a `title` tooltip - checked
    /// end-to-end (the `<select>`'s own attribute, the id it points at, and
    /// that the `<p>` carrying that id is actually rendered with the help
    /// text), not just that neither attribute is literally named `title`
    /// somewhere in the file. Falsify by reverting `SelectField`'s
    /// `aria_describedby` back to `title`: the first assertion then fails
    /// (no `aria-describedby` attribute at all).
    #[test]
    fn the_text_size_help_is_linked_not_only_a_tooltip() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! { SettingsModal {} }
        }

        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();

        let describedby: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute {
                    name: "aria-describedby",
                    value,
                    ..
                } => Some(format!("{value:?}")),
                _ => None,
            })
            .collect();
        assert!(
            describedby.iter().any(|v| v.contains("text-size-help")),
            "the Text size select must carry aria-describedby=\"text-size-help\", got {describedby:?}"
        );

        let texts: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("Enlarges everything in ForskScope")),
            "the linked help text itself must actually be rendered, got {texts:?}"
        );
    }

    /// Review 218 §3: the ignore-list live region is mounted — empty —
    /// from the very first render, and holds the real announcement after
    /// a real add, driven through the actual `oninput` listener (not a
    /// hand call to a private closure), the same `handle_event` mechanism
    /// `dioxus-core`'s own `event_propagation.rs` test uses. Falsify by
    /// reverting the live region to the old `if let Some(msg) = ...`
    /// shape: the first assertion (a `CreateTextNode` with an empty
    /// value, present at the very first mount) fails, since that shape
    /// renders no such node until the first message exists.
    #[test]
    fn the_ignore_list_live_region_is_always_present_and_updates_on_add() {
        use dioxus::html::{PlatformEventData, SerializedFormData};
        use dioxus_core::ElementId;
        use std::any::Any;
        use std::rc::Rc;

        set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

        fn root() -> Element {
            let csv = use_signal(String::new);
            rsx! {
                IgnoreListField {
                    lang: Lang::En,
                    label: "Ignore file extensions".to_string(),
                    label_id: "test-ignore-extensions".to_string(),
                    placeholder: "Add an extension".to_string(),
                    kind: IgnoreListKind::Extension,
                    csv: csv.read().clone(),
                    on_change: move |v: String| csv.clone().set(v),
                }
            }
        }

        let mut dom = VirtualDom::new(root);
        let first = dom.rebuild_to_vec();

        // The live region exists, with an empty text node, before any
        // add — not absent, the way the old `if let Some(msg)` shape
        // left it until the first message.
        let empty_text_ids: Vec<ElementId> = first
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, id } if value.is_empty() => {
                    Some(*id)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            empty_text_ids.len(),
            1,
            "expected exactly one empty text node (the live region) at first mount, got {:?}",
            first.edits
        );
        let live_region_text_id = empty_text_ids[0];

        let input_id = first
            .edits
            .iter()
            .find_map(|m| match m {
                dioxus_core::Mutation::NewEventListener { name, id } if name == "input" => {
                    Some(*id)
                }
                _ => None,
            })
            .expect("the input must register an oninput listener");

        // Drive the real listener, as the browser would: an `input`
        // event carrying a comma, which `IgnoreListField`'s own
        // `oninput` routes straight to `try_add`.
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                "tmp, log".to_string(),
                Vec::new(),
            )))) as Rc<dyn Any>,
            true,
        );
        dom.runtime().handle_event("input", event, input_id);
        let after_add = dom.render_immediate_to_vec();

        let live_region_text_after_add = after_add.edits.iter().find_map(|m| match m {
            dioxus_core::Mutation::SetText { value, id } if *id == live_region_text_id => {
                Some(value.clone())
            }
            _ => None,
        });
        assert_eq!(
            live_region_text_after_add.as_deref(),
            Some("Added: tmp, log"),
            "the live region must hold the real announcement after a real add, got {after_add:?}"
        );
    }

    /// Review 219 §2: typing the same duplicate twice left the live
    /// region's own text unchanged the second time ("Already in the
    /// list" both times), so there was nothing for a screen reader to
    /// announce as a change. `oninput` now clears `live_message` (not
    /// only `duplicate_notice`) on every keystroke, so retyping the same
    /// text — at least one keystroke of its own, every real retype has
    /// one — commits the clear in its own render before the repeated
    /// add's own render sets the message again. Falsify by reverting
    /// that one line: the second `SetText` assertion below finds none
    /// (the signal's value never actually changes from the first
    /// duplicate's, so Dioxus emits no mutation for it).
    ///
    /// Scope: this covers retyping, the realistic path (every `onblur`/
    /// `onkeydown` add is already preceded by an `oninput` of its own,
    /// which is where the clear lives). It does *not* cover two whole-
    /// string `oninput` events back to back with no keystroke between
    /// them (e.g. two full pastes) — the clear and the re-set then both
    /// land in the one render Dioxus coalesces, so the net value never
    /// changes and nothing is announced the second time. Found and left
    /// open; narrow enough, and distant enough from how a person
    /// actually types, not to hold up this fix.
    #[test]
    fn the_ignore_list_live_region_announces_a_repeated_duplicate() {
        use dioxus::html::{PlatformEventData, SerializedFormData};
        use std::any::Any;
        use std::rc::Rc;

        set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

        fn root() -> Element {
            // Seeded with "tmp" already in the list, so every add below is
            // a duplicate from the first one.
            let csv = use_signal(|| "tmp".to_string());
            rsx! {
                IgnoreListField {
                    lang: Lang::En,
                    label: "Ignore file extensions".to_string(),
                    label_id: "test-ignore-extensions".to_string(),
                    placeholder: "Add an extension".to_string(),
                    kind: IgnoreListKind::Extension,
                    csv: csv.read().clone(),
                    on_change: move |v: String| csv.clone().set(v),
                }
            }
        }

        let mut dom = VirtualDom::new(root);
        let first = dom.rebuild_to_vec();
        let live_region_text_id = first
            .edits
            .iter()
            .find_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, id } if value.is_empty() => {
                    Some(*id)
                }
                _ => None,
            })
            .expect("the live region's own empty text node must exist at first mount");
        let input_id = first
            .edits
            .iter()
            .find_map(|m| match m {
                dioxus_core::Mutation::NewEventListener { name, id } if name == "input" => {
                    Some(*id)
                }
                _ => None,
            })
            .expect("the input must register an oninput listener");

        // Each keystroke is its own `input` event, as a real browser sends
        // them — "t", "tm", "tmp", "tmp," — not one event with the whole
        // string, so the clear-then-set each do happen in their own render.
        let mut send = |value: &str| {
            let event = Event::new(
                Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                    value.to_string(),
                    Vec::new(),
                )))) as Rc<dyn Any>,
                true,
            );
            dom.runtime().handle_event("input", event, input_id);
            dom.render_immediate_to_vec()
        };

        for partial in ["t", "tm", "tmp"] {
            send(partial);
        }
        let after_first_duplicate = send("tmp,");
        let first_text = after_first_duplicate.edits.iter().find_map(|m| match m {
            dioxus_core::Mutation::SetText { value, id } if *id == live_region_text_id => {
                Some(value.clone())
            }
            _ => None,
        });
        assert_eq!(
            first_text.as_deref(),
            Some("Already in the list"),
            "the first duplicate must announce, got {after_first_duplicate:?}"
        );

        for partial in ["t", "tm", "tmp"] {
            send(partial);
        }
        let after_second_duplicate = send("tmp,");
        let second_text = after_second_duplicate.edits.iter().find_map(|m| match m {
            dioxus_core::Mutation::SetText { value, id } if *id == live_region_text_id => {
                Some(value.clone())
            }
            _ => None,
        });
        assert_eq!(
            second_text.as_deref(),
            Some("Already in the list"),
            "the second, repeated duplicate must announce too (a real SetText \
             mutation, not just an unchanged value), got {after_second_duplicate:?}"
        );
    }
}
