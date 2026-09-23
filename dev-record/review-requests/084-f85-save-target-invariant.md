# Review Request 084: F85 — save_target must track save_destination

**Governing task.** `dev-record/handoffs/015-f85-save-target-invariant.md`
**Register.** F85 (Critical, audit blocker B5). Governing: RFC-082 §D2 and §D6.
**Baseline.** `main` at `9b18165` (docs: F85 handed off, handoff 015, with RFC-082's D6)
**Commit.** `c94638a`

## 1. Falsification of §6.1 — the exact reproduction

Temporarily removed the `refresh_save_target(tab)` call from `swap_sides`, ran `cargo test -p forskscope-ui state::tab::tests --lib`:

```
test state::tab::tests::swap_sides_rederives_save_target_from_the_new_right_input ... FAILED
...
thread '...' panicked at .../tab/tests.rs:168:9:
assertion `left == right` failed: save_target.path must follow the swap to the new right input, not remain pinned to /tmp/f85-right.txt
  left: Some("/tmp/f85-right.txt")
 right: Some("/tmp/f85-left.txt")
test result: FAILED. 10 passed; 1 failed
```

Restored; all 11 pass again.

## 2. How I expressed the invariant, and the shared helper

"`save_target` is a function of `save_destination`, re-derived exactly when the inputs it derives from change" (§3) is a `Normal`-mode-only formula: `save_target == save_target_from_loaded(right_path, right_doc)`. I wrote that as one function, `assert_save_target_matches_right_input(tab: &CompareTab)` (`state/tab.rs`, `#[cfg(test)] pub(crate)`), used by:

- `swap_sides_rederives_save_target_from_the_new_right_input` (`tab::tests`) — after a swap.
- `save_target_matches_right_input_after_load_and_reload` (`compare::tests`) — after two calls to `load_and_diff` with genuinely different right-side content, standing in for "load" then "reload" (both go through the identical function; testing it directly avoids driving `spawn_forever`'s async task to completion through a full `Store`).

Both pass, confirming §6.3's expectation ("these should already pass") — the load path's own comment about committing `save_target` alongside the documents it's derived from holds.

**One helper does not cover the mergetool case**, and I did not force it to: mergetool mode's invariant is "unchanged", not "equals a formula" — there's nothing to recompute it against without disk I/O, which is exactly what §3a says not to do. `swap_sides_leaves_save_target_untouched_in_mergetool_mode` instead asserts full `SaveTargetSnapshot` equality (`Some(&original_save_target)`) between before and after the swap. Two different shapes of assertion for two different invariants, not one generalized past where it fits.

## 3. §3a's trap — the fingerprint is asserted, and I proved the assertion catches what path-only wouldn't

To confirm the mergetool test is doing real work, not just checking a field that happens not to move: temporarily made `refresh_save_target` call `inspect_save_target($MERGED, ...)` unconditionally, including in mergetool mode (simulating exactly the trap §3a warns against). Ran `cargo test -p forskscope-ui state::tab::tests --lib`:

```
test state::tab::tests::swap_sides_leaves_save_target_untouched_in_mergetool_mode ... FAILED
...
thread '...' panicked at .../tab/tests.rs:209:9:
assertion `left == right` failed: mergetool mode: swap must not touch save_target at all — path, expectation, and fingerprint must all stay byte-identical
  left: Some(SaveTargetSnapshot { path: "/tmp/f85-merged.txt", state: Writable { expectation: MustBeAbsent, encoding_label: "UTF-8" } })
 right: Some(SaveTargetSnapshot { path: "/tmp/f85-merged.txt", state: Writable { expectation: MustMatch(FileFingerprint { len: 99, ... }), encoding_label: "UTF-8" } })
test result: FAILED. 10 passed; 1 failed
```

The `path` field was identical both times (`/tmp/f85-merged.txt` didn't exist, so `inspect_save_target` classified it `Missing`/`MustBeAbsent`) — an assertion on `path` alone would have passed here, silently. Only the fingerprint/expectation moved, and that's what the test catches. Restored the real (Normal-only) `refresh_save_target`; both tab tests pass again.

## 4. Status bar — normal mode vs. mergetool mode

Added a unit-testable `save_target` display to `StatusBar` (`ui/layout/statusbar.rs`): a `span.save-target` reading `"Save target: {short}"` where `{short}` is `save_target.path`'s file name, with the full path in the element's `title` attribute (tooltip). Shown unconditionally whenever `tab.save_target` is `Some` — same in both launch modes, since the component only ever reads `tab.save_target`, not `launch_mode`. In `Normal` mode this reads the right pane's own name (redundant-looking but confirms the instinct, per §4); in `MergeTool` mode it's the only place `$MERGED` appears anywhere in the UI, since it names neither compared pane.

New i18n key `"Save target"` → `"保存先"`, added to `crates/forskscope-ui/src/i18n.rs`; `cargo xtask i18n` passes at 237 keys (was 236). New CSS rule `.statusbar .save-target { color: var(--muted); }` added to the split source and regenerated via `cargo xtask css`.

**Verified visually, not just asserted**: built the real debug binary, opened a two-file compare (`left.txt ↔ right.txt`), screenshotted the running window via `niri msg action screenshot-window`. The status bar reads `left.txt ↔ right.txt   UTF-8   +1/-1   Save target: right.txt   🔒 Local only` — confirmed against the actual rendered UI, not inferred from the component code. (Whether it reaches the accessibility tree is a P07-class AT-SPI question, not claimed here — same limit recorded for F74.)

## 5. Scope discipline

- `build_request`, `handle_result`, the conflict arm (`diff_actions.rs`) — untouched.
- `can_save`'s pair-wide expression (F87/F88) — untouched, not this handoff's.
- No temp-file change (F89) — untouched.
- `ROADMAP.md`, RFCs — not touched by this commit.
- Diff: `state/tab.rs` (+ its `tests.rs`), `state/compare/tests.rs` (test only, no `compare.rs` production change), `ui/layout/statusbar.rs`, `i18n.rs`, and the CSS split source + generated `main.css`.

## 6. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo test --workspace` (all green: 86 `forskscope-ui` lib tests, up from 83 — 3 new F85 tests; all other crates unchanged), `cargo xtask css --check`, `cargo xtask version-sync`, `cargo xtask i18n` (237 keys), `cargo xtask rfc-sync`, `git diff --check`. Pushed as `c94638a`; CI dispatched on push (run 33506764679).
