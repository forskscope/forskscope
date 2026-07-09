//! Spreadsheet (.xlsx) comparison boundary (RFC-058).
//!
//! XLSX structural comparison is currently disabled. The previous adapter
//! depended on `sheets-diff -> calamine -> quick-xml`; `quick-xml 0.39`
//! has active denial-of-service advisories for XML input. Because XLSX files
//! are user-supplied local archives containing XML, ForskScope fails closed
//! instead of parsing workbook content through that dependency chain.
//!
//! ## Two entry points
//!
//! - [`diff_xlsx`] — returns `Unsupported` without parsing workbook XML.
//! - [`derive_pair_text`] — returns empty read-only documents for callers that
//!   still expect a pairwise text projection.
//!
//! `.xlsx` comparison is always read-only: `FileKind::ExcelXlsx` is never
//! mergeable or saveable.

use std::path::Path;

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
    Renamed {
        old_name: String,
        new_name: String,
    },
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
    /// 1-based row coordinate.
    pub row: u32,
    /// 1-based column coordinate.
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
    pub sheets_added: usize,
    pub sheets_removed: usize,
    pub sheets_modified: usize,
    pub sheets_renamed: usize,
    pub sheets_moved: usize,
    pub cells_changed: usize,
    pub values_changed: usize,
    pub formulas_changed: usize,
}

/// The complete, app-owned diff of two `.xlsx` workbooks.
/// Kept as the app-owned model for a future fixed parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadsheetDiff {
    pub sheets: Vec<SheetChange>,
    pub cells: Vec<SheetCellChanges>,
    pub stats: SpreadsheetDiffStats,
}

impl SpreadsheetDiff {
    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty() && self.cells.iter().all(|s| s.cells.is_empty())
    }
}

// ── Adapter (RFC-058 §"v2 migration") ────────────────────────────────────────

/// Compute the structured diff of two `.xlsx` files.
///
/// Currently fails closed without parsing workbook XML. This protects users
/// from the vulnerable `quick-xml` path formerly reached through the XLSX
/// adapter while keeping the public boundary stable for a future fixed parser.
pub fn diff_xlsx(
    old_path: &Path,
    _new_path: &Path,
    _cancel: Option<&crate::cancel::CancellationToken>,
) -> Result<SpreadsheetDiff> {
    Err(CoreError::Unsupported {
        message: format!(
            ".xlsx comparison is temporarily disabled for security; '{}' was not parsed",
            old_path.display()
        ),
    })
}

// ── Per-side text projection (RFC-058 §"Presentation") ──────────────────────

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
        Err(_) => (excel_doc(String::new()), excel_doc(String::new())),
    }
}

#[derive(Clone, Copy)]
enum Side {
    Old,
    New,
}

fn build_side_text(diff: &SpreadsheetDiff, side: Side) -> String {
    let mut out = String::new();

    for sc in &diff.sheets {
        match (sc, side) {
            (SheetChange::Added(name), Side::New) => out.push_str(&format!("+ Sheet: {name}\n")),
            (SheetChange::Added(name), Side::Old) => out.push_str(&format!("  Sheet: {name}\n")),
            (SheetChange::Removed(name), Side::Old) => out.push_str(&format!("- Sheet: {name}\n")),
            (SheetChange::Removed(name), Side::New) => out.push_str(&format!("  Sheet: {name}\n")),
            (SheetChange::Renamed { old_name, new_name }, _) => {
                let label = match side {
                    Side::Old => old_name.as_str(),
                    Side::New => new_name.as_str(),
                };
                out.push_str(&format!("~ Sheet: {label}\n"));
            }
            (SheetChange::Moved(name), _) => out.push_str(&format!("  Sheet: {name} (moved)\n")),
            (SheetChange::Modified(name), _) => out.push_str(&format!("  Sheet: {name}\n")),
            #[allow(unreachable_patterns)]
            _ => {} // forward compat: new SheetChange variants
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
        encoding: TextEncoding {
            label: "(Excel)".into(),
        },
        newline_style: NewlineStyle::Lf,
        had_decode_errors: false,
    }
}

// ── load_placeholder ─────────────────────────────────────────────────────────

/// Load metadata for an `.xlsx` side without parsing workbook XML.
pub fn load_placeholder(path: &Path) -> Result<LoadedDocument> {
    let fingerprint = FileFingerprint::capture(path, None)?;
    Ok(LoadedDocument {
        file_id: Some(FileId::new(path)),
        fingerprint_at_load: Some(fingerprint),
        kind: FileKind::ExcelXlsx,
        bytes_len: fingerprint.len,
        text: None,
        warnings: vec![LoadWarning::ExcelComparisonDisabled],
    })
}
