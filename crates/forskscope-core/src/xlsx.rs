//! Spreadsheet (.xlsx) structural diff adapter (RFC-058, RFC-085).
//!
//! This module owns the boundary between `sheets-diff` and ForskScope's app
//! model. No `sheets-diff` types appear in the public API; the upstream
//! crate can be upgraded without touching the UI or tests.
//!
//! RFC-058 suspended this adapter for one release cycle: `sheets-diff ->
//! calamine -> quick-xml 0.39` carried active XML denial-of-service
//! advisories, and `.xlsx` files are user-supplied archives of untrusted
//! XML. RFC-085 restored it at `sheets-diff` 2.5.0, whose dependency chain
//! (`calamine 0.36.1 -> quick-xml 0.41.0, zip 8.6.0`) carries no known
//! advisories, and which is also the first release polling cancellation
//! *inside* a sheet (every 50,000 cells, in both the read and compare
//! phases) rather than only between sheets — see [`diff_xlsx`]'s doc. F130
//! moved it to 3.0.0: the same chain, and a read that streams (2.5.1), so a
//! sheet costs memory in proportion to its populated cells.
//!
//! ## The size bound (F117, RFC-058 condition 4)
//!
//! [`CellBounds::PRODUCT`] caps a comparison at 2,000,000 compared
//! coordinates and 4,000,000 cells read, on top of `Limits::hardened()`'s
//! other dimensions. A comparison that reaches a bound **fails** with
//! [`CoreError::Unsupported`]; it never returns a truncated diff. The value
//! and its measured basis are on [`CellBounds`].
//!
//! ## Two entry points
//!
//! - [`diff_xlsx`] — returns a structured [`SpreadsheetDiff`] from two paths.
//! - [`derive_pair_text_from_diff`] — derives the per-side comparable text
//!   used by the current diff view, driven from the structured model.
//!
//! ## sheets-diff 3.0.0 API notes
//!
//! - `compare_paths_with_options` returns `Result` — no panic risk, no
//!   `catch_unwind` needed.
//! - One `CellDiff` per address: `.value: Option<ValueChange>` and
//!   `.formula: Option<FormulaChange>` are independent facets of the same
//!   entry, not separate rows — `is_some()` *is* the changed flag for each.
//! - `SheetChange` is `#[non_exhaustive]` and includes `Unchanged` (dropped
//!   — nothing to show, see [`convert`]) and `RenamedAndMoved` (collapsed
//!   into our `Renamed`, see [`convert`]) alongside the five variants our
//!   own [`SheetChange`] already modelled.
//! - Non-UTF-8 paths are safe: `compare_paths_with_options` passes `Path`
//!   raw to `std::fs::read`; no internal `to_str().unwrap()`.
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

/// Where one sheet sits among the tabs of each workbook (0-based), for the
/// sheet it is reported under. `None` on the side it does not exist on.
///
/// This is what the model lacked when a reorder-only pair rendered as two
/// identical documents (F129): [`SheetChange::Moved`] said *that* a sheet moved
/// and neither side said *where*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetTab {
    pub old: Option<usize>,
    pub new: Option<usize>,
}

/// What a warning from the parser says is uncertain about the comparison,
/// in the order of urgency (F131). A `Warning` severity means "the answer you
/// are looking at may be wrong or incomplete in a way the diff itself does not
/// show"; the kind says which way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpreadsheetWarningKind {
    /// Rows share an alignment key, so they may have been **paired wrongly**.
    DuplicateAlignmentKey,
    /// A sheet was too large to align and fell back to position-by-position
    /// comparison, so an inserted row may show as a long cascade.
    AlignmentFellBack,
    /// Several sheets could have been renamed, so none was matched: the user
    /// sees an add and a remove where they may expect a rename.
    AmbiguousSheetMatch,
    /// A sheet that is not a worksheet (a chart sheet, say) was **not
    /// compared at all**: a coverage gap.
    SheetNotCompared,
    /// A warning this crate does not know by name. Surfaced anyway: the match
    /// is on severity, so a warning `sheets-diff` adds later is inherited, not
    /// dropped. `detail` carries the parser's own (English) message.
    Other,
}

/// One warning, all its sheets gathered: a condition that hit fifty sheets is
/// one message naming them, not fifty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadsheetWarning {
    pub kind: SpreadsheetWarningKind,
    /// The sheets it concerns, in the order the parser reported them. Empty
    /// when the parser named none.
    pub sheets: Vec<String>,
    /// The parser's own message; set only for [`SpreadsheetWarningKind::Other`].
    pub detail: Option<String>,
}

/// The complete, app-owned diff of two `.xlsx` workbooks.
/// Kept as the app-owned model for a future fixed parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadsheetDiff {
    /// Everything at `Severity::Warning` or above, grouped by kind and ordered
    /// by urgency (F131). Empty means the parser had no doubt to report.
    pub warnings: Vec<SpreadsheetWarning>,
    /// How many `Info` diagnostics there were, **counted, never listed**: one
    /// of them (`FormulaUnavailable`) is emitted per numeric cell.
    pub info_notes: usize,
    pub sheets: Vec<SheetChange>,
    /// `tabs[i]` is where `sheets[i]` sits on each side. Same length as
    /// `sheets`, built beside it in [`convert`]; read them together through
    /// [`SpreadsheetDiff::sheet_entries`].
    pub tabs: Vec<SheetTab>,
    /// How many tabs each workbook has: `(old, new)`.
    pub tab_counts: (usize, usize),
    pub cells: Vec<SheetCellChanges>,
    pub stats: SpreadsheetDiffStats,
}

impl SpreadsheetDiff {
    /// Each reported sheet change with its tab position on both sides.
    pub fn sheet_entries(&self) -> impl Iterator<Item = (&SheetChange, SheetTab)> {
        self.sheets.iter().zip(self.tabs.iter().copied())
    }

    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty() && self.cells.iter().all(|s| s.cells.is_empty())
    }
}

// ── Adapter (RFC-058 §"v2 migration") ────────────────────────────────────────

/// The cell bounds one `.xlsx` comparison runs under (F117).
///
/// **Basis, measured in a release build**, two identical single-sheet
/// workbooks of numeric cells (one differing cell), unbounded, on the
/// 2026-09-24 audit machine. The `sheets-diff` 2.5.0 column is the original
/// basis; the 3.0.0 column is F130's re-run of the same method, and agrees:
///
/// | compared coordinates | 2.5.0 time, peak RSS | 3.0.0 time, peak RSS |
/// |---|---|---|
/// | 1,000,000 | 1.6 s, 0.97 GB | 1.3 s, 0.97 GB |
/// | 2,000,000 | 2.7 s, 1.9 GB | 2.7 s, 1.9 GB |
/// | 5,000,000 | 7.1 s, 4.8 GB | 6.7 s, 4.8 GB |
///
/// Cost is linear at roughly 1 KB of peak memory per compared coordinate,
/// so the bound is a memory decision. **2,000,000 compared coordinates**
/// admits a workbook of about 200 columns by 10,000 rows on each side —
/// larger than an ordinary office workbook — at about 2 GB and under 3
/// seconds. It refuses anything larger, which includes a full-height
/// sheet of more than two columns. `Limits::hardened()`'s own 5,000,000
/// was rejected: it would admit ~4.8 GB from a file the user opened but did
/// not write.
///
/// `max_cells_read` counts the *populated* cells of both sides cumulatively
/// (`sheets-diff` 3.0.0; through 2.6.0 it counted the area of each sheet's
/// bounding box), so a symmetric pair at the compared bound reads exactly
/// twice as many cells: it is set to `2 * max_cells_compared`. It fires
/// mid-read, before the compare phase and before the cell it counts is kept.
///
/// **The parse, measured on 3.0.0 (F123, F130).** Memory follows the populated
/// cells, not the area between them: a 5 KB workbook with one cell at `A1` and
/// one at Excel's last position (1,048,576 × 16,384) compares in 4 MB and
/// 0.2 ms, where on 2.5.0 it asked for 512 GiB and aborted the process, and
/// 100M box cells cost 3.1 GB. What a bound can still admit is the populated
/// cells themselves: about **177 bytes each once read** (707 MB at the moment
/// `max_cells_read` refuses a 20M-cell sheet, which is 4,000,001 cells), and a
/// comparison that reaches `max_cells_compared` peaks near 1.9 GB (the table
/// above). The tables for the 2.5.0 defect are in RFC-058's amendment.
/// `max_input_bytes` (50 MiB, from `hardened()`) limits compressed size only.
///
/// **`AlignmentMode` (RFC-058 condition 4): `Positional`, kept.** It is
/// `sheets-diff`'s default, the cheapest mode, and the one measured above.
/// The row-key and row-signature modes add an `m × n` alignment table
/// (bounded separately by `max_alignment_product`) that this product does
/// not need: nothing in the UI selects an alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellBounds {
    pub max_cells_read: u64,
    pub max_cells_compared: u64,
}

impl CellBounds {
    pub const PRODUCT: CellBounds = CellBounds {
        max_cells_read: 2 * 2_000_000,
        max_cells_compared: 2_000_000,
    };
}

/// Compute the structured diff of two `.xlsx` files.
///
/// `cancel`, when given, is polled by `sheets-diff` every 50,000 cell records
/// streamed (blank ones count) during the read, and every 50,000 coordinates
/// during the compare, of *each* sheet — not only between sheets, which is all
/// releases before 2.5.0 offered. A sheet under 50,000 cells still runs to
/// completion uninterrupted; it also completes in well under the ~100ms
/// worst-case checkpoint interval, so there is nothing to interrupt. Measured
/// on 3.0.0 against a 20M-cell sheet: a cancel requested at 100 ms is observed
/// at 112 ms, and one at 500 ms at 547 ms (on 2.5.0, with the dense range in
/// the way, both returned at 4.7 s). Passing `None` runs the comparison
/// uncancellable, same as no token at all.
///
/// Runs under [`CellBounds::PRODUCT`]. A comparison that reaches a bound
/// returns `Err`, never a partial diff: see [`diff_xlsx_with_bounds`].
pub fn diff_xlsx(
    old_path: &Path,
    new_path: &Path,
    cancel: Option<&CancellationToken>,
) -> Result<SpreadsheetDiff> {
    diff_xlsx_with_bounds(old_path, new_path, cancel, CellBounds::PRODUCT)
}

/// [`diff_xlsx`] with explicit bounds, so a test can exercise the bound on a
/// small fixture instead of a multi-million-cell workbook.
///
/// A bound that is reached is an `Err(CoreError::Unsupported)` whose message
/// names the bound and says the comparison was stopped. It is never an
/// `Ok` with fewer differences: that would report "these sheets match" for
/// a file that was not finished being read.
pub fn diff_xlsx_with_bounds(
    old_path: &Path,
    new_path: &Path,
    cancel: Option<&CancellationToken>,
    bounds: CellBounds,
) -> Result<SpreadsheetDiff> {
    let opts = build_options(cancel, bounds)?;

    let workbook_diff =
        sheets_diff::compare_paths_with_options(old_path, new_path, opts).map_err(|e| match e {
            sheets_diff::SheetsDiffError::LimitExceeded { limit, observed } => {
                CoreError::Unsupported {
                    message: format!(
                        "{} is too large to compare: it exceeded the size bound ({limit}, \
                         reached {observed}). The comparison was stopped, not completed, so no \
                         result is shown",
                        display_name(old_path)
                    ),
                }
            }
            e => CoreError::Unsupported {
                message: format!("could not diff workbook '{}': {}", old_path.display(), e),
            },
        })?;

    Ok(convert(workbook_diff))
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| format!("'{}'", n.to_string_lossy()))
        .unwrap_or_else(|| format!("'{}'", path.display()))
}

fn build_options(
    cancel: Option<&CancellationToken>,
    bounds: CellBounds,
) -> Result<sheets_diff::DiffOptions> {
    let mut limits = sheets_diff::Limits::hardened();
    limits.max_cells_read = Some(bounds.max_cells_read);
    limits.max_cells_compared = Some(bounds.max_cells_compared);
    let mut builder = sheets_diff::DiffOptions::builder().limits(limits);
    if let Some(token) = cancel {
        let tok = token.clone();
        builder = builder.cancellation(move || tok.is_cancelled());
    }
    // `validate()` only rejects a `formula_compare`/`format_compare`
    // combination neither of which this builder touches, so this should not
    // fail. If it ever does, refuse the comparison: falling back to
    // `default()` would silently drop the bound.
    builder.build().map_err(|e| CoreError::InternalInvariant {
        message: format!("could not build spreadsheet comparison options: {e}"),
    })
}

/// Maps `sheets-diff`'s v2 model onto our own (RFC-085 Q1: upstream declined
/// to reshape their model to ours, so this boundary is where their naming
/// stops and ours begins).
fn convert(wb: sheets_diff::WorkbookDiff) -> SpreadsheetDiff {
    use sheets_diff::SheetChange as UpSheetChange;

    let mut sheets = Vec::new();
    let mut tabs = Vec::new();
    let mut cells: Vec<SheetCellChanges> = Vec::new();

    for sd in &wb.sheets {
        let name = |sr: &Option<sheets_diff::SheetRef>| {
            sr.as_ref().map(|s| s.name.clone()).unwrap_or_default()
        };

        let entry = match &sd.change {
            UpSheetChange::Added => SheetChange::Added(name(&sd.new_sheet)),
            UpSheetChange::Removed => SheetChange::Removed(name(&sd.old_sheet)),
            UpSheetChange::Modified => SheetChange::Modified(name(&sd.new_sheet)),
            UpSheetChange::Moved => SheetChange::Moved(name(&sd.new_sheet)),
            // RFC-085 §3: `RenamedAndMoved` carries both facts, but our
            // `SheetChange` has no variant/field for "renamed and moved" —
            // widening it is out of this handoff's scope (SpreadsheetDiff's
            // shape is frozen). Emitting a second, separate `Moved` entry
            // for the same sheet would change the sheet count for what is
            // structurally one sheet's worth of change, which is worse than
            // losing the move fact while keeping sheet identity accurate.
            // Collapsing to `Renamed` matches the pre-suspension code's own
            // choice for this exact case.
            UpSheetChange::Renamed { .. } | UpSheetChange::RenamedAndMoved { .. } => {
                SheetChange::Renamed {
                    old_name: name(&sd.old_sheet),
                    new_name: name(&sd.new_sheet),
                }
            }
            // RFC-085 §3: name-matched, same tab position, no cell
            // differences — nothing for either side of the diff to show.
            // `build_side_text` renders every `SheetChange` it's given, so
            // including `Unchanged` would add a noise line per untouched
            // sheet to both panes.
            UpSheetChange::Unchanged => continue,
            // `SheetChange` is `#[non_exhaustive]`: a future sheets-diff
            // release may add a variant this match doesn't know about yet.
            // Drop it rather than guess at meaning this crate doesn't have —
            // the same posture `Unchanged`/`RenamedAndMoved` held before
            // RFC-085 named them.
            _ => continue,
        };
        sheets.push(entry);
        tabs.push(SheetTab {
            old: sd.old_sheet.as_ref().map(|s| s.index),
            new: sd.new_sheet.as_ref().map(|s| s.index),
        });

        if sd.cell_diffs.is_empty() {
            continue;
        }
        let sheet_name = sd
            .new_sheet
            .as_ref()
            .or(sd.old_sheet.as_ref())
            .map(|s| s.name.clone())
            .unwrap_or_default();

        let mut sheet_cells = Vec::new();
        for cd in &sd.cell_diffs {
            // Q1: `is_some()` *is* the changed flag for each facet, and the
            // two move independently — a cell can have a value change with
            // no formula change, or vice versa, or both at once.
            let value_changed = cd.value.is_some();
            let formula_changed = cd.formula.is_some();
            if !value_changed && !formula_changed {
                // Never observed under this crate's options (format-only
                // changes are excluded — `FormatCompareMode` other than
                // `Ignore` is rejected by `validate()`, and this builder
                // never sets it), but `CellDiff` carries a `format` facet
                // too; skip rather than emit a "changed cell" with nothing
                // changed on either facet this model tracks.
                continue;
            }
            let (old_value, new_value) = match &cd.value {
                Some(vc) => (
                    (!vc.old.is_empty()).then(|| vc.old.display_string()),
                    (!vc.new.is_empty()).then(|| vc.new.display_string()),
                ),
                None => (None, None),
            };
            let (old_formula, new_formula) = match &cd.formula {
                Some(fc) => (
                    fc.old.as_ref().map(|t| t.raw.clone()),
                    fc.new.as_ref().map(|t| t.raw.clone()),
                ),
                None => (None, None),
            };
            sheet_cells.push(CellChange {
                addr: cd.address.a1.clone(),
                row: cd.address.row,
                col: cd.address.col,
                value_changed,
                formula_changed,
                old_value,
                new_value,
                old_formula,
                new_formula,
            });
        }
        if !sheet_cells.is_empty() {
            cells.push(SheetCellChanges {
                sheet: sheet_name,
                cells: sheet_cells,
            });
        }
    }

    // Stats — driven directly from wb.summary (no manual counting). Their
    // own `derive_summary` counts a `RenamedAndMoved` sheet in *both*
    // `sheets_renamed` and `sheets_moved` — a sheet-count mismatch against
    // our single collapsed `sheets` entry for it that already existed in
    // their own aggregate semantics, not one this adapter introduces.
    let (warnings, info_notes) = summarize_notes(collect_notes(&wb));
    let s = &wb.summary;
    let stats = SpreadsheetDiffStats {
        sheets_added: s.sheets_added,
        sheets_removed: s.sheets_removed,
        sheets_modified: s.sheets_changed,
        sheets_renamed: s.sheets_renamed,
        sheets_moved: s.sheets_moved,
        cells_changed: cells.iter().map(|sc| sc.cells.len()).sum(),
        values_changed: s.values_changed,
        formulas_changed: s.formulas_changed,
    };

    SpreadsheetDiff {
        warnings,
        info_notes,
        sheets,
        tabs,
        tab_counts: (wb.old.sheet_count, wb.new.sheet_count),
        cells,
        stats,
    }
}

// ── Diagnostics (F131) ───────────────────────────────────────────────────────
//
// `sheets-diff` reports doubt in two places — `WorkbookDiff::diagnostics` and
// each `SheetDiff::diagnostics` — and it is easy to read one. Two of its four
// warnings are sheet-level, so reading only the workbook vector surfaces
// neither; that is the defect its own CLI shipped for six releases. Both are
// read here, in one function, and the match is on **severity**, never on the
// variant.

/// One diagnostic reduced to what the summary needs. Built from upstream's
/// type by [`note_from`]; kept separate because `sheets_diff::Diagnostic` is
/// `#[non_exhaustive]` and cannot be constructed in a test, and the grouping
/// rules (an unknown warning is surfaced; `Info` is only counted) are worth
/// asserting directly.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Note {
    is_warning: bool,
    kind: SpreadsheetWarningKind,
    sheets: Vec<String>,
    message: String,
}

fn note_from(d: &sheets_diff::Diagnostic) -> Note {
    use sheets_diff::DiagnosticKind as K;

    // The severity, not the kind, decides whether this is heard about. `Ord`
    // means a level added above `Warning` is inherited too.
    let is_warning = d.severity >= sheets_diff::Severity::Warning;
    let own_sheet: Vec<String> = d.location.sheet_name.iter().cloned().collect();
    let (kind, sheets) = match &d.kind {
        K::DuplicateAlignmentKey { .. } => {
            (SpreadsheetWarningKind::DuplicateAlignmentKey, own_sheet)
        }
        K::AlignmentBoundExceeded { .. } => (SpreadsheetWarningKind::AlignmentFellBack, own_sheet),
        K::AmbiguousSheetMatch { candidates } => (
            SpreadsheetWarningKind::AmbiguousSheetMatch,
            candidates.iter().map(|c| c.name.clone()).collect(),
        ),
        // A warning-level `UnsupportedWorkbookFeature` is a sheet that was
        // skipped; the blanket "non-cell objects" note is `Info`, and is only
        // counted.
        K::UnsupportedWorkbookFeature { .. } => {
            (SpreadsheetWarningKind::SheetNotCompared, own_sheet)
        }
        // `DiagnosticKind` is `#[non_exhaustive]`; so is anything else a
        // warning-level metadata note says.
        _ => (SpreadsheetWarningKind::Other, own_sheet),
    };
    Note {
        is_warning,
        kind,
        sheets,
        message: d.message.clone(),
    }
}

/// Every diagnostic of the comparison, workbook-level **and** sheet-level.
fn collect_notes(wb: &sheets_diff::WorkbookDiff) -> Vec<Note> {
    let mut notes: Vec<Note> = wb.diagnostics.iter().map(note_from).collect();
    for sd in &wb.sheets {
        notes.extend(sd.diagnostics.iter().map(note_from));
    }
    notes
}

/// Group warnings by kind (one message naming every sheet), order them by
/// urgency, and count the rest.
fn summarize_notes(notes: Vec<Note>) -> (Vec<SpreadsheetWarning>, usize) {
    let mut info = 0;
    let mut warnings: Vec<SpreadsheetWarning> = Vec::new();
    for note in notes {
        if !note.is_warning {
            info += 1;
            continue;
        }
        let existing = warnings.iter_mut().find(|w| {
            w.kind == note.kind
                && (note.kind != SpreadsheetWarningKind::Other
                    || w.detail.as_ref() == Some(&note.message))
        });
        let warning = match existing {
            Some(w) => w,
            None => {
                warnings.push(SpreadsheetWarning {
                    kind: note.kind,
                    sheets: Vec::new(),
                    detail: (note.kind == SpreadsheetWarningKind::Other)
                        .then(|| note.message.clone()),
                });
                warnings.last_mut().unwrap()
            }
        };
        for sheet in note.sheets {
            if !warning.sheets.contains(&sheet) {
                warning.sheets.push(sheet);
            }
        }
    }
    warnings.sort_by_key(|w| w.kind); // stable: reported order within a kind
    (warnings, info)
}

// ── Per-side text projection (RFC-058 §"Presentation") ──────────────────────

/// Derive the per-side comparable text for the diff view from the structured model.
pub fn derive_pair_text_from_diff(diff: &SpreadsheetDiff) -> (TextDocument, TextDocument) {
    let old_text = build_side_text(diff, Side::Old);
    let new_text = build_side_text(diff, Side::New);
    (excel_doc(old_text), excel_doc(new_text))
}

/// Both sides' comparable text and what the parser doubted, from one
/// comparison (F131).
#[derive(Debug, Clone)]
pub struct SpreadsheetPair {
    pub left: TextDocument,
    pub right: TextDocument,
    pub warnings: Vec<SpreadsheetWarning>,
    pub info_notes: usize,
}

/// [`derive_pair_text`] plus the warnings the comparison raised. This is what
/// the comparison view uses: a result that "may be wrong" has to arrive with
/// the text it qualifies.
pub fn compare_pair(old_path: &Path, new_path: &Path) -> Result<SpreadsheetPair> {
    let diff = diff_xlsx(old_path, new_path, None)?;
    let (left, right) = derive_pair_text_from_diff(&diff);
    Ok(SpreadsheetPair {
        left,
        right,
        warnings: diff.warnings,
        info_notes: diff.info_notes,
    })
}

/// Entry point for callers that don't yet hold a `SpreadsheetDiff`.
///
/// An `Err` is returned to the caller, never turned into two empty
/// documents: two empty sides diff as identical, so swallowing the error
/// would show "these workbooks match" for a pair that was corrupt, or that
/// stopped at the size bound (F117).
pub fn derive_pair_text(old_path: &Path, new_path: &Path) -> Result<(TextDocument, TextDocument)> {
    Ok(derive_pair_text_from_diff(&diff_xlsx(
        old_path, new_path, None,
    )?))
}

#[derive(Clone, Copy)]
enum Side {
    Old,
    New,
}

fn build_side_text(diff: &SpreadsheetDiff, side: Side) -> String {
    let mut out = String::new();

    // The sheet list is each side's own: ordered by that side's tab position.
    // A sheet with no tab on this side (added, on the old side) is a placeholder
    // line that keeps the panes aligned; it sorts where it sits on the other
    // side, so it lands next to its neighbours rather than at an end.
    let mut lines: Vec<(usize, String)> = Vec::new();
    for (sc, tab) in diff.sheet_entries() {
        let (own, other, count) = match side {
            Side::Old => (tab.old, tab.new, diff.tab_counts.0),
            Side::New => (tab.new, tab.old, diff.tab_counts.1),
        };
        let line = match (sc, side) {
            (SheetChange::Added(name), Side::New) => format!("+ Sheet: {name}\n"),
            (SheetChange::Added(name), Side::Old) => format!("  Sheet: {name}\n"),
            (SheetChange::Removed(name), Side::Old) => format!("- Sheet: {name}\n"),
            (SheetChange::Removed(name), Side::New) => format!("  Sheet: {name}\n"),
            (SheetChange::Renamed { old_name, new_name }, _) => {
                let label = match side {
                    Side::Old => old_name.as_str(),
                    Side::New => new_name.as_str(),
                };
                format!("~ Sheet: {label}\n")
            }
            // Where the sheet is on *this* side, so the two panes differ by
            // exactly the fact that changed. Without it both sides said the same
            // thing and a reorder-only pair compared as identical (F129).
            (SheetChange::Moved(name), _) => match own {
                Some(tab) => format!("  Sheet: {name} (moved: tab {} of {count})\n", tab + 1),
                None => format!("  Sheet: {name} (moved)\n"),
            },
            (SheetChange::Modified(name), _) => format!("  Sheet: {name}\n"),
            #[allow(unreachable_patterns)]
            _ => continue, // forward compat: new SheetChange variants
        };
        lines.push((own.or(other).unwrap_or(usize::MAX), line));
    }
    lines.sort_by_key(|(key, _)| *key); // stable: ties keep the diff's order
    for (_, line) in lines {
        out.push_str(&line);
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
        bom: crate::encoding::BomPresence::Absent,
        raw_bytes: Vec::new(),
    }
}

// ── load_placeholder (unchanged) ─────────────────────────────────────────────

/// Load metadata for an `.xlsx` side. The comparable text is produced
/// pairwise by [`derive_pair_text`]; this placeholder holds no content.
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

// ── Tests (RFC-085) ──────────────────────────────────────────────────────────
//
// Fixtures under `src/tests/fixtures/xlsx/<case>/{old,new}.xlsx` are real
// workbooks (built with `rust_xlsxwriter`, not committed as a project
// dependency — see handoff 022's review request for why) read through the
// real `sheets-diff` parser (2.5.0 when written, 3.0.0 since F130; the model
// and both sides' rendered text are identical on every fixture across the two),
// not hand-constructed `WorkbookDiff`
// values: `sheets_diff::SheetChange`/`SheetDiff`/`WorkbookDiff` are
// `#[non_exhaustive]`, so this crate cannot build one via struct-literal
// syntax at all, and doing so would test a copy of `convert()`'s match arms
// rather than what a real parse actually produces.

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(case: &str, name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/fixtures/xlsx")
            .join(case)
            .join(name)
    }

    fn diff(case: &str) -> SpreadsheetDiff {
        diff_xlsx(&fixture(case, "old.xlsx"), &fixture(case, "new.xlsx"), None).unwrap()
    }

    #[test]
    fn a_cell_value_change_is_reported_as_modified() {
        let diff = diff("basic");
        assert_eq!(diff.sheets, vec![SheetChange::Modified("Sheet1".into())]);
        assert_eq!(diff.cells.len(), 1);
        let sheet = &diff.cells[0];
        assert_eq!(sheet.sheet, "Sheet1");
        assert_eq!(
            sheet.cells,
            vec![CellChange {
                addr: "A1".into(),
                row: 1,
                col: 1,
                value_changed: true,
                formula_changed: false,
                old_value: Some("hello".into()),
                new_value: Some("world".into()),
                old_formula: None,
                new_formula: None,
            }]
        );
    }

    /// RFC-085 §3: `Unchanged` — name-matched, same position, no cell
    /// differences — is dropped entirely, not rendered as a no-op sheet
    /// line. Falsify by making the `Unchanged` arm push an entry instead of
    /// `continue`: the untouched "Same" sheet must then appear and this
    /// must fail.
    #[test]
    fn an_unchanged_sheet_is_dropped_from_the_sheet_list() {
        let diff = diff("unchanged_sheet");
        assert_eq!(
            diff.sheets,
            vec![SheetChange::Modified("Changed".into())],
            "the untouched \"Same\" sheet must not appear at all"
        );
    }

    /// A sheet renamed at the *same* tab position — `sheets-diff`'s
    /// `SheetChange::Renamed { .. }` maps directly onto ours.
    #[test]
    fn a_renamed_sheet_at_the_same_position_is_reported_as_renamed() {
        let diff = diff("renamed");
        assert_eq!(
            diff.sheets,
            vec![SheetChange::Renamed {
                old_name: "Original".into(),
                new_name: "Renamed".into(),
            }]
        );
    }

    /// RFC-085 §3: `RenamedAndMoved` collapses into our `Renamed` — the
    /// move fact is dropped, sheet identity is not, and no second `Moved`
    /// entry is fabricated for the same sheet (which would misrepresent
    /// one sheet's change as two). The fixture's *other* sheet ("Anchor")
    /// keeps its name but changes tab position independently, so this also
    /// confirms a real `Moved` classification survives unchanged alongside
    /// the collapsed one.
    #[test]
    fn a_sheet_renamed_and_moved_collapses_into_renamed_not_two_entries() {
        let diff = diff("renamed_and_moved");
        assert_eq!(
            diff.sheets,
            vec![
                SheetChange::Moved("Anchor".into()),
                SheetChange::Renamed {
                    old_name: "ToRename".into(),
                    new_name: "RenamedSheet".into(),
                },
            ],
            "exactly one entry for the renamed-and-moved sheet, not a \
             second Moved entry alongside it"
        );
    }

    #[test]
    fn a_formula_change_is_isolated_from_value_change() {
        let diff = diff("formula");
        assert_eq!(diff.cells.len(), 1);
        let cell = &diff.cells[0].cells[0];
        assert!(cell.formula_changed);
        assert!(
            !cell.value_changed,
            "neither side set a cached result, so both sides' value must \
             read as the same empty cell — only the formula differs"
        );
        assert_eq!(cell.old_formula.as_deref(), Some("1+1"));
        assert_eq!(cell.new_formula.as_deref(), Some("2+2"));
    }

    /// Handoff 022 §4: cancellation must actually interrupt a comparison
    /// large enough to cross sheets-diff's 50,000-cell checkpoint —
    /// not merely accept a token that does nothing. The "large" fixture is
    /// 51,000 cells per side in one sheet; cancelling before comparing even starts
    /// means the very first checkpoint must observe it. Falsified for real
    /// (see the review request): reverting `build_options` to ignore
    /// `cancel` made this fail, taking ~1.3s and returning `Ok` instead of
    /// an immediate `Err`.
    #[test]
    fn cancellation_interrupts_a_comparison_that_crosses_the_checkpoint() {
        let old = fixture("large", "old.xlsx");
        let new = fixture("large", "new.xlsx");

        let token = CancellationToken::new();
        token.cancel();
        let result = diff_xlsx(&old, &new, Some(&token));

        let err = result.expect_err("a cancelled comparison must not succeed");
        let message = err.to_string();
        assert!(
            message.contains("cancelled"),
            "expected a cancellation error, got: {message}"
        );
    }

    #[test]
    fn an_uncancelled_large_comparison_still_completes() {
        let old = fixture("large", "old.xlsx");
        let new = fixture("large", "new.xlsx");
        let diff = diff_xlsx(&old, &new, None).unwrap();
        // Identical fixtures on both sides: no sheet or cell differences.
        assert!(diff.is_empty());
    }

    // ── F117: the size bound ─────────────────────────────────────────────
    //
    // The `large` fixture is 51,000 populated cells per side (found by
    // bisecting the bound; the earlier "60,000" in this file was never
    // measured). `PRODUCT`'s 2,000,000 is too large to reach from a
    // committed fixture, so these tests pass smaller bounds through
    // `diff_xlsx_with_bounds`; the real value's basis is on `CellBounds`.

    const BIG: u64 = 100_000_000;

    fn large() -> (PathBuf, PathBuf) {
        (fixture("large", "old.xlsx"), fixture("large", "new.xlsx"))
    }

    /// The bound is on coordinates *compared*, and the boundary is exact:
    /// the fixture's 51,000 coordinates pass at 51,000 and are refused at
    /// 50,999. Falsify by dropping `max_cells_compared` from `build_options`
    /// (the tight case then returns `Ok`) — see the review request.
    #[test]
    fn the_compared_bound_is_exact_at_the_fixtures_cell_count() {
        let (old, new) = large();
        let at = CellBounds {
            max_cells_read: BIG,
            max_cells_compared: 51_000,
        };
        diff_xlsx_with_bounds(&old, &new, None, at)
            .expect("a comparison exactly at the bound is admitted");

        let under = CellBounds {
            max_cells_read: BIG,
            max_cells_compared: 50_999,
        };
        let err = diff_xlsx_with_bounds(&old, &new, None, under)
            .expect_err("one coordinate over the bound must be refused");
        let message = err.to_string();
        assert!(
            message.contains("max_cells_compared") && message.contains("too large"),
            "the refusal must name the bound: {message}"
        );
    }

    /// F117 §3: reaching the bound is an `Err`, never an `Ok` carrying fewer
    /// differences. The fixture pair is identical, so a truncating (or
    /// unbounded) implementation returns `Ok` with an empty diff — which the
    /// view renders as "these workbooks match", for a file it did not
    /// finish reading. Falsified for real: removing the limits made this
    /// return exactly that `Ok`.
    #[test]
    fn a_bounded_comparison_is_an_error_never_a_partial_diff() {
        let (old, new) = large();
        let bounds = CellBounds {
            max_cells_read: BIG,
            max_cells_compared: 1_000,
        };
        let result = diff_xlsx_with_bounds(&old, &new, None, bounds);
        assert!(
            matches!(result, Err(CoreError::Unsupported { .. })),
            "a bounded comparison must not produce a diff: {result:?}"
        );
    }

    /// The read bound fires mid-read, before the compare phase, and counts
    /// both sides cumulatively (51,000 + 51,000).
    #[test]
    fn the_read_bound_counts_both_sides_and_fires_first() {
        let (old, new) = large();
        let ok = CellBounds {
            max_cells_read: 102_000,
            max_cells_compared: BIG,
        };
        diff_xlsx_with_bounds(&old, &new, None, ok).expect("102,000 reads are admitted");

        let tight = CellBounds {
            max_cells_read: 101_999,
            max_cells_compared: BIG,
        };
        let message = diff_xlsx_with_bounds(&old, &new, None, tight)
            .expect_err("one read over the bound must be refused")
            .to_string();
        assert!(message.contains("max_cells_read"), "{message}");
    }

    /// Ordinary workbooks are untouched by the product bound: every small
    /// fixture, and the 51,000-cell one, run to completion under `PRODUCT`
    /// (they all go through `diff_xlsx`, which uses it).
    #[test]
    fn the_product_bound_admits_the_ordinary_and_the_large_fixture() {
        for case in [
            "basic",
            "formula",
            "renamed",
            "renamed_and_moved",
            "unchanged_sheet",
        ] {
            diff(case);
        }
        let (old, new) = large();
        diff_xlsx(&old, &new, None).expect("51,000 cells is far under the bound");
        assert_eq!(CellBounds::PRODUCT.max_cells_compared, 2_000_000);
        assert_eq!(
            CellBounds::PRODUCT.max_cells_read,
            2 * CellBounds::PRODUCT.max_cells_compared
        );
    }

    /// Cancellation still interrupts *during* a comparison, not only before
    /// one starts (F65 records that the mid-sheet case was the defect we
    /// reported upstream). The token is cancelled from another thread while
    /// the 51,000-cell comparison is running; the error must be the
    /// cancellation, not the size bound and not `Ok`.
    #[test]
    fn cancellation_still_interrupts_mid_comparison_under_the_bound() {
        let (old, new) = large();
        let token = CancellationToken::new();
        let canceller = token.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1));
            canceller.cancel();
        });
        let result = diff_xlsx(&old, &new, Some(&token));
        handle.join().unwrap();

        let message = result
            .expect_err("a comparison cancelled mid-run must not complete")
            .to_string();
        assert!(message.contains("cancelled"), "{message}");
        assert!(!message.contains("too large"), "{message}");
    }

    // ── The rendered pair (F129) ─────────────────────────────────────────────
    //
    // The tests above assert the *model*, and the model was right when a
    // reorder-only pair shipped as two byte-identical documents. What the user
    // sees is the pair `derive_pair_text_from_diff` renders, so that is what
    // these assert.

    fn rendered(case: &str) -> (String, String) {
        let (l, r) =
            derive_pair_text(&fixture(case, "old.xlsx"), &fixture(case, "new.xlsx")).unwrap();
        (l.content, r.content)
    }

    /// For every fixture case, in either direction: a pair of workbooks that
    /// differ renders two different sides, and a pair that does not renders two
    /// equal ones. "Differ" is decided by the files' bytes, independently of the
    /// model under test. It walks the directory, so a new fixture is covered by
    /// construction, with no exception list.
    ///
    /// The `large` case is byte-identical by design (it exists to size the
    /// bounds and to be cancelled), so it takes the second branch: its sides are
    /// legitimately equal, which is what a first draft of this test that assumed
    /// "every fixture differs" tripped over.
    #[test]
    fn every_fixture_pair_renders_two_different_sides_in_both_directions() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/fixtures/xlsx");
        let mut cases: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().into_string().unwrap())
            .collect();
        cases.sort();
        assert!(
            cases.len() >= 9,
            "fixture directory not found or emptied: {cases:?}"
        );
        for case in &cases {
            let (old, new) = (fixture(case, "old.xlsx"), fixture(case, "new.xlsx"));
            let workbooks_differ = std::fs::read(&old).unwrap() != std::fs::read(&new).unwrap();
            for (from, to, direction) in [(&old, &new, "old→new"), (&new, &old, "new→old")] {
                let (l, r) = derive_pair_text(from, to).unwrap();
                if workbooks_differ {
                    assert_ne!(
                        l.content, r.content,
                        "{case} ({direction}): the workbooks differ, but both sides render as:\n{}",
                        l.content
                    );
                } else {
                    assert_eq!(
                        l.content, r.content,
                        "{case} ({direction}): identical files"
                    );
                }
            }
        }
    }

    /// The control that keeps the test above honest: a pair that does not
    /// differ renders identical sides, so "the sides differ" is a real claim.
    #[test]
    fn a_pair_that_does_not_differ_renders_identical_sides() {
        let same = fixture("unchanged_sheet", "old.xlsx");
        let (l, r) = derive_pair_text(&same, &same).unwrap();
        assert_eq!(l.content, r.content);
        assert!(l.content.is_empty(), "{:?}", l.content);
    }

    /// A reorder-only pair: same sheets, same cells, tabs swapped. The model
    /// says both sheets moved and — new in F129 — where each one is on each
    /// side, and each side's sheet list is in its own tab order. Falsify by
    /// restoring the old `Moved` arm (`"  Sheet: {name} (moved)"` on both
    /// sides): the sides become identical and the first assertion fails.
    #[test]
    fn a_reorder_only_pair_shows_where_each_sheet_is_on_each_side() {
        let diff = diff("reorder_only");
        assert_eq!(
            diff.sheets,
            vec![
                SheetChange::Moved("Same".into()),
                SheetChange::Moved("Other".into())
            ]
        );
        assert!(diff.cells.is_empty(), "no cell changed");
        assert_eq!(diff.tab_counts, (2, 2));

        let (old, new) = rendered("reorder_only");
        assert_ne!(old, new, "a reorder must not render as no difference");
        assert_eq!(
            old,
            "  Sheet: Same (moved: tab 1 of 2)\n  Sheet: Other (moved: tab 2 of 2)\n"
        );
        assert_eq!(
            new,
            "  Sheet: Other (moved: tab 1 of 2)\n  Sheet: Same (moved: tab 2 of 2)\n"
        );
    }

    /// A pure rename, end to end: each pane names the sheet as *its* workbook
    /// does. (Upstream reported that their own renderer showed nothing for
    /// this; the sheet list is ours, so it is checked here.)
    #[test]
    fn a_pure_rename_renders_each_sides_own_name() {
        let (old, new) = rendered("renamed");
        assert_eq!(old, "~ Sheet: Original\n");
        assert_eq!(new, "~ Sheet: Renamed\n");
    }

    /// The sheet list is each side's own order, so a renamed-and-moved pair
    /// puts the sheets in different places on the two sides.
    #[test]
    fn the_sheet_list_follows_each_sides_own_tab_order() {
        let (old, new) = rendered("renamed_and_moved");
        assert_eq!(
            old,
            "  Sheet: Anchor (moved: tab 1 of 2)\n~ Sheet: ToRename\n"
        );
        assert_eq!(
            new,
            "~ Sheet: RenamedSheet\n  Sheet: Anchor (moved: tab 2 of 2)\n"
        );
    }

    /// F123: a 5 KB workbook with a cell at `A1` and one at Excel's last position
    /// (`XFD1048576`). Through `sheets-diff` 2.5.0 the read allocated the box
    /// between them — 512 GiB, which aborts the process, so on that version this
    /// test does not fail, it takes the test binary down. From 2.5.1 the read
    /// streams and memory follows the two cells, so it is compared like any
    /// other pair. The fixture is what keeps the user-facing warning retired.
    #[test]
    fn a_stray_cell_at_the_last_position_is_compared_not_refused_or_fatal() {
        let diff = diff("stray_far_cell");
        assert_eq!(diff.stats.cells_changed, 1);
        let cell = &diff.cells[0].cells[0];
        assert_eq!(cell.addr, "XFD1048576");
        assert_eq!(
            (cell.old_value.as_deref(), cell.new_value.as_deref()),
            (Some("x"), Some("y"))
        );
    }

    /// F129 found no fixture for an added or a removed sheet, and their
    /// rendering arms have the shape of the one that was wrong.
    #[test]
    fn an_added_sheet_is_reported_and_marked_on_the_new_side_only() {
        let diff = diff("sheet_added");
        assert_eq!(diff.sheets, vec![SheetChange::Added("Added".into())]);
        assert_eq!(diff.stats.sheets_added, 1);
        let (old, new) = rendered("sheet_added");
        assert!(new.starts_with("+ Sheet: Added\n"), "{new}");
        assert!(old.starts_with("  Sheet: Added\n"), "{old}");
        assert!(old.contains("A1 [value]: (empty)") && new.contains("A1 [value]: other content"));
    }

    #[test]
    fn a_removed_sheet_is_reported_and_marked_on_the_old_side_only() {
        let diff = diff("sheet_removed");
        assert_eq!(diff.sheets, vec![SheetChange::Removed("Removed".into())]);
        assert_eq!(diff.stats.sheets_removed, 1);
        let (old, new) = rendered("sheet_removed");
        assert!(old.starts_with("- Sheet: Removed\n"), "{old}");
        assert!(new.starts_with("  Sheet: Removed\n"), "{new}");
        assert!(old.contains("A1 [value]: other content") && new.contains("A1 [value]: (empty)"));
    }

    // ── Diagnostics (F131) ───────────────────────────────────────────────────

    fn warnings_of(case: &str) -> Vec<SpreadsheetWarning> {
        diff(case).warnings
    }

    /// A chart sheet is not compared at all, and the parser says so at `Warning`
    /// (workbook-level). Falsify by dropping the warning from `convert`: the
    /// list is empty and this fails.
    #[test]
    fn a_sheet_that_was_not_compared_is_a_warning_naming_it() {
        assert_eq!(
            warnings_of("chart_sheet_not_compared"),
            vec![SpreadsheetWarning {
                kind: SpreadsheetWarningKind::SheetNotCompared,
                sheets: vec!["Chart1".into()],
                detail: None,
            }]
        );
    }

    /// Several removed and several added sheets, none matchable: one message
    /// naming the candidates, not one per sheet.
    #[test]
    fn an_ambiguous_sheet_match_is_one_warning_naming_the_candidates() {
        assert_eq!(
            warnings_of("ambiguous_rename"),
            vec![SpreadsheetWarning {
                kind: SpreadsheetWarningKind::AmbiguousSheetMatch,
                sheets: vec!["Gamma".into(), "Delta".into()],
                detail: None,
            }]
        );
    }

    /// The defect `sheets-diff` shipped for six releases: warnings live in
    /// `WorkbookDiff::diagnostics` **and** in each `SheetDiff::diagnostics`, and
    /// reading one hides the other. Two of its four warnings are sheet-level.
    /// The product never selects a keyed alignment, so through `diff_xlsx` a
    /// sheet-level warning cannot arise; this drives `convert` with the options
    /// that produce one. Falsify by removing the per-sheet loop from
    /// `collect_notes`: this fails, and the workbook-level tests above still
    /// pass, which is why it is a test of its own.
    #[test]
    fn a_sheet_level_warning_is_read_as_well_as_the_workbook_level_ones() {
        let opts = sheets_diff::DiffOptions::builder()
            .alignment(sheets_diff::AlignmentMode::RowKey { columns: vec![1] })
            .build()
            .unwrap();
        let wb = sheets_diff::compare_paths_with_options(
            fixture("duplicate_row_keys", "old.xlsx"),
            fixture("duplicate_row_keys", "new.xlsx"),
            opts,
        )
        .unwrap();
        assert!(
            wb.diagnostics
                .iter()
                .all(|d| d.severity < sheets_diff::Severity::Warning),
            "the premise: no workbook-level warning, so any warning found is sheet-level"
        );
        let warnings = convert(wb).warnings;
        assert_eq!(
            warnings,
            vec![SpreadsheetWarning {
                kind: SpreadsheetWarningKind::DuplicateAlignmentKey,
                sheets: vec!["Keyed".into()],
                detail: None,
            }]
        );
    }

    fn note(
        is_warning: bool,
        kind: SpreadsheetWarningKind,
        sheets: &[&str],
        message: &str,
    ) -> Note {
        Note {
            is_warning,
            kind,
            sheets: sheets.iter().map(|s| s.to_string()).collect(),
            message: message.into(),
        }
    }

    /// The match is on severity: a warning this crate has no name for is
    /// surfaced, with the parser's own message. (`sheets_diff::Diagnostic` is
    /// `#[non_exhaustive]`, so a future variant cannot be built here; the
    /// grouping is tested on our own `Note`, and `note_from`'s severity test is
    /// exercised by the real warnings above.) Falsify by keeping only the kinds
    /// this crate knows: this fails.
    #[test]
    fn an_unknown_warning_is_surfaced_not_dropped() {
        let (warnings, info) = summarize_notes(vec![note(
            true,
            SpreadsheetWarningKind::Other,
            &[],
            "something new upstream",
        )]);
        assert_eq!(info, 0);
        assert_eq!(
            warnings,
            vec![SpreadsheetWarning {
                kind: SpreadsheetWarningKind::Other,
                sheets: vec![],
                detail: Some("something new upstream".into()),
            }]
        );
    }

    /// One warning across many sheets is one message; kinds are ordered by
    /// urgency whatever order the parser reported them in; `Info` is counted.
    #[test]
    fn warnings_are_grouped_by_kind_ordered_by_urgency_and_info_is_counted() {
        use SpreadsheetWarningKind::*;
        let (warnings, info) = summarize_notes(vec![
            note(true, SheetNotCompared, &["Chart1"], ""),
            note(false, Other, &[], "info"),
            note(true, DuplicateAlignmentKey, &["A"], ""),
            note(false, Other, &[], "info"),
            note(true, DuplicateAlignmentKey, &["B"], ""),
            note(true, DuplicateAlignmentKey, &["A"], ""),
        ]);
        assert_eq!(info, 2);
        let shape: Vec<(SpreadsheetWarningKind, Vec<&str>)> = warnings
            .iter()
            .map(|w| (w.kind, w.sheets.iter().map(String::as_str).collect()))
            .collect();
        assert_eq!(
            shape,
            vec![
                (DuplicateAlignmentKey, vec!["A", "B"]),
                (SheetNotCompared, vec!["Chart1"])
            ]
        );
    }

    /// `FormulaUnavailable` is `Info` and is emitted per numeric cell. On the
    /// `large` fixture (tens of thousands of numeric cells) it is counted and
    /// nothing per-item reaches the model or the rendered text. Falsify by
    /// turning the `Info` branch of `summarize_notes` into a push onto
    /// `warnings`: the assertion on `warnings` fails.
    #[test]
    fn formula_unavailable_at_scale_is_counted_never_listed() {
        let (old, new) = large();
        let diff = diff_xlsx(&old, &new, None).unwrap();
        assert!(diff.warnings.is_empty(), "{:?}", diff.warnings);
        assert!(
            diff.info_notes > 10_000,
            "the premise: this fixture emits a diagnostic per cell, got {}",
            diff.info_notes
        );
        let (left, right) = derive_pair_text_from_diff(&diff);
        for text in [&left.content, &right.content] {
            assert!(!text.contains("unavailable"), "{text}");
            assert!(text.lines().count() < 10, "{} lines", text.lines().count());
        }
    }

    /// The pair the comparison view receives carries the warnings with the text
    /// they qualify.
    #[test]
    fn compare_pair_returns_the_warnings_with_the_text() {
        let pair = compare_pair(
            &fixture("chart_sheet_not_compared", "old.xlsx"),
            &fixture("chart_sheet_not_compared", "new.xlsx"),
        )
        .unwrap();
        assert_eq!(pair.warnings.len(), 1);
        assert!(pair.left.content.contains("A1 [value]: before"));
    }
}
