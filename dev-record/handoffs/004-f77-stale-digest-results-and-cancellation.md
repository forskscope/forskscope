# Developer Handoff 004 — F77: stale digest results, and a comparison that cannot be interrupted

**From:** architect
**Date:** 2026-08-21
**Register:** F77
**Gate:** Gate D input. Assume this blocks the next candidate until reviewed.
**Not governed by RFC-080.** That RFC is accepted but sequenced after Gate D.
This is a defect in shipped code and must not wait behind it — see §6.

---

## 1. Task title

Stop a file comparison's result from landing on the wrong file, and make a
comparison interruptible.

## 2. Purpose

Two independent defects in one code path, both reachable today.

**(a) A verdict computed for one pair of roots can be applied to a different
file.** `explorer.rs`'s digest effect clears `digest_map` when the roots change,
but comparisons already `spawn`ed under the old roots are not cancelled and hold
a copy of the signal plus their `rel` key. When one finishes it inserts
unconditionally. Navigate from A to B while a comparison runs, and if B contains
a file at the same relative path, **it receives A's verdict — including a
possible `Equal` for a pair that was never compared.**

The paths captured are correct, so the comparison itself is sound. It is the
*destination* of the result that is stale. Same shape as F73 one layer up.

**(b) A comparison cannot be interrupted once started.**
`forskscope-core::dir::digest::file_digest_equal` polls nothing in its read loop.
Two identical multi-gigabyte files are read to the end with no way to stop.
`recursive_diff_with_cancel` inherits this: it checks cancellation *between*
files and never *within* one.

## 3. Background

Found while answering an owner question during RFC-080's design — *if thorough
file comparison also costs much, why is only directory comparison treated as
costly?* The premise was right, and checking it turned this up.

**Worth knowing, because it is the same finding twice.** This project told the
`sheets-diff` team, in the 2.4.x correspondence, that cancellation observed only
between sheets and never mid-sheet is a frozen window in a GUI, and that we would
test it rather than assume it when we adopted. Defect (b) is that exact defect,
in our own code, and it has been there the whole time. Nobody looked because
nobody had reason to — which is the recurring lesson, not a criticism.

## 4. Applicable RFC and requirements

- **RFC-037 §"Cancellation"** — the convention `recursive.rs`'s `_with_cancel`
  variants already follow. Match it; do not invent a second mechanism.
- **RFC-080 §5** — requires cancellation on navigation. This handoff delivers the
  token that requirement needs, ahead of the RFC that will rely on it.

## 5. Change scope

- `crates/forskscope-core/src/dir/digest.rs` — a cancellable variant.
- `crates/forskscope-ui/src/ui/view/explorer.rs` — generation guard, token,
  cancel on root change.

## 6. Explicit non-change scope

- **Do not implement any part of RFC-080.** No tiers, no new status states, no
  vocabulary changes. That work is sequenced after Gate D.
- **Do not add a size bound or threshold.** This is the important one, and the
  register entry for F77 originally said otherwise — it was corrected before this
  handoff. A file exceeding a bound needs somewhere to rest, and the only honest
  resting state is RFC-080's tier-1 state, which does not exist yet. Leaving such
  a file at `Computing` forever, or showing nothing, would each be a fresh defect.
  **The bound ships with RFC-080. This handoff is the guard and the token only.**
- **No user-visible change.** If a screenshot would differ, something is out of
  scope. Correct behaviour here looks identical and is simply not wrong.
- **Do not change `file_digest_equal`'s existing signature or behaviour.** Add a
  variant beside it, as `recursive.rs` does.
- F74, F75, F76 — untouched.

## 7. Required implementation

### 7a. `file_digest_equal_with_cancel` (core)

A variant taking `&CancellationToken`, polling it **inside the read loop** so a
large-file comparison is interruptible. `file_digest_equal` becomes a wrapper
passing a fresh token, exactly as `recursive_diff` wraps
`recursive_diff_with_cancel`.

Decide and document what a cancelled comparison returns. It must be
distinguishable from "compared, and they differ" — a cancelled comparison
established nothing, and collapsing it to `false` would be a smaller copy of the
bug in §2a. Returning `Result` with a dedicated variant, or `Option`, are both
acceptable; state your choice and why in the review request.

**Poll per chunk, not per byte.** The existing loop reads `BUFFER`-sized chunks;
one check per iteration is the right granularity and costs nothing measurable.

**Then use it in `recursive_diff_with_cancel`**, which currently calls the
uncancellable version — that is defect (b) reaching Deep Compare, and it is one
line once the variant exists.

### 7b. Generation guard (UI)

A monotonic counter incremented whenever the roots change. Each spawned
comparison captures the value current when it started; on completion it applies
its result **only if the counter still matches**.

**The guard is required even with the token**, and this is not belt-and-braces
for its own sake: cancellation is inherently racy — a comparison can finish in
the window between the root change and the token being observed. The token stops
wasted work; the guard stops wrong results. Neither substitutes for the other.

Cancel the outgoing token at the same place `digest_map` is cleared.

## 8. Required tests

The standard is unchanged: each check demonstrated failing against the defect it
exists to catch — **the shipped defect, not a helper the fix introduces.** Review
072 returned a fix on exactly that point; handoff 003 §8 has the detail.

1. **A stale result must not mutate the map.** Extract the apply step so a test
   can drive it — something shaped like "given a result, its generation, and the
   current generation, apply or discard" operating on the real map. Assert that a
   result carrying a superseded generation leaves the map **unchanged**, and that
   a current one is applied.

   **Falsify by removing the guard**, confirm the stale result lands, restore.
   A test that only checks `stale != current` in isolation does not meet this.

2. **A cancelled comparison stops early and says so.** Build a file pair large
   enough to span several `BUFFER` chunks, cancel the token before or during, and
   assert the result is the cancelled outcome — **not** `false`, and not a
   completed verdict. `explorer.rs`'s `temp_dir(tag)` pattern is the precedent; no
   new dependency.

   **Falsify by removing the in-loop poll**, confirm it runs to completion,
   restore.

3. **`file_digest_equal`'s existing behaviour is unchanged** — the current tests
   must pass untouched. If any needs editing, stop and say so in the review
   request rather than editing it.

## 9. Required documentation updates

None. F77's register entry already carries the analysis and the scope correction.
**Do not edit `ROADMAP.md`.**

## 10. Acceptance criteria

- Removing the generation guard makes a test fail.
- Removing the in-loop cancellation poll makes a test fail.
- A cancelled comparison is distinguishable from a completed one at the type
  level, not by convention.
- `recursive_diff_with_cancel` no longer calls the uncancellable variant.
- No user-visible behaviour change.
- Gates green: `cargo fmt --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo xtask css --check`,
  `cargo xtask i18n`, `git diff --check`.

## 11. Prohibited shortcuts

- **Do not map a cancelled comparison to `false`.** It is the same class of bug
  as the one being fixed: asserting a verdict nothing established.
- **Do not rely on the token alone** and skip the guard. See §7b.
- **Do not test only the extracted predicate.** See §8.1.
- **Do not report a falsification you did not run.**

## 12. Relevant code or module boundaries

Core stays GTK-free and Dioxus-free; the token is already a core type
(`forskscope_core::cancel::CancellationToken`, an `Arc<AtomicBool>`). The
generation counter is UI state and stays in `forskscope-ui`.

## 13. Compatibility and security constraints

No public format, no persistence, no new dependency. `file_digest_equal` keeps
its signature so existing callers are unaffected.

## 14. Known risks

- **The race is timing-dependent, so a test that reproduces it by timing would be
  flaky.** Do not try. Test the *guard* deterministically by driving the apply
  step with an explicit stale generation — the race is what makes the defect
  reachable, not what the test needs to recreate.
- **Cancellation semantics leak into callers.** Every caller of the new variant
  must handle the cancelled case explicitly. That is the point; if a call site
  can ignore it, the return type is wrong.

## 15. Required evidence

- Observed failure output for §8.1 and §8.2's falsifications, quoted.
- Gate results per §10.
- The chosen cancelled-comparison return shape, and why.

## 16. Required review-request format

As in requests 070 and 071. Lead with the two falsifications and their pasted
output — those are the acceptance criteria and everything else is supporting
work.

State plainly whether any existing test needed editing. The answer should be no.
