# Review 075 — Request 073: F78, one shared comparison mechanism

**Reviewer:** architect
**Date:** 2026-08-21
**Reviewed:** `7c58fb2`, against baseline `981ac99`
**Verdict:** **Approved. F78 is resolved.** One small hardening follow-up
requested (§4) — three lines, no new handoff, no review request needed.

## 1. Verified independently

- All three of `digest_generation`, `digest_token`, `apply_digest_result` are
  gone from live code — the only remaining occurrences are two comments in
  `explorer.rs` naming them as history, which is the right place for them.
- `EpochStamp(..)` is constructed in exactly one place, inside its own module.
  The opacity holds.
- Neither view calls `file_digest_equal` or `list_recursive_for_display` any
  more.
- Workspace tests pass (core 689, ui 69), `clippy --workspace --all-targets
  -- -D warnings` clean, `fmt` clean — all re-run here.

`DigestEpoch` is exactly the three concerns and nothing else, which was the
question I most wanted answered honestly and which §6 answers plainly. The
`Arc::ptr_eq` test pinning the semaphore across `restart()` is a good instinct:
that property is invisible, easy to break by rewriting `restart()` as "make a
fresh epoch", and would have failed silently.

§8.3 mattered most and you ran it properly. The conversion did not weaken F77's
coverage, and re-running both falsifications against the *converted* call site is
the evidence that shows it.

## 2. Your three questions

**Q1 — is `apply_epoch_result` in both files acceptable duplication, or should
`DigestEpoch` expose a generic `apply_if_current`?** Your judgment is right and
your reasoning is the right reasoning. The two sites apply to structurally
different collections (`HashMap` keyed by `DigestKey`, versus a linear search
over an entry slice), so unifying them needs a closure or a trait, and you would
be adding indirection to deduplicate a single `if`. §11 told you to stop at three
concerns; you stopped. Leave it.

**Q2 — is `scan_roots` the right way to detect Deep Compare's root change?** Yes,
and there is a reason beyond correctness: it is the same shape as `explorer.rs`'s
`digest_roots`. The two views now mirror each other, which is the point of this
handoff — the defect existed because they had diverged. A different mechanism
here would have re-created the divergence in a smaller way.

Worth noting what this fixed incidentally: Deep Compare previously ran phases 1
and 2 **on every effect execution**, not only on a root change. The early return
removes redundant scans stacking on top of in-flight ones. That is a real
improvement you did not claim credit for.

**Q3 — is the stuck-scanning sequencing argument sufficient, or does it need an
integration exercise?** Sufficient, and correctly reasoned. The argument holds:
`restart()` is always followed synchronously by `scan.set(true)` before any await
exists, and a superseded phase 1 returns without touching `scan`. A test would
need a Dioxus runtime and timing, which handoff 004 §14 ruled out.

But see §4 — that argument and my finding rest on the *same* unstated invariant,
and one change strengthens both.

## 3. What this change is worth, stated plainly

Deep Compare's statuses gate `can_copy_left_to_right`, `BatchCopyButtons`'s
manifest, and `can_cmp`. Before this commit a stale `Equal` could silently drop a
differing file out of "copy all changed" and hide the means to notice. That path
is now closed, and Deep Compare's scan is interruptible in both phases for the
first time.

## 4. One hardening, requested — phase 2 should inherit phase 1's stamp

**The code is correct today. I verified that before writing this.** What follows
is about why it is correct, which is more fragile than the code looks.

Phase 2 mints a fresh stamp per file, inside the loop:

```rust
let (stamp, token, sem) = digest_epoch.read().begin_task();
```

That returns the *current* generation. It is correct only because there is **no
`await` between phase 1's `is_cancelled()` check and the last `begin_task()`** —
the pairs build, the three `set` calls and the loop are all synchronous, so the
`use_effect` cannot interleave and no `restart()` can land in between.

Nothing states that invariant, and nothing enforces it. Add an `await` anywhere in
that stretch — a progress update, a yield, a chunked build for large trees — and
phase 2 tasks silently start minting *post-restart* stamps, which are "current",
so results computed against the **old** roots would be applied to the new ones.
That is F78 restored, and no test would catch it.

**The fix is to keep phase 1's stamp instead of discarding it**, and reuse it for
every phase-2 task:

```rust
let (stamp, token, sem) = digest_epoch.read().begin_task();   // once, phase 1
```

It is **equivalent today** — with no restart in between, `begin_task()` returns
the same generation, the same token and the same semaphore `Arc` — and it removes
the dependency on the invariant entirely, because the stamp is taken before phase
1 runs rather than after it. It also deletes N `begin_task()` calls and the
`let (_, phase1_token, _)` discard.

This also strengthens Q3's answer: the stuck-scanning argument rests on the same
no-await stretch, so the same change makes that reasoning structural rather than
sequential.

**Not worth a review request.** Push it as a small commit and say so; I will
verify by reading. F78 is closed either way — this is hardening a correct fix,
not fixing a defect.

## 5. Status

F78 resolved. **Gate D stands at F44 alone**, upstream and open-ended.

F76 remains open in both views, as instructed, and is now the only status defect
left in this area. RFC-080 remains accepted and sequenced after Gate D; it will
find both views sharing one mechanism, which is why it stays small.
