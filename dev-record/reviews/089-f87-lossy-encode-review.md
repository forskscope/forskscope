# Review 089 — Request 086: F87, lossy-encode guard

**Reviewer:** architect
**Date:** 2026-09-01
**Reviewed:** `c8aaeca`, against baseline `55d965d`
**Verdict:** **Approved. F87 is closed** — B5's last blocking *corruption*
defect. One blocking item remains in B5: **F88a**, handoff 018.

## 1. Verified independently

**The ordering.** I moved the refusal after the backup step and ran the test:

```
assertion failed: the backup step must never run for a refused save — a later
refusal would already have destroyed the user's prior backup for a save that
never happens
```

Exactly as designed: the **target-untouched** assertion in that same test passed
either way, and **only the `.bak` assertion** caught it. That is why the handoff
named it, and it is now the thing standing between a future reorder and a
destroyed backup.

**The fast path.** I added a scan call to `encode_text`'s success path and
`encode_text_success_path_never_calls_the_unmappable_scan` **fails**. The
guarantee is enforced, not argued — and the counter is `#[cfg(test)]`-gated, so
production pays nothing.

**The subset test is extended, not loosened** — four kinds against four actions,
the original three of each untouched exactly as review 083 left them.

Workspace green: core 716, ui 87, ui-logic 200. Your `/tmp`-full note was
environmental — 30 G free here now, and the numbers match yours.

## 2. The thread-local counter is a better instrument than I asked for

Handoff §7 test 3 allowed "if you cannot express that as a test, say so." You
could, and the choice of a **thread-local** over a process-global `AtomicUsize` is
the part worth naming: a reset-then-check test against a global counter is flaky
the moment another test touches the same path on another thread, and it would
fail *intermittently*, which is worse than not existing.

You avoided a test that would have been abandoned as unreliable within a month.

## 3. §5 — you found that the taxonomy could not say anything specific

This is the substantive discovery. `AppError::from_core` built messages purely
from `UserMessage::for_kind(kind)` — a static per-kind template with **no path
for per-instance data**. So nothing in this codebase had ever put a real
character or filename into a dialog body, and the pre-existing `DecodeLossy`
could not have either, had anything constructed it.

Adding `for_core_error(kind, err)` — falling through to the static template for
every other case — is the smallest change that makes §5's three requirements
expressible at all. Without it, "name the characters" could not have been
implemented, only approximated.

The resulting message does the job:

> `'😀', '🎉', and 3 more cannot be represented in shift_jis. Save as UTF-8 to
> keep them, or go back and edit the file to remove them.`

Characters, encoding, and both escapes in one sentence — and asserted piecewise,
not merely for non-emptiness.

## 4. Two judgement calls I agree with

**The cap of five.** Fixed rather than content-dependent, with the overflow
counted and deduplication proved separately. A dialog is meant to be read; five
names is already at the edge.

**`SaveAsUtf8` updates nothing special.** It re-derives through `build_request`'s
normal path and overrides only the label, so `handle_result`'s existing success
arm picks the new encoding up on its own. **No special-cased "and also update the
encoding" step** — which is exactly right: a bespoke update path is where the two
sources of truth this whole RFC exists to eliminate would have grown back.

That the save target's label sticks is asserted, not assumed, which is what makes
"does not block again" true rather than "saved once".

## 5. Status

F87 closed. **B5's remaining blocking item is F88a** — `requires_save_guard()` is
still unwired, so a file that decoded with replacement characters is still
saveable without a guard. Handoff 018 covers it, with F88b riding along.
