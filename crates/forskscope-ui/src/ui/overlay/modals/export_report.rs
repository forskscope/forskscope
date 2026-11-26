//! The comparison-report / unified-diff export dialog for a file comparison
//! (RFC-092 §2, handoff 084). Small on purpose: a format choice (default
//! HTML report), with the path-privacy choice collapsed under "More
//! options ▸" per the owner's rule that detailed settings stay hidden.

use dioxus::prelude::*;
use forskscope_report::report::ReportPathMode;

use crate::i18n::t;
use crate::state::{Modal, Store};
use crate::ui::component::select_field::SelectField;
use crate::ui::view::diff_actions::{
    ReportExportFormat, export_file_report, export_format_options, export_path_mode_options,
};

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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "export-report"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "export-report")} }

                SelectField {
                    title: t(lang, "format"),
                    value: format_value(*format.read()).to_string(),
                    options: export_format_options(lang),
                    onchange: move |v: String| {
                        format.set(if v == "patch" { ReportExportFormat::Patch } else { ReportExportFormat::Html });
                    },
                }

                button {
                    class: "disclosure",
                    onclick: move |_| { let v = *more_options.read(); more_options.set(!v); },
                    if *more_options.read() { {t(lang, "less")} } else { {t(lang, "more-options-2")} }
                }
                if *more_options.read() {
                    div { class: "field",
                        SelectField {
                            title: t(lang, "paths"),
                            value: path_mode_value(*path_mode.read()).to_string(),
                            options: export_path_mode_options(lang),
                            onchange: move |v: String| path_mode.set(path_mode_from_value(&v)),
                        }
                    }
                }

                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| {
                            let fmt = *format.read();
                            let mode = *path_mode.read();
                            export_file_report(&mut store, index, fmt, mode);
                            store.modal.set(Modal::None);
                        },
                        {t(lang, "export")}
                    }
                }
            }
        }
    }
}
