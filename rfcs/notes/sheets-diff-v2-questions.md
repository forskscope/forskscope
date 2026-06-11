# Questions & Feature Requests for sheets-diff (ahead of ForskScope's v2 migration)

**From:** ForskScope (a local-first cross-platform diff/merge workstation)
**Context:** ForskScope consumes `sheets-diff` through a single adapter module
(`xlsx.rs`) that maps your `WorkbookDiff` into our own app-owned model. We are
migrating from v1.1.4 to v2.2.0. The v2 rewrite is excellent — typed values,
`Result`-based errors, cancellation, and the `output::view` module all directly
improve our integration. Before we commit the migration, a few grounded
questions and feature requests.

---

## Questions

### Q1. `CellDiff` with both a value change and a formula change — one row or two?

In v2, a single `CellDiff` at one address can carry **both** `value: Some(...)`
and `formula: Some(...)`. Our current (v1-derived) model emits a separate
`CellChange` entry per kind (`CellChangeKind::{Value, Formula}`), so a cell
where both changed becomes two rows.

In `output::view`, `CellChangeRow` collapses this into **one** row with
`formula_changed: bool`. That's a reasonable GUI choice, but it means the two
representations disagree on cardinality.

**Question:** Is the intended consumer model "one row per address" (the
`CellChangeRow` shape), with value and formula treated as facets of one change?
Or do you anticipate consumers wanting to split by facet? We want to align our
model with your intended direction rather than preserve a v1 artifact.

### Q2. `CellChangeRow` exposes `formula_changed: bool` but no formula text.

`CellChangeRow` gives `old_display` / `new_display` for the **value**, and a
boolean for whether a formula also changed — but not the old/new formula text
itself. To show "old formula `=A1+B1` → new formula `=A1+B2`" we have to drop
out of the view layer and read `CellDiff.formula.{old,new}.raw` directly.

**Question / FR:** Would you consider adding `old_formula: Option<&str>` /
`new_formula: Option<&str>` (or a small `FormulaDisplay` sub-struct) to
`CellChangeRow`? It would let GUI consumers render formula changes without
reaching past the view abstraction into the raw model.

### Q3. Ownership story for `output::view`.

`DiffView::rows()` returns `Vec<CellChangeRow<'a>>` borrowing from the
`WorkbookDiff`. Our adapter needs **owned** data because our `SpreadsheetDiff`
outlives the upstream `WorkbookDiff` (we drop the `WorkbookDiff` at the adapter
boundary so no `sheets-diff` types escape into our app).

Today we will map each `CellChangeRow` into our owned struct (cloning the
display strings), which is fine. But an owned variant would be convenient.

**Question / FR:** Is an owned row type (e.g. `OwnedCellChangeRow` with
`String` fields, or `CellChangeRow::to_owned()`) something you'd consider? Not
a blocker — we can clone at the boundary — but it would remove a small amount
of adapter boilerplate for consumers with the same lifetime constraint.

### Q4. `CellChangeKind` (Added/Removed/Modified) vs our value/formula split.

Your `CellChangeKind` is `{ Added, Removed, Modified }` — a content-presence
classification. Ours is `{ Value, Formula }` — a facet classification. They're
orthogonal. We'll keep our facet enum and additionally adopt your
presence-classification via `CellDiff::change_kind()`.

**Question:** Is `change_kind()`'s derivation (Added = all sub-changes have
`old == None`/empty; Removed = all have `new == None`/empty; else Modified)
considered stable API? We'd like to depend on it rather than re-deriving.

### Q5. `compare_paths` and non-UTF-8 / unusual paths.

`compare_paths(impl AsRef<Path>, ...)`. ForskScope runs on Linux, Windows, and
macOS and must handle paths that are not valid UTF-8 (especially on Linux).

**Question:** `compare_paths` takes `AsRef<Path>`, so we believe arbitrary
`Path` values (including non-UTF-8) flow through to calamine unchanged — is that
correct, i.e. no internal `path.to_str().unwrap()` that would panic or reject a
non-UTF-8 path? Our no-panic contract requires confidence here.

---

## Feature requests

### FR1. A `Cancellation` adapter example for `Arc<AtomicBool>`.

Your `Cancellation` trait has a blanket impl for `Fn() -> bool + Send + Sync`,
which is great — our `CancellationToken` (an `Arc<AtomicBool>`) maps in one
closure:

```rust
let tok = token.clone();
DiffOptionsBuilder::new().cancellation(move || tok.is_cancelled()).build()?
```

**Request:** A one-line example of exactly this pattern in the `Cancellation`
rustdoc would help adopters — `Arc<AtomicBool>` cancellation is the single most
common case and isn't shown.

### FR2. Document the cancellation latency / granularity.

We'd like to surface a responsive "Cancel" button for large-workbook diffs.

**Request:** A sentence in the docs on how often `is_cancelled()` is polled
(per sheet? per N cells? per row?) would let us set user expectations about
cancel responsiveness on a very large single sheet.

### FR3. Stable `code()` strings for `DiagnosticKind` — confirm the contract.

`DiagnosticKind::code()` returns a stable `&'static str` and the doc says "never
renamed within a major version." We intend to match on these codes (e.g.
`"unsupported_workbook_feature"`) to drive our own UI messaging rather than
matching the `#[non_exhaustive]` enum variants directly.

**Request:** Please confirm `code()` strings are intended as the stable
programmatic surface for diagnostics (so we build on them with confidence), and
consider listing the full set in one doc table.

### FR4. `WorkbookDiff` → owned summary without retaining the whole diff.

Our adapter wants `DiffSummary` (counts) and the sheet-change list, but for very
large diffs we don't want to retain every `CellDiff` after we've mapped what we
need.

**Request:** This is already easy (we just take `wb.summary` by clone and drop
the rest), so no API change needed — but a documented note that `DiffSummary`,
`DiffMetrics`, and the `SheetChange` list are cheap to extract and the bulky
`cell_diffs` can be dropped would reassure memory-conscious consumers.

---

## Not asking for

To be clear about scope — these are explicitly **not** requests, because they'd
push `sheets-diff` outside its lane or duplicate what we own:

- Merge/write capability (we own the merge model; you are read-only diff — correct).
- A GUI framework binding (your framework-neutral `output::view` is the right boundary).
- Formula evaluation (out of scope for a diff engine).
- Style/format diffs beyond what calamine 0.35 can expose (your `FormatCompareMode`
  honesty about this is appreciated).

---

## Summary

The v2 rewrite is a clear improvement for us. The migration is a one-file change
on our side thanks to the clean public API. The questions above (especially Q1
on row cardinality and Q2 on formula text in the view) are the only points where
our mapping has a genuine design choice to make, and we'd rather align with your
intended direction than encode our own assumption.
