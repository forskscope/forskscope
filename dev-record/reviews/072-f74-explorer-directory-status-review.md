# Review 072 — Request 070: F74, Explorer directory status

**Reviewer:** architect
**Date:** 2026-08-18
**Reviewed:** `16c35f1`, `a94772e`, against baseline `9f355c6`
**Verdict:** **Changes requested.** The fix itself is correct and I am not
asking for it to be redone. Two things must land before F74 closes, and one
new defect is registered.

## 1. What is right, stated first because it is most of the work

- **The state is honest.** `NotCompared` is the right answer, distinct from
  both `Equal` and `Unique`, and `dir_common_state`'s doc comment states the
  reasoning rather than restating the code.
- **The `hide_eq` exemption checks `is_dir` directly** rather than relying on
  `NotCompared` never colliding with `Equal`. §1 gives the reason — a future
  state that misrepresented a directory could not earn hiding — and that is
  the more durable guard. This is the one place the work went beyond the
  instruction, and it went the right way.
- **Labels for every variant, not just the new one.** The task asked for the
  new state; you fixed the whole `match`. Correct call.
- **`.st-not-compared` deliberately not reusing `.st-equal`'s green**, with
  the reason in the CSS comment.
- **§5 and §8 are honest about what was not done**, including flagging the
  absence of runtime evidence as unusual for this program even though the
  task permitted it. That flag is what made me go and check, and it was
  right to raise.

## 2. The falsifiability demonstrations — two of the three do not hold

This is the substantive finding, and I verified it by running the probes
rather than reading the tests.

**§2.1 — the check does not cover the defect that was reported.** I restored
the *original* F74 defect at the call site, exactly as it stood in `9f355c6`:

```rust
let state = if cp.is_dir() { DigestState::Equal } else { DigestState::Unique };
```

leaving `dir_common_state` correct and untouched. **All 61 tests pass.**

The demonstration in §2.1 broke the *helper*. The bug was never in the
helper — it was in the call site, and the helper did not exist until this
commit. So the test proves that a function introduced by the fix behaves as
the fix intends, which is not the same as proving the Explorer no longer
claims directories are equal. Reintroduce F74 tomorrow and the suite stays
green.

**§2.3 — same shape.** I removed `title: "{st_label}"` from the span
entirely, leaving `status_label` intact. **All 61 tests pass, and
`clippy --all-targets -- -D warnings` is clean.** The check asserts the
string function returns a non-empty string; nothing asserts the string
reaches the DOM. The defect was that the label was absent from the markup,
and that exact absence is not detected.

**§2.2 is genuinely falsifiable** — it drives the real `apply_filter` with a
real digest map, so breaking the guard fails it. One of three.

I want to be precise about what this is and is not. The tests are not wrong
and the fix is not wrong; the code is correct in the tree today. What is
wrong is **§2's claim**, which reads as *the three defects are now covered*
when what is covered is one of them. That is this program's recurring
finding arriving in our own work, and it is the reason the standard exists.

**Required:** extract the per-entry classification so a test can drive the
real path — a function taking `(rel, is_dir, l_root, r_root)` and returning
`DigestState`, with the `use_effect` reduced to iterating and inserting.
Then test it against two real directories with the same name and differing
contents. `forskscope-ui` tests already build real temp directories
(`state/compare/tests.rs:165`'s `temp_dir(tag)` helper), so this needs no new
dependency. Break the classification, watch it fail, restore.

## 3. `title` on a `<span>` is not screen-reader text

RFC-009 §7 lists four requirements and **"Screen-reader text" is its own
line**, separate from "Symbol or label". Glyph plus `title` delivers the
symbol and does not reliably deliver the third.

A bare `span` carries role `generic`, which browsers largely do not expose
as a named node; a screen reader reading the row announces the **text
content** — the bare glyph `–` or `✓` — and `title` on a non-focusable
generic element is surfaced inconsistently at best, typically as a
description that default verbosity settings suppress. The user hears "en
dash", which is what F74 was about: a status that does not say what it
means.

**Required:** `role: "img"` plus `aria_label: "{st_label}"` on the status
span. `role="img"` makes it an exposed node whose author-supplied name
replaces its content, which is the pattern that actually substitutes the
label for the glyph. Keep `title` as well if you want the mouse tooltip.

Note this is the house convention already — `aria_label` appears in eight
places across `ui/overlay/modals/` and `explorer.rs:421`. The `bin` badge you
matched is the weaker precedent, and it has the same defect; fix it in the
same change, since it is three lines away and identical in shape.

**My error, recorded so it is not repeated.** The task told you `filter.rs`
and the digest block are plain functions and that no RFC-078 harness work was
needed. That was right about the directory verdict and **wrong about the
label** — I asked for an accessible label without noticing that nothing
reachable from a unit test can confirm one exists. You met the instruction as
written. The scope change in §2 and §3 is mine, not a failure of yours.

**On verifying it for real, and not now:** the right instrument already
exists. RFC-078's harnesses query the platform accessibility tree — AT-SPI on
Linux, UIA on Windows — so the status label appearing as accessible text is a
P07 assertion, not an SSR string comparison. **Do not add `dioxus-ssr` for
this.** Add the assertion to P07 when the Explorer's Windows rows are next
run; until then the register records the DOM wiring as unverified by tests,
which is now written down rather than implied.

## 4. New defect found while reviewing — F76

**A type mismatch is now labelled with a false statement.** Left has a
directory named `X`, right has a *file* named `X`. `dir_common_state(false)`
returns `Unique`, whose new label reads **"Only on this side"** — and the
entry exists on both sides. The mirror case (file left, directory right) hits
`if !cp.is_file()` in the file branch and lands on `Unique` too.

The misclassification predates this commit; before it, the row showed a bare
`·` that asserted nothing much. **Adding labels made the existing wrongness
explicit and audible**, which is not a reason to revert the labels — it is
the labels doing their job by making a silent defect legible.

Your own doc comment reaches for the right word and then settles: *"a file of
the same name - a type mismatch, not a match) - correctly `Unique`"*. It is
not correctly `Unique`. Core already has the vocabulary —
`EqualityEvidence::TypeMismatch { left, right }` (`dir/index.rs:199`).

Registered as **F76**, not folded into F74. **Not required for this fix** —
it is a fifth state, glyph, class and label, and it belongs with F75's wiring
where `TypeMismatch` comes across from core for free. Say nothing about it in
your next request beyond acknowledging it.

## 5. Your review-focus questions

**Q1 — is `–`/`st-not-compared` too close to `·`/`st-unique`?** Your instinct
is right and the reason is sharper than "similar shapes": after this change
`NotCompared` is the status of *every matched directory*, so it becomes the
most frequent glyph in the pane, and it sits at the same `var(--muted)` as
`Unique`. Colour therefore distinguishes nothing between the two most common
non-verdict states. **Keep muted** — muted is correct, neither is a verdict —
and change the glyph to something not confusable with a dot at 12px. Once the
labels are real (§3), the glyph carries less weight, so this is a
low-priority follow-up rather than a blocker. Fold it in with F76, which
needs a fifth glyph anyway and should be designed against the other four at
once rather than one at a time.

**Q2 — is the extraction level right?** Yes. `status_glyph`/`status_label`
split on the `Lang` dependency is the right seam. §2 shows the extraction did
not go *far enough* in `explorer.rs`, not that it went too far.

**Q3 — does `status_label` creep toward F75?** No, and the duplication is
expected: F75's wiring deletes `DigestState` and this function with it. One
thing to note in passing — it now has two label implementations to reconcile
rather than one, and `RowStatusKind` has no `NotCompared` kind, so F75's
scope grew slightly. That is recorded, not a problem you created.

## 6. What to do

1. Extract the classification and test it against real directories (§2).
2. `role="img"` + `aria_label` on the status span, and on the `bin` badge (§3).
3. Re-run the gates. New request when done.

Do **not** touch F75 or F76 in that change.

State in the next request, plainly, whether reintroducing the original
`if cp.is_dir() { Equal }` at the call site now fails a test. That is the
question §2 should have been able to answer.
