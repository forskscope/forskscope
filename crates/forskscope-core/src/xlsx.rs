//! Spreadsheet (.xlsx) structural diff adapter (RFC-058).
//!
//! This module owns the boundary between `sheets-diff` and ForskScope's
//! app model. No `sheets-diff` types appear in the public API so the
//! upstream crate can be upgraded or replaced without touching the UI or
//! tests.
//!
//! ## Two entry points
//!
//! - [`diff_xlsx`] — returns a structured [`SpreadsheetDiff`] from two paths.
//!   All `sheets-diff` panic risk is isolated here behind `catch_unwind`.
//! - [`derive_pair_text`] — derives the per-side comparable text used by the
//!   current diff view, now driven from the structured model rather than the
//!   upstream unified-text renderer. User-visible output is unchanged.
//!
//! `.xlsx` comparison is always read-only: `FileKind::ExcelXlsx` is never
//! mergeable or savable.

use std::path::Path;

use crate::document::{FileFingerprint, FileId, LoadWarning, LoadedDocument, TextDocument};
use crate::encoding::{NewlineStyle, TextEncoding};
use crate::error::{CoreError, Result};
use crate::file_kind::FileKind;

// ── App-owned spreadsheet diff model (RFC-058 §"App-owned model") ─────────────

/// A sheet-level change: a sheet added, removed, or (future) renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetChange {
    /// Sheet present only in the new workbook.
    Added(String),
    /// Sheet present only in the old workbook.
    Removed(String),
}

/// Changed cells within one sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetCellChanges {
    pub sheet: String,
    pub cells: Vec<CellChange>,
}

/// One changed cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellChange {
    /// Spreadsheet address, e.g. `"B3"`.
    pub addr:  String,
    pub row:   u32,
    pub col:   u32,
    pub kind:  CellChangeKind,
    /// `None` when the cell was empty on that side.
    pub old:   Option<String>,
    pub new:   Option<String>,
}

/// What kind of cell content changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellChangeKind { Value, Formula }

/// Aggregate statistics for a spreadsheet diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpreadsheetDiffStats {
    pub sheets_added:   usize,
    pub sheets_removed: usize,
    pub cells_changed:  usize,
}

/// The complete, app-owned diff of two `.xlsx` workbooks.
/// Holds no `sheets-diff` types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadsheetDiff {
    pub sheets: Vec<SheetChange>,
    pub cells:  Vec<SheetCellChanges>,
    pub stats:  SpreadsheetDiffStats,
}

impl SpreadsheetDiff {
    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
            && self.cells.iter().all(|s| s.cells.is_empty())
    }
}

// ── Panic-guarded adapter (RFC-058 §"Panic isolation") ───────────────────────

/// Compute the structured diff of two `.xlsx` files.
///
/// `sheets-diff::Diff::new` panics when a workbook cannot be opened (it uses
/// `.expect()`). This function wraps the call in `catch_unwind` and maps any
/// caught panic to `CoreError::Unsupported`, satisfying the core's no-panic
/// contract (RFC-001 §6.5).
pub fn diff_xlsx(old_path: &Path, new_path: &Path) -> Result<SpreadsheetDiff> {
    let old_str = old_path.display().to_string();
    let new_str = new_path.display().to_string();

    let upstream = std::panic::catch_unwind(|| {
        sheets_diff::core::diff::Diff::new(&old_str, &new_str)
    })
    .map_err(|_| CoreError::Unsupported {
        message: format!(
            "could not open workbook '{}' — may be corrupt, locked, or not a valid .xlsx",
            old_path.display()
        ),
    })?;

    Ok(convert(upstream))
}

fn convert(d: sheets_diff::core::diff::Diff) -> SpreadsheetDiff {
    let mut sheets = Vec::new();
    for sd in &d.sheet_diff {
        match (&sd.old, &sd.new) {
            (None, Some(name)) => sheets.push(SheetChange::Added(name.clone())),
            (Some(name), None) => sheets.push(SheetChange::Removed(name.clone())),
            _ => {} // both Some (rename, not yet modelled) or both None (impossible)
        }
    }

    let cells: Vec<SheetCellChanges> = d
        .cell_diffs
        .iter()
        .map(|scd| SheetCellChanges {
            sheet: scd.sheet.clone(),
            cells: scd
                .cells
                .iter()
                .map(|c| CellChange {
                    addr: c.addr.clone(),
                    row:  c.row as u32,
                    col:  c.col as u32,
                    kind: match c.kind {
                        sheets_diff::core::diff::CellDiffKind::Value   => CellChangeKind::Value,
                        sheets_diff::core::diff::CellDiffKind::Formula => CellChangeKind::Formula,
                    },
                    old: c.old.clone(),
                    new: c.new.clone(),
                })
                .collect(),
        })
        .collect();

    let stats = SpreadsheetDiffStats {
        sheets_added:   sheets.iter().filter(|s| matches!(s, SheetChange::Added(_))).count(),
        sheets_removed: sheets.iter().filter(|s| matches!(s, SheetChange::Removed(_))).count(),
        cells_changed:  cells.iter().map(|s| s.cells.len()).sum(),
    };

    SpreadsheetDiff { sheets, cells, stats }
}

// ── Per-side derived text (RFC-058 §"Presentation now") ─────────────────────

/// Derive the per-side comparable text for the current diff view.
///
/// Now driven from the structured model (so the data is available to future
/// UI layers) rather than by flattening the upstream unified-text renderer.
/// User-visible output format is equivalent to the previous implementation.
pub fn derive_pair_text_from_diff(diff: &SpreadsheetDiff) -> (TextDocument, TextDocument) {
    let old_text = build_side_text(diff, Side::Old);
    let new_text = build_side_text(diff, Side::New);
    (excel_doc(old_text), excel_doc(new_text))
}

/// Original entry point — calls `diff_xlsx` and derives text.
/// Preserved for the document loader which does not yet use the structured model.
pub fn derive_pair_text(old_path: &Path, new_path: &Path) -> (TextDocument, TextDocument) {
    match diff_xlsx(old_path, new_path) {
        Ok(diff) => derive_pair_text_from_diff(&diff),
        Err(_) => (excel_doc(String::new()), excel_doc(String::new())),
    }
}

#[derive(Clone, Copy)]
enum Side { Old, New }

fn build_side_text(diff: &SpreadsheetDiff, side: Side) -> String {
    let mut out = String::new();

    // Sheet changes
    for sc in &diff.sheets {
        match (sc, side) {
            (SheetChange::Added(name), Side::New)   => out.push_str(&format!("+ Sheet: {name}\n")),
            (SheetChange::Added(name), Side::Old)   => out.push_str(&format!("  Sheet: {name}\n")),
            (SheetChange::Removed(name), Side::Old) => out.push_str(&format!("- Sheet: {name}\n")),
            (SheetChange::Removed(name), Side::New) => out.push_str(&format!("  Sheet: {name}\n")),
        }
    }

    // Cell changes
    for scd in &diff.cells {
        out.push_str(&format!("Sheet: {}\n", scd.sheet));
        for cell in &scd.cells {
            let value = match side {
                Side::Old => cell.old.as_deref().unwrap_or("(empty)"),
                Side::New => cell.new.as_deref().unwrap_or("(empty)"),
            };
            let kind = match cell.kind {
                CellChangeKind::Value   => "value",
                CellChangeKind::Formula => "formula",
            };
            out.push_str(&format!("  {} [{}]: {}\n", cell.addr, kind, value));
        }
    }

    out
}

fn excel_doc(content: String) -> TextDocument {
    TextDocument {
        content,
        encoding: TextEncoding { label: "(Excel)".into() },
        newline_style: NewlineStyle::Lf,
        had_decode_errors: false,
    }
}

// ── load_placeholder (unchanged) ─────────────────────────────────────────────

/// Load metadata for an `.xlsx` side.  The comparable text is produced
/// pairwise by [`derive_pair_text`]; the placeholder holds no content.
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
