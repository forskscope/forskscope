//! Spreadsheet input adapter (RFC-001 §6.2, RFC-013 of the roadmap).
//!
//! `.xlsx` comparison is derived through `sheets-diff` and is read-only:
//! the derived text participates in diffing but is never mergeable or
//! savable (`FileKind::ExcelXlsx::is_mergeable_text() == false`).

use std::path::Path;

use sheets_diff::core::diff::Diff;
use sheets_diff::core::unified_format::{SplitUnifiedDiffContent, unified_diff};

use crate::document::{FileFingerprint, FileId, LoadWarning, LoadedDocument, TextDocument};
use crate::encoding::{NewlineStyle, TextEncoding};
use crate::error::Result;
use crate::file_kind::FileKind;

/// Load metadata for an `.xlsx` side. The comparable text is produced
/// pairwise by [`derive_pair_text`], so the placeholder content stays empty
/// until both sides are known.
pub fn load_placeholder(path: &Path) -> Result<LoadedDocument> {
    let fingerprint = FileFingerprint::capture(path, None)?;
    Ok(LoadedDocument {
        file_id: Some(FileId::new(path)),
        fingerprint_at_load: Some(fingerprint),
        kind: FileKind::ExcelXlsx,
        bytes_len: fingerprint.len,
        text: None,
        warnings: vec![LoadWarning::ExcelRenderedAsDerivedText],
    })
}

/// Derive comparable per-side text for two `.xlsx` files.
///
/// The split unified diff from `sheets-diff` is flattened into
/// line-oriented text per side, preserving the v0.22.x user-visible
/// behavior while keeping the adapter behind the core boundary.
pub fn derive_pair_text(old_path: &Path, new_path: &Path) -> (TextDocument, TextDocument) {
    let diff = Diff::new(
        &old_path.display().to_string(),
        &new_path.display().to_string(),
    );
    let split = unified_diff(&diff).split();
    (
        derived_text(flatten(&split.old)),
        derived_text(flatten(&split.new)),
    )
}

fn flatten(contents: &[SplitUnifiedDiffContent]) -> String {
    let mut out = String::new();
    for content in contents {
        out.push_str(&content.title);
        out.push('\n');
        for line in &content.lines {
            if let Some(pos) = &line.pos {
                out.push_str(pos);
                out.push('\n');
            }
            if let Some(text) = &line.text {
                out.push_str(text);
                out.push('\n');
            }
        }
    }
    out
}

fn derived_text(content: String) -> TextDocument {
    TextDocument {
        content,
        encoding: TextEncoding {
            label: "(Excel)".into(),
        },
        newline_style: NewlineStyle::Lf,
        had_decode_errors: false,
    }
}
