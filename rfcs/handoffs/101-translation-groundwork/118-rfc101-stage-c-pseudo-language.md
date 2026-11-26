# Handoff 118 — RFC-101 stage C: the pseudo-language, and its first triage

**From:** architect. **Date:** 2026-11-26. **Release:** `0.202.0`, first.
**RFC:** `rfcs/accepted/101-translation-groundwork.md` §4 and *Stages*.
The owner scoped it on 2026-11-26.

**Runs alongside handoff 119.** Handoff 120 (screenshots) waits for
both.

## Rules

- **The register is mine.** Findings go in the reply; I register them.
- **An allowance or expected failure is my decision.** Propose it; don't
  apply it.
- **The probe's seed guard (`seed_not_applied`) stays on.** A view whose
  state didn't load fails.

## Evidence plan first

- **unit tests** for the generator;
- **the layout gate** for everything visual;
- **one capture,** of the worst case, looked at in full.

**Send the plan as the first section of your reply.**

## 1. The pseudo-language

- **Generated from the English catalog at build or test time,** never
  stored and never offered in a release:
  - every Latin letter accented (`Séttîñgš`);
  - each message stretched by 40 %, with the padding inside the
    brackets;
  - bracketed (`⟦…⟧`), so a cut-off or untranslated string shows;
  - **placeholders left intact,** so `{ $count }` still substitutes.
- **Behind the `layout-probe` feature,** with the release-marker check
  unchanged. **Show the marker absent** from a release build.
- **Plural forms:** the pseudo-language uses English's rule.
- **Unit tests:**
  - every catalog id has a pseudo form;
  - placeholders are preserved;
  - the stretch is 40 %, ±1 character;
  - brackets wrap everything.

## 2. It joins the layout matrix

**A third language on the layout row:** 3 windows × 5 text sizes × 3
languages × the views. Measure the added run time. It was about 30
seconds more in RFC-101's estimate.

## 3. The first triage, first

**Before fixing anything,** one table, script-counted, summing to the
total:

| Check | View | Element | Worst combination | Numbers | Your call |
|---|---|---|---|---|---|

**Your call** is one of:
- **a real defect, small:** what the fix would be;
- **a real defect, larger:** a one-line description, for me to register;
- **a checker issue:** what's wrong with the check.

**Send the table as its own reply**
(`.git-exclude/review-request/206a-pseudo-language-triage.md`) and
**stop.** I answer it, with the owner where needed. Then you fix what's
decided.

## 4. After the decision

- **Small real defects are fixed in this release.**
- **Larger ones go on the expected-failure list** with their F-numbers,
  named, so the gate stays hard.
- **Checker issues are fixed at their cause,** each with its `--inject`
  re-run.

## Gates

**The standard set,** including:
- `cargo clippy --manifest-path xtask/Cargo.toml`;
- `i18n`;
- the layout gate green on CI.

Final reply in `.git-exclude/review-request/206-pseudo-language.md`.

## Round 2 (2026-11-26, review 245)

1. **The generator:** spread the 40 % as separate, breakable filler
   words, with no run longer than about 6 characters. Tests for that.
2. **Re-run and re-triage.** Reply in
   `.git-exclude/review-request/206b-pseudo-language-triage.md`, then
   stop.
3. **The *Use* column** sizes to its content, keeping its words. It goes
   with whatever the re-triage leaves.

**Measure on a scratch branch or locally.** `main`'s gate stays green.

## Round 3 (2026-11-26, review 246)

**Fix all 95 remaining findings in this release:**
- the *Use* column;
- the tab title;
- Settings `div.field`;
- the shortcuts table;
- the status bar;
- the Deep Compare cell and row.

**Land them in one push** with `"pseudo"` back in `DEFAULT_LANGUAGES`
(`07940ed` holds it out). Show the full matrix with 0 failures, English
and Japanese unchanged, and the captures.

**Unpushed work stays on a local branch, never on `main`,** in this
shared checkout.

Final reply in `.git-exclude/review-request/206-pseudo-language.md`.

## Round 4 (2026-11-26, review 247)

**Approved, apart from two small changes:**
1. **A test that walks every `SelectField` option label in every real
   catalog** and fails over 42 characters, so a real language is never
   cut. It replaces the hand-picked test, and is falsified.
2. **One space between the generator's padding and the message,** then
   `.kb-desc` without `overflow-wrap: anywhere`, using `break-word` only
   if still needed.

Reply in `.git-exclude/review-request/206c-pseudo-language.md`.
**Handoff 120 starts after that.**

## Closed (2026-11-26, review 248)

**Complete.** RFC-101 stage C is done.
