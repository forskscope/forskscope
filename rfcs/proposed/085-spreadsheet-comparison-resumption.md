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

**Q1 — model alignment. ASKED AND ANSWERED: no (2026-09-04). `convert()`
stays, and their reasoning is better than a yes would have been.**

Everything this RFC listed, their model already provides — *the gap is shape,
not capability*:

| We need | They already have |
|---|---|
| Sheet change kinds | `SheetChange`: `Added`, `Removed`, `Renamed`, `Moved`, `Modified`, **plus `Unchanged` and `RenamedAndMoved`** |
| 1-based addressing | `CellAddress` — 1-based `row`/`col` plus the `a1` label, bounded to Excel's limits |
| Separate value/formula flags | `CellDiff.value` and `CellDiff.formula` are each `Option`; `is_some()` **is** the flag, and they move independently |
| Old/new strings for both | `ValueChange` and `FormulaChange` each carry `old`/`new`; `CellValue::display_string()` renders either side |

Their argument for declining: the part of `convert()` that would disappear is
the part mapping their names onto ours — **which is the insulation this RFC says
the adapter exists to provide**. Shipping a type in our shape would replace a
boundary we control with a dependency on a second type of theirs, coupling us to
their naming exactly where we deliberately decoupled. Accepted without
reservation.

**Two consequences for implementation, from their answer rather than guesswork:**

1. `convert()` must handle **`Unchanged` and `RenamedAndMoved`**, which our
   `SheetChange` does not model. The pre-suspension code had
   `#[allow(unreachable_patterns)] _ => {}` for forward compatibility; those two
   are now known, not hypothetical, and must be decided rather than swallowed.
2. The traversal is already written: *"Flattening v2 output into a v1-style
   list"* in their migration guide, **compiled and executed as a doctest in
   their CI** since 2.4.1. Start there rather than deriving it.

**Q2 — the text projection stays here.** `build_side_text`'s `+`/`-`/`~`
prefixes exist so *our* diff engine aligns the two sides, and `(empty)`/`(none)`
are our wording. It is a presentation choice tuned to one application's UI.
Proposing it upstream would couple a general-purpose library to our view.
**Recommended: do not ask.**

**Q3 — the correction we owed them. SENT, and the outcome was benign
(2026-09-04).** They had **not** deprioritised anything: the owner's instruction
had been to ship cancellation with the milestone rather than cut a release for
it, so the fix was already in 2.5.0. Their cancellation tests drive
`compare_bytes_with_options` — our entry point — and each was demonstrated
failing before the fix, not merely passing after.

**One disclosure of theirs belongs in this RFC, because it is this project's own
catalogued pattern arriving from outside.** Cancellation had been described in
their doc comment and an RFC status as a **latency** limitation for four
releases. It was not latency: on a single-sheet workbook there was no second
checkpoint, so a cancel request was never observed at all. In their words —
*"nobody noticed because the wrong word was in the record and everyone believed
the record."* That is F92's shape exactly, and it is the reason this RFC targets
2.5.0 rather than trusting a version's description of itself.

## Open questions

**None.** Q1 was asked and answered *no*; Q3 was sent and the outcome was benign.
Nothing in this RFC waits on the sheets-diff team, and they have confirmed they
have no open questions for us. They ask only to be told what breaks.
