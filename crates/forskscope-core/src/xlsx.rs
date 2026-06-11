//! Spreadsheet (.xlsx) structural diff adapter (RFC-058).
//!
//! This module owns the boundary between `sheets-diff` and ForskScope's
//! app model. No `sheets-diff` types appear in the public API; the upstream
//! crate can be upgraded without touching the UI or tests.
//!
//! ## Two entry points
//!
//! - [`diff_xlsx`] — returns a structured [`SpreadsheetDiff`] from two paths.
//! - [`derive_pair_text_from_diff`] — derives the per-side comparable text
//!   used by the current diff view, driven from the structured model.
//!
//! ## v2 migration notes (sheets-diff v2.2.1)
//!
//! - `compare_paths` returns `Result` — `catch_unwind` is no longer needed.
//! - One `CellChange` per address (value and formula are facets, not rows).
//! - `CellChange` now carries `old_formula`/`new_formula` (from v2.2.1's
//!   `CellChangeRow::old_formula`/`new_formula`).
//! - Non-UTF-8 paths are safe: `compare_paths` passes `Path` raw to
//!   `std::fs::read`; no internal `to_str().unwrap()`.
//! - Cancellation is per-sheet granularity only (sub-sheet cancellation
//!   is a planned enhancement in sheets-diff; see FR2 response).
//!
//! `.xlsx` comparison is always read-only: `FileKind::ExcelXlsx` is never
//! mergeable or saveable.

use std::path::Path;

use crate::cancel::CancellationToken;
use crate::document::{FileFingerprint, FileId, LoadWarning, LoadedDocument, TextDocument};
use crate::encoding::{NewlineStyle, TextEncoding};
use crate::error::{CoreError, Result};
use crate::file_kind::FileKind;

// ── App-owned spreadsheet diff model (RFC-058 §"App-owned model") ─────────────

/// A sheet-level structural change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetChange {
    Added(String),
    Removed(String),
    /// Sheet exists on both sides with cell differences.
    Modified(String),
    /// Sheet was renamed (heuristically matched); may also have cell changes.
    Renamed { old_name: String, new_name: String },
    /// Sheet moved to a different tab position; may also have cell changes.
    Moved(String),
}

/// Changed cells within one sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetCellChanges {
    pub sheet: String,
    pub cells: Vec<CellChange>,
}

/// One changed cell — one entry per address regardless of how many facets changed.
///
/// If both the value and formula changed at the same address, they are
/// combined into a single `CellChange` (Q1 answer: one row per address).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellChange {
    /// Spreadsheet address, e.g. `"B3"`.
    pub addr: String,
    /// 1-based row (sheets-diff v2 coordinate system).
    pub row: u32,
    /// 1-based column (sheets-diff v2 coordinate system).
    pub col: u32,
    /// Whether the cell value changed on this entry.
    pub value_changed: bool,
    /// Whether the formula changed on this entry.
    pub formula_changed: bool,
    /// Display string for the old value (`None` when the cell was empty).
    pub old_value: Option<String>,
    /// Display string for the new value (`None` when the cell was empty).
    pub new_value: Option<String>,
    /// Old formula text, if a formula change is present (Q2 addition).
    pub old_formula: Option<String>,
    /// New formula text, if a formula change is present (Q2 addition).
    pub new_formula: Option<String>,
}

/// Aggregate statistics for a spreadsheet diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpreadsheetDiffStats {
    pub sheets_added:    usize,
    pub sheets_removed:  usize,
    pub sheets_modified: usize,
    pub sheets_renamed:  usize,
    pub sheets_moved:    usize,
    pub cells_changed:   usize,
    pub values_changed:  usize,
    pub formulas_changed: usize,
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

// ── Adapter (RFC-058 §"v2 migration") ────────────────────────────────────────

/// Compute the structured diff of two `.xlsx` files.
///
/// Uses `sheets_diff::compare_paths` which returns `Result` — no panic risk.
/// An optional `CancellationToken` is wired at per-sheet granularity
/// (sub-sheet cancellation is a planned enhancement in sheets-diff; large
/// single-sheet workbooks are bounded via the default `max_cells_compared`
/// limit, or callers may pass explicit `Limits`).
pub fn diff_xlsx(
    old_path:  &Path,
    new_path:  &Path,
    cancel:    Option<&CancellationToken>,
) -> Result<SpreadsheetDiff> {
    let opts = build_options(cancel);

    let workbook_diff = sheets_diff::compare_paths_with_options(old_path, new_path, opts)
        .map_err(|e| CoreError::Unsupported {
            message: format!(
                "could not diff workbook '{}': {}",
                old_path.display(), e
            ),
        })?;

    Ok(convert(workbook_diff))
}

fn build_options(cancel: Option<&CancellationToken>) -> sheets_diff::DiffOptions {
    let mut builder = sheets_diff::DiffOptions::builder();
    if let Some(token) = cancel {
        let tok = token.clone();
        builder = builder.cancellation(move || tok.is_cancelled());
    }
    // Ignore invalid-options error: the default option set is always valid.
    builder.build().unwrap_or_default()
}

fn convert(wb: sheets_diff::WorkbookDiff) -> SpreadsheetDiff {
    use sheets_diff::model::SheetChange as UpSheetChange;
    use sheets_diff::output::view::{DiffView, ViewFilter};

    // Sheet-level structural changes.
    let mut sheets = Vec::new();
    for sd in &wb.sheets {
        let name = |sr: &Option<sheets_diff::model::SheetRef>| {
            sr.as_ref().map(|s| s.name.clone()).unwrap_or_default()
        };
        let entry = match &sd.change {
            UpSheetChange::Added             => SheetChange::Added(name(&sd.new_sheet)),
            UpSheetChange::Removed           => SheetChange::Removed(name(&sd.old_sheet)),
            UpSheetChange::Modified          => SheetChange::Modified(name(&sd.new_sheet)),
            UpSheetChange::Moved             => SheetChange::Moved(name(&sd.new_sheet)),
            UpSheetChange::Renamed { .. }
            | UpSheetChange::RenamedAndMoved { .. } => SheetChange::Renamed {
                old_name: name(&sd.old_sheet),
                new_name: name(&sd.new_sheet),
            },
            UpSheetChange::Unchanged         => continue,
            #[allow(unreachable_patterns)]
            _                                => continue, // forward compat: new variants in future sheets-diff
        };
        sheets.push(entry);
    }

    // Cell-level changes via the view adapter (Q3: use to_owned_row to drop wb).
    let view   = DiffView::new(&wb);
    let filter = ViewFilter::default();
    let rows   = view.rows(&filter);

    // Group by sheet name (rows are already in sheet order).
    let mut cells: Vec<SheetCellChanges> = Vec::new();
    for row in &rows {
        let cell = CellChange {
            addr:            row.address.a1.clone(),
            row:             row.address.row,
            col:             row.address.col,
            value_changed:   row.change_kind == sheets_diff::model::CellChangeKind::Modified
                             || !row.old_display.is_empty() || !row.new_display.is_empty(),
            formula_changed: row.formula_changed,
            old_value:       if row.old_display.is_empty() { None }
                             else { Some(row.old_display.clone()) },
            new_value:       if row.new_display.is_empty() { None }
                             else { Some(row.new_display.clone()) },
            old_formula:     row.old_formula.map(|s| s.to_owned()),
            new_formula:     row.new_formula.map(|s| s.to_owned()),
        };
        match cells.last_mut() {
            Some(last) if last.sheet == row.sheet_name => last.cells.push(cell),
            _ => cells.push(SheetCellChanges {
                sheet: row.sheet_name.to_owned(),
                cells: vec![cell],
            }),
        }
    }
    drop(rows); // release borrows before drop(wb)

    // Stats — driven directly from wb.summary (no manual counting).
    let s = &wb.summary;
    let stats = SpreadsheetDiffStats {
        sheets_added:     s.sheets_added,
        sheets_removed:   s.sheets_removed,
        sheets_modified:  s.sheets_changed,
        sheets_renamed:   s.sheets_renamed,
        sheets_moved:     s.sheets_moved,
        cells_changed:    cells.iter().map(|sc| sc.cells.len()).sum(),
        values_changed:   s.values_changed,
        formulas_changed: s.formulas_changed,
    };

    drop(wb); // all cell_diffs freed here; only owned SpreadsheetDiff survives

    SpreadsheetDiff { sheets, cells, stats }
}

// ── Per-side derived text (RFC-058 §"Presentation") ─────────────────────────

/// Derive the per-side comparable text for the diff view from the structured model.
pub fn derive_pair_text_from_diff(diff: &SpreadsheetDiff) -> (TextDocument, TextDocument) {
    let old_text = build_side_text(diff, Side::Old);
    let new_text = build_side_text(diff, Side::New);
    (excel_doc(old_text), excel_doc(new_text))
}

/// Entry point for callers that don't yet hold a `SpreadsheetDiff`.
pub fn derive_pair_text(old_path: &Path, new_path: &Path) -> (TextDocument, TextDocument) {
    match diff_xlsx(old_path, new_path, None) {
        Ok(diff) => derive_pair_text_from_diff(&diff),
        Err(_)   => (excel_doc(String::new()), excel_doc(String::new())),
    }
}

#[derive(Clone, Copy)]
enum Side { Old, New }

fn build_side_text(diff: &SpreadsheetDiff, side: Side) -> String {
    let mut out = String::new();

    for sc in &diff.sheets {
        match (sc, side) {
            (SheetChange::Added(name),   Side::New)   => out.push_str(&format!("+ Sheet: {name}\n")),
            (SheetChange::Added(name),   Side::Old)   => out.push_str(&format!("  Sheet: {name}\n")),
            (SheetChange::Removed(name), Side::Old)   => out.push_str(&format!("- Sheet: {name}\n")),
            (SheetChange::Removed(name), Side::New)   => out.push_str(&format!("  Sheet: {name}\n")),
            (SheetChange::Renamed { old_name, new_name }, _) => {
                let label = match side {
                    Side::Old => old_name.as_str(),
                    Side::New => new_name.as_str(),
                };
                out.push_str(&format!("~ Sheet: {label}\n"));
            }
            (SheetChange::Moved(name),    _)          => out.push_str(&format!("  Sheet: {name} (moved)\n")),
            (SheetChange::Modified(name), _)          => out.push_str(&format!("  Sheet: {name}\n")),
            #[allow(unreachable_patterns)]
            _                                         => {} // forward compat: new SheetChange variants
        }
    }

    for scd in &diff.cells {
        out.push_str(&format!("Sheet: {}\n", scd.sheet));
        for cell in &scd.cells {
            // Value line
            if cell.value_changed {
                let v = match side {
                    Side::Old => cell.old_value.as_deref().unwrap_or("(empty)"),
                    Side::New => cell.new_value.as_deref().unwrap_or("(empty)"),
                };
                out.push_str(&format!("  {} [value]: {}\n", cell.addr, v));
            }
            // Formula line
            if cell.formula_changed {
                let f = match side {
                    Side::Old => cell.old_formula.as_deref().unwrap_or("(none)"),
                    Side::New => cell.new_formula.as_deref().unwrap_or("(none)"),
                };
                out.push_str(&format!("  {} [formula]: {}\n", cell.addr, f));
            }
        }
    }

    out
}

fn excel_doc(content: String) -> TextDocument {
    TextDocument {
        content,
        encoding:          TextEncoding { label: "(Excel)".into() },
        newline_style:     NewlineStyle::Lf,
        had_decode_errors: false,
    }
}

// ── load_placeholder (unchanged) ─────────────────────────────────────────────

/// Load metadata for an `.xlsx` side. The comparable text is produced
/// pairwise by [`derive_pair_text`]; this placeholder holds no content.
pub fn load_placeholder(path: &Path) -> Result<LoadedDocument> {
    let fingerprint = FileFingerprint::capture(path, None)?;
    Ok(LoadedDocument {
        file_id:             Some(FileId::new(path)),
        fingerprint_at_load: Some(fingerprint),
        kind:                FileKind::ExcelXlsx,
        bytes_len:           fingerprint.len,
        text:                None,
        warnings:            vec![LoadWarning::ExcelRenderedAsDerivedText],
    })
}
