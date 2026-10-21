//! `forskscope-cli compare <left> <right> [options]` (RFC-094 §2/§3).
//!
//! Two files or two folders, decided from the paths before either is
//! loaded (a file against a folder is refused) — never through
//! `forskscope_compare::file_kind::classify`, which reports a directory as
//! `Unsupported { reason: "not a regular file" }` rather than as a folder
//! (F209: the reason the GUI's own two-folders-on-the-command-line case
//! shows a raw error instead of opening a folder comparison; out of scope
//! here, but the same trap, avoided deliberately).
//!
//! Binary and workbook (`.xlsx`) files report status only, never a
//! detailed diff (RFC-094 §2/§3, Non-goals: "Workbook and binary changes
//! in detail. Status only, as RFC-092") — matching exactly what
//! `build_dir_report` already does for the same two kinds inside a folder
//! comparison.
//!
//! Exit codes follow `diff`: 0 same, 1 different, 2 error or refusal, with
//! one line on standard error naming the path and the reason. Nothing is
//! read but the two paths, and nothing is written — every format prints
//! to standard output only.

use std::fs;
use std::path::{Path, PathBuf};

use forskscope_compare::diff::{DiffAlgorithm, DiffOptions, compute_diff};
use forskscope_compare::dir::{file_digest_equal, recursive_diff_with_rules};
use forskscope_compare::document::{LoadOptions, load_path};
use forskscope_compare::file_kind::FileKind;
use forskscope_compare::{CancellationToken, IgnoreRules};
use forskscope_report::report::{
    BuildOutcome, FileChecksum, FileComparisonDocument, FileCounts, ReportOptions, ReportPathMode,
    SideInfo, build_dir_report, build_file_report, dir_comparison_document,
    file_comparison_document, render_dir_report, render_dir_report_text, render_file_report,
    render_file_report_text,
};
use forskscope_report::{
    PatchOptions, patch_from_binary_notice, patch_from_directories_with_rules,
    patch_from_file_diff, to_unified,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Text,
    Json,
    Patch,
    Html,
}

pub(crate) struct Options {
    pub(crate) left: PathBuf,
    pub(crate) right: PathBuf,
    pub(crate) format: Format,
    pub(crate) diff_options: DiffOptions,
    pub(crate) context_lines: usize,
    pub(crate) ignore_rules: IgnoreRules,
    pub(crate) path_mode: ReportPathMode,
    pub(crate) max_files: usize,
    pub(crate) max_change_bytes: usize,
}

/// What a comparison produced, independent of how it is delivered - the
/// CLI's own `run` prints `output` and turns `identical` into an exit
/// code; the `mcp` module (review 182 §1) wraps the same `output` as a
/// tool result instead. Shared so neither path re-derives the other's
/// comparison logic (`execute`, below) — never a second copy of it.
pub(crate) struct ExecOutcome {
    pub(crate) output: String,
    pub(crate) identical: bool,
}

pub fn run(args: &[String]) -> i32 {
    let opts = match parse(args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("forskscope-cli compare: {msg}");
            return 2;
        }
    };

    match execute(&opts, &CancellationToken::new()) {
        Ok(outcome) => {
            print!("{}", outcome.output);
            if outcome.identical { 0 } else { 1 }
        }
        Err(msg) => {
            eprintln!("forskscope-cli compare: {msg}");
            2
        }
    }
}

/// The comparison itself, decided from the paths and reusable by both the
/// CLI's own `run` and the `mcp` module's `compare` tool. `token` lets a
/// caller cancel a folder comparison mid-walk (RFC-094 §4) - the CLI
/// passes a token nothing ever cancels.
pub(crate) fn execute(opts: &Options, token: &CancellationToken) -> Result<ExecOutcome, String> {
    let left_meta =
        fs::metadata(&opts.left).map_err(|e| format!("{}: {e}", opts.left.display()))?;
    let right_meta =
        fs::metadata(&opts.right).map_err(|e| format!("{}: {e}", opts.right.display()))?;

    match (left_meta.is_dir(), right_meta.is_dir()) {
        (true, true) => compare_dirs(opts, token),
        (false, false) => compare_files(opts),
        _ => Err(format!(
            "cannot compare a file against a folder ({}, {})",
            opts.left.display(),
            opts.right.display()
        )),
    }
}

// ── Argument parsing ─────────────────────────────────────────────────────────

pub(crate) fn parse(args: &[String]) -> Result<Options, String> {
    let mut positionals: Vec<&str> = Vec::new();
    let mut format = Format::Text;
    let mut diff_options = DiffOptions::default();
    let mut context_lines = 3usize;
    let mut ignore_ext = String::new();
    let mut ignore_dir = String::new();
    let mut hide_dotfiles = false;
    let mut path_mode = ReportPathMode::NameOnly;
    let mut max_files = forskscope_report::report::DEFAULT_MAX_CHANGED_FILES_WITH_CONTENT;
    let mut max_change_bytes = forskscope_report::report::DEFAULT_MAX_CHANGE_TEXT_BYTES;

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--format" => format = parse_format(next_value(args, &mut i, "--format")?)?,
            "--algorithm" => {
                diff_options.algorithm = parse_algorithm(next_value(args, &mut i, "--algorithm")?)?
            }
            "--ignore-whitespace" => diff_options.ignore_whitespace = true,
            "--ignore-case" => diff_options.ignore_case = true,
            "--ignore-newlines" => diff_options.ignore_newlines = true,
            "--context" => {
                context_lines = parse_usize(next_value(args, &mut i, "--context")?, "--context")?
            }
            "--ignore-ext" => ignore_ext = next_value(args, &mut i, "--ignore-ext")?.to_string(),
            "--ignore-dir" => ignore_dir = next_value(args, &mut i, "--ignore-dir")?.to_string(),
            "--hide-dotfiles" => hide_dotfiles = true,
            "--paths" => path_mode = parse_path_mode(next_value(args, &mut i, "--paths")?)?,
            "--max-files" => {
                max_files = parse_usize(next_value(args, &mut i, "--max-files")?, "--max-files")?
            }
            "--max-change-bytes" => {
                max_change_bytes = parse_usize(
                    next_value(args, &mut i, "--max-change-bytes")?,
                    "--max-change-bytes",
                )?
            }
            other if other.starts_with("--") => {
                return Err(format!("unrecognized option `{other}`"));
            }
            other => positionals.push(other),
        }
        i += 1;
    }

    if positionals.len() != 2 {
        return Err(format!(
            "expected exactly two paths (left, right), got {}",
            positionals.len()
        ));
    }

    let mut ignore_rules = IgnoreRules::from_settings(&ignore_ext, &ignore_dir);
    ignore_rules.hide_dotfiles = hide_dotfiles;

    Ok(Options {
        left: PathBuf::from(positionals[0]),
        right: PathBuf::from(positionals[1]),
        format,
        diff_options,
        context_lines,
        ignore_rules,
        path_mode,
        max_files,
        max_change_bytes,
    })
}

fn next_value<'a>(args: &'a [String], i: &mut usize, flag: &str) -> Result<&'a str, String> {
    *i += 1;
    args.get(*i)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} requires a value"))
}

pub(crate) fn parse_format(value: &str) -> Result<Format, String> {
    match value {
        "text" => Ok(Format::Text),
        "json" => Ok(Format::Json),
        "patch" => Ok(Format::Patch),
        "html" => Ok(Format::Html),
        other => Err(format!(
            "--format: unrecognized value `{other}` (expected text, json, patch or html)"
        )),
    }
}

pub(crate) fn parse_algorithm(value: &str) -> Result<DiffAlgorithm, String> {
    match value {
        "myers" => Ok(DiffAlgorithm::Myers),
        "patience" => Ok(DiffAlgorithm::Patience),
        "lcs" => Ok(DiffAlgorithm::Lcs),
        "histogram" => Ok(DiffAlgorithm::Histogram),
        other => Err(format!(
            "--algorithm: unrecognized value `{other}` (expected myers, patience, lcs or histogram)"
        )),
    }
}

pub(crate) fn parse_path_mode(value: &str) -> Result<ReportPathMode, String> {
    match value {
        "names" => Ok(ReportPathMode::NameOnly),
        "relative" => Ok(ReportPathMode::Relative),
        "full" => Ok(ReportPathMode::Absolute),
        other => Err(format!(
            "--paths: unrecognized value `{other}` (expected names, relative or full)"
        )),
    }
}

pub(crate) fn parse_usize(value: &str, flag: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{flag}: expected a non-negative integer, got `{value}`"))
}

// ── Shared helpers ────────────────────────────────────────────────────────────

pub(crate) fn timestamp() -> String {
    chrono::Local::now()
        .format("%Y-%m-%d %H:%M %:z")
        .to_string()
}

/// Identity translator: `forskscope-cli` carries no i18n (RFC-094: it
/// reads no settings, including a language setting).
pub(crate) fn tr(s: &str) -> String {
    s.to_string()
}

pub(crate) fn report_options(opts: &Options) -> ReportOptions {
    ReportOptions {
        path_mode: opts.path_mode,
        context_lines: opts.context_lines,
        max_files_with_content: opts.max_files,
        max_change_bytes: opts.max_change_bytes,
        ..ReportOptions::default()
    }
}

// ── File comparison ──────────────────────────────────────────────────────────

fn compare_files(opts: &Options) -> Result<ExecOutcome, String> {
    let load_opts = LoadOptions {
        allow_missing: false,
    };
    let left =
        load_path(&opts.left, load_opts).map_err(|e| format!("{}: {e}", opts.left.display()))?;
    let right =
        load_path(&opts.right, load_opts).map_err(|e| format!("{}: {e}", opts.right.display()))?;

    let both_text = matches!(left.kind, FileKind::Text) && matches!(right.kind, FileKind::Text);
    if !both_text {
        return compare_files_status_only(opts, &left.kind, &right.kind);
    }

    let diff = compute_diff(left.diff_text(), right.diff_text(), opts.diff_options);
    let identical = diff.is_identical();

    let output = match opts.format {
        Format::Patch => {
            let patch_opts = PatchOptions {
                context_lines: opts.context_lines,
                ..PatchOptions::default()
            };
            let rel = display_name(&opts.right, opts.path_mode);
            // Identical: nothing to export, matching the GUI's own
            // "Files are identical — nothing to export." (no file is
            // written there either; here, nothing is printed).
            patch_from_file_diff(rel, &diff, patch_opts)
                .map(|patch| to_unified(&patch))
                .unwrap_or_default()
        }
        Format::Text | Format::Json | Format::Html => {
            let left_checksum = left
                .text
                .as_ref()
                .map(|t| FileChecksum::of_bytes(&t.raw_bytes));
            let right_checksum = right
                .text
                .as_ref()
                .map(|t| FileChecksum::of_bytes(&t.raw_bytes));
            let report = build_file_report(
                &diff,
                Some(opts.left.as_path()),
                Some(opts.right.as_path()),
                report_options(opts),
                left_checksum,
                right_checksum,
            );
            match opts.format {
                Format::Text => render_file_report_text(&report),
                Format::Json => {
                    let left_encoding = left
                        .text
                        .as_ref()
                        .map(|t| t.encoding.label.clone())
                        .unwrap_or_default();
                    let right_encoding = right
                        .text
                        .as_ref()
                        .map(|t| t.encoding.label.clone())
                        .unwrap_or_default();
                    let doc = file_comparison_document(&report, &left_encoding, &right_encoding);
                    format!(
                        "{}\n",
                        doc.to_json_string().expect("document is serializable")
                    )
                }
                Format::Html => format!(
                    "{}\n",
                    render_file_report(
                        &report,
                        env!("CARGO_PKG_VERSION"),
                        &timestamp(),
                        false,
                        &tr
                    )
                ),
                Format::Patch => unreachable!("handled above"),
            }
        }
    };

    Ok(ExecOutcome { output, identical })
}

pub(crate) fn kind_label(kind: &FileKind) -> &'static str {
    match kind {
        FileKind::ExcelXlsx => "workbook",
        FileKind::Binary => "binary",
        _ => "binary",
    }
}

fn compare_files_status_only(
    opts: &Options,
    left_kind: &FileKind,
    right_kind: &FileKind,
) -> Result<ExecOutcome, String> {
    let identical = match file_digest_equal(&opts.left, &opts.right) {
        Ok(b) => b,
        Err(e) => return Err(e.to_string()),
    };
    let left_name = display_name(&opts.left, opts.path_mode);
    let right_name = display_name(&opts.right, opts.path_mode);

    let output = match opts.format {
        Format::Patch => {
            if identical {
                String::new()
            } else {
                let doc = patch_from_binary_notice(right_name);
                to_unified(&doc)
            }
        }
        Format::Text => format!(
            "{} and {} are {} ({} content — status only, RFC-094)\n",
            left_name,
            right_name,
            if identical { "identical" } else { "different" },
            kind_label(left_kind),
        ),
        Format::Json => {
            let side = |path: &Path, name: String, kind: &FileKind| -> SideInfo {
                let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                let sha256 = fs::read(path)
                    .ok()
                    .map(|bytes| forskscope_report::checksum::sha256_hex(&bytes));
                SideInfo {
                    name,
                    size,
                    sha256,
                    encoding: format!("n/a ({})", kind_label(kind)),
                }
            };
            let doc = FileComparisonDocument {
                schema: forskscope_report::report::COMPARISON_SCHEMA,
                kind: "file",
                left: side(&opts.left, left_name, left_kind),
                right: side(&opts.right, right_name, right_kind),
                identical,
                counts: FileCounts {
                    changes: 0,
                    lines_removed: 0,
                    lines_added: 0,
                },
                changes: Vec::new(),
                notes: vec![
                    "binary or workbook content: status only, no detailed diff (RFC-094)".into(),
                ],
            };
            format!(
                "{}\n",
                doc.to_json_string().expect("document is serializable")
            )
        }
        Format::Html => {
            return Err(
                "--format html has no detailed diff for binary or workbook content; use \
                 --format text or json for a status-only result"
                    .to_string(),
            );
        }
    };

    Ok(ExecOutcome { output, identical })
}

// ── Folder comparison ────────────────────────────────────────────────────────

fn compare_dirs(opts: &Options, token: &CancellationToken) -> Result<ExecOutcome, String> {
    let scan = recursive_diff_with_rules(&opts.left, &opts.right, token, &opts.ignore_rules);
    // `recursive_diff_with_rules`'s own doc comment: "the caller can
    // distinguish a cancelled result from a completed one by checking
    // `token.is_cancelled()` afterwards" - checked here, before
    // `build_dir_report`, because that function's own cancellation check
    // runs only inside its per-entry loop. A scan cancelled early enough
    // to produce zero (or very few) entries would otherwise reach that
    // loop with nothing - or almost nothing - to iterate, never observe
    // the cancellation, and return a misleadingly "successful" empty
    // report instead of saying the comparison was cancelled.
    if token.is_cancelled() {
        return Err("comparison was cancelled".to_string());
    }
    if scan.left_root_unreadable {
        return Err(format!("{}: could not be read", opts.left.display()));
    }
    if scan.right_root_unreadable {
        return Err(format!("{}: could not be read", opts.right.display()));
    }

    let outcome = build_dir_report(
        &opts.left,
        &opts.right,
        scan.entries,
        opts.diff_options,
        report_options(opts),
        token,
        &mut |_, _| {},
    );
    let report = match outcome {
        BuildOutcome::Report(r) => *r,
        BuildOutcome::Cancelled => return Err("comparison was cancelled".to_string()),
        BuildOutcome::Stale(path) => {
            return Err(format!(
                "{path}: changed while being read; refusing a stale result"
            ));
        }
    };

    let identical = report.changed == 0
        && report.left_only == 0
        && report.right_only == 0
        && report.unreadable == 0
        && report.symlinks == 0;

    let output = match opts.format {
        Format::Text => render_dir_report_text(&report),
        Format::Json => {
            let doc = dir_comparison_document(&report);
            format!(
                "{}\n",
                doc.to_json_string().expect("document is serializable")
            )
        }
        Format::Patch => {
            let patch_opts = PatchOptions {
                context_lines: opts.context_lines,
                ..PatchOptions::default()
            };
            let doc = patch_from_directories_with_rules(
                &opts.left,
                &opts.right,
                opts.diff_options,
                patch_opts,
                &opts.ignore_rules,
            )
            .map_err(|e| e.to_string())?;
            if doc.is_empty() {
                String::new()
            } else {
                to_unified(&doc)
            }
        }
        Format::Html => format!(
            "{}\n",
            render_dir_report(&report, env!("CARGO_PKG_VERSION"), &timestamp(), false, &tr)
        ),
    };

    Ok(ExecOutcome { output, identical })
}

/// A path's display name under `--paths` (RFC-094 §2): `names` (default)
/// shows the file name only, `full` the given path as-is. `relative` has
/// no root to be relative *to* for a single top-level path pair — falls
/// back to the file name, the same fallback `report::display_path` uses
/// for `Relative` with no root.
pub(crate) fn display_name(path: &Path, mode: ReportPathMode) -> String {
    match mode {
        ReportPathMode::Absolute => path.display().to_string(),
        ReportPathMode::NameOnly | ReportPathMode::Relative => path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
    }
}
