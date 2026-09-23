# Developer Handoff 014 — F86: dirty state must mean "differs from what was saved"

**From:** architect
**Date:** 2026-09-01
**Register:** F86 (Critical). **Governing: RFC-082 §D1** (accepted).
**Gate:** Release-blocking (audit blocker B5).

---

## 1. Task title

Replace undo-stack-depth dirty tracking with content identity, in both merge
sessions.

## 2. Purpose

`MergeSession::is_dirty()` is `undo_stack.len() != saved_baseline`, and
`mark_saved()` sets `saved_baseline = undo_stack.len()`. **Depth is not
identity.** Reproduced by the architect against the real session:

```
saved  = "a\nb\nc\nD\n"      // what mark_saved() recorded
buffer = "A\nb\nc\nd\n"      // what result_text() now returns
is_dirty() = false
```

Sequence: apply hunk A → save (depth 1) → undo (depth 0, correctly dirty) →
apply hunk B (depth back to 1) ⇒ **reported clean while the content differs.**

Every dirty guard reads this — the tab dot, the close prompt, Ctrl+W, the reload
and swap confirmations — and `disabled: !snap.is_dirty` gates the **Save button
itself**. So the user can see a change, cannot save it, and closing does not
warn. `ThreeWayMergeSession` has the identical predicate.

`apply → save → undo → apply` is an ordinary correction workflow, not a contrived
one. That is why this ranks above the other Critical.

## 3. Required implementation — RFC-082 §D1

**Record what was saved, not how deep the stack was.** `mark_saved()` captures
the identity of `result_text()`; `is_dirty()` compares the current
`result_text()` against it.

Both `MergeSession` and `ThreeWayMergeSession`.

**A monotonic revision counter is rejected, and the reason is a requirement, not
a preference.** A counter reports *dirty* after the user undoes back to exactly
the state they saved — the buffer on screen is byte-identical to the file on
disk and the app insists there is unsaved work. That is the safe direction, so it
would pass a naive test; it contradicts what the user sees, which RFC-082 §P2
makes a specification. **If you implement a counter, this handoff is not met.**

**Choose the identity representation yourself and argue it.** A hash of
`result_text()` is the obvious candidate; storing the string itself is simpler
and costs memory proportional to the file, which matters given the audit's
measured ~9× amplification (F95). State what you chose and what it costs.

**Watch `mark_saved()`'s redo clear.** It currently clears `redo_stack` because
"redo across a save boundary would desynchronize the baseline". Once the baseline
is content, that reason evaporates — but **do not remove the clear as part of
this change** unless you can show it is safe; it is a behaviour change with its
own user-visible effect, and it is not what this handoff is for. Flag it either
way.

## 4. Explicit non-change scope

- **F85 (`swap_sides` / `save_target`)** — handoff 015. Do not touch
  `state/tab.rs`.
- **F87/F88 (encoding, save capability)** and **F89 (temp file)** — later
  handoffs.
- No UI changes. The Save button's `disabled: !snap.is_dirty` is *correct* once
  `is_dirty` is; leave it alone.
- `ROADMAP.md`, the RFCs — not yours.

## 5. Required tests

Falsified against the shipped defect, not a helper the fix introduces.

1. **The exact reproduction above** — apply, save, undo, apply a different hunk;
   assert `is_dirty()` and assert `result_text() != saved`. **Falsify by
   restoring `undo_stack.len() != saved_baseline`**; the test must fail.
2. **The same for `ThreeWayMergeSession`.** It has the identical predicate and
   must not be left behind — that is the likeliest way half this fix ships.
3. **Undo back to the saved state reports clean.** apply → save → apply → undo
   ⇒ `is_dirty() == false`. **This is the test a revision counter fails**, and it
   is why §3 rejects one. It is also the P2 case: the screen matches the disk.
4. **Existing dirty-state tests still pass**, unedited. If one needs changing,
   stop and say so — it may be asserting the defect.

## 6. Acceptance criteria

- Restoring the depth comparison fails at least two tests.
- Both sessions fixed.
- Save is enabled whenever the buffer differs from the saved content.
- Gates green: `fmt`, `clippy --workspace --all-targets -- -D warnings`,
  `test --workspace`, `xtask css --check`, `xtask version-sync`, `xtask i18n`,
  `xtask rfc-sync`, `git diff --check`.

## 7. Prohibited shortcuts

- **Do not implement a revision counter.** §3.
- **Do not fix only `MergeSession`.**
- **Do not edit an existing test to make it pass.**
- **Do not report a falsification you did not run.**

## 8. Required review-request format

Lead with the falsification of §5.1 and its output.

State plainly:
- what you stored as the saved identity, and what it costs;
- that `ThreeWayMergeSession` is fixed and how you showed it;
- whether any existing test needed editing (the answer should be no);
- your reading of `mark_saved()`'s redo clear — kept, and why.
