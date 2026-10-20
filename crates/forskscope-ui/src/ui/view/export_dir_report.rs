//! A folder comparison's "Export report…" / "Export patch…" (RFC-092 §1,
//! §3a, §5; handoff 084). The modal that collects the format/paths choice,
//! and the off-UI-thread job that reads, hashes and diffs the entries —
//! with progress, and a Cancel that writes nothing (§5).

use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_core::spawn_forever;

use forskscope_compare::CancellationToken;
use forskscope_compare::dir::RecEntry;
use forskscope_report::report::{
    BuildOutcome, ReportOptions, ReportPathMode, build_dir_report, render_dir_report,
};

use crate::i18n::t;
use crate::state::{Modal, Store, settings::ColourVision};
use crate::ui::component::select_field::{SelectField, SelectOption};
use crate::ui::view::diff_actions::ReportExportFormat;

#[derive(Clone, PartialEq, Debug)]
enum DirExportPhase {
    ChoosingOptions,
    Working { done: usize, total: usize },
}

fn format_value(format: ReportExportFormat) -> &'static str {
    match format {
        ReportExportFormat::Html => "html",
        ReportExportFormat::Patch => "patch",
    }
}

fn path_mode_value(mode: ReportPathMode) -> &'static str {
    match mode {
        ReportPathMode::NameOnly => "name-only",
        ReportPathMode::Relative => "relative",
        ReportPathMode::Absolute => "absolute",
    }
}

fn path_mode_from_value(v: &str) -> ReportPathMode {
    match v {
        "relative" => ReportPathMode::Relative,
        "absolute" => ReportPathMode::Absolute,
        _ => ReportPathMode::NameOnly,
    }
}

/// `left_root`/`right_root`: the compared roots. `entries`: a snapshot of
/// Deep Compare's current table — taken once, at the dialog's open, not
/// re-read live; a comparison that finishes or is cancelled while the
/// dialog is open still describes the snapshot the user opened it on.
#[component]
pub fn ExportDirReportModal(
    left_root: PathBuf,
    right_root: PathBuf,
    entries: Vec<RecEntry>,
    initial_format: ReportExportFormat,
) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut format = use_signal(|| initial_format);
    let mut path_mode = use_signal(|| ReportPathMode::NameOnly);
    let mut more_options = use_signal(|| false);
    let phase = use_signal(|| DirExportPhase::ChoosingOptions);
    let cancel_token: Signal<Option<CancellationToken>> = use_signal(|| None);

    let current_phase = phase.read().clone();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Export report"), onmounted: super::super::overlay::modals::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "Export report")} }

                match current_phase {
                    DirExportPhase::ChoosingOptions => rsx! {
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
                                    let lr = left_root.clone();
                                    let rr = right_root.clone();
                                    let snapshot = entries.clone();
                                    let request = DirExportRequest {
                                        left_root: lr,
                                        right_root: rr,
                                        entries: snapshot,
                                        format: fmt,
                                        path_mode: mode,
                                    };
                                    // `spawn_forever`, not `spawn` (F208): for
                                    // consistency with the file-report path,
                                    // where closing the modal right after this
                                    // call drops a plain `spawn`-ed task before
                                    // its first `.await` — see
                                    // `diff_actions::export_file_report`'s own
                                    // note. Cancel keeps working regardless,
                                    // through `cancel_token`, not scope
                                    // teardown.
                                    spawn_forever(start_dir_export(store, request, phase, cancel_token));
                                },
                                {t(lang, "Export")}
                            }
                        }
                    },
                    DirExportPhase::Working { done, total } => rsx! {
                        p {
                            {if total > 0 {
                                format!("{} {done}/{total}…", t(lang, "Reading and hashing files"))
                            } else {
                                t(lang, "Working…")
                            }}
                        }
                        div { class: "actions",
                            button {
                                onclick: move |_| {
                                    if let Some(token) = cancel_token.read().as_ref() {
                                        token.cancel();
                                    }
                                },
                                {t(lang, "Cancel")}
                            }
                        }
                    },
                }
            }
        }
    }
}

/// What to export, bundled so `start_dir_export` stays under clippy's
/// argument-count gate without losing any of it.
struct DirExportRequest {
    left_root: PathBuf,
    right_root: PathBuf,
    entries: Vec<RecEntry>,
    format: ReportExportFormat,
    path_mode: ReportPathMode,
}

/// Opens the save dialog first — a cancelled file picker does no I/O at
/// all — then runs the read/hash/diff pass with progress, writing the
/// chosen path only once everything has succeeded (RFC-092 §5: Cancel
/// writes nothing).
async fn start_dir_export(
    mut store: Store,
    request: DirExportRequest,
    mut phase: Signal<DirExportPhase>,
    mut cancel_token: Signal<Option<CancellationToken>>,
) {
    let DirExportRequest {
        left_root,
        right_root,
        entries,
        format,
        path_mode,
    } = request;
    let lang = store.lang();
    let default_name = right_root
        .file_name()
        .map(|n| match format {
            ReportExportFormat::Html => format!("{}-report.html", n.to_string_lossy()),
            ReportExportFormat::Patch => format!("{}.patch", n.to_string_lossy()),
        })
        .unwrap_or_else(|| match format {
            ReportExportFormat::Html => "report.html".into(),
            ReportExportFormat::Patch => "changes.patch".into(),
        });

    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title("Export report")
        .set_file_name(&default_name);
    dialog = match format {
        ReportExportFormat::Html => dialog.add_filter("HTML", &["html"]),
        ReportExportFormat::Patch => dialog.add_filter("Patch files", &["patch", "diff"]),
    };
    let handle = dialog.add_filter("All files", &["*"]).save_file().await;
    let Some(file) = handle else {
        store.modal.set(Modal::None);
        return;
    };
    let path = file.path().to_path_buf();

    match format {
        ReportExportFormat::Patch => {
            let diff_options = forskscope_compare::DiffOptions::default();
            let patch_options = forskscope_report::PatchOptions {
                context_lines: store.settings.read().context_lines,
                ..Default::default()
            };
            let lr = left_root.clone();
            let rr = right_root.clone();
            let result = tokio::task::spawn_blocking(move || {
                forskscope_report::patch_from_directories(&lr, &rr, diff_options, patch_options)
            })
            .await;
            store.modal.set(Modal::None);
            match result {
                Ok(Ok(patch)) => {
                    let text = forskscope_report::to_unified(&patch);
                    match crate::ui::view::diff_actions::write_export_file(
                        &path,
                        text.as_bytes(),
                        &t(lang, "Could not save the patch"),
                    ) {
                        Ok(()) => store.notify_success_with_open(
                            t(lang, "Report saved."),
                            t(lang, "Open"),
                            path.display().to_string(),
                        ),
                        Err(msg) => store.notify(msg),
                    }
                }
                // F79: an unreadable entry fails the whole patch, naming it -
                // core's own rule, surfaced here rather than silently
                // producing an incomplete patch.
                Ok(Err(e)) => {
                    store.notify(format!("{}: {e}", t(lang, "Could not build the patch")))
                }
                Err(e) => store.notify(format!("{}: {e}.", t(lang, "Could not save the patch"))),
            }
        }
        ReportExportFormat::Html => {
            let token = CancellationToken::new();
            cancel_token.set(Some(token.clone()));
            phase.set(DirExportPhase::Working {
                done: 0,
                total: entries.len(),
            });

            let diff_options = forskscope_compare::DiffOptions::default();
            let report_options = ReportOptions {
                path_mode,
                context_lines: store.settings.read().context_lines,
                ..Default::default()
            };
            let lr = left_root.clone();
            let rr = right_root.clone();

            let outcome = tokio::task::spawn_blocking(move || {
                build_dir_report(
                    &lr,
                    &rr,
                    entries,
                    diff_options,
                    report_options,
                    &token,
                    &mut |done, total| {
                        // Progress is best-effort - a dropped/lagging signal
                        // write here would not be a correctness problem, only
                        // a stale progress number, and this closure runs on a
                        // blocking thread so it cannot `.await`.
                        let _ = (done, total);
                    },
                )
            })
            .await;

            store.modal.set(Modal::None);
            cancel_token.set(None);

            match outcome {
                Ok(BuildOutcome::Report(report)) => {
                    let color_vision_safe = matches!(
                        store.settings.read().colour_vision,
                        ColourVision::RedGreenSafe
                    );
                    let generated_at = crate::ui::view::diff_actions::local_timestamp_display();
                    let html = render_dir_report(
                        &report,
                        env!("CARGO_PKG_VERSION"),
                        &generated_at,
                        color_vision_safe,
                        &|s| t(lang, s),
                    );
                    match crate::ui::view::diff_actions::write_export_file(
                        &path,
                        html.as_bytes(),
                        &t(lang, "Could not save the report"),
                    ) {
                        Ok(()) => store.notify_success_with_open(
                            t(lang, "Report saved."),
                            t(lang, "Open"),
                            path.display().to_string(),
                        ),
                        Err(msg) => store.notify(msg),
                    }
                }
                // Cancelled: nothing written, nothing to notify beyond
                // silence - the user asked for this.
                Ok(BuildOutcome::Cancelled) => {}
                // RFC-092 §0: refuse before writing anything.
                Ok(BuildOutcome::Stale(path)) => store.notify(format!(
                    "{} ({path})",
                    t(
                        lang,
                        "A file changed since it was compared. Compare again, then export."
                    )
                )),
                Err(e) => store.notify(format!("{}: {e}.", t(lang, "Could not save the report"))),
            }
        }
    }
}
