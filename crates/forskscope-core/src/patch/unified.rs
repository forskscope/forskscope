//! Deterministic unified-diff serialization (RFC-039 §"Export Patch",
//! RFC-084 §"Patch Export Conformance").
//!
//! Output is byte-for-byte reproducible for a given `PatchDocument`:
//! file changes are emitted in their stored order, hunks in order, and
//! lines in order. Each line is terminated with its own source
//! `NewlineMarker` (falling back to `\n` when the line had none, so the
//! patch text stream still separates it from a following `\ No newline at
//! end of file` marker); the marker itself preserves the absence of a
//! trailing newline in the patched content, matching `git diff` and POSIX
//! `diff -u`.

use std::fmt::Write as _;

use crate::diff::NewlineMarker;

use super::model::{PatchDocument, PatchFileChange, PatchHunk};

const NO_NEWLINE_MARKER: &str = "\\ No newline at end of file";

/// Render a complete patch to a unified-diff string.
pub fn to_unified(patch: &PatchDocument) -> String {
    let mut out = String::new();
    write_summary_header(&mut out, patch);
    for change in &patch.files {
        write_file_change(&mut out, change);
    }
    out
}

fn write_summary_header(out: &mut String, patch: &PatchDocument) {
    let s = &patch.summary;
    let total_files = s.files_changed + s.files_added + s.files_deleted + s.binary_files;
    let _ = writeln!(
        out,
        "# forskscope patch: {total_files} files, {} additions(+), {} deletions(-)",
        s.additions, s.deletions
    );
}

fn write_file_change(out: &mut String, change: &PatchFileChange) {
    let path = display_path(change.path());
    let a_header = diff_path_header('a', &path);
    let b_header = diff_path_header('b', &path);
    match change {
        PatchFileChange::Modify { hunks, .. } => {
            let _ = writeln!(out, "--- {a_header}");
            let _ = writeln!(out, "+++ {b_header}");
            for hunk in hunks {
                write_hunk(out, hunk);
            }
        }
        PatchFileChange::Add { lines, .. } => {
            let _ = writeln!(out, "--- /dev/null");
            let _ = writeln!(out, "+++ {b_header}");
            let _ = writeln!(out, "@@ -0,0 +1,{} @@", lines.len());
            write_lines(out, lines);
        }
        PatchFileChange::Delete { lines, .. } => {
            let _ = writeln!(out, "--- {a_header}");
            let _ = writeln!(out, "+++ /dev/null");
            let _ = writeln!(out, "@@ -1,{} +0,0 @@", lines.len());
            write_lines(out, lines);
        }
        PatchFileChange::BinaryNotice { .. } => {
            // `git` never re-parses this notice line to locate a target
            // file, so it is never tab- or quote-disambiguated, even for
            // a path that would need it on `---`/`+++`.
            let _ = writeln!(out, "--- a/{path}");
            let _ = writeln!(out, "+++ b/{path}");
            let _ = writeln!(out, "Binary files a/{path} and b/{path} differ");
        }
    }
}

fn write_hunk(out: &mut String, hunk: &PatchHunk) {
    let _ = writeln!(
        out,
        "@@ -{} +{} @@",
        range(hunk.old_start, hunk.old_len),
        range(hunk.new_start, hunk.new_len)
    );
    write_lines(out, &hunk.lines);
}

fn write_lines(out: &mut String, lines: &[super::model::PatchLine]) {
    for line in lines {
        out.push(line.origin.marker());
        out.push_str(&line.content);
        match line.newline {
            NewlineMarker::None => out.push('\n'),
            other => out.push_str(other.as_str()),
        }
        if line.no_newline_at_eof() {
            out.push_str(NO_NEWLINE_MARKER);
            out.push('\n');
        }
    }
}

/// Format one side of a hunk header. A single-line range is written
/// without the trailing `,count`, matching standard `diff` output.
fn range(start: u32, len: u32) -> String {
    if len == 1 {
        format!("{start}")
    } else {
        format!("{start},{len}")
    }
}

/// Render a relative path with forward slashes regardless of host OS, so
/// patches are portable across platforms (RFC-005 path policy).
///
/// A component that is not valid UTF-8 is rendered lossily (invalid byte
/// sequences become U+FFFD) rather than dropped: dropping a whole path
/// component would silently emit a different path than the one compared,
/// which is worse than a visibly-mangled one.
fn display_path(path: &std::path::Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// `true` when `path` needs git's C-style quoting: a double quote, a
/// backslash, or a control character (bytes < 0x20, or DEL/0x7F).
fn needs_c_style_quote(path: &str) -> bool {
    path.chars()
        .any(|c| c == '"' || c == '\\' || (c as u32) < 0x20 || c == '\u{7f}')
}

/// C-style quote `s` the way `git diff` does: `"` and `\` are backslash-
/// escaped, control characters are octal-escaped (`\NNN`), and everything
/// else — including non-ASCII UTF-8 — passes through verbatim. This is
/// narrower than git's own default (`core.quotePath=true` also octal-
/// escapes plain non-ASCII bytes); that cosmetic behavior is deliberately
/// not replicated here since it is not required for `git apply`/`patch
/// -p1` to accept the result.
fn quote_c_style(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                let _ = write!(out, "\\{:03o}", c as u32);
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Format the `a/`/`b/` path header used on a `---`/`+++` line, following
/// `git diff`'s own disambiguation so `git apply`/`patch -p1` can locate
/// the file: a path needing C-style quoting gets the whole `a/`- or
/// `b/`-prefixed string quoted together; a path with a plain space gets a
/// trailing tab so the parser can find the end of the path; anything else
/// is emitted plain.
fn diff_path_header(prefix: char, path: &str) -> String {
    let combined = format!("{prefix}/{path}");
    if needs_c_style_quote(path) {
        quote_c_style(&combined)
    } else if path.contains(' ') {
        format!("{combined}\t")
    } else {
        combined
    }
}
