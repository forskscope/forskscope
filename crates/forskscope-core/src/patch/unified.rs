//! Deterministic unified-diff serialization (RFC-039 §"Export Patch").
//!
//! Output is byte-for-byte reproducible for a given `PatchDocument`:
//! file changes are emitted in their stored order, hunks in order, and
//! lines in order. The writer uses LF terminators in the patch stream
//! itself (the patch *file* is a text artifact); the `\ No newline at end
//! of file` marker preserves the absence of a trailing newline in the
//! patched content, matching `git diff` and POSIX `diff -u`.

use std::fmt::Write as _;

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
    let total_files =
        s.files_changed + s.files_added + s.files_deleted + s.binary_files;
    let _ = writeln!(
        out,
        "# forskscope patch: {total_files} files, {} additions(+), {} deletions(-)",
        s.additions, s.deletions
    );
}

fn write_file_change(out: &mut String, change: &PatchFileChange) {
    let path = display_path(change.path());
    match change {
        PatchFileChange::Modify { hunks, .. } => {
            let _ = writeln!(out, "--- a/{path}");
            let _ = writeln!(out, "+++ b/{path}");
            for hunk in hunks {
                write_hunk(out, hunk);
            }
        }
        PatchFileChange::Add { lines, .. } => {
            let _ = writeln!(out, "--- /dev/null");
            let _ = writeln!(out, "+++ b/{path}");
            let _ = writeln!(out, "@@ -0,0 +1,{} @@", lines.len());
            write_lines(out, lines);
        }
        PatchFileChange::Delete { lines, .. } => {
            let _ = writeln!(out, "--- a/{path}");
            let _ = writeln!(out, "+++ /dev/null");
            let _ = writeln!(out, "@@ -1,{} +0,0 @@", lines.len());
            write_lines(out, lines);
        }
        PatchFileChange::BinaryNotice { .. } => {
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
        out.push('\n');
        if line.no_newline_at_eof {
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
fn display_path(path: &std::path::Path) -> String {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}
