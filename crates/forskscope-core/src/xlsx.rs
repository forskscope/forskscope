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
//! sheet costs memory in proportion to its populated cells. F138 moved it to
//! 3.2.0: fixes a 512-byte file that could abort the process with a 9.26 GB
//! allocation (present in every release through 3.1.0; see
//! `tests/f138_decline_before_delegating.rs`) and two silent-wrong-answer sheet-matching
//! defects; the API and this module's notes below are unchanged.
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
//! ## sheets-diff API notes
//!
//! Written against 3.0.0; nothing below changed through 3.2.0 (the version
//! this workspace locks). 3.1.0 added one `#[non_exhaustive]` enum variant
//! this module does not match on; 3.2.0 added none.
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

/// Where one changed cell's row sits in each sheet. Our mirror of `sheets-diff`'s
/// `RowPlacement` (RFC-085 keeps upstream's naming on its own side of `convert`).
/// Exhaustive on purpose: a `_ =>` must not be able to put a change in the wrong
/// row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowPlacement {
    /// Paired by content: the row is `old_row` in the old sheet and `new_row` in the new.
    PairedByContent { old_row: u32, new_row: u32 },
    /// The row exists only in the old sheet (removed).
    OldSideOnly { old_row: u32 },
    /// The row exists only in the new sheet (inserted).
    NewSideOnly { new_row: u32 },
    /// Compared by position: row N of the old sheet against row N of the new.
    ByPosition { row: u32 },
}

/// How one sheet's rows were lined up, which is what its header line says (F132;
/// handoff 069 §3 and §5). Every variant but `Positional` is a claim the user is
/// owed the reason for, so each one carries its own sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetAlignment {
    /// Compared by position, and nothing needs saying.
    Positional,
    /// Rows paired by content. `matched` counts paired rows; `inserted` and
    /// `removed` count rows with no counterpart.
    ByContent {
        inserted: usize,
        removed: usize,
        matched: usize,
    },
    /// Compared by position because some rows are identical, so a pairing among
    /// them would have been a guess.
    PositionalIdenticalRows,
    /// Compared by position because the sheet is too large to align.
    PositionalTooLarge,
    /// Compared by position because aligning rows would report shifted formulas
    /// as changed (handoff 069 §3.1).
    PositionalShiftedFormulas,
}

impl SheetAlignment {
    /// The text that follows `Sheet: <name>` on the sheet's header line, both
    /// panes alike. English throughout: the side texts carry no locale.
    fn header_note(&self) -> String {
        match self {
            SheetAlignment::Positional => String::new(),
            SheetAlignment::ByContent {
                inserted,
                removed,
                matched,
            } => format!(
                " (aligned by content: {inserted} inserted, {removed} removed, {matched} matched)"
            ),
            SheetAlignment::PositionalIdenticalRows => {
                " (compared by position: some rows are identical)".into()
            }
            SheetAlignment::PositionalTooLarge => {
                " (compared by position: too large to align)".into()
            }
            SheetAlignment::PositionalShiftedFormulas => {
                " (compared by position: aligning rows would report shifted formulas as changed)"
                    .into()
            }
        }
    }
}

/// Changed cells within one sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetCellChanges {
    pub sheet: String,
    pub cells: Vec<CellChange>,
    /// `placements[i]` is where `cells[i]` sits. Parallel to `cells`, not a field on
    /// `CellChange`, so that struct's shape does not change.
    pub placements: Vec<RowPlacement>,
    /// How this sheet's rows were lined up.
    pub alignment: SheetAlignment,
}

/// One changed cell. A cell is one entry, however many facets changed: if its
/// value and its formula both changed, they are combined here (Q1 answer).
///
/// **A change is identified by its address together with its row placement**
/// (see [`SheetCellChanges::placements`]). Under an aligned sheet two entries
/// can share an address and still be different changes: one row removed from
/// the old sheet and another inserted into the new can both sit at `B6`. The
/// entries are kept in the order the comparison reports them, and never keyed
/// by address alone.
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
/// **`AlignmentMode` (RFC-058 condition 4): reversed in 0.183.0 (F132).** Each
/// sheet keeps its positional result or a content-aligned one, by the rule in
/// `pick`. The content-aligned leg is a second, full comparison, so it is
/// bounded by the same `CellBounds`, and by `max_alignment_product` (25,000,000
/// row pairs) inside `sheets-diff`, which falls back to positional above it.
/// The one-row insertion cases in `xlsx_fixtures.rs` are the reason the reversal
/// was made.
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
    // Two comparisons, not one: positional, which is today's result, and rows
    // paired by content (`RowSignature` over every column). Each sheet keeps one
    // of them, by the rule in `choose_leg`. The second is skipped when the
    // positional result has no cell changes, because nothing could be chosen
    // over it. Both go through `build_options`, so both honour the same bounds
    // and the same cancellation token.
    let positional = compare_leg(
        old_path,
        new_path,
        cancel,
        bounds,
        sheets_diff::AlignmentMode::Positional,
    )?;
    let aligned = if has_cell_changes(&positional) {
        Some(compare_leg(
            old_path,
            new_path,
            cancel,
            bounds,
            sheets_diff::AlignmentMode::RowSignature {
                sample_columns: None,
            },
        )?)
    } else {
        None
    };

    Ok(convert_legs(&positional, aligned.as_ref()))
}

/// One comparison of the two workbooks under one alignment mode. Its errors are
/// mapped the same way whichever mode ran.
fn compare_leg(
    old_path: &Path,
    new_path: &Path,
    cancel: Option<&CancellationToken>,
    bounds: CellBounds,
    alignment: sheets_diff::AlignmentMode,
) -> Result<sheets_diff::WorkbookDiff> {
    let opts = build_options(cancel, bounds, alignment)?;
    sheets_diff::compare_paths_with_options(old_path, new_path, opts).map_err(|e| match e {
        sheets_diff::SheetsDiffError::LimitExceeded { limit, observed } => CoreError::Unsupported {
            message: format!(
                "{} is too large to compare: it exceeded the size bound ({limit}, \
                     reached {observed}). The comparison was stopped, not completed, so no \
                     result is shown",
                display_name(old_path)
            ),
        },
        e => CoreError::Unsupported {
            message: format!("could not diff workbook '{}': {}", old_path.display(), e),
        },
    })
}

/// Whether any sheet of this comparison has a cell that changed in value or formula.
fn has_cell_changes(wb: &sheets_diff::WorkbookDiff) -> bool {
    wb.sheets
        .iter()
        .any(|sd| sd.cell_diffs.iter().any(is_changed_cell))
}

/// A cell diff that changes a value or a formula. Format-only entries are not
/// counted, as `convert` has always skipped them.
fn is_changed_cell(cd: &sheets_diff::CellDiff) -> bool {
    cd.value.is_some() || cd.formula.is_some()
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| format!("'{}'", n.to_string_lossy()))
        .unwrap_or_else(|| format!("'{}'", path.display()))
}

fn build_options(
    cancel: Option<&CancellationToken>,
    bounds: CellBounds,
    alignment: sheets_diff::AlignmentMode,
) -> Result<sheets_diff::DiffOptions> {
    let mut limits = sheets_diff::Limits::hardened();
    limits.max_cells_read = Some(bounds.max_cells_read);
    limits.max_cells_compared = Some(bounds.max_cells_compared);
    let mut builder = sheets_diff::DiffOptions::builder()
        .limits(limits)
        .alignment(alignment);
    if let Some(token) = cancel {
        let tok = token.clone();
        builder = builder.cancellation(move || tok.is_cancelled());
    }
    // F154: back to the default (`true`) — F138 Part B's `false` is retired.
    //
    // Through 3.0.0-3.2.0, `sheets-diff`'s per-sheet formula-diagnostic loop
    // was gated on `has_formulas`, which actually meant "the formula-reading
    // pass finished without error" - true of a plain-data sheet with no
    // formulas at all, not "this sheet has formulas". A 51,000-cell numeric
    // sheet (no formulas anywhere) flooded `info_notes` to 102,001. Turning
    // the flag off was a workaround: it also skipped the loop, for any sheet,
    // formulas or not, which is why F138 Part B's own test (now
    // `a_formula_cells_changed_cached_value_is_always_reported`) had to
    // confirm cached-value comparison did not quietly go with it.
    //
    // 3.3.0 fixed the actual bug: the gate is now whether the sheet genuinely
    // has at least one formula, captured before the formula list is drained,
    // not the pass's own success flag. A sheet with no formulas - the `large`
    // fixture included - never enters the loop at all, flag notwithstanding;
    // a sheet that does gets at most one `Info` diagnostic per side, carrying
    // a count in its message text, not one per cell. The workaround has
    // nothing left to work around.
    //
    // That test is kept rather than deleted (F154, handoff 063 §6): the
    // promise `sheets-diff`'s own doc comment makes - this flag never gates
    // cached-value comparison - is now one upstream has committed to
    // keeping, not merely true of the versions checked. A promise is not a
    // compile error, so the test stays as the thing that would notice if
    // that ever changed.
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
///
/// `wb` is the positional comparison; `aligned` is the `RowSignature` one when
/// it ran. Each sheet keeps one of them, by [`pick`].
fn convert_legs(
    wb: &sheets_diff::WorkbookDiff,
    aligned: Option<&sheets_diff::WorkbookDiff>,
) -> SpreadsheetDiff {
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

        let al = aligned.and_then(|a| a.sheets.iter().find(|x| same_sheet(x, sd)));
        let (chosen, alignment) = pick(sd, al);
        if chosen.cell_diffs.is_empty() {
            continue;
        }
        let sheet_name = sd
            .new_sheet
            .as_ref()
            .or(sd.old_sheet.as_ref())
            .map(|s| s.name.clone())
            .unwrap_or_default();

        let mut sheet_cells = Vec::new();
        let mut placements = Vec::new();
        for cd in &chosen.cell_diffs {
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
            placements.push(map_placement(&cd.row_placement));
        }
        if !sheet_cells.is_empty() {
            cells.push(SheetCellChanges {
                sheet: sheet_name,
                cells: sheet_cells,
                placements,
                alignment,
            });
        }
    }

    // Stats. `sheets_*` come from the positional summary, which describes the
    // sheet list both legs share. The cell counts come from the cells we kept.
    // With no aligned sheet, the positional summary is exactly what it was
    // before this change, so it is kept as it was.
    let (warnings, info_notes) = summarize_notes(collect_notes(wb, aligned));
    let s = &wb.summary;
    let any_aligned = cells
        .iter()
        .any(|sc| matches!(sc.alignment, SheetAlignment::ByContent { .. }));
    let (values_changed, formulas_changed) = if any_aligned {
        let all = cells.iter().flat_map(|sc| sc.cells.iter());
        (
            all.clone().filter(|c| c.value_changed).count(),
            all.filter(|c| c.formula_changed).count(),
        )
    } else {
        (s.values_changed, s.formulas_changed)
    };
    let stats = SpreadsheetDiffStats {
        sheets_added: s.sheets_added,
        sheets_removed: s.sheets_removed,
        sheets_modified: s.sheets_changed,
        sheets_renamed: s.sheets_renamed,
        sheets_moved: s.sheets_moved,
        cells_changed: cells.iter().map(|sc| sc.cells.len()).sum(),
        values_changed,
        formulas_changed,
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

/// A positional-only conversion. Tests use it on a single comparison, which is
/// the case `convert_legs` reduces to with no aligned leg.
#[cfg(test)]
fn convert(wb: sheets_diff::WorkbookDiff) -> SpreadsheetDiff {
    convert_legs(&wb, None)
}

/// The same sheet in the other comparison: sheets are identified by their tab
/// positions on each side, which both comparisons share.
fn same_sheet(a: &sheets_diff::SheetDiff, b: &sheets_diff::SheetDiff) -> bool {
    a.old_sheet.as_ref().map(|s| s.index) == b.old_sheet.as_ref().map(|s| s.index)
        && a.new_sheet.as_ref().map(|s| s.index) == b.new_sheet.as_ref().map(|s| s.index)
}

/// Changed cells in one sheet, as `convert` counts them.
fn changed_cells(sd: &sheets_diff::SheetDiff) -> usize {
    sd.cell_diffs.iter().filter(|c| is_changed_cell(c)).count()
}

/// Changed formulas in one sheet.
fn changed_formulas(sd: &sheets_diff::SheetDiff) -> usize {
    sd.cell_diffs.iter().filter(|c| c.formula.is_some()).count()
}

/// Whether the aligned leg could not align this sheet and fell back.
fn fell_back(sd: &sheets_diff::SheetDiff) -> bool {
    sd.diagnostics.iter().any(|d| {
        matches!(
            d.kind,
            sheets_diff::DiagnosticKind::AlignmentBoundExceeded { .. }
        )
    })
}

/// The per-sheet rule of handoff 069 §3. The outcome is the handoff's; the order
/// is not: the cell count is checked before the veto and the formula gate, so that
/// a header naming a refusal appears only where the refusal changed what the user
/// sees (review 148; the byte-identity guard of §4 requires it).
///
/// Returns the leg this sheet keeps and the reason its header states. `pos` is
/// the positional sheet; `aligned` is the same sheet from the `RowSignature` leg,
/// when that leg ran.
///
/// The veto (rule 1) is `is_ambiguous()` and nothing else from the summary:
/// `ConfidenceReason` is `#[non_exhaustive]`, and `confidence` is not part of
/// the rule. Under `RowSignature { sample_columns: None }` a pairing only ever
/// joins display-identical rows, so ambiguity is the only way it can be wrong.
/// The formula gate (rule 3) guards right pairings that would be reported
/// wrongly, since formula text moves with its row. Cell count decides only which
/// result is more useful, never which is more correct.
fn pick<'a>(
    pos: &'a sheets_diff::SheetDiff,
    aligned: Option<&'a sheets_diff::SheetDiff>,
) -> (&'a sheets_diff::SheetDiff, SheetAlignment) {
    let Some(al) = aligned else {
        return (pos, SheetAlignment::Positional);
    };
    if fell_back(al) {
        return (pos, SheetAlignment::PositionalTooLarge);
    }
    let Some(summary) = al.alignment_summary.as_ref() else {
        return (pos, SheetAlignment::Positional);
    };
    // A tie keeps positional, which is the status quo and needs no explanation.
    if changed_cells(al) >= changed_cells(pos) {
        // Whether or not the sheet is ambiguous, the aligned result does not win,
        // so there is nothing for the veto to explain. Saying "some rows are
        // identical" here would change the output of a sheet that was never
        // going to be aligned (handoff 069 §4's byte-identity guard).
        return (pos, SheetAlignment::Positional);
    }
    if summary.is_ambiguous() {
        return (pos, SheetAlignment::PositionalIdenticalRows);
    }
    if changed_formulas(al) > changed_formulas(pos) {
        return (pos, SheetAlignment::PositionalShiftedFormulas);
    }
    (
        al,
        SheetAlignment::ByContent {
            inserted: summary.inserted_rows,
            removed: summary.removed_rows,
            matched: summary.matched_rows,
        },
    )
}

/// Our placement from upstream's. Exhaustive, so a new upstream case is a compile
/// error here rather than a change placed in the wrong row.
fn map_placement(p: &sheets_diff::RowPlacement) -> RowPlacement {
    use sheets_diff::RowPlacement as Up;
    match *p {
        Up::PairedByAlignment { old_row, new_row } => {
            RowPlacement::PairedByContent { old_row, new_row }
        }
        Up::UnpairedInOldSheet { old_row } => RowPlacement::OldSideOnly { old_row },
        Up::UnpairedInNewSheet { new_row } => RowPlacement::NewSideOnly { new_row },
        Up::ComparedPositionally { row } => RowPlacement::ByPosition { row },
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

/// Every diagnostic the result stands on: workbook-level and sheet-level, from the
/// positional leg, plus one the aligned leg raises that the user is owed.
///
/// That one is `AlignmentBoundExceeded` on a sheet whose changes are reported:
/// the sheet was too large to align and was compared by position, which is what
/// `AlignmentFellBack` says (F131; handoff 069 §5). The aligned leg's other
/// diagnostics are not surfaced. Its duplicate-signature diagnostic is the veto,
/// and the sheet's header says why. `missing_row_signature` cannot fire under
/// `sample_columns: None`.
fn collect_notes(
    wb: &sheets_diff::WorkbookDiff,
    aligned: Option<&sheets_diff::WorkbookDiff>,
) -> Vec<Note> {
    let mut notes: Vec<Note> = wb.diagnostics.iter().map(note_from).collect();
    for sd in &wb.sheets {
        notes.extend(sd.diagnostics.iter().map(note_from));
        if !sd.cell_diffs.iter().any(is_changed_cell) {
            continue;
        }
        if let Some(al) = aligned.and_then(|a| a.sheets.iter().find(|x| same_sheet(x, sd))) {
            notes.extend(
                al.diagnostics
                    .iter()
                    .filter(|d| {
                        matches!(
                            d.kind,
                            sheets_diff::DiagnosticKind::AlignmentBoundExceeded { .. }
                        )
                    })
                    .map(note_from),
            );
        }
    }
    notes
}

/// Group warnings by kind (one message naming every sheet), order them by
/// urgency, and count the rest.
///
/// F154: confirmed, not assumed, that this is still the right treatment for
/// `FormulaUnavailable` now that `sheets-diff` 3.3.0 emits it once per sheet
/// per side (carrying a count in `message`) rather than once per cell. It is
/// still just counted here, its `message` discarded along with every other
/// `Info` - unlike before 3.3.0, that discarded text is now itself a short,
/// bounded summary rather than 102,001 near-identical repeats, so there was
/// a real question of whether discarding it is still right. Nothing reads
/// `info_notes` outside this crate today (`SpreadsheetPair` never exposes it
/// to the UI) and no open finding asks for it to, so there is nowhere for a
/// per-sheet count to go yet - this is the conservative choice, not a
/// considered "Info should never be seen" position. Revisit if a future
/// finding wants these surfaced.
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
        out.push_str(&format!(
            "Sheet: {}{}\n",
            scd.sheet,
            scd.alignment.header_note()
        ));
        // The cells this side shows, each at its row on this side. A sheet
        // compared by position keeps its order and its addresses unchanged. A
        // sheet paired by content lists each side in that side's own row order,
        // so a row that moved shows at its old row on the left and its new row
        // on the right.
        let by_content = matches!(scd.alignment, SheetAlignment::ByContent { .. });
        let mut shown: Vec<(u32, usize)> = scd
            .placements
            .iter()
            .enumerate()
            .filter_map(|(i, place)| side_row(*place, side).map(|row| (row, i)))
            .collect();
        if by_content {
            shown.sort_by_key(|(row, _)| *row); // stable: ties keep the diff's order
        }
        for (row, i) in shown {
            let cell = &scd.cells[i];
            let addr = if by_content {
                a1(cell.col, row)
            } else {
                cell.addr.clone()
            };
            // Value line
            if cell.value_changed {
                let v = match side {
                    Side::Old => cell.old_value.as_deref().unwrap_or("(empty)"),
                    Side::New => cell.new_value.as_deref().unwrap_or("(empty)"),
                };
                out.push_str(&format!("  {} [value]: {}\n", addr, v));
            }
            // Formula line
            if cell.formula_changed {
                let f = match side {
                    Side::Old => cell.old_formula.as_deref().unwrap_or("(none)"),
                    Side::New => cell.new_formula.as_deref().unwrap_or("(none)"),
                };
                out.push_str(&format!("  {} [formula]: {}\n", addr, f));
            }
        }
    }

    out
}

/// The row a placed cell shows at on this side, or `None` when the row does not
/// exist on this side (a row removed from the old sheet shows nothing on the new).
fn side_row(place: RowPlacement, side: Side) -> Option<u32> {
    match (place, side) {
        (RowPlacement::PairedByContent { old_row, .. }, Side::Old) => Some(old_row),
        (RowPlacement::PairedByContent { new_row, .. }, Side::New) => Some(new_row),
        (RowPlacement::OldSideOnly { old_row }, Side::Old) => Some(old_row),
        (RowPlacement::NewSideOnly { new_row }, Side::New) => Some(new_row),
        (RowPlacement::ByPosition { row }, _) => Some(row),
        (RowPlacement::OldSideOnly { .. }, Side::New)
        | (RowPlacement::NewSideOnly { .. }, Side::Old) => None,
    }
}

/// A spreadsheet address (`B6`) from a 1-based column and a row.
fn a1(col: u32, row: u32) -> String {
    let mut letters = String::new();
    let mut c = col;
    while c > 0 {
        let rem = (c - 1) % 26;
        letters.insert(0, char::from(b'A' + rem as u8));
        c = (c - 1) / 26;
    }
    format!("{letters}{row}")
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
// real `sheets-diff` parser (2.5.0 when written, 3.0.0 since F130, 3.2.0 since
// F138; the model and both sides' rendered text are identical on every fixture
// across all three),
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

    /// F154 (was F138 Part B): a pin on a promise, not a guard against a
    /// hazard — `build_options` no longer touches `include_formula_cached_values`
    /// at all (3.3.0 fixed the actual bug the `false` worked around), and
    /// upstream has committed that this flag will never gate cached-value
    /// comparison, correcting the doc comment that once said it did. A3's
    /// formula **text** is `A1+A2` on both sides of the fixture; only the
    /// cached result (2 → 3) differs. This test is what would notice if that
    /// promise were ever broken — a promise is not a compile error.
    #[test]
    fn a_formula_cells_changed_cached_value_is_always_reported() {
        let diff = diff("formula_cached_value_changed");
        assert_eq!(diff.cells.len(), 1);
        let cell = &diff.cells[0].cells[0];
        assert!(
            cell.value_changed,
            "the cached value differs (2 vs 3) and must be reported"
        );
        assert!(
            !cell.formula_changed,
            "the formula text (\"A1+A2\") is identical on both sides"
        );
        assert_eq!(cell.old_value.as_deref(), Some("2"));
        assert_eq!(cell.new_value.as_deref(), Some("3"));
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

    /// Through 3.0.0-3.2.0: `FormulaUnavailable` is `Info` and was emitted per
    /// numeric cell on any readable sheet (`sheets-diff`'s `has_formulas` meant
    /// "the formula pass finished", not "this sheet has formulas"). On the
    /// `large` fixture (51,000 numeric cells, no formulas) that meant 102,001
    /// `info_notes`, counted rather than rendered per item — proving
    /// `summarize_notes`' architecture held even at that scale, which
    /// `warnings_are_grouped_by_kind_ordered_by_urgency_and_info_is_counted`
    /// above proves in general on synthetic `Note`s, independent of any real
    /// flood. F138 Part B's `include_formula_cached_values(false)` was a
    /// workaround for this, from our side.
    ///
    /// **F154: 3.3.0 fixed it upstream instead**, and the workaround is
    /// retired (`build_options` no longer touches the flag) — the diagnostic
    /// loop is now gated on whether the sheet genuinely has a formula
    /// (captured directly, not inferred from the read pass succeeding), so a
    /// plain-numeric sheet like this one never enters it at all, flag or no
    /// flag. This fixture no longer exercises "counted at scale"; it pins
    /// that the flood itself stays gone now that the cause is fixed rather
    /// than merely routed around.
    #[test]
    fn the_large_fixture_no_longer_floods_info_notes() {
        let (old, new) = large();
        let diff = diff_xlsx(&old, &new, None).unwrap();
        assert!(diff.warnings.is_empty(), "{:?}", diff.warnings);
        assert!(
            diff.info_notes < 10,
            "a sheet with no formulas must never enter the per-sheet formula \
             diagnostic loop, got {} info_notes",
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

    // ── F132: rows aligned by content (handoff 069 §9) ───────────────────────
    //
    // Each case below states its truth, the change its recipe made, and the
    // outcome §3's rule must produce. A case is named for its recipe in
    // `xtask/src/xlsx_fixtures.rs`. Falsifications are listed on each branch.

    fn sheet_of<'a>(d: &'a SpreadsheetDiff, name: &str) -> &'a SheetCellChanges {
        d.cells
            .iter()
            .find(|s| s.sheet == name)
            .unwrap_or_else(|| panic!("no changed cells for sheet {name}"))
    }

    fn side_text(case: &str, side: Side) -> String {
        let d = diff(case);
        let (old, new) = derive_pair_text_from_diff(&d);
        match side {
            Side::Old => old.content,
            Side::New => new.content,
        }
    }

    fn count(sheet: &SheetCellChanges, pred: impl Fn(&RowPlacement) -> bool) -> usize {
        sheet.placements.iter().filter(|p| pred(p)).count()
    }

    /// A — rows inserted or deleted, no identical rows. The problem this release exists for.
    /// Truth: one row inserted at row 3. Outcome: aligned, four cells, all on the new side.
    #[test]
    fn a1_an_inserted_row_is_reported_as_one_row_not_a_cascade() {
        let d = diff("row_inserted_near_top");
        let sheet = sheet_of(&d, "Sheet1");
        assert_eq!(
            sheet.alignment,
            SheetAlignment::ByContent {
                inserted: 1,
                removed: 0,
                matched: 201
            }
        );
        assert_eq!(sheet.cells.len(), 4, "only the inserted row's cells");
        assert!(
            sheet
                .placements
                .iter()
                .all(|p| *p == RowPlacement::NewSideOnly { new_row: 3 }),
            "the inserted row is on the new side only, at its own row 3: {:?}",
            sheet.placements
        );
        assert!(
            !side_text("row_inserted_near_top", Side::Old).contains("[value]"),
            "the old pane shows nothing for a row that is not in it"
        );
        let new = side_text("row_inserted_near_top", Side::New);
        assert!(
            new.contains("(aligned by content: 1 inserted, 0 removed, 201 matched)"),
            "the header must say the sheet was aligned, and how: {new}"
        );
    }

    /// Mirror of A1: row 3 removed. Outcome: aligned, shown on the old side only.
    #[test]
    fn a2_a_removed_row_is_shown_on_the_old_side_only() {
        let d = diff("row_deleted_near_top");
        let sheet = sheet_of(&d, "Sheet1");
        assert!(matches!(
            sheet.alignment,
            SheetAlignment::ByContent {
                removed: 1,
                inserted: 0,
                ..
            }
        ));
        assert!(
            sheet
                .placements
                .iter()
                .all(|p| *p == RowPlacement::OldSideOnly { old_row: 3 })
        );
        assert!(!side_text("row_deleted_near_top", Side::New).contains("[value]"));
    }

    /// Truth: one row inserted at 199. Positional 16 cells, aligned 4. Outcome: aligned.
    #[test]
    fn a3_an_insertion_near_the_bottom_is_aligned_when_that_reports_fewer_cells() {
        let d = diff("row_inserted_near_bottom");
        assert!(matches!(
            sheet_of(&d, "Sheet1").alignment,
            SheetAlignment::ByContent { inserted: 1, .. }
        ));
        assert_eq!(sheet_of(&d, "Sheet1").cells.len(), 4);
    }

    /// Truth: one row inserted at 10 and one removed at 150. Outcome: aligned, both shown.
    #[test]
    fn a4_a_balanced_insert_and_delete_is_aligned_and_both_are_shown() {
        let d = diff("insert_and_delete_balanced");
        let sheet = sheet_of(&d, "Sheet1");
        assert!(matches!(
            sheet.alignment,
            SheetAlignment::ByContent {
                inserted: 1,
                removed: 1,
                ..
            }
        ));
        assert_eq!(
            count(sheet, |p| matches!(p, RowPlacement::NewSideOnly { .. })),
            4
        );
        assert_eq!(
            count(sheet, |p| matches!(p, RowPlacement::OldSideOnly { .. })),
            4
        );
    }

    /// B — where aligned must lose. Truth: three cells edited in place. Outcome: positional,
    /// no header, because aligning would report three removed rows and three inserted.
    #[test]
    fn b1_cells_edited_in_place_stay_positional_with_no_header() {
        let d = diff("cells_edited_in_place");
        let sheet = sheet_of(&d, "Sheet1");
        assert_eq!(sheet.alignment, SheetAlignment::Positional);
        assert_eq!(sheet.cells.len(), 3);
        assert!(
            sheet
                .placements
                .iter()
                .all(|p| matches!(p, RowPlacement::ByPosition { .. }))
        );
        assert!(!side_text("cells_edited_in_place", Side::New).contains("aligned by content"));
    }

    /// Truth: one row inserted at 3, and row 120's `B` edited. Outcome: aligned, and row 120
    /// reads as removed and re-inserted, which §2 documents as expected.
    #[test]
    fn b2_an_edit_below_an_insertion_reads_as_removed_and_reinserted() {
        let d = diff("insert_plus_edit_below");
        let sheet = sheet_of(&d, "Sheet1");
        assert!(matches!(
            sheet.alignment,
            SheetAlignment::ByContent {
                inserted: 2,
                removed: 1,
                ..
            }
        ));
        assert_eq!(
            sheet.cells.len(),
            12,
            "the inserted row, plus the edited row twice"
        );
        assert_eq!(
            count(sheet, |p| matches!(p, RowPlacement::OldSideOnly { .. })),
            4
        );
    }

    /// C — identical rows. Truth: one row inserted, and rows 150 and 151 identical in every
    /// column. Outcome: positional, with the identical-rows header. The aligned leg was
    /// correct here, so this case records what the veto costs.
    #[test]
    fn c1_identical_rows_veto_the_aligned_result_and_say_so() {
        let d = diff("identical_rows_away_from_edit");
        let sheet = sheet_of(&d, "Sheet1");
        assert_eq!(sheet.alignment, SheetAlignment::PositionalIdenticalRows);
        assert!(
            side_text("identical_rows_away_from_edit", Side::New)
                .contains("(compared by position: some rows are identical)")
        );
    }

    /// C2 — the veto's necessity. Truth: one insertion, and rows 150 and 151 swapped with
    /// display-identical content. The aligned leg reports two inserted and one removed,
    /// which is wrong, so the veto must hold here.
    #[test]
    fn c2_display_identical_rows_with_formulas_that_differ_are_not_paired_by_content() {
        let d = diff("display_identical_formulas_differ");
        assert_eq!(
            sheet_of(&d, "Sheet1").alignment,
            SheetAlignment::PositionalIdenticalRows
        );
    }

    /// C3 — truth: one of two identical rows deleted, and which one is undeterminable.
    /// Outcome: positional, with the identical-rows header.
    #[test]
    fn c3_a_deleted_row_among_identical_rows_is_not_claimed_as_a_specific_row() {
        let d = diff("identical_rows_one_deleted");
        assert_eq!(
            sheet_of(&d, "Sheet1").alignment,
            SheetAlignment::PositionalIdenticalRows
        );
    }

    /// C4 — the veto costs nothing where alignment would not have helped: the aligned
    /// result is larger, so positional stands by count, and the header stays silent,
    /// because the veto changes nothing a user would see. (Handoff 069 §9 expected the
    /// identical-rows header here; §4's byte-identity guard decided against it. See the
    /// review request.)
    #[test]
    fn c4_identical_rows_with_edits_only_stay_positional() {
        let d = diff("identical_rows_no_structure_change");
        let sheet = sheet_of(&d, "Sheet1");
        assert_eq!(sheet.alignment, SheetAlignment::Positional);
        assert_eq!(sheet.cells.len(), 3);
    }

    /// The tie rule (added case, see `xlsx_fixtures.rs`): one row appended at the bottom
    /// moves nothing, so both legs report four cells. A tie keeps positional, with no header.
    #[test]
    fn a_tie_keeps_positional_with_no_header() {
        let d = diff("row_appended_at_bottom");
        let sheet = sheet_of(&d, "Sheet1");
        assert_eq!(sheet.cells.len(), 4);
        assert_eq!(sheet.alignment, SheetAlignment::Positional);
    }

    /// F — formulas: the gate in handoff 069 §3.1. Truth: a row inserted above row-relative
    /// formulas, which Excel rewrites. Positional wins on formula text, so positional stays.
    #[test]
    fn f1_row_relative_formulas_keep_positional_and_say_why() {
        let d = diff("relative_formulas_row_inserted");
        let sheet = sheet_of(&d, "Sheet1");
        assert_eq!(sheet.alignment, SheetAlignment::PositionalShiftedFormulas);
        assert!(
            side_text("relative_formulas_row_inserted", Side::New).contains(
                "(compared by position: aligning rows would report shifted formulas as changed)"
            )
        );
    }

    /// F2 — the gate does not fire when the referenced row did not move: `$C$2` is above
    /// the insertion. The aligned leg wins, with no formula changes.
    #[test]
    fn f2_a_reference_above_the_edit_does_not_trigger_the_formula_gate() {
        let d = diff("formulas_above_the_edit");
        let sheet = sheet_of(&d, "Sheet1");
        assert!(matches!(
            sheet.alignment,
            SheetAlignment::ByContent { inserted: 1, .. }
        ));
        assert!(sheet.cells.iter().all(|c| !c.formula_changed));
    }

    /// F3 — the aligned leg would not have won on count, so there is no formula header.
    #[test]
    fn f3_row_relative_formulas_with_edits_only_stay_positional_with_no_header() {
        let d = diff("relative_formulas_edits_only");
        let sheet = sheet_of(&d, "Sheet1");
        assert_eq!(sheet.alignment, SheetAlignment::Positional);
    }

    /// F4 — the gate must not hide a real formula edit: row 120's formula changed to `*3`.
    #[test]
    fn f4_a_real_formula_edit_is_visible_when_the_gate_keeps_positional() {
        let d = diff("relative_formulas_real_formula_edit");
        assert_eq!(
            sheet_of(&d, "Sheet1").alignment,
            SheetAlignment::PositionalShiftedFormulas
        );
        assert!(side_text("relative_formulas_real_formula_edit", Side::New).contains("C120*3"));
    }

    /// E1 — per-sheet choice: the insertion sheet aligns, the in-place edits sheet does not.
    #[test]
    fn e1_each_sheet_chooses_its_own_leg() {
        let d = diff("two_sheets_mixed");
        assert!(matches!(
            sheet_of(&d, "Sheet1").alignment,
            SheetAlignment::ByContent { .. }
        ));
        assert_eq!(sheet_of(&d, "Sheet2").alignment, SheetAlignment::Positional);
    }

    /// Cancellation reaches the second leg: a token cancelled before the aligned leg runs
    /// stops that leg with an error, and nothing is returned for the comparison.
    #[test]
    fn cancellation_reaches_the_aligned_leg() {
        let token = CancellationToken::new();
        token.cancel();
        let r = compare_leg(
            &fixture("row_inserted_near_top", "old.xlsx"),
            &fixture("row_inserted_near_top", "new.xlsx"),
            Some(&token),
            CellBounds::PRODUCT,
            sheets_diff::AlignmentMode::RowSignature {
                sample_columns: None,
            },
        );
        assert!(
            r.is_err(),
            "a cancelled aligned leg must not return a result"
        );
    }

    /// Measurement, not a committed case: handoff 069 §9's size cases (D), which are
    /// generated by the measurement harness and never committed. Run with
    /// `FSK_D_DIR=<dir> cargo test -p forskscope-core -- --ignored --nocapture`,
    /// where `<dir>` holds one sub-directory per pair, each with `old.xlsx` and `new.xlsx`.
    #[test]
    #[ignore = "measurement over generated workbooks; set FSK_D_DIR"]
    fn d_cases_measured_outside_the_repository() {
        let root = std::env::var("FSK_D_DIR").expect("set FSK_D_DIR");
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.join("old.xlsx").exists())
            .collect();
        dirs.sort();
        for dir in dirs {
            let started = std::time::Instant::now();
            let d = diff_xlsx(&dir.join("old.xlsx"), &dir.join("new.xlsx"), None).unwrap();
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            let alignments: Vec<(String, SheetAlignment)> = d
                .cells
                .iter()
                .map(|s| (s.sheet.clone(), s.alignment.clone()))
                .collect();
            let warnings: Vec<SpreadsheetWarningKind> = d.warnings.iter().map(|w| w.kind).collect();
            println!(
                "{} ms={ms:.1} cells={} alignment={alignments:?} warnings={warnings:?}",
                dir.display(),
                d.stats.cells_changed
            );
        }
    }

    /// Handoff 069 §3.2: the workbook-level warnings must be identical between the two
    /// legs, because `convert_legs` reads them from the positional leg only. Checked on
    /// every committed case, both legs, by kind and message.
    #[test]
    fn workbook_level_warnings_are_identical_between_the_two_legs() {
        let cases = [
            "basic",
            "unchanged_sheet",
            "renamed",
            "renamed_and_moved",
            "formula",
            "formula_cached_value_changed",
            "large",
            "reorder_only",
            "sheet_added",
            "sheet_removed",
            "stray_far_cell",
            "chart_sheet_not_compared",
            "ambiguous_rename",
            "duplicate_row_keys",
            "row_inserted_near_top",
            "row_appended_at_bottom",
            "insert_and_delete_balanced",
            "cells_edited_in_place",
            "insert_plus_edit_below",
            "identical_rows_away_from_edit",
            "display_identical_formulas_differ",
            "identical_rows_one_deleted",
            "identical_rows_no_structure_change",
            "relative_formulas_row_inserted",
            "formulas_above_the_edit",
            "relative_formulas_edits_only",
            "relative_formulas_real_formula_edit",
            "two_sheets_mixed",
        ];
        for case in cases {
            let (old, new) = (fixture(case, "old.xlsx"), fixture(case, "new.xlsx"));
            let pos = compare_leg(
                &old,
                &new,
                None,
                CellBounds::PRODUCT,
                sheets_diff::AlignmentMode::Positional,
            )
            .unwrap();
            let al = compare_leg(
                &old,
                &new,
                None,
                CellBounds::PRODUCT,
                sheets_diff::AlignmentMode::RowSignature {
                    sample_columns: None,
                },
            )
            .unwrap();
            let key = |d: &sheets_diff::Diagnostic| {
                (d.severity, d.kind.code().to_string(), d.message.clone())
            };
            let a: Vec<_> = pos.diagnostics.iter().map(key).collect();
            let b: Vec<_> = al.diagnostics.iter().map(key).collect();
            assert_eq!(a, b, "{case}: workbook-level warnings differ between legs");
        }
    }
}
