# Developer Handoff 005 — F78: one comparison mechanism for both views

**From:** architect
**Date:** 2026-08-21
**Register:** F78 (fixed here). Also closes F77's recorded regression exposure — §7c.
**Gate:** F78 is a **Gate D blocker** (architect assessment, recorded in its entry).
**Not governed by RFC-080.** That RFC is accepted and sequenced after Gate D.

---

## 1. Task title

Give the Explorer and Deep Compare a single guarded, cancellable, bounded
comparison mechanism, replacing the two half-mechanisms they have now.

## 2. Purpose

`deep_compare.rs` carries both defects handoff 004 fixed in `explorer.rs`, and
they matter more there because the statuses drive **file operations**:

- `can_copy_left_to_right` admits only `Changed` or `LeftOnly`
- `BatchCopyButtons` builds its copy manifest with the same filter
- `can_cmp` excludes `Equal`

So a stale `Equal` on a genuinely differing file means that file is **silently
omitted from "copy all changed"**, its per-row copy button disappears, and it
cannot even be opened to compare. The user runs a merge, is shown success, and
one differing file was never copied.

## 3. Background

Your request 072 §5/§7 flagged the `deep_compare.rs` shape rather than expanding
handoff 004's scope. That was the right call and this handoff is the result.

Checking it found the converse: **`deep_compare.rs` bounds its digest fan-out
with a `Semaphore` at `DIGEST_CONCURRENCY_LIMIT`; `explorer.rs` has no bound at
all** and spawns one `spawn_blocking` per common file in the listed directory.

Neither view is wrong about what it needs. Each was built without the other's
lesson, and transplanting each half into the other would leave two copies of
three concerns, free to drift apart again. That is the design decision below,
and it is mine, not yours to re-open.

## 4. Applicable RFC and requirements

- **RFC-037 §"Cancellation"** — the existing convention. Do not add a second one.
- **RFC-080 §5** — will require exactly this mechanism; building it once now is
  why that RFC stays small later. Do not implement RFC-080 itself.

## 5. Change scope

- **New:** `crates/forskscope-ui/src/ui/view/digest_epoch.rs`
- `crates/forskscope-ui/src/ui/view/explorer.rs` — converted to it
- `crates/forskscope-ui/src/ui/view/deep_compare.rs` — converted to it

## 6. Explicit non-change scope

- **No RFC-080 work** — no tiers, no new status vocabulary.
- **No size bound or threshold.** Still RFC-080's, for the reason in handoff 004 §6.
- **Do not fix the I/O-error-to-`Changed` collapse.** It is F76's, in both views,
  and it stays visible-but-unchanged here. Preserve current behaviour exactly.
- **Do not put this in `forskscope-ui-logic`.** It needs `tokio::sync::Semaphore`,
  which `ui-logic` does not and should not depend on — it depends only on
  `forskscope-core` today. Splitting one small concept across two crates to make
  half of it "testable elsewhere" would be worse than keeping it whole.
- F74, F75, F76 — untouched.

## 7. Required implementation

### 7a. `DigestEpoch`

One type owning the three concerns that are currently split between the views:

- **generation** — which run a result belongs to
- **cancellation token** — so outstanding work stops
- **permits** — so the fan-out is bounded

Required API shape, with the reasoning that fixes the shape:

- **`EpochStamp` is opaque and only `DigestEpoch` can mint one.** No public
  constructor, no `From<u64>`. This is not decoration: it makes F77's recorded
  regression exposure a **compile error**. Today, reading the current generation
  where the spawn-time value belongs type-checks fine and no test catches it
  (verified by probe at review 074). With an opaque stamp there is no second
  value of the right type to pass by mistake.
- **`restart()`** — cancel the outgoing token, install a fresh one, bump the
  generation. Called exactly where the view's inputs change.
- **`begin_task()`** — returns the stamp, the token, and the semaphore handle for
  one about-to-be-spawned comparison. Mark the return `#[must_use]`.
- **`is_current(stamp)`** — the guard.

**The semaphore is created once and survives `restart()`.** Do not replace it: a
fresh one per epoch would let cancelled-but-still-draining tasks from the old
epoch run alongside a full new epoch's worth, so the bound would hold per epoch
and not overall. One semaphore for the object's lifetime bounds total in-flight
blocking work per view, which is the property worth having.

**Acquire the permit inside the spawned task, not while holding a signal guard.**
`begin_task()` hands back the `Arc<Semaphore>`; the `await` on `acquire_owned()`
belongs in the task, after any Dioxus `read()` guard has been dropped. Holding a
signal borrow across an await is a deadlock, and `deep_compare.rs` already has
the correct shape to copy.

### 7b. Convert `explorer.rs`

Replace `digest_generation`, `digest_token` and `apply_digest_result` with a
`Signal<DigestEpoch>`. **This is a real conversion, not an addition** — those
three must be gone when you are done, or there are two mechanisms again.

The Explorer **gains** the concurrency bound it does not have today. That is a
behaviour change in the sense that fan-out is now limited; it is not a
user-visible one.

### 7c. Convert `deep_compare.rs`

The defect fix. It gains everything the Explorer has:

- `restart()` when the observed roots change — the effect currently has no
  equivalent of the Explorer's `changed` block; it needs one.
- The stamp guard on **phase 2's** result application, which currently locates
  its entry by `rel_path` after `ent.set(initial)` may have replaced the list.
- `file_digest_equal_with_cancel` instead of `file_digest_equal`, with
  `DigestOutcome::Cancelled` applying nothing — the pattern from your own
  `explorer.rs` work.
- **Phase 1 too:** `list_recursive_for_display_with_cancel` instead of the
  uncancellable variant. A large tree's listing is currently uninterruptible and
  nobody has registered that; it is in scope here because it is the same token.

## 8. Required tests

Demonstrated failing against the defect, not against a helper the fix
introduces — the standard from reviews 072 and 074.

1. **A stale stamp must not mutate Deep Compare's entries.** Drive the real apply
   step with a stamp from a superseded epoch and assert the entry list is
   **unchanged**; then with a current stamp and assert it is applied. Extract the
   apply step so this is reachable without a Dioxus runtime, as
   `apply_digest_result` was.
   **Falsify by removing the guard**, confirm the stale result lands, restore.
2. **`DigestEpoch`'s own guard.** `restart()` must invalidate a stamp taken before
   it, and must cancel the token handed out before it.
   **Falsify by making `restart()` skip the generation bump**, restore.
3. **The Explorer's existing F77 tests must still pass** after §7b's conversion,
   and **their falsifications must still hold.** Re-run both from review 074 —
   removing the guard, and removing core's in-loop poll — and confirm each still
   fails a test. If the conversion silently weakened either, that is the thing
   this handoff must not do.

**No test for the concurrency bound**, and this is deliberate rather than an
omission: observing real concurrency needs timing, which handoff 004 §14 ruled
out for good reasons that have not changed. The bound is enforced by the API
shape — `begin_task()` is the only way to get a stamp, and it hands you the
semaphore with `#[must_use]`. Say so plainly in the review request rather than
implying coverage.

## 9. Required documentation updates

None. **Do not edit `ROADMAP.md`.**

## 10. Acceptance criteria

- `deep_compare.rs`'s stale-result path fails a test when the guard is removed.
- `digest_generation`, `digest_token` and `apply_digest_result` no longer exist
  in `explorer.rs`.
- Neither view calls `file_digest_equal` or `list_recursive_for_display`
  (the uncancellable variants) any more.
- Review 074's two falsifications still hold after the conversion.
- Passing a non-stamp where a stamp belongs does not compile.
- No user-visible behaviour change beyond results now being correctly discarded.
- Gates green: `cargo fmt --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo xtask css --check`,
  `cargo xtask i18n`, `git diff --check`.

## 11. Prohibited shortcuts

- **Do not leave `explorer.rs`'s existing mechanism in place beside the new one.**
  Two mechanisms is the defect this handoff exists to remove.
- **Do not make `EpochStamp` constructible outside `DigestEpoch`**, and do not
  give it a public `u64` field. The opacity is the point.
- **Do not widen the epoch into a general job scheduler.** It owns three things.
  If you find yourself adding a fourth, stop and say so in the review request.
- **Do not report a falsification you did not run.**

## 12. Relevant code or module boundaries

`forskscope-ui` only. Core is unchanged — everything needed
(`file_digest_equal_with_cancel`, `list_recursive_for_display_with_cancel`,
`DIGEST_CONCURRENCY_LIMIT`, `CancellationToken`) already exists there.

## 13. Compatibility and security constraints

No public format, no persistence, no new dependency. `tokio::sync::Semaphore` is
already used by `deep_compare.rs`.

## 14. Known risks

- **§7b touches working, reviewed code.** That is why §8.3 exists: the conversion
  must be shown not to have weakened F77's coverage. If re-running review 074's
  falsifications is awkward after the conversion, that is a signal the shape is
  wrong, not a reason to skip them.
- **Deadlock via a signal guard held across an await.** See §7a. This is the most
  likely way to break the app while getting the logic right.
- **Deep Compare's phase 1 and phase 2 share one token.** Intended: a root change
  should stop both. Make sure a phase-1 cancellation does not leave the view
  showing a permanent "scanning" state.

## 15. Required evidence

- Observed failure output for §8.1 and §8.2's falsifications, quoted.
- Confirmation that §8.3's two re-runs still fail, quoted.
- Gate results per §10.

## 16. Required review-request format

As in requests 071 and 072, which were both well structured. Lead with the
falsifications and their output.

State plainly:
- whether anything from `explorer.rs`'s F77 mechanism survived, and if so what;
- whether the concurrency bound is enforced anywhere other than by the API shape;
- what, if anything, you had to add to `DigestEpoch` beyond the three concerns in
  §7a, and why.

That last one is the question I most want answered honestly. If the type grew, I
would rather hear it than find it.
