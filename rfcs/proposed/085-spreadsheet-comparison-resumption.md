# RFC 085: Spreadsheet (.xlsx) Comparison Resumption

**Status.** Proposed
**Scheduling.** Ready to start on acceptance — the owner directed the dependency be added (2026-09-02). See `ROADMAP.md` § "Remaining proposed RFCs", which must list every file in `proposed/` and `accepted/` and nothing else (F83).
**Tracks.** Lifting RFC-058's security suspension; restoring structural `.xlsx`
comparison; gate coverage for a returning dependency.
**Touches.** `crates/forskscope-core/src/xlsx.rs`, `forskscope-core/Cargo.toml`,
`xtask`'s `audit-deps` reviewed set, RFC-078 P10, `docs/src/users/file-types.md`.

## Summary

Restore structural `.xlsx` comparison by re-adding `sheets-diff` at **2.5.0**.

This is **not a rebuild**. The suspension removed exactly two functions —
`build_options` and `convert` — and the body of `diff_xlsx`. The domain model,
the text projection, the placeholder loader and the cancellation parameter all
remain in the tree, byte-identical to their pre-suspension form. Roughly 115
lines return behind a seam that was deliberately left standing.

## Why now

RFC-058 suspended the parser path for one reason: `sheets-diff → calamine →
quick-xml 0.39` carried active XML denial-of-service advisories, and `.xlsx`
files are user-supplied archives of untrusted XML. Failing closed was correct.

**That reason is gone**, verified on the current version rather than inherited
from the 2.3.0 check (F65 warns explicitly against inheriting it):

```text
sheets-diff 2.5.0 → calamine 0.36.1 → quick-xml 0.41.0, zip 8.6.0
cargo audit: exit 0, 32 crates scanned, no advisories
MSRV 1.88 (ours: 1.91)
```

**The delay after that point was the architect's, not the owner's.** F65 records
it: a cleared blocker was written as *"decide whether to re-enable"* and filed as
an owner decision for five weeks instead of being recommended.

## Why 2.5.0 specifically, and not 2.4.1

`diff_xlsx` takes a `CancellationToken` — currently `_cancel`, unused. Before
2.5.0 that parameter could not have been honoured meaningfully: cancellation was
checked only *between* sheets, so a single large sheet was uninterruptible.

2.5.0 polls at a 50,000-cell interval inside both the read and compare phases,
with a worst case near 100 ms. A sheet under 50,000 cells is still not
interruptible, and does not need to be — it completes well inside that budget.

This matters beyond convenience. F77, F78 and F79 all concerned work that could
not be cancelled or whose results landed after their inputs changed;
`DigestEpoch` exists because of it. Restoring `.xlsx` on a version whose
cancellation is decorative would add a new uninterruptible path to the one place
this project has repeatedly been bitten.

## Design — unchanged, and already present

```text
sheets_diff::compare_paths_with_options → WorkbookDiff
convert()                               → SpreadsheetDiff     ← our model
build_side_text()                       → (TextDocument, TextDocument)
                                        → the ordinary diff view
```

`SpreadsheetDiff` is deliberately **ours**, not a re-export: upstream model
churn stops at `convert()` instead of propagating. `SheetChange` covers
Added/Removed/Renamed/Moved/Modified; `CellChange` carries the address, 1-based
row/col, separate `value_changed`/`formula_changed` flags, and old/new value and
formula strings.

The UI needs **no spreadsheet view**. The pair projects to two text documents
rendered by the existing diff engine, inheriting scrolling, search and hunk
navigation.

## Scope boundaries

- **Read-only, unchanged.** `FileKind::ExcelXlsx → EditabilityClass::ReadOnly`;
  `save_capability` blocks it in two places. Comparison only. **Merge is
  explicitly out of scope** — it would touch every write path B5 just repaired.
- No change to `build_side_text`. Its output format is settled.
- No new UI.

## Two gates that currently pass for the wrong reason

**This is the part most likely to be missed, because nothing will fail.**

1. `cargo audit` and `cargo xtask audit-deps` are green today **because
   `sheets-diff` is absent** — not because it passes. F65 records that reporting
   otherwise would be the *credited-with-more-than-it-measures* pattern. Re-adding
   the dependency means `audit-deps`' reviewed network-capable-path set must gain
   `sheets-diff → calamine → quick-xml → zip` **deliberately, in the same
   change**.
2. **RFC-078 P10 — "Binary/XLSX fail-closed policy"** currently asserts the
   *disabled* behaviour. It must change with the feature, or the platform
   evidence starts certifying something the product no longer does.

## Questions for the sheets-diff team

The crate is this project's own (`github.com/forskscope/sheets-diff-rs`), so
these are proposals to a sibling team, not requests to an upstream we wait on.

**Q1 — model alignment.** Would `sheets-diff` consider exposing a result shaped
like our `SpreadsheetDiff` (sheet-level change kinds plus addressed cell changes
with separate value/formula deltas)? If so, `convert()` largely disappears.
**Recommended: ask.**

**Q2 — the text projection stays here.** `build_side_text`'s `+`/`-`/`~`
prefixes exist so *our* diff engine aligns the two sides, and `(empty)`/`(none)`
are our wording. It is a presentation choice tuned to one application's UI.
Proposing it upstream would couple a general-purpose library to our view.
**Recommended: do not ask.**

**Q3 — a correction we owe them.** The unsent 2.4.1 reply tells them
cancellation is *"not load-bearing, do not reprioritise"*. That was true only
while `.xlsx` was disabled. **This RFC reverses it**, and they should be told,
because they may have deprioritised work on our answer.

## Open questions

None blocking. Q1 and Q3 are correspondence, not preconditions.
