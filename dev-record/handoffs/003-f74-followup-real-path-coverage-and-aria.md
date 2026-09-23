# Developer Handoff 003 — F74 follow-up: real-path test coverage and accessible status labels

**From:** architect
**Date:** 2026-08-18
**Supersedes nothing.** Continues handoff 002.
**Governing review:** `dev-record/reviews/072-f74-explorer-directory-status-review.md`
**Register:** F74 (returned, not resolved). F75 and F76 are **out of scope** — see §6.

---

## 1. Task title

Make the F74 fix testable against the real code path, and deliver the accessible
status label in a form a screen reader actually announces.

## 2. Purpose

`16c35f1` fixed F74 correctly. The fix is not being redone. This handoff closes
two gaps that leave the fix unprotected and one of its three requirements
undelivered in substance.

## 3. Background

Review 072 probed the new tests rather than reading them, and found:

- **Restoring the original F74 defect at the call site leaves all 61 tests
  passing.** With `dir_common_state` left correct and untouched, putting
  `let state = if cp.is_dir() { DigestState::Equal } else { DigestState::Unique };`
  back into `explorer.rs` — the exact code from `9f355c6` — changes no test
  outcome.
- **Removing `title: "{st_label}"` from the status span leaves all 61 tests
  passing and `clippy --workspace --all-targets -- -D warnings` clean.**

Both demonstrations in review request 070 §2 broke the *helper the fix
introduced*. The reported defect was at the call site and in the markup, and
neither is covered. The `filter.rs` check (§2.2) is genuinely falsifiable and
needs no change.

**On the label specifically, the scope error is the architect's, not yours.**
Handoff 002 §2.3 required an accessible label while §3 stated no RFC-078 harness
work was needed. That was correct for the directory verdict and wrong for the
label: nothing reachable from a unit test can confirm an attribute reached the
DOM. You implemented the instruction as written.

## 4. Applicable RFC and requirements

- **RFC-009 §7 "Diff Accessibility Rules"** lists four requirements. **"Screen-reader
  text" is a separate line from "Symbol or label."** A glyph plus `title` supplies
  the symbol and does not reliably supply the screen-reader text.
- **RFC-078** — no new evidence rows or harness work in this handoff. See §14 for
  where the runtime verification belongs instead.

## 5. Change scope

- `crates/forskscope-ui/src/ui/view/explorer.rs` — extract the per-entry
  classification; add a test driving it against real directories.
- `crates/forskscope-ui/src/ui/view/dir_pane.rs` — `role`/`aria_label` on the
  status span and on the `bin` badge.

## 6. Explicit non-change scope

Do **not** touch any of the following. Each is registered and deliberately
sequenced elsewhere.

- **F75** — do not wire `RowStatusKind`, do not delete `DigestState`. Unchanged
  from handoff 002 §5.
- **F76** — the type-mismatch case (directory one side, same-named file the
  other, currently `Unique` and labelled "Only on this side" while existing on
  both sides). **Do not add a `TypeMismatch` state.** It arrives with F75's
  wiring, where core's `EqualityEvidence::TypeMismatch` comes across for free.
- **The `NotCompared` glyph.** Your Q1 was well founded — `–` and `·` share
  `var(--muted)`, and `NotCompared` is now the most frequent glyph in the pane —
  but the glyph set should be redesigned once, against all five states, with F76.
  Leave `–` alone for now.
- **No subtree recursion or digesting.** Unchanged from handoff 002 §2.
- **Do not add `dioxus-ssr`** or any other rendering dependency. See §14.

## 7. Required implementation

### 7a. Extract the classification so a test can drive the real path

The digest block's per-entry decision currently lives inside the `use_effect`
closure, where no test can reach it. Extract it — a function taking the entry's
relative path, its `is_dir`, and both roots, returning the `DigestState` to
insert — and reduce the closure to iterating entries and inserting results.

The exact signature is yours. The requirement is behavioural: **a test must be
able to reach the decision that `9f355c6` got wrong**, without constructing a
`VirtualDom` and without a renderer.

Keep `dir_common_state`. It is not wrong; it is simply not where the bug was.

### 7b. Deliver the label as screen-reader text

Replace `title` as the label mechanism on the status span with **`role: "img"`
plus `aria_label: "{st_label}"`**. `role="img"` makes the span an exposed node
whose author-supplied name replaces its text content, which is what substitutes
the label for the glyph. A bare `span` is role `generic`, which browsers largely
do not expose as a named node, so the row announces the glyph character instead.

Keep `title` as well if you want the mouse tooltip — it is not harmful, it is
just not the accessible name.

**Apply the same repair to the `bin` badge** three lines above. It has the
identical defect and predates F74; fixing it here avoids leaving two adjacent
spans with two different conventions.

`aria_label` is already the house convention — eight occurrences across
`ui/overlay/modals/` and `explorer.rs:421`. Follow those.

## 8. Required tests

**The standard is unchanged and it is the point of this handoff: each check must
be demonstrated failing against the defect it exists to catch — the *reported*
defect, not a helper introduced by the fix.**

1. **Real-path directory classification.** Build two real directories with the
   same name and differing contents, run the extracted classification, assert the
   result is `NotCompared` and not `Equal`. `forskscope-ui` tests already create
   real temp directories — follow `state/compare/tests.rs:165`'s `temp_dir(tag)`
   pattern. **No new dependency; do not add `tempfile` to `forskscope-ui`.**

   **Falsification that counts:** put the original
   `if cp.is_dir() { DigestState::Equal }` back at the call site, confirm this
   test fails, restore. Report that specific result.

2. **Existing tests stay.** The five from `16c35f1` are not to be removed. They
   are correct about what they cover; they simply do not cover this.

The `aria_label` change is **not** unit-testable in this codebase and you should
not attempt to make it so — see §14.

## 9. Required documentation updates

None. `ROADMAP.md`'s F74 entry already records both gaps and the architect's
scope correction; I will update it on review. **Do not edit `ROADMAP.md`.**

## 10. Acceptance criteria

- Reintroducing `if cp.is_dir() { DigestState::Equal }` at the call site **fails
  at least one test.** This is the acceptance criterion; everything else is
  supporting work.
- The status span and the `bin` badge both carry `role="img"` and a localised
  `aria_label`.
- All existing tests still pass; `forskscope-ui`'s count rises by the tests added.
- Gates green: `cargo fmt --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo xtask css --check`,
  `cargo xtask i18n`, `git diff --check`.

## 11. Prohibited shortcuts

- **Do not satisfy §8.1 by testing a helper.** A test that passes with the
  original call-site defect restored does not meet this handoff, however
  well-named.
- **Do not assert on a string that never reaches the DOM.** If a check cannot
  fail when the attribute is absent, do not describe it as covering the
  attribute.
- **Do not report a falsification you did not run.** State the observed failure
  output, as request 070 did — that part of 070's reporting was good and is worth
  keeping.

## 12. Relevant code or module boundaries

- The classification must remain GTK-free and renderer-free — it is plain
  filesystem predicates over paths, callable from a unit test.
- Nothing moves to `forskscope-ui-logic` in this handoff. That is F75's change,
  and moving it early would collide with it.

## 13. Compatibility and security constraints

None. No public API, no persistence format, no file writes, no new dependency.
The classification performs the same `is_dir`/`is_file` probes it already does —
**no additional filesystem traversal**, which is the constraint that keeps this
out of F76's and Deep Compare's territory.

## 14. Known risks

- **The `aria_label` change ships unverified by tests, and that is accepted and
  recorded.** The correct instrument is P07's platform accessibility-tree query
  (AT-SPI on Linux, UIA on Windows), which RFC-078's harnesses already perform —
  the status label appearing as accessible text is a runtime assertion, not a
  string comparison. It will be added when the Explorer's Windows rows next run.
  Until then F74's register entry records the DOM wiring as unverified rather
  than leaving it implied. **This is why §6 forbids `dioxus-ssr`:** it would buy
  a weaker check than the one already available, plus a dependency.
- **Extraction risk.** Pulling logic out of a `use_effect` closure can change
  when signals are read. Keep signal reads in the closure and pass plain values
  into the extracted function.

## 15. Required evidence

- The observed failure output for §8.1's falsification, quoted.
- Gate results per §10.

## 16. Required review-request format

As in request 070, which was well structured. One addition, and it is the first
thing I will look for:

> **State plainly whether reintroducing `if cp.is_dir() { DigestState::Equal }`
> at the call site now fails a test, and paste the failure.**

If it does not, say so rather than reframing the check — that answer is more
useful than a fix, and request 070's §8 showed you will report against interest,
which is why this handoff is short.
