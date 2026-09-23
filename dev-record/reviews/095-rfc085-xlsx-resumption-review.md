# Review 095 — Request 092: RFC-085 spreadsheet comparison resumption

**Reviewer:** architect. **Date:** 2026-09-04. **Reviewed:** `d492557`.
**Verdict:** **Approved. F65 closed, RFC-085 moves to `done/`.** No follow-up.

## 1. Your scope call was right, and my handoff was wrong

Handoff §6 listed *"any UI work"* as out of scope. You went outside it, disclosed
it, and argued it was forced. **It was, and the handoff would have produced dead
code without it.** Verified against the parent commit:

```rust
// d492557^:state/compare.rs:469
if ld.kind == FileKind::ExcelXlsx || rd.kind == FileKind::ExcelXlsx {
    return Err(t(lang, "Spreadsheet comparison is temporarily disabled for security."));
}
```

**`||`, with an unconditional refusal, before anything in `xlsx.rs` is
reached.** Restoring `diff_xlsx` alone would have left every `.xlsx` pair
refused at that line — the functions would have compiled, been tested, and never
executed.

That is **exactly** this register's most-repeated pattern: *builds the right
abstraction, tests it, documents it, then does not connect it* — F52, F75, F84,
F88a. My handoff would have caused the fifth instance, in the handoff written by
the person who keeps recording it. Your `&&` restoration is byte-identical to
pre-suspension; I diffed it.

The `diff.rs` notice string is the same judgement and equally right. Leaving
*"Spreadsheet comparison is temporarily disabled for security."* on screen while
the feature worked would have been F92 pointed the other way — a document
denying a control the product has.

## 2. Two corrections to my handoff, both yours

**`load_placeholder` was not byte-identical**, contrary to §1. You checked with a
real diff instead of accepting it: `LoadWarning::ExcelComparisonDisabled` had
been renamed during RFC-058 from `ExcelRenderedAsDerivedText`. Renaming it back
is right, and your reasoning is better than the original name — it now describes
why `text` is `None` *regardless* of whether comparison is enabled, so it cannot
go stale the next time this toggles.

**`docs/src/users/file-types.md` does not exist**; it is under `intermediate/`.
Mine.

## 3. The §3 decisions

**`Unchanged` dropped** — falsified by making the arm push `Modified`, correct
failure observed.

**`RenamedAndMoved` → `Renamed`** — and you established the trigger condition
from `sheets-diff`'s own `matcher.rs` (`conservative_rename`: one unmatched-by-name
pair with `old.index != new.index`) **before building the fixture**, rather than
by trial and error. The `Anchor` sheet in that fixture independently confirms
plain `Moved` still works, so the test cannot pass by the variant never firing.
The reasoning — emitting a second `Moved` would change the sheet count for one
sheet's worth of change — is sound, and it matches the pre-suspension choice.

## 4. Falsifications, reproduced here

I neutered the cancellation wiring (`build_options` ignoring the token):

```
cancellation_interrupts_a_comparison_that_crosses_the_checkpoint ... FAILED
an_uncancelled_large_comparison_still_completes ................. ok
```

One failure, precisely targeted — the companion test correctly still passes,
which is what proves it is not a blanket assertion. Full suite: **1213 passed,
0 failed.**

I also falsified `audit-deps` myself, since it now gates a real dependency for
the first time since July: a wrong allowed-dependent gives **exit 1**, restored
gives **exit 0**.

## 5. The gates, and the trap you avoided

You replaced `assert_package_absent` — *a gate that passed because the dependency
did not exist* — with real dependent checks. That was the central risk in §5 of
the handoff and it is properly closed.

**The `quick-xml` ambiguity is the part I would have got wrong.** Two majors now
coexist (`0.39.4` via `wayland-scanner`, `0.41.0` via `calamine`), and the old
check would have hard-failed on the ambiguity itself before evaluating either
path. Generalising to resolve every version present, and checking each
independently, is the correct fix rather than special-casing.

**And you caught a trap I did not think of:** adding `rust_xlsxwriter` as a
dev-dependency would have given `zip` a second immediate dependent and broken
`audit-deps` — conflating a never-shipped fixture tool with the runtime parsing
path the gate exists to police. Committing real workbooks instead is right, and
`#[non_exhaustive]` on the upstream types means there was no alternative anyway.

## 6. The register edit

Correct, and correctly bounded. The top-of-file bullet asserted in present tense
that XLSX comparison *"fails closed"* — false the moment this shipped, and
leaving it would have been a false claim in the register that catalogues false
claims. **You left F65's own row untouched**, reading that closing a row with a
verdict is the architect's step. That is exactly the line, and you found it
without being told.

## 7. Verified

`cargo audit` exit 0 measuring a real dependency rather than its absence, both
locally and in CI's separate advisory workflow. Read-only held under a real save
attempt with bytes confirmed unchanged — not merely asserted through
`save_capability`. `build_side_text`, `SpreadsheetDiff`'s shape,
`EditabilityClass::ReadOnly` and both `save_capability` block sites untouched, as
scoped.

**F65 is closed after five weeks, four of which were mine.** RFC-085 moves to
`done/`.
