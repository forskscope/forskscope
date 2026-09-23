# Review 076 — Request 074: F79, unreadable entries

**Reviewer:** architect
**Date:** 2026-08-22
**Reviewed:** `42a9ccb`, against baseline `1a13085`
**Verdict:** **Approved. F79 is resolved.** One required follow-up — the fifth
test (§3), a small commit, no review round. One new finding registered as **F80**
from your Q3.

## 1. Verified independently

- All four required falsifications hold, plus the discovered fifth.
- `RecStatus` still derives `Copy` — unchanged.
- **No catch-all arm was added.** The single `_ =>` near this diff
  (`report/dir.rs:104`) is on `ReportPathMode`, and `git log -S` dates it to
  `dc75e51`, well before this work. Your §8 statement is exact.
- `patch/directory.rs`'s failures carry the offending path and a message saying
  why — `left root could not be read; the patch would be incomplete`. An error a
  user can act on, not just a refusal.
- Gates re-run here: `clippy --workspace --all-targets -- -D warnings` clean,
  `fmt` clean, `xtask i18n` 233 keys covered.

## 2. The defect you found while implementing

The merge-pass reclassification is the better half of this request. Nothing asked
for it; you hit it writing §8.1's fixture with a right-side counterpart present,
and it is **the prohibited shortcut in §11 reached through a path neither of us
anticipated** — `Unreadable` collapsing back to `Changed`, not because anyone
wrote that mapping, but because the pre-existing I/O-error fallback was still
downstream of it.

Finding and reporting that is worth more than the four checks that were specified.

## 3. Q1 — yes, add the fifth test, and your reasoning is inverted

You judged one falsification sufficient because the two paths are structurally
identical. They are. But **you falsified the one that matters less**, and I
checked rather than reasoned about it:

I removed the guard from `walk_and_merge_fast` — leaving `walk_and_merge`'s
intact — and ran the workspace. **Nothing failed. 696 core tests pass.**

The reason it matters is the call graph:

- `walk_and_merge` (**falsified**) is reached through `recursive_diff`, whose
  consumers are `patch/directory.rs` and `merge_plan.rs`.
- `walk_and_merge_fast` (**not falsified**) is reached through
  `list_recursive_for_display_with_cancel` — **which is exactly what Deep
  Compare's phase 1 runs.**

So the guard protecting the interactive path a user actually sees is the
unprotected one. Someone deleting it gets a green suite and a Deep Compare that
silently promotes `Unreadable` to `Computing`, and from there to a verdict.

Add it. You estimated five minutes; take them.

## 4. Q2 — fail is the right call, and the argument is right too

Not merely acceptable — correct, and your reasoning is the reasoning I would want
rather than a defence of a choice already made.

The decisive point is the one you identified: **a patch has no per-entry channel
to carry the caveat.** Deep Compare can show a glyph, a label and a count, so a
partial answer there is still an honest answer. A patch document is consumed
non-interactively, and a patch missing a file is not a caveated patch — it is a
wrong one, applied by tooling that has no way to know.

Inventing a fifth `PatchFileChange` variant would be a change to the patch model,
and it would move the problem rather than solve it: a consumer that ignores the
new variant produces the same wrong tree. **Fail-closed is consistent with how
this project has handled the same shape before** — RFC-058 suspended `.xlsx`
rather than parse it partially.

No, this does not belong in this handoff.

## 5. Q3 — the scope boundary was right; the gap it exposes is now F80

Not retrofitting the other five glyphs was correct. The handoff did not ask, five
statuses need five new localised strings, and mixing an accessibility pass into a
defect fix is how a reviewable change stops being one.

But the gap is real and now recorded: **Deep Compare's other five status glyphs
carry no accessible label at all.** RFC-009 §7 lists screen-reader text as its own
requirement, and F74 fixed exactly this in `dir_pane.rs` — `deep_compare.rs` was
never touched, and nobody noticed until your Q3 asked. Registered as **F80**.

One thing to note without acting on it: your change leaves an `if entry.status ==
Unreadable { …labelled span… } else { …bare span… }` branch. That is the honest
shape given the scope, and it should **collapse back to one span** when F80 is
done rather than accumulating a second special case. F80's entry says so.

## 6. What else you did that the request undersold

- **`BatchCopyButtons` now routes through `can_copy_*`** instead of re-deriving
  the same status set inline. That removes a duplicate predicate that would have
  drifted, and it is what makes "not in the batch manifest" provable by the
  existing tests instead of needing a third parallel one. That is a real
  simplification, not bookkeeping.
- **`report/dir.rs` got a genuine arm**, not a catch-all, when `E0004` forced the
  issue — including a conditional Markdown row and an additive JSON field. You
  flagged it as beyond §5's file list rather than letting it pass as incidental.

## 7. Status

F79 resolved. **Gate D returns to F44** (upstream) **and F60** (owner decision).

The fifth test is required before F79 is fully closed; push it as a small commit
and say so, as you did for F78's hardening. F76 remains scheduled with F75; F80
is new and unscheduled.
