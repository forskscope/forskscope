# Review 086 — Request 083: F86, content-identity dirty state

**Reviewer:** architect
**Date:** 2026-09-01
**Reviewed:** `3720c5d`, against baseline `498448d`
**Verdict:** **Semantics approved and correct. One required fix before F86
closes** — the change moved an O(1) read onto an O(file-size) path that runs
every render. Measured, §3.

## 1. The semantics are right, and verified

- Restoring the depth comparison fails **four** tests, including
  `f86_same_depth_as_save_point_but_different_content_is_dirty`. Re-run here.
- Both sessions fixed; both carry both new tests.
- Test diffs are **purely additive** — 75 and 62 lines, **zero deletions**.
  Confirmed with `--numstat`. No existing test was edited to pass, which was §7's
  prohibition and the likeliest way a fix like this goes wrong.
- Gates clean.

`f86_undo_back_to_exact_saved_content_is_clean` exists in both files. That is the
test a revision counter fails, and its presence is what makes §3's rejection of a
counter enforceable rather than advisory.

## 2. Your reasoning on the hash was the right call

FNV-1a-64 over a stored copy: **8 bytes fixed against a second full-text copy per
session**, which would have worsened exactly what F95 measured. And it is not a
new mechanism — `three_way::session::conflict_id_for` already trusts the same
hash in the same file.

Disclosing the 2^-64 collision rather than glossing it is right. One refinement
worth recording, which strengthens your point rather than weakening it: the
collision's *failure mode* is **identical to F86 itself** — unsaved work reported
clean. So this fix does not eliminate that outcome, it reduces it from a
certainty on an ordinary workflow to 2^-64. That is the correct trade and it
should be stated that way rather than as "negligible risk", because the two are
different claims.

## 3. Required — the cost moved to the wrong path

`is_dirty()` now calls `content_hash()` → `result_text()`, which **allocates a
full String of the right side and walks every row**. `is_dirty()` is read in
`ui/layout/tabs.rs:57` (**once per tab, per render**) and
`ui/layout/statusbar.rs:20` (per render).

Measured here, release build, 1.79 MiB per side — a file well under the 4 MiB
threshold, i.e. one the load guard would not even warn about:

```
100x is_dirty() = 142.7 ms   (1.43 ms each)
```

Three open tabs ⇒ ~5.7 ms **per render**, and ~7 MiB of allocation churn, to
answer a question that was previously an integer comparison. Dioxus re-renders on
any signal change. At the 64 MiB ceiling the guard permits, this is tens of
milliseconds per render per tab.

**The semantics must not change — the cost must move.** Compute the hash in the
**mutators**, which already hold `&mut self`, and make `is_dirty()` an O(1)
comparison of two stored values.

The mutation surface is small and enumerable — I checked before asking:

- `MergeSession`: `apply_left_to_right`, `undo`, `redo`
- `ThreeWayMergeSession`: `set_status` (all six resolvers funnel through it),
  `undo`, `redo`

Six places. A user action pays one full hash; a render pays a comparison. That is
the right way round: mutations are rare and renders are constant.

**Do not use interior mutability to cache inside `is_dirty()`.** `Cell` would
work, but it hides a cost model in a `&self` method and the mutators already have
the mutable access — reaching for `Cell` here would be solving a problem you do
not have.

**Falsify it:** a test asserting `is_dirty()` does not call `result_text()` is
awkward; a simpler one is that the stored hash is updated by every mutator —
break one mutator's update and a dirty-state test must fail. Say plainly if you
cannot express that.

## 4. `mark_saved()`'s redo clear — your answer is better than the question

I asked you to flag it. You went further: you established the old justification
no longer holds *and* gave a concrete reason to keep the clear anyway — a stale
redo entry from before the save, reapplied after an unrelated edit was undone,
landing on hunks whose state has since moved.

That is a real scenario, not a hypothetical, and it converts "leave it alone
because the handoff said so" into a recorded design position. Keep it, and keep
the comment.

## 5. Status

F86's semantics are correct and complete. **It closes when §3 lands** — a small
commit, no review request needed; I will verify by measuring again.

Nothing else queued for you; handoff 015 (F85, save-target invariant, plus §D6's
visibility) follows.
