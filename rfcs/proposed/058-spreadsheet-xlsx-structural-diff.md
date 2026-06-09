# RFC 058: Spreadsheet (`.xlsx`) Structural Diff and Adapter Contract

**Status.** Proposed

## Status
Proposed. First written for the v0.40+ feature line. No prior RFC owns
spreadsheet comparison as a first-class concern; `.xlsx` has so far been
handled incidentally (RFC-001 §6.2 classification, RFC-012 as a generic
content kind, and the never-written "RFC-013 Spreadsheet Input Adapter
Policy" candidate from the migration roadmap). This RFC consolidates that
ownership.

## Summary

Define how ForskScope compares Microsoft Excel `.xlsx` workbooks: the
adapter boundary around the `sheets-diff` crate, the app-owned spreadsheet
diff model, the presentation contract (derived text now, aligned cell view
later), failure handling, and the criteria under which spreadsheet diff
graduates from a derived-text view to a first-class structured mode.

The guiding constraint is the project's own non-goals policy: weak,
shallow structured-data modes are worse than none (NG-005, D-009). A
spreadsheet mode must be either genuinely first-class or explicitly
labelled as a limited derived-text preview — never a credible-looking
wrapper that silently produces misleading results.

## Motivation

Spreadsheets are a common comparison target for the same users ForskScope
serves: config-as-spreadsheet, exported reports, data tables, financial
models. The reverse-engineered v0.22.x baseline already supported `.xlsx`
via `sheets-diff`, and that behaviour was preserved through the migration
(`forskscope-core::xlsx`). But the current implementation has three
concrete weaknesses that this RFC exists to address:

1. **Structure is discarded.** `sheets-diff` produces a *structured* diff —
   `Diff { sheet_diff, cell_diffs }` where each `CellDiff` carries
   `{ sheet, row, col, addr, kind, old, new }`. The adapter currently calls
   `unified_diff(&diff).split()` and flattens the *text* rendering into
   per-side line strings, throwing away the cell coordinates. The richest
   part of the upstream output never reaches the model.

2. **No tests.** The adapter has zero unit tests. There is no fixture
   workbook and no assertion that a known cell change produces the expected
   diff. Encoding (RFC-012) and diff (RFC-002) modules are well tested; the
   spreadsheet path is not.

3. **Unclear contract and failure modes.** `sheets-diff::Diff::new` opens
   workbooks with `.expect(...)`, i.e. it panics on a missing or malformed
   file. ForskScope's core forbids panics for user-facing failures
   (RFC-001 §6.5 / `CoreError`). The adapter does not currently isolate
   this risk.

The non-goals addendum (§3.6) records direct user evidence: people found
generic text tools "thin wrappers over console diff" for spreadsheet/CSV
use, while an aligned tabular view was the feature they valued. That is the
target experience, gated behind the first-class bar.

## Goals

- Make `.xlsx` comparison an explicit, owned feature with a documented
  adapter contract, not an incidental code path.
- Capture the **structured** cell-level diff from `sheets-diff` in an
  app-owned model, independent of the upstream crate's types and of its
  unified-text rendering.
- Keep the current derived-text view working as the v1 presentation, but
  drive it from the structured model so richer views can be added without a
  rewrite.
- Isolate all `sheets-diff` failure and panic risk behind a `Result`-typed
  core boundary.
- Define graduation criteria for a first-class aligned cell view (a future
  UI workspace), and the test corpus that must exist before it ships.
- Record concrete questions and feature requests for the `sheets-diff`
  author so the upstream contract can be firmed up.

## Non-Goals

- Excel **editing**, formula evaluation, or write-back. `.xlsx` stays
  read-only (`FileKind::ExcelXlsx::is_mergeable_text() == false`). Merge,
  save, and patch export do not apply to spreadsheets in this RFC.
- Other spreadsheet formats (`.xls`, `.ods`, `.csv` as a structured table).
  CSV may be a separate future RFC; `.xls` depends on upstream support.
- Becoming a data-audit or BI tool (non-goals NG-009).
- Chart, image, pivot-table, macro, or styling/format diff. Out of scope;
  may be listed as upstream feature requests only.

## Background: the `sheets-diff` contract (as of v1.1.4)

Observed public surface:

```rust
// sheets_diff::core::diff
pub struct Diff {
    pub old_filepath: String,
    pub new_filepath: String,
    pub sheet_diff: Vec<SheetDiff>,        // added/removed/renamed sheets
    pub cell_diffs: Vec<SheetCellDiff>,    // per-sheet changed cells
}
pub struct SheetDiff   { pub old: Option<String>, pub new: Option<String> }
pub struct SheetCellDiff { pub sheet: String, pub cells: Vec<CellDiff> }
pub struct CellDiff {
    pub row: usize, pub col: usize, pub addr: String,
    pub kind: CellDiffKind,                // Value | Formula
    pub old: Option<String>, pub new: Option<String>,
}
pub enum CellDiffKind { Value, Formula }

// sheets_diff::core::unified_format
pub fn unified_diff(&Diff) -> /* unified text */;   // .split() -> per-side
```

Observed behaviour and constraints:

- Backed by `calamine` for reading; `.xlsx` only in practice.
- `Diff::new` **panics** (`.expect`) if a workbook cannot be opened.
- Same-name sheets are matched by name (`filter_same_name_sheets`); a sheet
  rename appears as a remove + add in `sheet_diff`, not a rename.
- Cell diffs are sorted by sheet, then address, then kind. Both a value and
  a formula change on one cell produce two `CellDiff` entries.
- Only `Value` and `Formula` cell-diff kinds exist. No styling, merged
  cells, comments, data validation, or number-format diff.

## External Design

### Presentation now (v1): derived text, structurally sourced

The diff workspace continues to render `.xlsx` comparisons as two derived
text panes with the `(Excel)` charset label, as today. The difference is
that the text is generated from the app-owned structured model (below), not
by flattening the upstream unified string. This keeps the current UX while
making the structure available to future views and to tests.

A status note must remain visible that this is a read-only spreadsheet
comparison (no merge/save), consistent with D-009/D-015 (never hide an
unsupported case or pretend a limited mode is complete).

### Presentation later (future UI RFC): aligned cell view

The first-class target — deferred to a UI RFC, not built here — is a
sheet-tabbed, aligned grid:

```text
+-----------------------------------------------------------------------+
| Sheet: [ Sheet1 ▾ ]   12 changed cells   (1 added sheet, 0 removed)   |
+-------+----------------------------+----------------------------------+
| Cell  | Old                        | New                              |
+-------+----------------------------+----------------------------------+
| B2    | 100                        | 120        (value)               |
| C5    | =A1*2                      | =A1*3      (formula)             |
| D9    | (empty)                    | hello      (value, added)        |
+-------+----------------------------+----------------------------------+
```

Sheet add/remove/rename is shown above the grid. No cell is editable.

## Internal Design

### App-owned spreadsheet diff model

A new model in `forskscope-core`, holding **no** `sheets-diff` types so the
upstream crate can change or be replaced without touching the UI or tests
(same boundary rule the diff engine follows for `similar`, RFC-002 §5):

```rust
pub struct SpreadsheetDiff {
    pub sheets: Vec<SheetChange>,
    pub cells: Vec<SheetCellChanges>,
    pub stats: SpreadsheetDiffStats,
}

pub enum SheetChange {
    Added(String),
    Removed(String),
    // Upstream reports rename as remove+add; this variant is produced only
    // if/when rename detection is added (see upstream questions).
    Renamed { from: String, to: String },
}

pub struct SheetCellChanges {
    pub sheet: String,
    pub cells: Vec<CellChange>,
}

pub struct CellChange {
    pub addr: String,           // "B2"
    pub row: u32,
    pub col: u32,
    pub kind: CellChangeKind,   // Value | Formula
    pub old: Option<String>,
    pub new: Option<String>,
}

pub enum CellChangeKind { Value, Formula }

pub struct SpreadsheetDiffStats {
    pub sheets_added: usize,
    pub sheets_removed: usize,
    pub cells_changed: usize,
}
```

### Adapter boundary

```rust
/// Compute the structured spreadsheet diff for two `.xlsx` paths.
/// All upstream panic/IO risk is contained here and surfaced as CoreError.
pub fn diff_xlsx(old: &Path, new: &Path) -> Result<SpreadsheetDiff>;

/// Derive the per-side comparable text used by the current diff view,
/// rendered from the structured model (replaces the unified-text flatten).
pub fn derive_pair_text(diff: &SpreadsheetDiff) -> (TextDocument, TextDocument);
```

Panic isolation: until `sheets-diff` exposes a non-panicking constructor,
`diff_xlsx` guards the call with `std::panic::catch_unwind` and maps a
caught panic to `CoreError::Unsupported`/`CoreError::Io` with the offending
path. Pre-validation (`classify` already confirms the path is a regular
`.xlsx` file) reduces but does not eliminate the risk, so the guard stays
until upstream changes.

### Determinism

`sheets-diff` already sorts sheets and cells; the adapter preserves that
order and adds no nondeterminism, so the derived text and stats are stable
across runs (matching the determinism guarantee patch export relies on,
RFC-039).

## Test Corpus

Per D-009 ("each structured mode needs a test corpus"), spreadsheet diff
must ship with fixtures. Minimum set, committed under
`crates/forskscope-core/tests/fixtures/xlsx/`:

| Fixture pair | Exercises |
|---|---|
| identical workbooks | empty diff, `cells_changed == 0` |
| one value cell changed | single `CellChange { kind: Value }` with correct `addr` |
| one formula changed | `CellChange { kind: Formula }` |
| added sheet | `SheetChange::Added` |
| removed sheet | `SheetChange::Removed` |
| empty vs non-empty cell | `old: None` / `new: Some(..)` |
| malformed / non-workbook `.xlsx` | `diff_xlsx` returns `Err`, no panic |

Generating `.xlsx` fixtures in tests: prefer constructing them at test time
with a tiny writer (e.g. `rust_xlsxwriter`) so the repo stores generators,
not opaque binaries; fall back to checked-in fixtures if a writer dependency
is undesirable. The choice is settled during implementation, not by this
RFC.

## Graduation Criteria (derived-text → first-class aligned view)

The aligned cell view ships only when all hold:

- The structured model and adapter (this RFC) are implemented and tested.
- The test corpus above passes, including the no-panic malformed case.
- A UI RFC defines the sheet-tabbed grid, navigation, and large-workbook
  bounds (cell-count cap, lazy sheet rendering).
- Performance bounds for large workbooks are defined (consistent with
  RFC-013 large-file policy): a cell-count threshold beyond which the view
  falls back to summary + derived text.

Until then, the derived-text preview remains the shipped presentation, with
its read-only / limited-mode status visible.

## Questions and Feature Requests for the `sheets-diff` Author

`sheets-diff` is an active dependency and the author is reachable. The
following should be raised upstream; answers may simplify this RFC's
adapter and unlock the first-class view sooner.

**Questions (contract clarification):**

1. Is `Diff::new` intended to panic on unreadable/locked/malformed
   workbooks, or would a `try_new() -> Result<Diff, _>` (or
   `Diff::open() -> Result<...>`) be accepted? A non-panicking constructor
   would let ForskScope drop its `catch_unwind` guard.
2. What is the stability guarantee for the `cell_diffs` ordering (currently
   sheet → addr → kind)? Can downstreams rely on it across minor versions?
3. How are these represented (or are they ignored): merged cells, cells
   with only a number-format change, cached vs. live formula values,
   date/time serial values, and very large sheets?
4. Are sheet renames detectable, or always remove+add? Is the
   `sheet_diff: Vec<SheetDiff>` with `Option<String>` old/new intended to
   express rename pairs?
5. Is the `serde` feature considered stable for serializing `Diff`? (Useful
   for session persistence, RFC-011.)

**Feature requests (nice-to-have, prioritized):**

- A non-panicking constructor returning `Result` (highest value).
- Optional rename detection for sheets.
- A way to request "values only" vs "values + formulas" to reduce noise.
- A bound/streaming mode or cell cap for very large workbooks.
- Exposure of cell *type* (number / text / bool / date) alongside the
  stringified value, to enable type-aware rendering.

These are requests, not blockers: the v1 derived-text path works with the
crate as-is via the panic guard.

## Acceptance Criteria

- A documented `forskscope-core` adapter exposes a structured
  `SpreadsheetDiff` built from `sheets-diff`, holding no upstream types.
- The current derived-text diff view is driven by the structured model and
  remains visually unchanged for users.
- A malformed or unreadable `.xlsx` yields a `CoreError`, never a panic.
- A committed test corpus asserts value-change, formula-change, sheet
  add/remove, empty-cell, and malformed-file behaviour.
- `.xlsx` remains read-only: no merge, save, or patch export path is
  enabled for spreadsheets.
- Upstream questions/requests are filed (tracked in the PR description or an
  issue link recorded in the implementing changelog entry).

## Dependencies

- RFC 001 — Core Extraction and Canonical Domain Model (file classification,
  error taxonomy)
- RFC 002 — `similar` v3 Diff Engine (adapter-boundary precedent)
- RFC 012 — Text Encoding, Newline, and Binary Policy (content-kind model)
- RFC 013 — Large File, Performance, and Virtualization (large-workbook
  bounds for the future view)

## Open Questions

- Should the derived text encode cell addresses inline (e.g.
  `B2: 100 -> 120`) so the existing text view becomes more navigable before
  the aligned grid exists? (Cheap, improves the interim UX; decide at
  implementation.)
- Should a future CSV structured mode share `SpreadsheetDiff`, or have its
  own model? (Defer to a CSV RFC.)
