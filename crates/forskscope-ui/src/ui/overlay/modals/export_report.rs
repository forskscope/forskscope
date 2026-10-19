//! The comparison-report / unified-diff export dialog for a file comparison
//! (RFC-092 §2, handoff 084). Small on purpose: a format choice (default
//! HTML report), with the path-privacy choice collapsed under "More
//! options ▸" per the owner's rule that detailed settings stay hidden.

use dioxus::prelude::*;
use forskscope_core::report::ReportPathMode;

use crate::i18n::t;
use crate::state::{Modal, Store};
use crate::ui::component::select_field::{SelectField, SelectOption};
use crate::ui::view::diff_actions::{ReportExportFormat, export_file_report};

fn path_mode_value(mode: ReportPathMode) -> &'static str {
    match mode {
        ReportPathMode::NameOnly => "name-only",
        ReportPathMode::Relative => "relative",
        ReportPathMode::Absolute => "absolute",
    }
}

fn format_value(format: ReportExportFormat) -> &'static str {
    match format {
        ReportExportFormat::Html => "html",
        ReportExportFormat::Patch => "patch",
    }
}

fn path_mode_from_value(v: &str) -> ReportPathMode {
    match v {
        "relative" => ReportPathMode::Relative,
        "absolute" => ReportPathMode::Absolute,
        _ => ReportPathMode::NameOnly,
    }
}

#[component]
pub fn ExportReportModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut format = use_signal(|| ReportExportFormat::Html);
    let mut path_mode = use_signal(|| ReportPathMode::NameOnly);
    let mut more_options = use_signal(|| false);

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Export report"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "Export report")} }

                SelectField {
                    title: t(lang, "Format"),
                    value: format_value(*format.read()).to_string(),
                    options: vec![
                        SelectOption::new("html", t(lang, "HTML report")),
                        SelectOption::new("patch", t(lang, "Unified diff (.patch)")),
                    ],
                    onchange: move |v: String| {
                        format.set(if v == "patch" { ReportExportFormat::Patch } else { ReportExportFormat::Html });
                    },
                }

                button {
                    class: "disclosure",
                    onclick: move |_| { let v = *more_options.read(); more_options.set(!v); },
                    if *more_options.read() { {t(lang, "Less ▲")} } else { {t(lang, "More options ▸")} }
                }
                if *more_options.read() {
                    div { class: "field",
                        SelectField {
                            title: t(lang, "Paths"),
                            value: path_mode_value(*path_mode.read()).to_string(),
                            options: vec![
                                SelectOption::new("name-only", t(lang, "File names only")),
                                SelectOption::new("relative", t(lang, "Paths relative to the compared folders")),
                                SelectOption::new("absolute", t(lang, "Full paths")),
                            ],
                            onchange: move |v: String| path_mode.set(path_mode_from_value(&v)),
                        }
                    }
                }

                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            let fmt = *format.read();
                            let mode = *path_mode.read();
                            export_file_report(&mut store, index, fmt, mode);
                            store.modal.set(Modal::None);
                        },
                        {t(lang, "Export")}
                    }
                }
            }
        }
    }
}
