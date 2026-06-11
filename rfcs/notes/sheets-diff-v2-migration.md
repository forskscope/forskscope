# sheets-diff v2.2.0 — Migration Evaluation

**Date evaluated:** 2026-06-10
**Version in use:** sheets-diff v1.1.4 (via `xlsx.rs` adapter)
**Version reviewed:** v2.2.0 (complete rewrite; v2.0–v2.2 all landed same day)
**Decision:** Migration complete in v0.57.0. Archive this note.
  No action needed now.

---

## What changed (v1 → v2)

v2 is a ground-up rewrite. The v1 API is entirely gone.

| v1 | v2 |
|---|---|
| `Diff::new(old, new)` — panics on bad input | `compare_paths(old, new) -> Result<WorkbookDiff, SheetsDiffError>` |
| Stringly-typed `old: Option<String>` / `new: Option<String>` per cell | `CellValue` typed enum: `Empty \| Text \| Integer \| Number \| Bool \| DateTime \| Duration \| Error \| Unsupported` |
| Only `Added` / `Removed` sheet changes | `SheetChange::Unchanged \| Modified \| Added \| Removed \| Moved \| Renamed \| RenamedAndMoved` |
| No cancellation | `DiffOptionsBuilder::cancellation(Box<dyn Cancellation>)` |
| No progress | `DiffOptionsBuilder::progress(Box<dyn ProgressSink>)` |
| No resource limits | `Limits { max_sheets, max_cells_read, max_cells_compared, max_diffs_returned }` |
| No structured errors | `SheetsDiffError` with `OpenWorkbook`, `ReadSheet`, `EncryptedWorkbook`, `InvalidOptions`, `Cancelled`, `LimitExceeded` |
| `calamine` leaked into public types | All calamine types hidden behind `CalamiLineError` wrapper |

### v2.0 → v2.1 additions
- Row alignment modes (`AlignmentMode::RowKey`, `RowSignature`)
- Workbook metadata diffs (defined-name changes, sheet visibility)
- GUI view adapters (`output::view`: `DiffView`, `CellChangeRow`, `ViewFilter`)
- `FormatCompareMode` (honours current calamine 0.35 limitation honestly)

### v2.1 → v2.2 additions
- `DiffMetrics` (`sheets_read`, `cells_read`, `cells_compared`, `diffs_emitted`)
- Object/unsupported-feature diagnostics (`ObjectCompareMode`)
- Parallel sheet comparison (`ExecutionMode::Parallel` via `rayon` feature)
- `CellDisplay` / `CellSnapshot` / `CellNumberFormat` display metadata types
- Fuzz targets and benchmarks

---

## Impact on ForskScope

### Boundary is clean

`xlsx.rs` was designed so that `sheets-diff` types appear nowhere outside it.
`SpreadsheetDiff`, `SheetChange`, `CellChange`, `CellChangeKind`, and
`SpreadsheetDiffStats` are ForskScope's own types. The UI, tests, `report.rs`,
and `document.rs` are all unaffected by the migration.

### What improves

**1. Drop `catch_unwind`.** v1 panicked; `xlsx.rs` used `std::panic::catch_unwind`
to satisfy RFC-001's no-panic contract. v2 returns `Result` — `catch_unwind`
is removed entirely.

**2. Typed cell values.** `CellChange.old` / `.new` are currently
`Option<String>` (v1 display strings). v2's `ValueChange { old: CellValue,
new: CellValue, reason: ValueDifferenceKind }` lets the adapter pass
`CellValue::display_string()` for display (same output) while making the typed
value available for future UI features (e.g. `Integer → Number` type-change
warning, numeric tolerance diff).

**3. Richer sheet changes.** Current adapter only handles `Added`/`Removed`.
v2 adds `Modified`, `Moved`, `Renamed { confidence, reason }`,
`RenamedAndMoved`. ForskScope's `SheetChange` enum can be extended to surface
these in the report and diff view.

**4. Cancellation wired to `CancellationToken`.** `CancellationToken` in
`cancel.rs` wraps `Arc<AtomicBool>`. v2's `Cancellation` trait has a blanket
impl for `Fn() -> bool + Send + Sync`. Adapter:
```rust
let token_clone = token.clone();
let opts = DiffOptionsBuilder::new()
    .cancellation(move || token_clone.is_cancelled())
    .build()?;
```
Large workbooks can now be cancelled from the UI without blocking.

**5. `DiffSummary` from v2 directly.** Currently `SpreadsheetDiffStats` is
counted manually. `WorkbookDiff.summary` has `sheets_added`, `sheets_removed`,
`sheets_renamed`, `sheets_moved`, `sheets_changed`, `cells_changed`,
`values_changed`, `formulas_changed`, `diagnostics` — a complete superset.

---

## Migration scope

Only two files change:

### `crates/forskscope-core/Cargo.toml`
```toml
# Before
sheets-diff = "1"

# After
sheets-diff = "2.2"
```

### `crates/forskscope-core/src/xlsx.rs`

- Remove `std::panic::catch_unwind` and the `std::panic` import.
- Replace `sheets_diff::core::diff::Diff::new(&old_str, &new_str)` with
  `sheets_diff::compare_paths(old_path, new_path)`.
- Map `SheetsDiffError` to `CoreError::Unsupported { message }` via `.map_err`.
- Rewrite `convert(d: Diff)` → `convert(wb: WorkbookDiff)`:
  - Iterate `wb.sheets` → match `sd.change` for all six variants
  - For `Modified` / `Renamed` / `Moved`: iterate `sd.cell_diffs`, map
    `cd.value` → `CellChange { old: cv.old.display_string(), new: cv.new.display_string() }`
  - For `Added` / `Removed`: push `SheetChange` (no cells to map)
- Optionally extend `SpreadsheetDiffStats` from `wb.summary` directly.
- Optionally wire `CancellationToken` via `DiffOptionsBuilder::cancellation`.

Estimated: ~60 lines changed in `xlsx.rs`. No change anywhere else.

---

## No new transitive dependencies

v2 uses `calamine 0.35` (same as v1), plus optional `serde`, `chrono`,
`rayon`, `clap` — all behind feature flags, none enabled by ForskScope.
Zero new mandatory dependencies.

---

## What stays unchanged in ForskScope

- `SpreadsheetDiff`, `SheetCellChanges`, `CellChange`, `CellChangeKind`
  (ForskScope's own types — no change)
- `derive_pair_text_from_diff` (works on ForskScope types — no change)
- `load_placeholder` (no `sheets-diff` dependency — no change)
- All 9 xlsx tests (test ForskScope's model — no change)
- `xlsx_tests.rs` may gain new test cases for `Renamed`/`Moved`/`Cancelled`
