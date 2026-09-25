# RFC 058: Spreadsheet (`.xlsx`) Structural Diff and Adapter Contract

**Status.** Implemented (v0.57.0) — migrated to sheets-diff v2.2.1; structured result, no catch_unwind, cancellation wired

**Security suspension (F11, audit N4, current as of 2026-08-13):** the
runtime path this status line describes is not what ships today.
`sheets-diff -> calamine -> quick-xml` carries active denial-of-service
advisories for XML input; because `.xlsx` files are user-supplied local
archives containing XML, `.xlsx` structural comparison **fails closed**
instead of parsing workbook content through that chain (`crates/
forskscope-core/src/xlsx.rs`, `diff_xlsx` returns `Unsupported` without
parsing). `.xlsx` files are still recognized and always read-only
(`FileKind::ExcelXlsx` is never mergeable or saveable). This note documents
the current decision on top of the historical implementation record above,
rather than rewriting it — the v0.57.0 migration described above did happen
and shipped; it is simply not reachable at runtime right now. Re-enabling
requires the dependency path to be remediated first (tracked in
`docs/src/maintainers/release-evidence/*/advisories.md`).

**Lifting condition (added 2026-08-17, F65).** The dependency condition is
now **met**: `sheets-diff` 2.5.0 resolves `calamine 0.36.1 -> quick-xml
0.41.0` and `zip 8.6.0`, with `cargo audit` exiting 0 — verified from a
scratch resolve, not from upstream's lockfile or their word. Their MSRV
(1.88) is below this workspace's 1.91 floor.

**That is necessary and not sufficient.** Remediating the advisories removes
the reason the suspension was imposed; it does not by itself discharge the
suspension, because lifting it re-opens a parser to user-supplied archives —
this project's stated threat model. Four conditions must be met, and **none
of them depends on any other milestone**:

1. **This suspension is lifted here, deliberately, with reasoning** — not by
   a `Cargo.toml` edit. The threat-model entry moves with it: what the parser
   now defends against, and what it does not.
2. **`cargo xtask audit-deps` denies `sheets-diff` by name.** That deny is
   the mechanism by which the suspension is enforced, and it must be removed
   as an explicit act. It exists so re-adoption cannot happen incidentally.
3. **Platform case P10 inverts.** It currently asserts that the fail-closed
   message reaches the user; with `.xlsx` re-enabled it no longer tests what
   it claims. Changing it is a change to a **frozen** `matrix-plan.md`, so it
   requires the freeze to be lifted or the plan re-cut — not an edit in place.
4. **New evidence is required**, not inherited: a deliberately chosen
   `Limits::max_cells_compared` (upstream's `hardened()` preset bounded
   nothing before 2.4.0, and the value should be ours rather than theirs);
   mid-sheet cancellation observed rather than assumed, which this project
   told upstream it would test; and an `AlignmentMode` decision, since
   non-`Positional` alignment is what a diff tool wants and is the mode whose
   per-cell clone upstream removed in 2.5.0.

**Architect recommendation (2026-08-17): do not lift during v1
stabilization.** Not because the dependency is doubtful — it is not — but
because when the outstanding upstream blocker (F44) clears, this project
should be one dependency bump and one platform re-run away from a Gate D
verdict. Lifting first replaces that with a new runtime dependency, an
inverted platform case and a re-frozen matrix plan. The owner may decide
otherwise; the cost is a longer path to Gate D, not a safety one.

## Amendment — 2026-09-24: the suspension is lifted (F117)

The security suspension above was lifted on 2026-09-04 by commit `d492557`
(RFC-085, `sheets-diff` 2.5.0). The lifting condition names **this document**
as the place where that is recorded, with reasoning; it was not, for three
weeks. This amendment does that. The text above is left as written: it is the
record of what was decided then, and its present-tense statements ("fails
closed", "re-enabling requires…") were true until `d492557` and are **not true
now**.

**Where the four conditions stand**

1. *Lifted here, deliberately.* This amendment. What the parser now defends
   against and what it does not is set out below; the threat model's `.xlsx`
   sections describe the same (handoff 042).
2. *`audit-deps` denies `sheets-diff` by name.* Removed by `d492557`.
   `xtask/src/main.rs` now asserts the reviewed chain is **present**:
   `sheets-diff` under `forskscope-core`, `calamine` under `sheets-diff`,
   `zip` under `calamine`.
3. *P10 inverts.* RFC-085 names this as its own item; this amendment neither
   changes nor verifies it.
4. *New evidence.* The bound, the cancellation evidence and the alignment
   decision are below.

**What the parser now defends against.** The advisories that motivated the
suspension are gone from the resolved chain (`calamine 0.36.1 → quick-xml
0.41.0`, `zip 8.6.0`). A comparison runs under `Limits::hardened()` with two
values overridden, all set in `crates/forskscope-core/src/xlsx.rs`
(`CellBounds`):

| Bound | Value | Source |
|---|---|---|
| `max_input_bytes` | 50 MiB | `hardened()`, checked before any read |
| `max_sheets` | 256 | `hardened()` |
| `max_diffs_returned` | 1,000,000 | `hardened()` |
| `max_cells_compared` | **2,000,000** | chosen here |
| `max_cells_read` | **4,000,000** | chosen here (both sides, cumulative) |

`sheets-diff` 2.5.0's `max_cells_compared` (unchanged in 3.0.0) counts the coordinates visited,
not the differences found, which is the correction F65 recorded against
2.3.0. The value is measured, not taken from `hardened()`'s 5,000,000: an
unbounded comparison of two identical single-sheet workbooks used 1.6 s and
0.97 GB at 1,000,000 coordinates, 2.7 s and 1.9 GB at 2,000,000, and 7.1 s and
4.8 GB at 5,000,000 (release build, 2026-09-24). Cost is linear at about 1 KB
of peak memory per coordinate, so `hardened()`'s value would admit ~4.8 GB
from a file the user opened and did not write. 2,000,000 admits about 200
columns by 10,000 rows on each side at ~1.9 GB.

**A bound that is reached is an error, not a shorter diff.** Every
`sheets-diff` limit returns `Err(LimitExceeded)`; `diff_xlsx` reports it as
`CoreError::Unsupported` naming the bound, and the comparison view shows an
error tab. Until F117, `derive_pair_text` turned *every* error into two empty
documents, which diff as identical: a workbook pair that was corrupt, or that
stopped at a bound, was displayed as "these workbooks match". That path now
returns the error.

**Cancellation** is observed, not assumed: `diff_xlsx` is cancelled from
another thread while a 51,000-cell comparison runs
(`cancellation_still_interrupts_mid_comparison_under_the_bound`), and before it
starts, and both were falsified by removing the wiring.

**`AlignmentMode`: `Positional`, kept.** It is `sheets-diff`'s default, the
cheapest mode and the one measured above. Row-key and row-signature alignment
add an `m × n` table (bounded by `max_alignment_product`, 25,000,000) that
nothing in the UI selects. A future aligned-view RFC should revisit this with
its own measurement.

**What it did not defend against, and what changed (F123, F130).**

- **The parse phase was not bounded, and a 5 KB file could abort the process
  (measured 2026-09-24, F123). Closed by `sheets-diff` 2.5.1 and 3.0.0.**
  Through 2.5.0, `calamine`'s `worksheet_range` built a **dense** `Range` covering
  the bounding box of the populated cells (`Range::from_sparse` allocates
  `rows × cols` values) before `sheets-diff` counted a cell, so a workbook with
  one cell at `A1` and one far away cost memory in proportion to the *area
  between them*, about 31 bytes a cell, and F117's bounds fired only after that
  memory was spent. `sheets-diff` 2.5.1 reads through `calamine`'s streaming
  reader into its sparse map and checks `max_cells_read` and the cancellation
  poll inside the loop; 3.0.0 keeps that. Release build, this machine (32
  logical CPUs, 59 GB), address space limited to 20 GB, **same harness and files
  in both columns**, re-run 2026-09-26. The 5.4 KB workbook has a real cell at
  `A1` and one far away:

| Declared area | 2.5.0: peak memory, result, time | 3.0.0: peak memory, result, time |
|---|---|---|
| 1,000 × 1,000 = 1M | 35 MB, compared, 14 ms | **4 MB**, compared, 0.35 ms |
| 10,000 × 1,000 = 10M | 316 MB, refused | 4 MB, **compared**, 1.5 ms |
| 50,000 × 1,000 = 50M | 1.57 GB, refused | 4 MB, compared, 0.17 ms |
| 100,000 × 1,000 = 100M | 3.13 GB, refused | 4 MB, compared, 0.19 ms |
| 300,000 × 1,000 = 300M | 9.38 GB, refused, 3.4 s | 4 MB, compared, 0.49 ms |
| 1,048,576 × 16,384 (Excel's maximum sheet, 17.2 billion) | **process aborted:** `memory allocation of 549755813888 bytes failed` | 4 MB, compared, 0.16 ms |

  The rows that were "refused" are now **compared**: 3.0.0 counts populated
  cells against `max_cells_read`, not the area of their box (a workbook the new
  bound refuses, the old one refused too; some the old one refused are now
  accepted, and these two-cell workbooks are those). Memory follows the two
  populated cells. A declared `<dimension ref="A1:XFD1048576"/>` with no far cell
  was and remains harmless.
- **What remains is the cost of populated cells, and it is bounded.** 20,000 ×
  1,000 = 20M populated cells in a 51.5 MB file (just under the 50 MiB input
  bound): 2.5.0 refused it after 5.8 s at 2.58 GB; 3.0.0 refuses it after
  **0.97 s at 707 MB**, when `max_cells_read` counts its 4,000,001st cell. That
  is about **177 bytes per populated cell read**, and it is proportional and
  bounded by the bound, which is what the earlier "the bound fired late" was
  not. A comparison that reaches `max_cells_compared` (20,000 × 100 per side,
  2,000,000 coordinates) is unchanged: 2.5.0 2.7 s at 1.93 GB, 3.0.0 2.6 s at
  1.93 GB; F117's 1M/2M/5M basis reproduces (1.3 s at 0.97 GB, 2.7 s at 1.9 GB,
  6.7 s at 4.8 GB on 3.0.0). Whether 4,000,000 cells read is the right bound on
  a memory basis is open, and this is the evidence for it.
- **The parse can now be cancelled.** On the 20M-cell file, a cancel requested at
  100 ms returned at 4.71 s on 2.5.0 and at **112 ms** on 3.0.0; at 500 ms,
  4.74 s and **547 ms**. On 2.5.0 the first poll came only after the dense range
  existed.
- **The cheap route in this crate never had to exist.** A pre-parse check of the
  populated bounding box needed `calamine`'s streaming reader, and
  `forskscope-core` does not depend on `calamine` (`audit-deps` asserts
  `sheets-diff` is its only dependent). The fix was in `sheets-diff` — cells read
  through that streaming reader into the sparse map it already builds — and
  landed there upstream.
- Advisories published after 2026-09-24 against `quick-xml`, `zip` or
  `calamine`. `audit.yml` reports them daily; nothing here prevents them.

## Status
Implemented (v0.45.0). The core-layer deliverables from RFC-058 are shipped:

- **App-owned `SpreadsheetDiff` model** — no `sheets-diff` types in the
  public API: `SpreadsheetDiff { sheets, cells, stats }`, `SheetChange`
  (Added/Removed), `SheetCellChanges`, `CellChange { addr, row, col, kind,
  old, new }`, `CellChangeKind` (Value/Formula), `SpreadsheetDiffStats`.
- **`diff_xlsx(old, new) -> Result<SpreadsheetDiff>`** — wraps `Diff::new`
  in `std::panic::catch_unwind`; maps caught panics to `CoreError::Unsupported`
  (the upstream `.expect()` panic risk is now isolated, not silently risked).
- **`derive_pair_text_from_diff`** — drives the existing derived-text view
  from the structured model; user-visible output format is equivalent to
  before but now grounded in cell coordinates.
- **Test corpus** (9 tests) generated at test time via the `zip` dev-dep:
  identical workbooks → empty diff, value change → correct `addr`/old/new,
  empty→non-empty cell → `old: None`, sheet add/remove → `SheetChange`,
  malformed file → `Err` (not panic), malformed second file → `Err`,
  multiple changed cells, `derive_pair_text_from_diff` non-empty for changes,
  empty for identical.

Deferred (per graduation criteria in RFC-058): the aligned cell-grid UI
workspace (requires a UI RFC), performance bounds for very large workbooks,
and formula-diff fixtures (the structured model supports them; `sheets-diff`
emits `CellChangeKind::Formula` which is now preserved in `CellChange`). No prior RFC owns
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
