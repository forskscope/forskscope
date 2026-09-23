# Review Request: RFC-077 Patch 3 — Normal Compare Migration

**Date:** 2026-08-04
**Reviewer stance:** focused implementation review
**Repository baseline:** `064584f` (`ui: RFC-077 patch 3 - normal compare
returns PreparedCompare`), on top of `5500b81` (review 046 N1 fix, folded in
as instructed) and the `043`/`fe234f5` checkpoint
**Governing documents:** `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`;
RFC-077

## Summary

Patch 3 per the sequence: `state/compare.rs`'s blocking `load_and_diff`
now builds and returns core's `PreparedCompare` instead of an ad hoc
5-tuple/`LoadedComparison` struct, and `commit_load_result` installs
`save_target` in the same RFC-075 token-gated commit as
`left_doc`/`right_doc`/`diff`/`merge`/`can_save`. `CompareTab` gains
`save_target: Option<SaveTargetSnapshot>`. Normal two-file compare's target
is derived from the already-loaded right document (no new I/O). Proves
unchanged behavior: `diff_actions.rs`, `app.rs`, and `main.rs` are untouched
— save still reads `right_doc.fingerprint_at_load` exactly as before.

N1 from review 046 was folded in first, as its own commit (`5500b81`), before
this patch — `check_precondition`'s `MustBeAbsent` now uses
`fs::symlink_metadata` and propagates genuine read failures as `Io` instead
of silently treating them as absent.

## Report: §3.4 (`StartupRequest`/`CompareRequest` placement)

**The test doesn't fire at patch 3 — it fires at patch 4.** Patch 3's scope
(per the handoff's own patch boundary: "normal compare migration, proving
unchanged behaviour") never constructs or consumes a `CompareRequest`
anywhere — `open_compare`/`reload_tab`/`load_and_diff` still take
`(left: PathBuf, right: PathBuf)` directly, exactly as before this patch.
`git diff` for this commit contains zero occurrences of `CompareRequest` or
`StartupRequest`. Confirmed by grep, not by omission.

The actual test — whether `CompareRequest`'s fields get destructured one at a
time into a core function, or whether the UI keeps genuinely orchestrating
between the two types — can only be observed once `main.rs`/`app.rs` are
migrated onto `StartupRequest`/`CompareRequest` and `open_compare` is
changed to accept one. That's patch 4. I'll report the outcome there rather
than guess now.

## Files Changed

- `crates/forskscope-ui/src/state/compare.rs` — `load_and_diff` returns
  `Result<PreparedCompare, String>`; `LoadedComparison`/the 5-tuple return
  type are gone; `commit_load_result` installs `save_target`.
- `crates/forskscope-ui/src/state/tab.rs` — `CompareTab.save_target:
  Option<SaveTargetSnapshot>`.
- `crates/forskscope-ui/src/state/compare/tests.rs` — `ready_result()`'s test
  helper updated to build a `PreparedCompare`; 6 new tests (below).

## Design Decisions

- **`save_target` is `None` only while `TabState::Loading`.** Once a tab
  reaches `Ready` or `Error`, a `Ready` tab always has `Some(save_target)` —
  installed atomically with the other four fields, never independently. An
  `Error` tab has no meaningful target to report, so it stays `None` from
  `open_compare`'s initial construction (never overwritten on the error
  path).
- **`save_target_from_loaded(&right, &rd)` is called before `rd` moves into
  `PreparedCompare.right`.** `right: PathBuf` is only borrowed by that call
  (the function needs the path for `SaveTargetSnapshot.path`, not to read the
  file again — `rd`'s already-loaded fingerprint/encoding supply everything
  else), so no ownership conflict; confirmed by the compiler, not just by
  reading.
- **Did not add `CompareLaunchMode` to `CompareTab` in this patch.** RFC-077's
  tab-state code block lists it alongside `save_target`, but nothing reads it
  until mergetool tabs exist (patch 4). Adding an enum with only one
  meaningful variant (`Normal`) felt like it would just be dead weight until
  `MergeTool` gives it a second one — flagging this omission explicitly in
  case you'd rather it land now for a smaller patch 4 diff.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1069 (682+27+16+2 core, 267 ui-logic, 70 forskscope-ui, 6 css, 7 doctests), exactly 1063 + 6
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 220 keys (unchanged)
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
```

CI run `30880930331`: Test & Lint green, on `064584f`.

**Test-count delta:** +6, all in `crates/forskscope-ui/src/state/compare/tests.rs`
(counted twice — lib + bin targets — so +12 in the raw `cargo test
--workspace` total, +6 distinct tests):

| Test | Covers |
|---|---|
| `normal_compare_save_target_is_the_right_input_must_match_its_own_fingerprint` | Existing right → `Writable(MustMatch)` with the right file's own fingerprint |
| `normal_compare_save_target_is_must_be_absent_when_right_is_missing` | Missing right → `Writable(MustBeAbsent)` |
| `normal_compare_can_save_and_diff_are_unaffected_by_the_prepared_compare_refactor` | `can_save`/`diff.hunks` still populate correctly through the new return type |
| `binary_comparison_disabled_error_message_is_unchanged` | Existing error path, byte-for-byte message |
| `binary_vs_text_mismatch_error_message_is_unchanged` | Existing error path, byte-for-byte message |
| `xlsx_target_error_message_is_unchanged` | Existing error path, byte-for-byte message |

The three error-message tests exist because `load_and_diff` had **no direct
unit test before this patch** — every existing test in
`state/compare/tests.rs` exercised `commit_load_result`'s token/generation
logic via a synthetic `ready_result()`, never the real loading/validation
function. Worth naming: "proving unchanged behaviour" for a function with
zero prior direct coverage means these three tests are new coverage, not
regression tests in the strict sense — I read the pre-patch source to
confirm the exact strings, rather than relying on the refactor alone to
prove equivalence.

## Not Addressed Here

- Mergetool startup migration, `save`/`save-as`/`overwrite`/`reload` routed
  through `save_target` — patch 4.
- `CompareLaunchMode` — see Design Decisions above; deferred, flagged for a
  decision.
- Runtime evidence — nothing user-facing changed in this patch; the handoff's
  runtime-evidence requirement applies to patch 4's mergetool path.

## Recommended Next Step

Confirm the §3.4 deferral to patch 4 is acceptable, and whether patch 4
(mergetool startup + save-path migration — the patch that actually closes
B3 and needs runtime evidence per the handoff §6) should proceed directly or
wait for another checkpoint given its size and risk.
