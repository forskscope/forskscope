# Developer Handoff 015 — F85: Save must write to the file the user is editing

**From:** architect
**Date:** 2026-09-01
**Register:** F85 (Critical). **Governing: RFC-082 §D2 and §D6** (accepted).
**Gate:** Release-blocking (audit blocker B5).

---

## 1. Task title

Maintain `save_target` across every mutation of the compared inputs, and show the
user where Save will write.

## 2. Purpose

`swap_sides` (`state/tab.rs:100-101`) swaps `left_doc`/`right_doc` and
`left_path`/`right_path`, recomputes `can_save`, recomputes the diff and persists
the session. **It never touches `tab.save_target`.**

For a normal save, `build_request`'s `None` arm takes the destination path, the
precondition **and** the encoding label *solely* from `save_target`.

So after swapping A and B: the merge result reflects A's side, `save_target`
still names B, and B's `MustMatch` fingerprint still matches disk **because B was
never touched** — so no conflict dialog fires. B is backed up to `B.bak` and
overwritten with A's content while the user believes they are editing A.

**This project already wrote the invariant down.** `commit_load_result`
(`state/compare.rs:98-102`):

> `PreparedCompare` commits `left_doc`/`right_doc`/`diff`/`merge`/`can_save`/
> `save_target` **together** (RFC-077) … extended so a save target is never
> installed from a different load than the documents it was derived from.

The load path honours it. `swap_sides` mutates the same fields outside that path
and does not. **That is the whole defect** — not a missing feature, a bypassed
invariant.

## 3. Required implementation — §D2

**`save_target` is a function of `save_destination`, re-derived exactly when the
inputs it derives from change.**

- `SaveDestination::RightInput` → `save_target_from_loaded(&right, &right_doc)`
- `SaveDestination::Explicit(merged)` → `$MERGED`, **independent of both panes**

In `swap_sides`, that means: re-derive in `RightInput` mode; **do nothing in
mergetool mode**, because `$MERGED` did not change.

### 3a. Do not re-derive unconditionally — this is a trap, not a style note

Calling `inspect_save_target($MERGED)` on a swap would refresh that target's
`MustMatch` fingerprint **against a file this tab has not re-read**. If `$MERGED`
changed on disk since load, the refresh silently adopts the new state as expected
— destroying external-modification detection for the one file the mergetool
contract exists to protect.

Derive from `save_destination` and act only when its input changed. An earlier
draft of RFC-082 said "refuse the swap in mergetool mode" instead; that was
withdrawn because it adds a restriction to cover a missing invariant, and
swapping there is legitimate — *resolve using LOCAL as the base*.

### 3b. The invariant, not the call site

`swap_sides` is the only current violation — I checked: the only other mutations
of `right_path`/`right_doc` are in `commit_load_result`, which already commits
`save_target` alongside them.

**Write the test against the invariant, not against `swap_sides`.** A test that
only covers swap will not catch the next function that mutates a pane. If you can
express "after any mutation of the compared inputs, `save_target` agrees with
`save_destination`" as a single assertion helper used by both the swap test and
the load test, do that.

## 4. Required implementation — §D6

**Show the resolved save target in the status bar.**

`save_target` reaches the UI in exactly one place today — prefilling the Save As
dialog — and is **never displayed**. That is why F85 could exist *and stay
silent*: a wrong destination looks identical to a right one until the file is
overwritten.

- The status bar (`ui/layout/statusbar.rs`, `role="status"`, passive context) is
  the right home; it already carries the file pair, encoding and dirty marker.
- **Show it always, not only when it differs from the right pane.** If it
  appeared only on disagreement, its absence would carry meaning the user has to
  know to read. Always-present confirms the instinct *"Save writes to the right
  pane"* in the normal case and is the only thing that reveals `$MERGED` in
  mergetool mode, where the destination appears in neither pane.
- Match the bar's existing style: short name in the bar, full path in the
  tooltip.
- **Display only.** No control, no editing from the status line — Save As exists.

## 5. Explicit non-change scope

- **F87/F88 (encoding guard, save capability)** — handoff 017. `can_save`'s
  pair-wide expression is wrong and is **not yours to fix here**.
- **F89 (temp file)** — handoff 016.
- `build_request`, `handle_result`, the conflict arm — untouched.
- `ROADMAP.md`, the RFCs — not yours.

## 6. Required tests

Falsified against the shipped defect.

1. **After `swap_sides`, `save_target.path` equals the new right path**, and a
   save writes there. Drive the real `Store` with `with_test_store`, as the audit
   did. **Falsify by removing the re-derive** — the test must fail.
2. **In mergetool mode, `swap_sides` leaves `save_target` unchanged** — same
   path, **and the same expectation/fingerprint**. Assert the fingerprint too;
   asserting only the path would pass while §3a's trap is present.
3. **The invariant holds after a load and after a reload**, via the same helper
   as (1). These should already pass — if one fails, tell me, because it means
   the load path is not as sound as its comment claims.

For §D6, a unit test asserts the resolved target is what the status bar renders.
Whether it *reaches the screen* is a P07-class question, as recorded for F74 —
do not claim otherwise.

## 7. Acceptance criteria

- Removing the re-derive fails a test.
- Mergetool mode: swap changes nothing about `save_target`, fingerprint included.
- The save destination is visible in the workspace at all times.
- No `can_save` change, no encoding change, no temp-file change.
- Gates green: `fmt`, `clippy --workspace --all-targets -- -D warnings`,
  `test --workspace`, `xtask css --check`, `xtask version-sync`, `xtask i18n`,
  `xtask rfc-sync`, `git diff --check`.

## 8. Prohibited shortcuts

- **Do not re-derive unconditionally.** §3a.
- **Do not refuse the swap in mergetool mode.** Withdrawn deliberately.
- **Do not test only `swap_sides`.** §3b.
- **Do not report a falsification you did not run.**

## 9. Required review-request format

Lead with §6.1's falsification and its output.

State plainly:
- how you expressed the invariant, and whether one helper covers swap and load;
- that the mergetool fingerprint is asserted unchanged, not just the path;
- what the status bar shows in normal mode versus mergetool mode.
