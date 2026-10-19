//! Self-contained HTML comparison reports (RFC-092 §3, §3a).
//!
//! One file: inline CSS, no JavaScript, nothing loaded from elsewhere. Reads
//! without colour, prints cleanly, and is inert whatever the compared files
//! contain — every name and line is HTML-escaped (`escape_html`, tested on
//! its own below) **and** the page carries a CSP meta forbidding any load or
//! script, so an escaping mistake alone could not make it do anything.
//!
//! Localization follows this codebase's own established convention (see
//! `deep_compare.rs`'s `{} {count} {}` toolbar labels): a translated word or
//! phrase next to a raw number, concatenated — not a single templated
//! sentence — because `t(lang, key)` maps whole fixed strings, not an
//! ICU-style message format. `forskscope-ui`'s `i18n` depends on this crate,
//! not the other way around, so this module takes a translator closure
//! (`tr`) rather than calling `t()` itself — the UI caller passes
//! `|s| t(lang, s)`.
//!
//! Colours reuse the exact token values `assets/css/01-tokens.css` defines
//! for `.theme-light`/`.theme-light.cv-rgsafe` (RFC-088/090) — the light
//! theme specifically, since a report is meant to print cleanly and travel
//! outside the app, not mirror whichever dark/light/night theme happened to
//! be active when it was made. Only the *Colour vision* setting (Standard
//! vs red-green-safe) is honoured, per RFC-092 §3's explicit scope.

use super::FileChecksum;
use super::changes::{ChangeHunk, ChangeLineKind};
use super::dir::{DirComparisonReport, DirFileRow};
use super::file::FileComparisonReport;
use std::fmt::Write as _;

/// `<meta http-equiv="Content-Security-Policy" ...>`, exactly as RFC-092 §3
/// specifies it — forbids every load and every script, so an escaping
/// mistake in the body could not make the page do anything either.
const CSP_META: &str = r#"<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'">"#;

/// HTML-escape one string. The report's only defence against a file name
/// or a line of file content containing markup — tested against the RFC's
/// own fixtures below. Order matters: `&` first, or escaping the other
/// characters would re-escape the ampersands it just introduced.
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Light-theme diff colour tokens (RFC-088/090's `.theme-light` /
/// `.theme-light.cv-rgsafe`, `crates/forskscope-ui/assets/css/01-tokens.css`).
/// `warn` (used for a replace hunk's mark) has no red-green-safe override in
/// the source tokens either — "already told apart from these by hue and
/// glyph" — so both palettes share the same value.
struct Palette {
    del_bg: &'static str,
    ins_bg: &'static str,
    del_mark: &'static str,
    ins_mark: &'static str,
    rep_bg: &'static str,
    warn: &'static str,
}

const STANDARD: Palette = Palette {
    del_bg: "#ffd7d5",
    ins_bg: "#d3f5e0",
    del_mark: "#cf222e",
    ins_mark: "#1a7f37",
    rep_bg: "#fff3cd",
    warn: "#9a6700",
};

const RED_GREEN_SAFE: Palette = Palette {
    del_bg: "#ffe2c7",
    ins_bg: "#d4e6ff",
    del_mark: "#944800",
    ins_mark: "#0a5fd6",
    rep_bg: "#abbab7",
    warn: "#9a6700",
};

fn palette(color_vision_safe: bool) -> &'static Palette {
    if color_vision_safe {
        &RED_GREEN_SAFE
    } else {
        &STANDARD
    }
}

fn style_block(p: &Palette) -> String {
    format!(
        r#"<style>
body {{ font-family: system-ui, -apple-system, "Segoe UI", sans-serif; color: #1f2328; background: #fff; max-width: 80em; margin: 2em auto; padding: 0 1em; line-height: 1.5; }}
h1, h2 {{ border-bottom: 1px solid #d0d7de; padding-bottom: 0.3em; }}
table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
th, td {{ border: 1px solid #d0d7de; padding: 0.3em 0.6em; text-align: left; font-size: 0.95em; }}
.note {{ color: #57606a; font-style: italic; }}
.hunk {{ margin: 1.2em 0; border: 1px solid #d0d7de; border-radius: 4px; overflow: hidden; page-break-inside: avoid; }}
.hunk-header {{ background: #f6f8fa; padding: 0.3em 0.6em; font-family: ui-monospace, monospace; font-size: 0.9em; }}
.diff-row {{ display: flex; font-family: ui-monospace, "SFMono-Regular", Consolas, monospace; font-size: 0.9em; white-space: pre-wrap; word-break: break-all; }}
.diff-lineno {{ flex: 0 0 4.5em; color: #57606a; text-align: right; padding-right: 0.6em; user-select: none; }}
.diff-mark {{ flex: 0 0 1.4em; text-align: center; font-weight: bold; }}
.diff-content {{ flex: 1 1 auto; padding-right: 0.6em; }}
.hunk-ctx .diff-content {{ color: #1f2328; }}
.hunk-del {{ background: {del_bg}; }}
.hunk-del .diff-mark {{ color: {del_mark}; }}
.hunk-ins {{ background: {ins_bg}; }}
.hunk-ins .diff-mark {{ color: {ins_mark}; }}
.hunk-rep {{ background: {rep_bg}; }}
.hunk-rep .diff-mark {{ color: {warn}; }}
mark.ins {{ background: {ins_bg}; text-decoration: underline; }}
mark.del {{ background: {del_bg}; text-decoration: line-through; }}
.checksum {{ font-family: ui-monospace, monospace; font-size: 0.9em; word-break: break-all; }}
@media print {{
  body {{ max-width: none; margin: 0; }}
  .hunk {{ break-inside: avoid; }}
  a {{ color: inherit; text-decoration: none; }}
}}
</style>"#,
        del_bg = p.del_bg,
        ins_bg = p.ins_bg,
        del_mark = p.del_mark,
        ins_mark = p.ins_mark,
        rep_bg = p.rep_bg,
        warn = p.warn,
    )
}

fn head(title: &str, p: &Palette) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n{CSP_META}\n<title>{}</title>\n{}\n</head>\n<body>\n",
        escape_html(title),
        style_block(p)
    )
}

const FOOT: &str = "</body>\n</html>\n";

fn mark_for(kind: ChangeLineKind) -> &'static str {
    match kind {
        ChangeLineKind::Context => " ",
        ChangeLineKind::Delete => "\u{2212}", // U+2212 MINUS SIGN, not a hyphen - RFC-092 §3's "− and + marks"
        ChangeLineKind::Insert => "+",
    }
}

fn row_class(kind: ChangeLineKind, is_replace_hunk: bool) -> &'static str {
    match (kind, is_replace_hunk) {
        (ChangeLineKind::Context, _) => "diff-row hunk-ctx",
        (ChangeLineKind::Delete, true) => "diff-row hunk-rep",
        (ChangeLineKind::Insert, true) => "diff-row hunk-rep",
        (ChangeLineKind::Delete, false) => "diff-row hunk-del",
        (ChangeLineKind::Insert, false) => "diff-row hunk-ins",
    }
}

/// Render one line's content, wrapping any inline highlight spans in
/// `<mark>` — escaped span-by-span so a span boundary can never split an
/// HTML entity in half.
fn render_content(line: &super::changes::ChangeLine) -> String {
    match &line.spans {
        None => escape_html(&line.content),
        Some(spans) => {
            let mut out = String::new();
            for span in spans {
                let escaped = escape_html(&span.text);
                match span.kind {
                    crate::diff::InlineKind::Equal => out.push_str(&escaped),
                    crate::diff::InlineKind::Insert => {
                        let _ = write!(out, "<mark class=\"ins\">{escaped}</mark>");
                    }
                    crate::diff::InlineKind::Delete => {
                        let _ = write!(out, "<mark class=\"del\">{escaped}</mark>");
                    }
                }
            }
            out
        }
    }
}

/// Whether each line in `lines` belongs to a replacement, by run rather than
/// by whole display block (review 175 §1): a display block can merge an
/// actual replacement with a separate pure addition or removal that happens
/// to sit within context distance of it, and painting the whole block as
/// replaced would then colour that separate change wrongly. A "run" is a
/// maximal span of consecutive non-`Context` lines; a run counts as a
/// replacement only if it contains both a `Delete` and an `Insert`.
fn is_replace_runs(lines: &[super::changes::ChangeLine]) -> Vec<bool> {
    let mut flags = vec![false; lines.len()];
    let mut run_start = None;
    for i in 0..=lines.len() {
        let at_boundary = i == lines.len() || lines[i].kind == ChangeLineKind::Context;
        if at_boundary {
            if let Some(start) = run_start {
                let run = &lines[start..i];
                let has_delete = run.iter().any(|l| l.kind == ChangeLineKind::Delete);
                let has_insert = run.iter().any(|l| l.kind == ChangeLineKind::Insert);
                if has_delete && has_insert {
                    flags[start..i].fill(true);
                }
                run_start = None;
            }
        } else if run_start.is_none() {
            run_start = Some(i);
        }
    }
    flags
}

fn render_hunk(out: &mut String, hunk: &ChangeHunk) {
    let is_replace = is_replace_runs(&hunk.lines);
    let _ = writeln!(out, "<div class=\"hunk\">");
    let _ = writeln!(
        out,
        "<div class=\"hunk-header\">@@ -{},{} +{},{} @@</div>",
        hunk.old_start, hunk.old_len, hunk.new_start, hunk.new_len
    );
    for (line, is_replace) in hunk.lines.iter().zip(is_replace.iter()) {
        let left = line.left_line.map(|n| n.to_string()).unwrap_or_default();
        let right = line.right_line.map(|n| n.to_string()).unwrap_or_default();
        let _ = writeln!(
            out,
            "<div class=\"{}\"><span class=\"diff-lineno\">{left}</span><span class=\"diff-lineno\">{right}</span><span class=\"diff-mark\">{}</span><span class=\"diff-content\">{}</span></div>",
            row_class(line.kind, *is_replace),
            mark_for(line.kind),
            render_content(line)
        );
    }
    let _ = writeln!(out, "</div>");
}

/// Builds the checksum caveat sentence as ready-to-embed HTML: the
/// translated template is escaped first, and `<code>sha256sum</code>` is
/// substituted into its `{cmd}` placeholder afterward (review 175 §2) — a
/// literal backtick-quoted `` `sha256sum` `` prints as literal backticks,
/// since nothing in this renderer converts Markdown to HTML. Callers must
/// not run this result through `escape_html` again, or the inserted
/// `<code>` markup would itself get escaped.
fn checksum_caveat(tr: &dyn Fn(&str) -> String) -> String {
    let template = tr(
        "Run {cmd} on a file to confirm it is the file this report describes. This does not prove the report itself is unaltered.",
    );
    escape_html(&template).replace("{cmd}", "<code>sha256sum</code>")
}

fn render_checksum_row(out: &mut String, label: &str, checksum: &FileChecksum) {
    let _ = writeln!(
        out,
        "<tr><th>{}</th><td>{} B — <span class=\"checksum\">{}</span></td></tr>",
        escape_html(label),
        checksum.size,
        escape_html(&checksum.sha256_hex)
    );
}

/// The one-sentence summary (RFC-092 §3 item 4), built word-by-word through
/// `tr` rather than by translating `FileComparisonReport::one_sentence_summary()`'s
/// pre-composed English output — that method exists as a standalone,
/// English-only convenience (documented and tested on its own), but handing
/// its whole rendered sentence to `tr` could never match a fixed-string
/// translation table once it contains live numbers. This rebuilds the same
/// facts from the report's raw fields instead, one translated word next to
/// each raw number, the same concatenation convention the rest of the UI
/// uses for a count in a label (e.g. `deep_compare.rs`'s `{} {count} {}`).
/// The English template *key* for `(n, d, a)` — hunks changed, lines
/// removed, lines added — with the correct singular/plural word already
/// baked into the literal text (review 174 §4: "whole-sentence templates
/// per language... with singular forms in English"). `{n}`/`{d}`/`{a}`
/// stay as literal placeholder text; `tr()` translates this whole string
/// as one unit (English gets it back verbatim; Japanese's `ja()` match
/// has its own entry for each of the 18 shapes this can produce — see
/// `i18n.rs`), then the placeholders are substituted. Deliberately not a
/// composed-from-words sentence: Japanese's own template has different
/// punctuation and word order throughout, not just translated words in
/// the same slots English uses — concatenating translated fragments can't
/// produce that.
///
/// No "hunks replaced": a count of *hunks* next to counts of *lines* reads
/// as more change stacked on top of the lines already counted, and the
/// change list below already shows which hunks are replacements.
fn summary_template_key(n: usize, d: usize, a: usize) -> String {
    let n_word = if n == 1 { "change" } else { "changes" };
    match (d > 0, a > 0) {
        (true, true) => {
            let d_word = if d == 1 {
                "line removed"
            } else {
                "lines removed"
            };
            let a_word = if a == 1 { "line added" } else { "lines added" };
            format!("{{n}} {n_word}: {{d}} {d_word}, {{a}} {a_word}.")
        }
        (true, false) => {
            let d_word = if d == 1 {
                "line removed"
            } else {
                "lines removed"
            };
            format!("{{n}} {n_word}: {{d}} {d_word}.")
        }
        (false, true) => {
            let a_word = if a == 1 { "line added" } else { "lines added" };
            format!("{{n}} {n_word}: {{a}} {a_word}.")
        }
        (false, false) => format!("{{n}} {n_word}."),
    }
}

fn localized_summary_sentence(
    report: &FileComparisonReport,
    tr: &dyn Fn(&str) -> String,
) -> String {
    if report.is_identical {
        return tr("No differences — the two files are identical.");
    }
    let n = report.hunks_changed;
    let d = report.lines_deleted;
    let a = report.lines_added;
    let key = summary_template_key(n, d, a);
    tr(&key)
        .replace("{n}", &n.to_string())
        .replace("{d}", &d.to_string())
        .replace("{a}", &a.to_string())
}

// ── File report ────────────────────────────────────────────────────────────────

pub fn render_file_report(
    report: &FileComparisonReport,
    app_version: &str,
    generated_at: &str,
    color_vision_safe: bool,
    tr: &dyn Fn(&str) -> String,
) -> String {
    let p = palette(color_vision_safe);
    let mut out = head(&tr("Comparison report"), p);

    let _ = writeln!(out, "<h1>{}</h1>", escape_html(&tr("Comparison report")));
    let _ = writeln!(
        out,
        "<p>{} {} — ForskScope {}</p>",
        escape_html(&tr("Generated")),
        escape_html(generated_at),
        escape_html(app_version)
    );

    let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("What was compared")));
    let _ = writeln!(out, "<table>");
    let _ = writeln!(
        out,
        "<tr><th>{}</th><td>{}</td></tr>",
        escape_html(&tr("Left")),
        escape_html(&report.left_display)
    );
    let _ = writeln!(
        out,
        "<tr><th>{}</th><td>{}</td></tr>",
        escape_html(&tr("Right")),
        escape_html(&report.right_display)
    );
    if let Some(c) = &report.left_checksum {
        render_checksum_row(&mut out, &format!("{} {}", tr("Left"), tr("SHA-256")), c);
    }
    if let Some(c) = &report.right_checksum {
        render_checksum_row(&mut out, &format!("{} {}", tr("Right"), tr("SHA-256")), c);
    }
    let _ = writeln!(out, "</table>");
    if report.left_checksum.is_some() || report.right_checksum.is_some() {
        let _ = writeln!(out, "<p class=\"note\">{}</p>", checksum_caveat(tr));
    }

    let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("How")));
    let _ = writeln!(
        out,
        "<p>{}: {}</p>",
        escape_html(&tr("Compare profile")),
        escape_html(&tr(&report.profile_name))
    );
    if !report.non_default_options.is_empty() {
        let _ = writeln!(out, "<ul>");
        for (k, v) in &report.non_default_options {
            let _ = writeln!(
                out,
                "<li>{}: {}</li>",
                escape_html(&tr(k)),
                escape_html(&tr(v))
            );
        }
        let _ = writeln!(out, "</ul>");
    }

    let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("Summary")));
    let _ = writeln!(
        out,
        "<p>{}</p>",
        escape_html(&localized_summary_sentence(report, tr))
    );

    if !report.change_hunks.is_empty() {
        let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("The changes")));
        for hunk in &report.change_hunks {
            render_hunk(&mut out, hunk);
        }
    }

    if !report.warnings.is_empty() {
        let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("Notes")));
        let _ = writeln!(out, "<ul>");
        for w in &report.warnings {
            let _ = writeln!(out, "<li>{}</li>", escape_html(&tr(w)));
        }
        let _ = writeln!(out, "</ul>");
    }

    out.push_str(FOOT);
    out
}

// ── Directory report ──────────────────────────────────────────────────────────

/// Joins `label: count` for every non-zero count (review 175 §3) —
/// `"Only on the left: 0"` is noise this report must not print, the same
/// rule already applied to the summary sentence's own numbers. `None` when
/// every count given is zero, so a caller can decide what that means (a
/// "No files." substitute for the main counts, or simply nothing for the
/// already-optional Symlink/Unreadable line).
fn counts_line(tr: &dyn Fn(&str) -> String, counts: &[(&str, usize)]) -> Option<String> {
    let parts: Vec<String> = counts
        .iter()
        .filter(|(_, n)| *n > 0)
        .map(|(key, n)| format!("{}: {n}", escape_html(&tr(key))))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

pub fn render_dir_report(
    report: &DirComparisonReport,
    app_version: &str,
    generated_at: &str,
    color_vision_safe: bool,
    tr: &dyn Fn(&str) -> String,
) -> String {
    let p = palette(color_vision_safe);
    let mut out = head(&tr("Comparison report"), p);

    let _ = writeln!(out, "<h1>{}</h1>", escape_html(&tr("Comparison report")));
    let _ = writeln!(
        out,
        "<p>{} {} — ForskScope {}</p>",
        escape_html(&tr("Generated")),
        escape_html(generated_at),
        escape_html(app_version)
    );

    let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("What was compared")));
    let _ = writeln!(out, "<table>");
    let _ = writeln!(
        out,
        "<tr><th>{}</th><td>{}</td></tr>",
        escape_html(&tr("Left")),
        escape_html(&report.left_display)
    );
    let _ = writeln!(
        out,
        "<tr><th>{}</th><td>{}</td></tr>",
        escape_html(&tr("Right")),
        escape_html(&report.right_display)
    );
    let _ = writeln!(out, "</table>");
    let main_counts = [
        (
            crate::dir::status_key(crate::dir::RecStatus::Equal, false),
            report.equal,
        ),
        (
            crate::dir::status_key(crate::dir::RecStatus::Changed, false),
            report.changed,
        ),
        (
            crate::dir::status_key(crate::dir::RecStatus::LeftOnly, false),
            report.left_only,
        ),
        (
            crate::dir::status_key(crate::dir::RecStatus::RightOnly, false),
            report.right_only,
        ),
    ];
    let line = counts_line(tr, &main_counts).unwrap_or_else(|| escape_html(&tr("No files.")));
    let _ = writeln!(out, "<p>{line}</p>");

    let extra_counts = [
        (
            crate::dir::status_key(crate::dir::RecStatus::Symlink, false),
            report.symlinks,
        ),
        (
            crate::dir::status_key(crate::dir::RecStatus::Unreadable, false),
            report.unreadable,
        ),
    ];
    if let Some(line) = counts_line(tr, &extra_counts) {
        let _ = writeln!(out, "<p>{line}</p>");
    }

    if let Some(name) = &report.profile_name {
        let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("How")));
        let _ = writeln!(
            out,
            "<p>{}: {}</p>",
            escape_html(&tr("Compare profile")),
            escape_html(&tr(name))
        );
        if !report.non_default_options.is_empty() {
            let _ = writeln!(out, "<ul>");
            for (k, v) in &report.non_default_options {
                let _ = writeln!(
                    out,
                    "<li>{}: {}</li>",
                    escape_html(&tr(k)),
                    escape_html(&tr(v))
                );
            }
            let _ = writeln!(out, "</ul>");
        }
    }

    let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("Files")));
    let _ = writeln!(out, "<table>");
    let show_sizes = report.options.include_sizes;
    let _ = writeln!(
        out,
        "<tr><th>{}</th><th>{}</th>{}<th>{}</th></tr>",
        escape_html(&tr("Path")),
        escape_html(&tr("Status")),
        if show_sizes {
            format!(
                "<th>{}</th><th>{}</th>",
                escape_html(&tr("Left size")),
                escape_html(&tr("Right size"))
            )
        } else {
            String::new()
        },
        escape_html(&tr("SHA-256"))
    );
    for row in &report.file_rows {
        render_dir_file_row(&mut out, row, show_sizes, tr);
    }
    let _ = writeln!(out, "</table>");
    if report
        .file_rows
        .iter()
        .any(|r| r.left_checksum.is_some() || r.right_checksum.is_some())
    {
        let _ = writeln!(out, "<p class=\"note\">{}</p>", checksum_caveat(tr));
    }

    let changed_with_hunks: Vec<&DirFileRow> = report
        .file_rows
        .iter()
        .filter(|r| r.change_hunks.as_ref().is_some_and(|h| !h.is_empty()))
        .collect();
    if !changed_with_hunks.is_empty() {
        let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("The changes")));
        for row in changed_with_hunks {
            let _ = writeln!(out, "<h3>{}</h3>", escape_html(&row.path));
            for hunk in row.change_hunks.as_ref().unwrap() {
                render_hunk(&mut out, hunk);
            }
        }
    }

    if !report.change_hunks_omitted.is_empty() {
        let _ = writeln!(out, "<h2>{}</h2>", escape_html(&tr("Notes")));
        let _ = writeln!(
            out,
            "<p>{} {}:</p>",
            report.change_hunks_omitted.len(),
            escape_html(&tr(
                "changed files' content is not shown in this report (the cap of 50 files or 5 MiB of change text was reached)"
            ))
        );
        let _ = writeln!(out, "<ul>");
        for name in &report.change_hunks_omitted {
            let _ = writeln!(out, "<li>{}</li>", escape_html(name));
        }
        let _ = writeln!(out, "</ul>");
    }

    out.push_str(FOOT);
    out
}

fn render_dir_file_row(
    out: &mut String,
    row: &DirFileRow,
    show_sizes: bool,
    tr: &dyn Fn(&str) -> String,
) {
    let sizes = if show_sizes {
        format!(
            "<td>{}</td><td>{}</td>",
            row.left_size.map(super::dir::fmt_size).unwrap_or_default(),
            row.right_size.map(super::dir::fmt_size).unwrap_or_default()
        )
    } else {
        String::new()
    };
    let checksums = match (&row.left_checksum, &row.right_checksum) {
        (None, None) => String::new(),
        (left, right) => {
            let mut s = String::new();
            if let Some(c) = left {
                let _ = write!(
                    s,
                    "L: <span class=\"checksum\">{}</span>",
                    escape_html(&c.sha256_hex)
                );
            }
            if let Some(c) = right {
                if !s.is_empty() {
                    s.push_str("<br>");
                }
                let _ = write!(
                    s,
                    "R: <span class=\"checksum\">{}</span>",
                    escape_html(&c.sha256_hex)
                );
            }
            s
        }
    };
    let _ = writeln!(
        out,
        "<tr><td>{}</td><td>{}</td>{sizes}<td>{checksums}</td></tr>",
        escape_html(&row.path),
        // review 174 §3: the same `status_key` Deep Compare's live view
        // reads, not `row.status` (the machine word `to_markdown`/
        // `to_json` keep - RFC-092 §6, those two stay as they are). A
        // report never renders while its scan is still live (export is
        // disabled until it finishes or is cancelled), so `in_progress` is
        // always `false` here - a `Computing` row always reads "Not
        // compared", never "Comparing…", in a report.
        escape_html(&tr(crate::dir::status_key(row.rec_status, false)))
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── RFC-092 §3's two deliberate defences, each tested to fail alone ────

    #[test]
    fn escape_html_neutralizes_a_script_tag() {
        let escaped = escape_html("</pre><script>alert(1)</script>");
        assert!(!escaped.contains("<script>"));
        assert!(escaped.contains("&lt;script&gt;"));
    }

    #[test]
    fn escape_html_neutralizes_an_onerror_image_attribute() {
        let escaped = escape_html(r#"<img src=x onerror=alert(1)>"#);
        assert!(!escaped.contains("<img"));
        assert!(escaped.contains("&lt;img"));
    }

    #[test]
    fn escape_html_escapes_ampersand_before_other_entities_not_after() {
        // If `&` were escaped last, "<" -> "&lt;" would itself get
        // re-escaped into "&amp;lt;" - this fails loudly if the order in
        // `escape_html` above is ever reordered.
        assert_eq!(escape_html("<"), "&lt;");
        assert_eq!(escape_html("&lt;"), "&amp;lt;");
    }

    #[test]
    fn a_report_without_the_csp_meta_would_fail_this_check_removed_it_is_present() {
        let report = crate::report::FileComparisonReport::from_diff(
            &crate::diff::compute_diff("a\n", "b\n", crate::diff::DiffOptions::default()),
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        // A fixed literal, not `CSP_META` itself - comparing against the
        // same constant the page is built from would pass even if that
        // constant were emptied out (caught live: it did, until this was
        // fixed to check the exact required directive text instead).
        assert!(html.contains(
            r#"<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'">"#
        ));
    }

    #[test]
    fn a_fixture_containing_markup_appears_as_text_not_as_markup_in_the_rendered_report() {
        let diff = crate::diff::compute_diff(
            "safe\n",
            "</pre><script>alert(1)</script>\n<img src=x onerror=alert(1)>\n",
            crate::diff::DiffOptions::default(),
        );
        let report = crate::report::FileComparisonReport::from_diff(
            &diff,
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img src=x"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn the_report_carries_no_script_tag_and_loads_nothing_external() {
        let report = crate::report::FileComparisonReport::from_diff(
            &crate::diff::compute_diff("a\n", "b\n", crate::diff::DiffOptions::default()),
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(!html.contains("<script"));
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
    }

    #[test]
    fn checksums_render_only_when_attached() {
        let report = crate::report::FileComparisonReport::from_diff(
            &crate::diff::compute_diff("a\n", "b\n", crate::diff::DiffOptions::default()),
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let without = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        // The CSS rule `.checksum { ... }` is always present; what must be
        // absent is the *caveat sentence*, shown only once a checksum is
        // actually attached and rendered.
        assert!(!without.contains("sha256sum"));

        let report = report.with_checksums(
            Some(FileChecksum::of_bytes(b"a\n")),
            Some(FileChecksum::of_bytes(b"b\n")),
        );
        let with = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(with.contains(&crate::checksum::sha256_hex(b"a\n")));
        assert!(with.contains(&crate::checksum::sha256_hex(b"b\n")));
    }

    #[test]
    fn a_pure_addition_sharing_a_block_with_a_replacement_keeps_its_own_colour() {
        // review 175 §1: a display block merges nearby changes within
        // context distance. Here it holds a genuine replacement (one
        // deleted line immediately followed by one inserted line) and,
        // separated by an unchanged context line, a pure addition with no
        // paired delete - exactly the `file-default-rgsafe.pdf`/
        // `file-ja.pdf` shape the review cites ("line six" wrongly
        // inheriting the replacement colour).
        fn line(kind: ChangeLineKind, content: &str) -> super::super::changes::ChangeLine {
            super::super::changes::ChangeLine {
                kind,
                left_line: None,
                right_line: None,
                content: content.to_string(),
                spans: None,
            }
        }
        let lines = vec![
            line(ChangeLineKind::Context, "line one"),
            line(ChangeLineKind::Delete, "line two old"), // } replacement
            line(ChangeLineKind::Insert, "line two new"), // }
            line(ChangeLineKind::Context, "line five"),
            line(ChangeLineKind::Insert, "line six"), // pure addition, no delete
        ];
        let flags = is_replace_runs(&lines);
        assert_eq!(
            flags,
            vec![false, true, true, false, false],
            "only the delete/insert pair is a replacement; the later pure \
             addition must not inherit it"
        );

        // Confirmed this test catches the regression: temporarily restoring
        // the old per-block rule (`has_delete && has_insert` over the whole
        // slice) made every changed line read `true`, including "line six".
    }

    #[test]
    fn a_change_hunk_shows_both_marks_and_the_changed_line_numbers() {
        let diff = crate::diff::compute_diff(
            "one\ntwo\n",
            "one\nTWO\n",
            crate::diff::DiffOptions::default(),
        );
        let report = crate::report::FileComparisonReport::from_diff(
            &diff,
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(
            html.contains('\u{2212}'),
            "a minus mark on the deleted line"
        );
        assert!(html.contains('+'), "a plus mark on the inserted line");
        assert!(html.contains(">two<") || html.contains("two"));
        assert!(html.contains("TWO"));
    }

    // ── Review 174 §4: whole-sentence summary templates ────────────────────

    #[test]
    fn summary_template_key_matches_the_reviews_own_examples() {
        // "3 changes: 2 lines removed, 3 lines added." (review 174 §4)
        assert_eq!(
            summary_template_key(3, 2, 3),
            "{n} changes: {d} lines removed, {a} lines added."
        );
        // "1 change: 1 line added." (review 174 §4)
        assert_eq!(summary_template_key(1, 0, 1), "{n} change: {a} line added.");
    }

    #[test]
    fn singular_and_plural_are_resolved_independently_per_number() {
        // n, d and a each get their own singular/plural word, not just n's.
        assert_eq!(
            summary_template_key(5, 1, 3),
            "{n} changes: {d} line removed, {a} lines added."
        );
        assert_eq!(
            summary_template_key(1, 4, 1),
            "{n} change: {d} lines removed, {a} line added."
        );
    }

    #[test]
    fn a_zero_count_is_omitted_never_shown_as_zero() {
        assert_eq!(
            summary_template_key(2, 2, 0),
            "{n} changes: {d} lines removed."
        );
        assert_eq!(
            summary_template_key(2, 0, 2),
            "{n} changes: {a} lines added."
        );
        assert_eq!(summary_template_key(2, 0, 0), "{n} changes.");
    }

    #[test]
    fn no_hunks_or_replaced_jargon_appears_in_any_template() {
        for n in 1..=3 {
            for d in 0..=2 {
                for a in 0..=2 {
                    let key = summary_template_key(n, d, a);
                    assert!(!key.contains("hunk"), "{key:?} must not mention hunks");
                    assert!(!key.contains("replace"), "{key:?} must not mention replace");
                }
            }
        }
    }

    #[test]
    fn the_rendered_sentence_substitutes_real_numbers_into_the_chosen_template() {
        let diff = crate::diff::compute_diff(
            "one\ntwo\n",
            "one\nTWO\n",
            crate::diff::DiffOptions::default(),
        );
        let report = crate::report::FileComparisonReport::from_diff(
            &diff,
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(html.contains("1 change: 1 line removed, 1 line added."));
    }

    #[test]
    fn a_translator_that_only_knows_whole_templates_still_produces_a_real_japanese_sentence() {
        // Mirrors i18n.rs's real ja() shape: whole templates, not
        // individual words - this translator cannot answer "lines
        // removed" on its own, only the full key.
        // Two widely-separated changes, so they land in separate hunks and
        // hunks_changed is unambiguously > 1 (picking the plural template
        // key deterministically, regardless of windowing details).
        let left = "a\nb\n".to_string() + &"x\n".repeat(10) + "c\n";
        let right = "a\nB\n".to_string() + &"x\n".repeat(10) + "C\nd\n";
        let html = render_file_report(
            &crate::report::FileComparisonReport::from_diff(
                &crate::diff::compute_diff(&left, &right, crate::diff::DiffOptions::default()),
                None,
                None,
                None,
                crate::report::ReportOptions::default(),
            ),
            "0.0.0",
            "now",
            false,
            &|s| match s {
                "{n} changes: {d} lines removed, {a} lines added." => {
                    "{n} か所の変更（削除 {d} 行、追加 {a} 行）。".to_string()
                }
                other => other.to_string(),
            },
        );
        assert!(
            html.contains("か所の変更"),
            "expected the Japanese whole-sentence template to be used, got: {html}"
        );
        assert!(
            !html.contains("changes:"),
            "the raw English template must not leak through"
        );
    }

    #[test]
    fn a_translator_closure_is_honoured() {
        let report = crate::report::FileComparisonReport::from_diff(
            &crate::diff::compute_diff("a\n", "b\n", crate::diff::DiffOptions::default()),
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_file_report(&report, "0.0.0", "now", false, &|s| {
            if s == "Comparison report" {
                "比較レポート".to_string()
            } else {
                s.to_string()
            }
        });
        assert!(html.contains("比較レポート"));
        assert!(!html.contains(">Comparison report<"));
    }

    #[test]
    fn red_green_safe_uses_a_different_palette_than_standard() {
        let report = crate::report::FileComparisonReport::from_diff(
            &crate::diff::compute_diff("a\n", "b\n", crate::diff::DiffOptions::default()),
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let standard = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        let safe = render_file_report(&report, "0.0.0", "now", true, &|s| s.to_string());
        assert_ne!(standard, safe);
        assert!(standard.contains(STANDARD.del_mark));
        assert!(safe.contains(RED_GREEN_SAFE.del_mark));
    }

    #[test]
    fn an_identical_file_comparison_still_renders_a_complete_page() {
        let report = crate::report::FileComparisonReport::from_diff(
            &crate::diff::compute_diff("same\n", "same\n", crate::diff::DiffOptions::default()),
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_file_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.trim_end().ends_with("</html>"));
        assert!(html.contains("identical"));
    }

    #[test]
    fn a_dir_report_lists_files_with_the_apps_own_status_words() {
        use crate::dir::{RecEntry, RecStatus};
        use std::path::PathBuf;

        let entries = vec![
            RecEntry {
                rel_path: PathBuf::from("a.txt"),
                status: RecStatus::Equal,
                left_size: Some(1),
                right_size: Some(1),
            },
            RecEntry {
                rel_path: PathBuf::from("b.txt"),
                status: RecStatus::Changed,
                left_size: Some(2),
                right_size: Some(3),
            },
        ];
        let report = crate::report::DirComparisonReport::from_entries(
            &entries,
            None,
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_dir_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(html.contains("a.txt"));
        assert!(html.contains("b.txt"));
        // Review 174 §3: the app's own words - the same ones
        // deep_compare.rs's status_label reads, via the shared
        // forskscope_core::dir::status_key - not the report model's own
        // invented "equal"/"modified" (those stay in to_markdown/to_json
        // only, RFC-092 §6).
        assert!(html.contains("Identical"));
        assert!(html.contains("Different"));

        // review 175 §3: no "Only on the left: 0" / "Only on the right: 0"
        // noise - both are genuinely zero in this fixture.
        assert!(!html.contains("Only on the left: 0"));
        assert!(!html.contains("Only on the right: 0"));
        // And the sizes carry the same unit the file report's checksum
        // row does ("71 B"), not a bare number.
        assert!(html.contains("2 B"));
        assert!(html.contains("3 B"));
    }

    #[test]
    fn a_folder_report_with_every_count_at_zero_says_no_files() {
        let report = crate::report::DirComparisonReport::from_entries(
            &[],
            None,
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_dir_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(html.contains("No files."));
        assert!(!html.contains("Identical: 0"));
        assert!(!html.contains("Different: 0"));
        assert!(!html.contains("Only on the left: 0"));
        assert!(!html.contains("Only on the right: 0"));
    }

    #[test]
    fn a_dir_reports_cap_notice_names_every_omitted_file() {
        use crate::dir::{RecEntry, RecStatus};
        use std::path::PathBuf;

        let entries = vec![RecEntry {
            rel_path: PathBuf::from("huge.txt"),
            status: RecStatus::Changed,
            left_size: Some(1),
            right_size: Some(1),
        }];
        let mut report = crate::report::DirComparisonReport::from_entries(
            &entries,
            None,
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        report.change_hunks_omitted.push("huge.txt".into());
        let html = render_dir_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        assert!(html.contains("huge.txt"));
        assert!(html.contains("1")); // the cap count
    }

    #[test]
    fn a_cancelled_scans_unfinished_entry_renders_not_compared() {
        use crate::dir::{RecEntry, RecStatus};
        use std::path::PathBuf;

        let entries = vec![RecEntry {
            rel_path: PathBuf::from("still-going.txt"),
            status: RecStatus::Computing,
            left_size: Some(1),
            right_size: Some(1),
        }];
        let report = crate::report::DirComparisonReport::from_entries(
            &entries,
            None,
            None,
            None,
            None,
            crate::report::ReportOptions::default(),
        );
        let html = render_dir_report(&report, "0.0.0", "now", false, &|s| s.to_string());
        // Review 174 §2a/§3: the app's own word, via status_key - a report
        // never renders mid-scan, so a Computing entry always reads "Not
        // compared", never "Comparing…".
        assert!(html.contains("Not compared"));
        assert!(!html.contains("Comparing"));
    }
}
