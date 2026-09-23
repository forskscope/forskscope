# Review 077 — Request 075: F75(a) + F76, wiring `RowStatusKind`

**Reviewer:** architect
**Date:** 2026-08-22
**Reviewed:** `163a548`, against baseline `7fa9b43`
**Verdict:** **Approved in substance. F76 is resolved; F75(a) is resolved.**
Two required fixes first (§4), both consequences of the glyph change and both
small — a commit, not a review round.

## 1. Verified independently

`DigestState` has no live references — only past-tense prose in comments.
Workspace tests green (core 697, ui-logic 257, ui 72), `clippy --all-targets
-- -D warnings` clean, `fmt` clean, `css --check` up to date, `i18n` 234 keys
covered. All re-run here.

Doing §7a standalone, and running `status.rs`'s suite green **before** any wiring
existed to retrofit it to, is exactly what §11 asked for and the reason that
ordering was specified. It is also the part most easily skipped without anyone
noticing.

§8.5 mattered most: both of F74's checks still bite after the refactor. That was
the risk this handoff carried, and it is discharged.

## 2. Your correction to my handoff is right, and reported the right way

**`DigestState` was never `Copy`.** I verified it at `7fa9b43`:
`#[derive(Clone, PartialEq, Debug)]`. My §7b framed a trade-off — richer evidence
versus a `Copy` ripple — and **the ripple did not exist**. Every call site already
used `.cloned()`.

Reporting that you checked the premise and it did not hold, rather than
proceeding silently on the corrected version, is what makes the difference
between a handoff being followed and a handoff being *read*. That is the second
time this has caught one of my errors (review 074's `unreachable!` reasoning was
the first).

**Q1 — is storing `EqualityEvidence` still the call?** Yes, and now
unconditionally. The only argument against it was a cost that turns out to be
zero. Storing the evidence keeps the `Error` message, which RFC-080's §4 labels
will want and which nothing can recover once discarded.

## 3. Q3 — `classify_digest_outcome`'s extraction is the right granularity

Yes. It matches the precedent this file already carries (`classify_entry`,
`apply_epoch_result`), and §8.2 goes **further than the standard requires**:
feeding it a real `Err` from `file_digest_equal_with_cancel` against a
nonexistent path, rather than a hand-built `CoreError`, means the test proves the
real call site's output. Review 072 asked for the real call site rather than a
helper; this is that lesson applied without being told.

## 4. Q2 — the glyph change is right, and it broke two things

**Adopting `RowStatusKind`'s glyphs was correct**, and better than my §7c framing
implied. `=` and `!` are RFC-009 §7's own examples; `✓` reads as *verified* and
`⚠` as *warning*, neither of which states a comparison outcome. And you undersold
the real gain: **`←`/`→` distinguish direction where `·` never did.**

But two consequences need fixing before this closes.

### 4a. The spinner now rotates an ellipsis

`.tree-status.status-computing` still carries `animation: spin .8s linear
infinite` (`10-view-explorer.css:47`), written for `⟳`. The glyph is now `…`.
**A rotating `…` reads as a rendering fault, not as progress.** Either drop the
animation for this glyph or keep a rotatable one; the CSS was renamed 1:1 from
`st-computing` without anyone asking whether the rule still fit the character.

### 4b. The glyph now says more than the label

`LeftOnly → ←` and `RightOnly → →` are visually distinct, and
`status_kind_label` announces **"Only on this side"** for both.

You are right that this is not a regression — `·` did not distinguish either. But
it is a **newly opened gap between what a sighted user learns and what a
screen-reader user learns**, created by this change, and RFC-009 §7 exists
precisely to prevent that: it requires colour, symbol *and* screen-reader text to
carry the meaning. The symbol now carries direction; the text does not.

Give them distinct labels — "Only on the left" / "Only on the right" — through
the same `t(lang, …)` path. Two keys.

## 5. One observation, no action

`NotCompared`'s label is *"Directory contents not compared — use Deep Compare"*,
but the kind is now reachable from `EqualityEvidence::Unknown` generally, not only
from directories. Today `classify_entry` only produces `Unknown` for a
directory pair, so the label is accurate. RFC-080 §4 already specifies per-row-kind
wording, which is where this gets resolved. Noted so it is not rediscovered.

## 6. What this closes

**F76 is resolved, both instances**, and by construction rather than by patching:
a type mismatch is `TypeMismatch` rather than a false *"Only on this side"*, and a
failed comparison is `Error` rather than a fabricated `Different`. F76's second
instance previously had **no UI representation at all** — errors were silently
`Different` — so `"Comparison failed"` is the first time this product can say a
comparison failed.

**F75(a) is resolved.** One of the nine unwired modules is wired, and wiring it
found two defects in it (§7a) that its own green tests could not. The remaining
eight modules and the no-allowlist gate are F75(b), a separate handoff.

## 7. What to do

1. §4a — the spin animation.
2. §4b — distinct left/right labels.

Push as one small commit and say so. No review request needed; I will verify by
reading. F80 and F75(b) stay untouched.
