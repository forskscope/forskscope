# Review Request 083: F86 — dirty state must mean "differs from what was saved"

**Governing task.** `dev-record/handoffs/014-f86-content-identity-dirty-state.md`
**Register.** F86 (Critical, audit blocker B5). Governing: RFC-082 §D1.
**Baseline.** `main` at `498448d` (docs: review 085 - F92's two control claims closed)
**Commit.** `3720c5d`

## 1. Falsification of §5.1 — the exact reproduction

Reproduced against both sessions with a real fixture (two independent hunks / two independent conflicts), not a synthetic one: apply A → save → undo → apply a *different* B. `undo_stack.len()` returns to exactly the depth it was at save time, but the content is not what was saved.

Temporarily restored `undo_stack.len() != saved_baseline` (re-adding the field alongside the new hash, so both existed) in `MergeSession`, ran `cargo test -p forskscope-core tests::merge_tests --lib`:

```
test tests::merge_tests::f86_same_depth_as_save_point_but_different_content_is_dirty ... FAILED
...
thread '...' panicked at .../merge_tests.rs:...:
undo-stack depth matches the save point again, but the content differs — depth is not identity
test result: FAILED. 18 passed; 1 failed
```

Restored the real code (hash-based `is_dirty`); all 19 pass again. Did the identical restore-and-revert in `ThreeWayMergeSession`, same result: exactly `f86_same_depth_as_save_point_but_different_content_is_dirty` failed (20 passed, 1 failed), nothing else moved.

## 2. What I stored as the saved identity, and what it costs

**FNV-1a 64-bit hash of `result_text()`** — `saved_hash: u64` on both structs, computed at construction (so a freshly loaded session with no edits is clean) and recomputed in `mark_saved()`.

Chose the hash over storing the string: **8 bytes per session, fixed, regardless of file size** — versus a stored copy of the full text, which the handoff itself flags matters given F95's measured ~9× memory amplification; a second full-text copy per session would make that worse, not better. This is also not a new mechanism: `three_way::session::conflict_id_for` already trusts the same FNV-1a-64 hash for conflict identity in this exact file, so `MergeSession` and `ThreeWayMergeSession` now use one hash function for the same purpose, consistently.

**Cost disclosed honestly, not glossed over:** a 64-bit hash carries a theoretical collision — an edit that happens to hash-collide with the saved content would be misreported as clean. At 2^-64 for any single comparison, and given `is_dirty()` is queried on every render rather than once, this is the same order-of-magnitude risk this codebase already accepts for conflict identity, not a new one introduced here.

## 3. `ThreeWayMergeSession` is fixed, and how I showed it

Identical predicate, identical fix, identical fixture shape (two independent conflicts instead of two independent hunks) — falsified exactly as in §1 above, with its own failing test name and output shown there. Both `f86_same_depth_as_save_point_but_different_content_is_dirty` and `f86_undo_back_to_exact_saved_content_is_clean` exist in both `merge_tests.rs` and `three_way_tests.rs`.

## 4. No existing test needed editing

Confirmed: `git diff` on `merge_tests.rs` and `three_way_tests.rs` is purely additive (75 and 62 new lines respectively, 0 deletions in either). Ran the full pre-existing suite for both — `mark_saved_clears_dirty_state`, `dirty_tracks_resolution_and_save_baseline`, `session_dirty_after_apply_clean_after_mark_saved`, `undo_and_redo_restore_exact_state`, `session_from_identical_diff_has_zero_pending_changes`, and every other test in both files — all pass unedited, both before and after the falsification round-trips. None of them happened to exercise the specific depth-vs-identity gap (they don't apply-save-undo-apply-different), which is exactly why the defect shipped in the first place.

## 5. `mark_saved()`'s redo clear — kept, and why

**Kept, in both sessions.** The stale justification in the comment ("redo across a save boundary would desynchronize the baseline") no longer strictly holds once the baseline is content rather than depth — a redo replayed after a save still lands on the same content it always would have, so nothing desynchronizes in the sense the old comment meant.

But I did not remove it, per §3's explicit instruction. Reasoning for *why not*, beyond just following the instruction: removing it is a distinct, user-visible behavior change (redo becomes available across a save boundary) with its own failure mode worth thinking through on its own — e.g. a user saves, makes an *unrelated* edit, undoes that edit, and now has a stale redo entry from before the save available to reapply, potentially interacting with hunks that have since changed state. That is a real question, not a hypothetical one, and it deserves its own review rather than riding along inside a dirty-state fix. Left the `redo_stack.clear()` call in both `mark_saved()` implementations, with a comment in `MergeSession` explaining the reasoning and a pointer to it from `ThreeWayMergeSession`.

## 6. Scope discipline

- `state/tab.rs` — untouched (F85 is handoff 015).
- F87/F88 (encoding, save capability), F89 (temp file) — untouched.
- No UI change: `disabled: !snap.is_dirty` on the Save button is untouched, per the handoff's own reasoning that it's already correct once `is_dirty` is.
- `ROADMAP.md`, RFCs — not touched by this commit.
- Diff touches exactly 4 files: `merge/session.rs`, `merge/three_way/session.rs`, and their two test files.

## 7. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo test --workspace` (701 core tests, up from 697 — the 4 new F86 tests; all other crates unchanged), `cargo xtask css --check`, `cargo xtask version-sync`, `cargo xtask i18n` (236 keys, unchanged), `cargo xtask rfc-sync`, `git diff --check`. Pushed as `3720c5d`; CI dispatched on push (run 33476162502).
