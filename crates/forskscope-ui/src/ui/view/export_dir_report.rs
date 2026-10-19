//! A folder comparison's "Export report…" / "Export patch…" (RFC-092 §1,
//! §3a, §5; handoff 084). The modal that collects the format/paths choice,
//! and the off-UI-thread job that reads, hashes and diffs the entries —
//! with progress, and a Cancel that writes nothing (§5).

use std::path::{Path, PathBuf};

use dioxus::prelude::*;

use forskscope_core::CancellationToken;
use forskscope_core::dir::{RecEntry, RecStatus};
use forskscope_core::report::{
    ChangeHunk, DirComparisonReport, FileChecksum, ReportOptions, ReportPathMode,
    change_hunks_from_diff, render_dir_report,
};

use crate::i18n::t;
use crate::state::{Modal, Store, settings::ColourVision};
use crate::ui::component::select_field::{SelectField, SelectOption};
use crate::ui::view::diff_actions::ReportExportFormat;

/// RFC-092 §3a: "50 files or 5 MiB of change text, whichever comes first."
const MAX_CHANGED_FILES_WITH_CONTENT: usize = 50;
const MAX_CHANGE_TEXT_BYTES: usize = 5 * 1024 * 1024;

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
                                    spawn(start_dir_export(store, request, phase, cancel_token));
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
            let diff_options = forskscope_core::DiffOptions::default();
            let patch_options = forskscope_core::patch::PatchOptions {
                context_lines: store.settings.read().context_lines,
                ..Default::default()
            };
            let lr = left_root.clone();
            let rr = right_root.clone();
            let result = tokio::task::spawn_blocking(move || {
                forskscope_core::patch::patch_from_directories(
                    &lr,
                    &rr,
                    diff_options,
                    patch_options,
                )
            })
            .await;
            store.modal.set(Modal::None);
            match result {
                Ok(Ok(patch)) => {
                    let text = forskscope_core::patch::to_unified(&patch);
                    match std::fs::write(&path, &text) {
                        Ok(()) => store.notify_success_with_open(
                            t(lang, "Report saved."),
                            t(lang, "Open"),
                            path.display().to_string(),
                        ),
                        Err(e) => eprintln!("export_dir_report (patch): write error: {e}"),
                    }
                }
                // F79: an unreadable entry fails the whole patch, naming it -
                // core's own rule, surfaced here rather than silently
                // producing an incomplete patch.
                Ok(Err(e)) => {
                    store.notify(format!("{}: {e}", t(lang, "Could not build the patch")))
                }
                Err(e) => eprintln!("export_dir_report (patch): join error: {e}"),
            }
        }
        ReportExportFormat::Html => {
            let token = CancellationToken::new();
            cancel_token.set(Some(token.clone()));
            phase.set(DirExportPhase::Working {
                done: 0,
                total: entries.len(),
            });

            let diff_options = forskscope_core::DiffOptions::default();
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
                Ok(Some(report)) => {
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
                    match std::fs::write(&path, &html) {
                        Ok(()) => store.notify_success_with_open(
                            t(lang, "Report saved."),
                            t(lang, "Open"),
                            path.display().to_string(),
                        ),
                        Err(e) => eprintln!("export_dir_report (html): write error: {e}"),
                    }
                }
                // Cancelled: nothing written, nothing to notify beyond
                // silence - the user asked for this.
                Ok(None) => {}
                Err(e) => eprintln!("export_dir_report (html): join error: {e}"),
            }
        }
    }
}

/// Reads, hashes and (for changed text files, under the cap) diffs every
/// entry — off the UI thread via `spawn_blocking`'s caller. Returns `None`
/// if `token` was cancelled before finishing (nothing partial is ever
/// handed back to be written).
fn build_dir_report(
    left_root: &Path,
    right_root: &Path,
    entries: Vec<RecEntry>,
    diff_options: forskscope_core::DiffOptions,
    report_options: ReportOptions,
    token: &CancellationToken,
    on_progress: &mut dyn FnMut(usize, usize),
) -> Option<DirComparisonReport> {
    let total = entries.len();
    let mut report = DirComparisonReport::from_entries(
        &entries,
        Some(left_root),
        Some(right_root),
        None,
        Some(&diff_options),
        report_options,
    );

    let context_lines = report_options_context_lines(&report);
    let mut change_text_bytes = 0usize;
    let mut files_with_content = 0usize;

    for (i, entry) in entries.iter().enumerate() {
        if token.is_cancelled() {
            return None;
        }
        on_progress(i, total);

        let Some(row) = report.file_rows.get_mut(i) else {
            continue;
        };

        let left_path = left_root.join(&entry.rel_path);
        let right_path = right_root.join(&entry.rel_path);

        let left_bytes = matches!(
            entry.status,
            RecStatus::Equal | RecStatus::Changed | RecStatus::LeftOnly
        )
        .then(|| std::fs::read(&left_path).ok())
        .flatten();
        let right_bytes = matches!(
            entry.status,
            RecStatus::Equal | RecStatus::Changed | RecStatus::RightOnly
        )
        .then(|| std::fs::read(&right_path).ok())
        .flatten();

        if let Some(bytes) = &left_bytes {
            row.left_checksum = Some(FileChecksum::of_bytes(bytes));
        }
        if let Some(bytes) = &right_bytes {
            row.right_checksum = Some(FileChecksum::of_bytes(bytes));
        }

        if entry.status == RecStatus::Changed
            && files_with_content < MAX_CHANGED_FILES_WITH_CONTENT
            && change_text_bytes < MAX_CHANGE_TEXT_BYTES
        {
            if let (Some(left_bytes), Some(right_bytes)) = (&left_bytes, &right_bytes) {
                // Binary/spreadsheet entries show status only (RFC-092
                // §3a) - only a file `classify` calls Text gets diffed.
                let is_text = matches!(
                    forskscope_core::file_kind::classify(&left_path),
                    Ok(forskscope_core::FileKind::Text)
                ) && matches!(
                    forskscope_core::file_kind::classify(&right_path),
                    Ok(forskscope_core::FileKind::Text)
                );
                if is_text
                    && let (Ok(left_text), Ok(right_text)) = (
                        String::from_utf8(left_bytes.clone()),
                        String::from_utf8(right_bytes.clone()),
                    )
                {
                    let diff = forskscope_core::compute_diff(&left_text, &right_text, diff_options);
                    let hunks = change_hunks_from_diff(&diff, context_lines);
                    let hunk_text_len: usize = hunks
                        .iter()
                        .flat_map(|h: &ChangeHunk| h.lines.iter())
                        .map(|l| l.content.len())
                        .sum();
                    change_text_bytes += hunk_text_len;
                    files_with_content += 1;
                    row.change_hunks = Some(hunks);
                } else {
                    report.change_hunks_omitted.push(row.path.clone());
                }
            }
        } else if entry.status == RecStatus::Changed {
            report.change_hunks_omitted.push(row.path.clone());
        }
    }
    on_progress(total, total);
    Some(report)
}

fn report_options_context_lines(report: &DirComparisonReport) -> usize {
    report.options.context_lines
}
