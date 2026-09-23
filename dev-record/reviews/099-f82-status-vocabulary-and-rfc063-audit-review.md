# Review 099 — Request 096: F82 vocabulary, RFC-063 audit

**Reviewer:** architect. **Date:** 2026-09-08. **Reviewed:** `1a44a2a`.
**Verdict:** **Part A approved — F82 closed.** **Part B accepted as delivered**,
and it found two real defects, not just unbuilt polish. Registered as **F99**.

## Part A

### 1. The F75 guard is real, verified here

I reverted `status_glyph` to a local hardcoded match — the exact pre-change
shape:

```
status_glyph_matches_the_shared_vocabulary_for_every_status ... FAILED
deep_compare_adopts_the_target_glyphs_not_its_old_ones ...... FAILED
```

Two failures. **The table is genuinely wired twice**, which is the whole point:
a shared table in `ui-logic` wired by one view would have been F75's tenth
module, and this handoff's central risk is closed rather than assumed.

Full suite **1247 passed, 0 failed**. Both enums untouched — `RecStatus` is in
`forskscope-core`, which this commit does not touch at all, and
`RowStatusKind`'s variants are unchanged (the deletions are the old `glyph()`
body being replaced by delegation).

### 2. `StatusGlyph` as a third type was the right shape

Neither enum absorbed the other, and the two single-view concepts
(`NotCompared`, `Symlink`) still route through the shared table rather than
staying local — so there is exactly one definition of every glyph, including the
ones only one view can produce. That last detail is what makes it a vocabulary
rather than an overlap table.

### 3. The CSS decision you made beyond the handoff

§2 specified the `Unreadable` swap and said LeftOnly/RightOnly already agree —
**on glyphs**. It did not notice that Deep Compare's `status-only` and the
Explorer's `status-left-only`/`status-right-only` were never the same value.

Taking the Explorer's more specific pair is right, and your reasoning is the
part worth keeping: the rules were already visually identical, so it costs
nothing now, and **merging two classes into one is information you cannot get
back later**. Asymmetric costs, chosen in the cheap direction.

Finding and removing the orphaned `.status-err` rule — left from F79's
rename — is the kind of thing only someone actually reading the file finds.

### 4. `css_coverage` extended

Asserting `main.css` defines all eight classes means a typo or a forgotten CSS
regeneration fails a test instead of shipping an unstyled status silently. Not
asked for; correct.

## Part B — the audit, and what it found

You followed *report and stop* exactly: no code changed, every verdict carrying
file:line, and the two items I had already spot-checked listed rather than
re-derived.

**Verdicts: 4 shipped, 5 partly shipped, 1 correctly-nothing-built (C8).**

**Two of the partials are defects rather than absent polish, and I verified
both:**

- **C10** — `overlay/modals/recovery.rs:142` and `:252` call
  `store.notify(e.to_string())`. Raw OS error strings, **in the recovery
  dialogs** — the place a user is already in trouble and least able to act on
  `No such file or directory (os error 2)`. This is precisely the pattern C10
  exists to eliminate, surviving in the worst location.
- **C3** — `dir_pane.rs:159-168`'s PathBar buttons carry `title` but their
  content is a bare glyph (`←`, `→`). A screen reader announces the character,
  not the action. That is **F74/F80's defect class in the one view neither
  touched**, and the downscope's own stated condition ("an accessible label")
  is unmet.

The rest — C1's first-run persistence, C6's missing plain-language tier, C7's
narrow-layout variant — are genuinely unbuilt polish, and cheaper to leave
recorded than to build unasked.

**Registered as F99** with the per-item detail. Scheduling is mine; you were
right to stop.

**One thing you noticed and correctly did not fix:** the stale doc comments at
`diff_actions.rs:321` and `file.rs:268` describing the old raw-string arm as
"established" when it has been migrated. A comment edit is still a code change
in an audit. I will fold that into F99's work.

**F82 closed.**
