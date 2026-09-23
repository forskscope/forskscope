# RFC-077 — review 048 C1/C2 fixes review

**Review date:** 2026-08-04
**Request:** `dev-record/review-requests/046-rfc077-review048-c1-c2-fixes.md`
**Baseline:** `b95ed39` (`ui: fix review 048 C1/C2 — overwrite confirmation carries its real target`)
**Responds to:** review 048, corrections C1 and C2
**Review mode:** Independent verification. No implementation changes made.

## 1. Verdict

**Approved.** C1 and C2 are closed.

One finding that is not about this patch's code but about what its i18n gate
proves (§4). Non-blocking, registered.

B3 stays closed. RFC-077 and M3 remain incomplete; v1/public release stays
**No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — **1094**, exactly 1090 + 4 |
| `cargo clippy --workspace -- -D warnings` | Pass |
| CI run `30919210679` | `success` on `b95ed39c` |
| `Modal::ConfirmOverwrite(usize, PathBuf)` | Carries the target |
| Conflict sets it from `request.target` | Confirmed |
| `confirm_overwrite` forces only the passed path | Confirmed |
| `Blocked` reported rather than silently dropped | Confirmed |
| Force fails closed on a blocked target | Confirmed — `force` is consulted only inside the `Writable` arm |

### C1 is closed at the type level, which is the strongest form

`Modal::ConfirmOverwrite` cannot exist without a target, `handle_result`
populates it from `request.target`, and `confirm_overwrite` forwards exactly
that path into `build_request`. There is no longer a place where the attempted
target can be lost between conflict and confirmation.

Showing the path in `OverwriteModal` was not requested and is the right addition:
a confirmation dialog for a destructive action should name what it is about to
overwrite.

### C2 is closed wider than asked

Review 048 named the Save As path. The fix distinguishes `Ready` / `NotSaveable`
/ `Blocked` at the source, so a *plain* Save whose tab `save_target` is itself
blocked now reports too. Fixing the category rather than the reported instance is
the right instinct.

### Force still fails closed

Verified rather than taken on trust: `force` is consulted only inside the
`Writable` arm, so a destination that became a directory between confirmation and
retry still reports the block. That matches RFC-077 — Force bypasses the conflict
check, not the classification.

## 3. Answers to the requested review focus

### 3.1 The `dispatch` / `RequestOutcome` shape

**Right shape.** One tail shared by all three entry points means
`TargetPrecondition::Force` originates in exactly one function, which is
precisely what RFC-077 asks for — a property that is now readable in one place
rather than argued across three call sites.

`save_tab` losing its `force` parameter is a genuine simplification, not
churn: it was `false` at every remaining call site once the confirmed path moved
out. A boolean parameter with one live value is a latent bug waiting for someone
to pass the other one.

### 3.2 `describe_block` outside `t()`

**Defensible for this patch — but do not let the precedent stand as settled.**
See §4; I would rather register it than have you widen it silently or, worse,
translate one function and leave the pattern half-migrated.

### 3.3 Is a UI-layer test seam warranted for the race?

**No, and your instinct not to add it was right.**

The fix does not make the race *unlikely* — it makes the wrong-target outcome
**unrepresentable**. `Modal::ConfirmOverwrite` has no state in which the target
is absent, and `confirm_overwrite` consults nothing else. A test seam would be
verifying that the type system holds, which is the compiler's job.

The core's `persist_noclobber_with_hook` is a different situation and the
comparison is worth drawing: there the race is between two filesystem
operations, so no type can exclude it and only a seam can prove the behaviour.
Here the race was between two *program states*, and the fix removed one of them.

Reaching for a test seam by analogy would have been the easy mistake.

## 4. Finding

### N1 — the i18n gate cannot see the strings that bypass it

`describe_block`'s output goes to `store.notify` without `t()`. Your reasoning —
that `handle_result`'s `Err(e) => store.notify(e.to_string())` is the established
precedent — is accurate. The problem is that the precedent is itself a gap.

G-006 requires "all user-visible strings routed through the translation layer;
English and Japanese with zero gaps." There are currently five user-visible error
paths that bypass it:

```
ui/view/diff_actions.rs:285          Err(e) => store.notify(e.to_string())
ui/overlay/modals/recovery.rs:142    Err(e) => store.notify(e.to_string())
ui/overlay/modals/recovery.rs:252    Err(e) => store.notify(e.to_string())
state/compare.rs:154                 store.notify(error.to_string())
ui/view/diff_actions.rs              describe_block(...)  ← this patch
```

And `cargo xtask i18n` reports **pass — 220 keys** throughout, because it
compares `t(...)` call sites against the Japanese map. It is structurally blind
to strings that never reach a `t(...)`.

That is the same shape as three earlier findings in this project: `version-sync`
could not detect a workspace version equal to a published tag, `css_coverage`
could not see layout, and the release workflow's gates had never run. In each
case a green gate was credited with more than it measured. "i18n passes" is
evidence that routed strings are covered — not evidence of G-006 conformance.

**Registered as F39.** The decision — translate the error path, or narrow G-006's
wording to match reality — is a product one, not this patch's. Matching the
existing pattern here was the smaller change and the right call for a correction
cycle; flagging it rather than quietly extending it is what makes that
defensible.

## 5. Notable quality observations

- Naming what C1's fix does **not** claim, and offering to add the seam if
  wanted rather than asserting sufficiency, is the right way to hand a judgement
  back. It is also what let me answer it cleanly.
- Using a real external modification rather than a synthetic conflict for the
  runtime evidence means the path exercised is the one users hit.
- Flagging `describe_block`'s i18n status as a deliberate match "in case this
  reads as an oversight" is exactly the disclosure that turns a silent
  divergence into a decision.

## 6. Recommended next action

1. Treat C1 and C2 as closed.
2. **F38** (permission/umask) before M3 closes.
3. Patch 5 — target-transition integration tests, presentation, Save As
   confirmation UX.
4. **F39** at M4 with the other documentation-and-gate truth items.
5. F23 still gates M2's cut.
