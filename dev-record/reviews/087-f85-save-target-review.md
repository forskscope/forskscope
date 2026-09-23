# Review 087 — Request 084: F85, save-target invariant

**Reviewer:** architect
**Date:** 2026-09-01
**Reviewed:** `c94638a`, against baseline `9b18165`
**Verdict:** **Approved. F85 is closed** — B5's second Critical. One observation
(§4), no action required.

## 1. Verified independently

Both falsifications re-run here, and each fails **exactly one** test:

- removing `refresh_save_target` ⇒
  `swap_sides_rederives_save_target_from_the_new_right_input`
- making the re-derive unconditional (§3a's trap) ⇒
  `swap_sides_leaves_save_target_untouched_in_mergetool_mode`

Workspace suite green across all 11 targets (core 701, ui 86, ui-logic 199);
`fmt`, `clippy`, `css --check`, `i18n` (237 keys) clean.

The implementation is four lines and the right four: `Normal` re-derives,
`MergeTool` is untouched because the `if let` simply does not match. No mode
flag threaded anywhere, no branch at the call site.

## 2. §3 is the best thing in this request

You did not stop at "the mergetool test passes." You **falsified the test** — ran
the trap and showed that `path` was **identical in both cases**
(`/tmp/f85-merged.txt`), and that only the expectation moved, from
`MustBeAbsent` to `MustMatch(...)`.

So an assertion on `path` alone — the obvious thing to write, and what I would
have accepted — **would have passed silently while §3a's trap was live.** The
test earns its keep because of the fingerprint assertion, and you proved that
rather than asserting it.

That is the standard this program has been building toward, applied to a test
rather than to code. It is also the only way anyone could know the mergetool test
is not vacuous.

## 3. Two judgement calls, both right

**One helper does not cover mergetool, and you did not force it to.** Normal mode
has a *formula* (`save_target == save_target_from_loaded(right_path, right_doc)`);
mergetool mode has an *identity* ("unchanged"), and there is nothing to recompute
it against without the disk I/O §3a forbids. Two invariant shapes, two assertion
shapes. Generalising the helper past where it fits would have produced a worse
test that looked tidier.

**§6.3 passed, as predicted.** `save_target_matches_right_input_after_load_and_reload`
confirms the load path's own comment about committing `save_target` alongside the
documents. Worth stating: that comment is now backed by a test rather than by
discipline.

**D6 verified against the running application.** You built the binary, opened a
compare and screenshotted it:
`left.txt ↔ right.txt   UTF-8   +1/-1   Save target: right.txt   🔒 Local only`.
The handoff asked for a unit test; you confirmed the pixels. And you correctly
did **not** claim the accessibility tree, which is P07's question — the same
limit recorded for F74.

## 4. One observation — not reachable today, and the shape is familiar

`refresh_save_target` does `tab.right_path.clone().unwrap_or_default()`. If
`right_path` were `None`, that yields an **empty path**, and
`save_target_from_loaded` classifies by `doc.kind` rather than by the path — so
an empty path with `FileKind::Text` would produce a `Writable` save target
pointing at `""`.

**It is not reachable.** I checked: `CompareTab` has exactly one construction
site (`compare.rs:341`) and it sets both paths to `Some`. So this is
dead-defensive code, not a defect, and I am not asking you to change it now.

Recording it because the *shape* is the one this project keeps finding: a silent
fallback that converts "absent" into a plausible-looking value, in a save path.
`if let Some(right_path) = &tab.right_path` would decline instead of inventing,
costs nothing, and is what I would write if this function is ever touched again.

## 5. Status

**F85 closed.** B5's two Criticals are both closed.

Remaining in B5: **F89** (secure atomic write, handoff 016) and **F87 + F88a**
(lossy-encode guard and save capability, handoff 017). F88b — the pair-wide
`can_save` that prevents restoring a deleted file — rides with 017 and does not
block.
