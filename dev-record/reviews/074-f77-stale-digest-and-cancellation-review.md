# Review 074 — Request 072: F77, stale digest results and cancellation

**Reviewer:** architect
**Date:** 2026-08-21
**Reviewed:** `98dde50`, against baseline `b4a5d1e`
**Verdict:** **Approved. F77 is resolved.** One new finding registered as **F78**,
from your own §5/§7 flag plus a check it prompted.

## 1. Verified rather than accepted on report

Both falsifications hold, and I probed deeper than re-running yours. Workspace
tests pass (`forskscope-core` 252, `forskscope-ui` 64), `clippy --workspace
--all-targets -- -D warnings` clean, `fmt` clean — all re-run here.

`DigestOutcome` is the right shape and §1's reasoning for it is the correct
reasoning, not a rationalisation after the fact: a cancelled comparison
established nothing, and the argument that the compiler should enforce that
rather than a convention is exactly why the handoff asked for type-level
distinction. Making `file_digest_equal` a wrapper mirrors `recursive_diff`
faithfully.

Wiring `walk_and_merge` to the cancellable variant means **Deep Compare's scan is
interruptible mid-file for the first time**. That is the larger half of this
change and it is easy to undersell — it is the defect we reported *to* the
sheets-diff team, closed in our own code.

Updating `RecStatus::Computing`'s doc comment, which had claimed it was never
returned by `recursive_diff_with_cancel`, is the kind of thing that gets skipped.
It was true before and false after, and you noticed.

## 2. A coverage limit — mine, not yours, and recorded rather than left implied

The tests cover the guard's **logic**, not its **wiring**. I defeated it at the
call site — reading the current generation at completion instead of the value
captured at spawn, leaving `apply_digest_result` untouched — and **all 64 tests
pass**.

I am not asking you to fix this, and it is not a failure of the work. Handoff 004
§8.1 specified the helper's shape and you built exactly that. Unlike F74's
`classify_entry`, extracting further does not close the gap here: what can break
is the *capture point*, which lives in an async spawn inside a `use_effect`, and
the only test that would cover it is the timing test §14 explicitly forbade.

**The code is correct today**; the exposure is future regression. Recorded in
F77's entry with a recommendation for whoever next touches it: **give the two
generations distinct newtypes**, so passing the current one where the spawn one
belongs is a compile error. That is your own `DigestOutcome`-over-`bool`
argument, applied one level out — which is why I expect you would have reached it
yourselves given the brief.

## 3. Your three questions

**Q1 — `Result<DigestOutcome>` or fold I/O errors into a fourth variant?**
Keep `Result`. An error and a cancellation are different kinds of event: an error
is a failure to perform the operation, a cancellation is the operation
deliberately stopped. Folding errors in would cost `?` at every call site and
turn error propagation into matching, against the grain of the rest of core.

Worth noting what your shape already achieves: `Result<DigestOutcome>` gives
callers everything they need to stop collapsing errors into `Different`. The
collapse is now a **visible choice at the call site** rather than something
hidden inside `.unwrap_or(false)`. That is a real improvement even though the
behaviour is unchanged, and it is what makes F76 straightforwardly fixable later.

**Q2 — was preserving the I/O-error-to-`Changed` mapping correct scope
discipline?** Yes, and for a better reason than "it was out of scope": you made
the existing defect **legible**. `Err(_) => DigestState::Different` states plainly
what `.unwrap_or(false)` concealed. Fixing it here would have mixed a behaviour
change into a change whose whole claim is that behaviour is unchanged.

**Q3 — is §5's reasoning for not adding a timing test sound?** Yes. It matches
handoff 004 §14, and your argument is stronger than the instruction: you cited
the existing precedent in `dir_cancel_tests.rs` and named the two ways such a
test fails — flaky if it asserts the interruption point, vacuous if it does not.
Proving the primitive deterministically and verifying one line of wiring by
reading it is the right trade. **Do not add it.**

## 4. F78 — registered from your flag, plus one you did not mention

Flagging `deep_compare.rs` rather than silently fixing it was the right call;
handoff 004 §5 named `explorer.rs` only, and quietly widening scope is how a
reviewable change stops being one. Registering it is the correct disposal.

It carries **both** F77 defects — the uncancellable `file_digest_equal`, and no
generation guard, locating its entry by `rel_path` after `ent.set(initial)` may
have replaced the list. It matters more there than in the Explorer: **RFC-080
names Deep Compare as the authoritative directory verdict**, so a stale status
undermines the premise that RFC rests on.

**The one you did not mention, found while checking yours:** `deep_compare.rs`
bounds its digest fan-out with a `Semaphore` at `DIGEST_CONCURRENCY_LIMIT`, and
**`explorer.rs` has no bound at all** — one `spawn_blocking` per common file in
the listed directory. Each view is missing what the other has. Neither was built
wrong; each was built without the other's lesson.

Most of F78 is transplant rather than design — the pieces now exist in your
commit. The real question left is whether the two views should share one
mechanism instead of keeping two parallel copies, which is why it gets a design
pass rather than a repeat handoff.

## 5. One observation, no action

`file_digest_equal`'s `unreachable!()` is genuinely unreachable — the token is
created fresh in the same expression and nothing can call `cancel()` on it — and
your message documents the invariant. Fine as written. The only way it becomes
reachable is if someone later lets that wrapper take a caller-supplied token, and
the message would tell them exactly what they broke.

## 6. Status

F77 resolved. Gate D stands at **F44 (upstream) and F78 (pending assessment)**.
No further work on this handoff.
