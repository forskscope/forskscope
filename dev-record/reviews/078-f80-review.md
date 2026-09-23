# Review 078 — Request 076: F80, Deep Compare status labels

**Reviewer:** architect
**Date:** 2026-08-22
**Reviewed:** `d393c39`, against baseline `960ec55`
**Verdict:** **Approved. F80 is resolved.** One question for the owner about the
Japanese wording (§3) — not a defect, and not blocking. The glyph divergence is
registered as **F82** with your proposed shape.

## 1. Verified independently

- **The glyph match is byte-identical to baseline** — I diffed it rather than
  trusting the claim. No glyph, no CSS class, no behaviour changed; `xtask css
  --check` reports no diff because there were no CSS edits at all.
- One span, no per-status branch. F79's special case is gone.
- Six labels reused exactly from the Explorer work; one new key, translated.
- 76 tests, i18n 236 keys, css clean — re-run here.

Both falsifications hold. Neither claims a label reaches a screen reader, which
is right: that is P07's assertion, and saying so plainly rather than letting the
green imply it is the distinction this program keeps having to make.

## 2. `"Symlink not followed"` is the right wording, for the right reason

Your reasoning is the reasoning I would have wanted, and it is better than
"pick a string": **`Symlink` is the only status here that means *nothing was
examined*.** Every other one reports an outcome — a comparison, or a failure to
read. A label saying merely "Symlink" would be true and would leave a
screen-reader user unable to tell this row apart from one carrying a verdict.
"Not followed" is the fact that explains why `can_cmp` excludes it and why no
copy button appears.

**Pinning it with `symlink_label_says_not_followed_not_a_verdict` is the better
half of the decision.** A wording choice defended only in a comment survives
until someone tidies the string; one a test asserts survives until someone
argues with the test. And you scoped it to `Lang::En`, so it cannot false-fail
when the Japanese is rephrased — which is exactly the trap that kind of test
usually sets.

## 3. One question for the owner, not for you

The Japanese is `シンボリックリンク（未追跡）`. **未追跡 commonly reads as
"untracked" in the git sense**, and this is a diff and merge tool sitting next to
version control — so there is a plausible misreading where the label sounds like
a VCS state rather than "we did not follow the link."

I am not asking you to change it. The owner writes and owns the Japanese voice
here, and it is their call whether 未追跡 is the right register or whether
something like 「追跡しません」or 「リンク先は未比較」carries it better. Raised
because the English was chosen carefully for a specific reason, and the
translation should carry that same reason rather than only the words.

## 4. §3 — the glyph divergence, and your proposal

Your view is right on both counts, and I am adopting it.

**Right to leave it here.** Unifying inside a presentation-only accessibility fix
would mean picking a side in a design decision, which is the "resolving it badly"
this handoff's §8 warned about.

**Right about the eventual shape.** A single shared glyph/CSS/label table in
`ui-logic` that both views render through is where this belongs — the same shape
`status.rs` already gives the Explorer, rather than either view importing the
other's constants. Registered as **F82** with that shape credited to you, and
with the note that **RFC-080's implementation is the natural moment**, since it
will already be adding states to the Explorer's vocabulary.

One caution recorded there: `ui-logic` is where nine modules already sit unwired
(F75). A shared table added there must be wired by **both** views in the same
change, or it becomes the tenth.

## 5. Status

F80 resolved. Gate D unchanged: **F44** (upstream) and **F60** (owner).

Nothing queued for you. F75(b) and F82 are unscheduled; RFC-081's implementation
waits on two small owner decisions.
