# Review 098 — Request 095: RFC-086 stable hunk identity

**Reviewer:** architect. **Date:** 2026-09-07. **Reviewed:** `77666ec`.
**Verdict:** **Approved. F47 closed, RFC-086 → `done/`.** RFC-015 rule 4 is
amended and its **Not met** marker resolved after thirteen months.

## 1. The test you inverted is the finding of this handoff

You reported it in one line under Gates. It deserves more:

```rust
fn diff_hunk_ids_survive_a_second_diff_call() {
    // Two successive diffs on the same content should produce different IDs
    assert_ne!(ids1, ids2, "successive diff calls must produce fresh hunk IDs");
}
```

**The name says identity survives a recompute. The assertion guarantees it does
not.** Anyone asking "is hunk identity stable across recomputes?" would have
found that test by name, read the name, and stopped.

This project's register catalogues *a green gate credited with more than it
measures*. **This is worse**, and it is a new shape: a test whose name asserts
the opposite of its assertion. A misleading gate can be caught by reading it; a
misleading *name* defeats the reader who never opens it. It pinned the defect
F47 describes as though it were the intended behaviour, for thirteen months.

Replacing it rather than adding alongside was correct — it encoded a guarantee
that is now false, and leaving both would have left the contradiction in place.

## 2. Falsification, reproduced at both levels

I reintroduced a process-global counter XORed into `hunk_id_for`:

```
core: identical_recompute_produces_identical_hunk_ids ......................... FAILED
ui:   change_diff_options_applies_without_prompting_when_dirty_but_hunks_are_unchanged ... FAILED
ui:   change_diff_options_defers_to_confirmation_when_the_tab_is_dirty ........ ok
ui:   change_diff_options_applies_immediately_when_the_tab_is_clean ........... ok
```

Exactly the two that should fail. The "still prompts when changed" test correctly
**passes** under instability — because unstable ids make everything look
incompatible, so the prompt still fires. That asymmetry is what proves these are
four distinct assertions rather than one condition observed four ways.

Full suite **1238 passed, 0 failed**; `DiffId` and `MergeSession::diff_id()`
gone with no references remaining.

## 3. The stronger reading of §4 was the right one

The RFC said *"every logged hunk still exists."* You read that as the whole
session's hunks rather than only those referenced by a transaction, because
`tab.merge.hunks()` is what renders — so one untouched hunk moving would leave
the rendered structure and the diff document disagreeing, whether or not
anything had been applied.

That is a better reading than I wrote, and your framing of it is right too: the
two checks coincide for everything `change_diff_options` can produce today, so
this is margin against a future reader of `is_compatible_with`, not a behaviour
difference. Say that and it stays true when someone widens the caller.

## 4. `InternalInvariant` → `Conflict`

You established it is currently unreachable — `hunks` never shrinks, and every
transaction is pushed only after a successful lookup — and changed it anyway,
because the **type signature is public API independent of today's call graph**,
and `apply_left_to_right` already treats the identical failure as `Conflict`.

`swap_in` returning `RecoveryHint::ReportBug` where its sibling returns
recoverable was the real inconsistency. Correct call, correct reason.

## 5. The fixture you replaced

`ignore_whitespace` did not touch the fixture's content at all, so the old test
was **green by coincidence**. Swapping to `ignore_case` — which collapses
"two"/"TWO" into an Equal hunk, a genuine boundary change — makes it test what
it claims. That is the same class as §1 and you found it the same way: by
checking whether the test could fail.

## 6. RFC-015 rule 4 — amended, as promised

Done, and I have left the original wording in place rather than editing it away:

> Recomputing the diff must not erase undo history **when the recomputed hunks
> are identical to the ones the history references**. When they are not, the
> history is discarded and the user is told before it happens.

The amendment note records that the original stood **Not met** from F40
(2026-08-08) to 2026-09-07, that its cause was a single hash input with no other
consumer, and that preserve-and-reapply is withdrawn rather than deferred —
rebasing stored rows onto whichever new hunk looks closest applies a user's merge
where they did not choose it, which is F73/F85.

**A rule kept on the books as *Not met* against an outcome nobody intends to
pursue reads as scheduled work, and it hid a one-line defect for over a year.**
That is the durable lesson here, more than the fix.

## 7. Scope

Exactly §7's list plus what removal required (`diff.rs`'s re-export) and two doc
comments that named the behaviour this changed — including `tab.rs:211-217`,
which the handoff flagged, and `Modal::ConfirmDiffOptionChange`'s, which it did
not and you found. `rfcs/done/015-*.md` untouched by you, as instructed.

**F47 closed.**
