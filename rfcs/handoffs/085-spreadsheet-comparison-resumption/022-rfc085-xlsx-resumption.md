# Handoff 022 — RFC-085: restore .xlsx comparison on sheets-diff 2.5.0

**From:** architect. **RFC:** `rfcs/accepted/085-spreadsheet-comparison-resumption.md`
(accepted by the owner 2026-09-04 — read it first; it carries the reasoning this
handoff assumes).
**Priority:** not release-blocking. Gate D has no in-project blocker.

## 1. This is a restoration, not a build

The July suspension removed **exactly two functions** — `build_options` and
`convert` — plus the body of `diff_xlsx`. Everything else is still in the tree
and byte-identical: `SpreadsheetDiff`, `SheetChange`, `CellChange`,
`build_side_text`, `derive_pair_text`, `load_placeholder`, and `diff_xlsx`'s
signature including its (currently unused) `_cancel` parameter.

**The prior implementation is in git**: `git show 1156b86^:crates/forskscope-core/src/xlsx.rs`.
It targeted an older sheets-diff, so treat it as a reference for *shape*, not a
patch to re-apply.

Add `sheets-diff = "2.5.0"` to `forskscope-core`.

## 2. What the upstream API actually looks like

**From the sheets-diff team directly (2026-09-04), not from reading their docs
second-hand.** They declined to reshape their model to ours, correctly — see
RFC-085 Q1 — so `convert()` stays and this is what it maps from:

| Ours | Theirs |
|---|---|
| `SheetChange` | `SheetChange`: `Added`, `Removed`, `Renamed`, `Moved`, `Modified`, **`Unchanged`**, **`RenamedAndMoved`** |
| `CellChange.addr` / `.row` / `.col` | `CellAddress` — 1-based `row`/`col` plus the `a1` label |
| `.value_changed` / `.formula_changed` | `CellDiff.value` / `CellDiff.formula`, each `Option` — **`is_some()` *is* the flag**, and they move independently |
| `.old_value` / `.new_value` / `.old_formula` / `.new_formula` | `ValueChange` / `FormulaChange`, each carrying `old` / `new`; `CellValue::display_string()` renders either side |

**Start from their migration guide**, section *"Flattening v2 output into a
v1-style list"* — it is **compiled and executed as a doctest in their CI** since
2.4.1, so it is known-good traversal code rather than prose.

## 3. The one decision this handoff does not make for you

**`Unchanged` and `RenamedAndMoved` have no counterpart in our `SheetChange`.**

The pre-suspension code ended its match with
`#[allow(unreachable_patterns)] _ => {}` for forward compatibility. Those two
variants are now **known**, not hypothetical — so silently swallowing them
would be choosing a behaviour without recording it.

Decide and say which:

- **`Unchanged`** — most likely dropped, since `build_side_text` renders every
  `SheetChange` and an unchanged sheet would add noise to both panes. But check
  what `Modified` already implies before assuming.
- **`RenamedAndMoved`** — carries information both our `Renamed` and `Moved`
  carry separately. Collapsing it to `Renamed` loses the move; emitting both
  changes the sheet count. Neither is obviously right.

**If either decision changes what a user sees, say so in the review request as a
behaviour change** — the way you disclosed the search/path-input swallow in
handoff 020.

## 4. Cancellation is the reason for 2.5.0, so prove it works

`diff_xlsx` takes `_cancel: Option<&CancellationToken>`. Restore it to a real
parameter and thread it into `build_options`.

2.5.0 is the first release polling cancellation **inside** a sheet — every
50,000 cells across the read and compare phases — rather than only between
sheets. Before it, a single large sheet was uninterruptible.

**This matters more here than the feature does.** F77, F78 and F79 were all
uninterruptible-or-stale-result defects; `DigestEpoch` exists because of them.
Adding a new uninterruptible path in that exact area would be the worst possible
place to be careless.

**Falsification:** a test that cancels mid-comparison on a workbook large enough
to cross a polling interval, demonstrated **failing** with the token ignored.
Their own tests do this against `compare_bytes_with_options`, so the shape is
known to be testable.

## 5. Two gates that will pass for the wrong reason

**Nothing will fail to remind you of either. That is why they are here.**

1. **`cargo audit` / `cargo xtask audit-deps` are green today because the
   dependency is absent** — not because it passes. Re-adding it means
   `audit-deps`' reviewed network-capable-path set must gain
   `sheets-diff → calamine → quick-xml → zip` **deliberately, in this change**.
   F65 records that reporting otherwise would be the credited-with-more-than-it-
   measures pattern.
2. **RFC-078 P10 — "Binary/XLSX fail-closed policy"** currently asserts the
   *disabled* behaviour. Update it with the feature, or the platform evidence
   starts certifying something the product no longer does.

Also update `docs/src/users/file-types.md`, whose `.xlsx` row states the current
read-only-and-not-compared behaviour. **Comparison returns; read-only does not
change** — do not let the doc drift the other way.

## 6. Scope

**In:** `xlsx.rs`'s two functions and `diff_xlsx`'s body, the dependency,
`audit-deps`' reviewed set, P10, `file-types.md`, tests.

**Out:** `build_side_text` (its format is settled — do not retune it),
`SpreadsheetDiff`'s shape, any UI work, and **merge/save for `.xlsx`**.
`FileKind::ExcelXlsx → EditabilityClass::ReadOnly` and `save_capability`'s two
block sites stay exactly as they are. Making spreadsheets mergeable would touch
every write path B5 just repaired; it is explicitly a separate RFC.

## 7. Gates

The usual set. Note `cargo audit` now measures something real for the first time
since July — if it reports anything, stop and tell me rather than dispositioning
it yourself.
