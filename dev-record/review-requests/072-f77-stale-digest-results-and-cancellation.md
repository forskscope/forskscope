# Review Request 072: F77 — stale digest results, and a comparison that cannot be interrupted

**Governing task.** `dev-record/handoffs/004-f77-stale-digest-results-and-cancellation.md`
**Register.** F77 (fixed here). F74, F75, F76 — untouched.
**Baseline.** `main` at `b4a5d1e`
**Commit.** `98dde50`

## The two falsifications, first

### §8.1 — the generation guard

```
thread 'ui::view::explorer::tests::a_stale_generation_result_does_not_mutate_the_map' panicked at crates/forskscope-ui/src/ui/view/explorer.rs:636:9:
a stale-generation result must not mutate the map
```

Broken by temporarily deleting the `if result_generation == current_generation` check inside
`apply_digest_result` (so it always inserts). Restored; the suite is green.

### §8.2 — the in-loop cancellation poll

```
thread 'tests::dir_cancel_tests::file_digest_equal_with_cancel_stops_early_and_says_so' panicked at crates/forskscope-core/src/tests/dir_cancel_tests.rs:165:5:
assertion `left == right` failed: a cancelled comparison must report Cancelled, not a completed verdict
  left: Equal
 right: Cancelled
```

Broken by temporarily removing the `if token.is_cancelled() { return Ok(DigestOutcome::Cancelled); }`
check from the read loop in `file_digest_equal_with_cancel` (so it runs to completion). Restored;
the suite is green.

**No existing test needed editing.** `file_digest_equal_compares_content` (`dir_tests.rs`) and every
test in `dir_cancel_tests.rs` pass unmodified — confirmed by running them both before and after this
change.

## 1. The chosen cancelled-comparison return shape, and why

`Result<DigestOutcome>` where `DigestOutcome` is `Equal | Different | Cancelled`, not a `bool` and
not folding `Cancelled` into the existing `Result<bool>`. A cancelled comparison establishes nothing
— collapsing it to `false` (→ `Different`) would be a smaller copy of defect (a): a status asserting
more than was measured. Making it a third enum variant rather than a documented convention is what
the handoff's acceptance criterion asks for literally — "distinguishable at the type level, not by
convention." A caller that pattern-matches `Result<DigestOutcome>` cannot accidentally treat
`Cancelled` as `Different`; the compiler makes them handle it.

`file_digest_equal` becomes a thin wrapper: calls the cancellable variant with a fresh,
never-cancelled token and `unreachable!()`s on the `Cancelled` arm (a token nothing ever calls
`.cancel()` on cannot produce it) — the exact relationship `recursive_diff` already has to
`recursive_diff_with_cancel`.

## 2. `recursive_diff_with_cancel` no longer calls the uncancellable variant

`walk_and_merge` — the one caller that runs a full blocking comparison rather than the fast/display
listing — now calls `file_digest_equal_with_cancel(&left_path, &right_path, token)`. A `Cancelled`
outcome leaves that entry at `RecStatus::Computing` ("not yet complete," which is now literally
true) rather than asserting `Equal`/`Changed`. Updated `RecStatus::Computing`'s doc comment, which
previously claimed it was "never returned by `recursive_diff_with_cancel`" — that was true before
this change and is not anymore.

**An I/O error still collapses to `Changed`, unchanged from before this handoff.** The old code did
`.unwrap_or(false)`, mapping both a genuine error and (hypothetically) any other non-`Ok(true)`
outcome to `Different`. Preserving that for the error case (only) keeps this handoff's scope to
cancellation — an IO-error-classification bug is a different, already-registered finding (F76's
territory: a status asserting more than was measured, in the *safe* direction), not this one.

**Not independently tested at the `recursive_diff_with_cancel` level**, on purpose — see §5.

## 3. The generation guard (§7b)

`digest_generation: Signal<u64>` and `digest_token: Signal<CancellationToken>`, both changed at the
exact place `digest_map` is cleared: the token is cancelled, a fresh token replaces it, and the
generation increments. Each spawned comparison captures `my_generation = *digest_generation.read()`
and `token = digest_token.read().clone()` at spawn time (before the `await`), and applies its result
through `apply_digest_result` — extracted to take the map by value (`&mut HashMap<...>`, not a
`Signal`) specifically so §8.1's test could drive it without a Dioxus runtime.

**The guard runs even for outcomes the token didn't stop** — a completed `Equal`/`Different` result
is *also* subject to the generation check, per the handoff's explicit reasoning: cancellation is
racy, a comparison can finish in the window between the root change and the token being observed.
Only `Cancelled` skips `apply_digest_result` entirely (nothing to conditionally apply — the root
change that caused it already cleared the map).

## 4. Scope discipline

- No RFC-080 work — no tiers, no new status vocabulary beyond `DigestOutcome`/`Computing` (both
  already-established concepts, not new ones).
- No size bound or threshold added.
- No user-visible behavior change — every code path here changes *when* a result is discarded
  (never before, now correctly) or *whether* work continues past cancellation (never before, now
  it stops); nothing about what a completed, non-raced comparison shows changes.
- `file_digest_equal`'s signature and behavior unchanged (confirmed: its own test passes unedited).
- F74, F75, F76 untouched. `ROADMAP.md` not edited, per the handoff's explicit §9 instruction.

## 5. What was not done, and why

**No test drives `recursive_diff_with_cancel` end-to-end with real mid-file cancellation.** Considered
it, following review 072's own lesson (test the real call site, not just a helper) — but concluded
it would be the wrong kind of test, not a missing one. `dir_cancel_tests.rs`'s existing
`cancel_during_scan_produces_partial_results_without_panic` already establishes this file's accepted
pattern for that: a background thread, a short sleep, then `cancel()`, with a deliberately weak
assertion (doesn't hang, doesn't over-produce) because *exactly when* the walk gets interrupted is
genuinely timing-dependent. Pinning an exact interruption point for a single large file's mid-digest
cancellation would be either flaky (assert `Computing`, sometimes lose the race and get `Changed`
instead) or vacuous (assert nothing meaningful). §8.2's test proves the primitive
(`file_digest_equal_with_cancel`) stops early, deterministically; `walk_and_merge`'s wiring to it is
one line, verified by reading the diff (§2) rather than by a test built to race real disk I/O against
a `cancel()` call.

**`deep_compare.rs`'s own per-file digest task is untouched**, even though it has a structurally
similar shape (spawns a `file_digest_equal` call per changed entry). Not in the handoff's §5 change
scope (`explorer.rs` only, on the UI side) — flagging it in §7 rather than silently expanding scope
to fix it.

## 6. Changed files

- `crates/forskscope-core/src/dir/digest.rs` — `DigestOutcome`, `file_digest_equal_with_cancel`;
  `file_digest_equal` becomes a wrapper.
- `crates/forskscope-core/src/dir.rs` — exports `DigestOutcome`, `file_digest_equal_with_cancel`.
- `crates/forskscope-core/src/dir/recursive.rs` — `walk_and_merge` uses the cancellable variant;
  `RecStatus::Computing`'s doc comment updated.
- `crates/forskscope-core/src/tests/dir_cancel_tests.rs` — 1 new test.
- `crates/forskscope-ui/src/ui/view/explorer.rs` — `digest_generation`, `digest_token`,
  `apply_digest_result`; the spawn block wired through both; 2 new tests.

## 7. Unresolved issues

- **`deep_compare.rs` has an analogous, unfixed shape** (§5) — not this handoff's scope, flagged
  rather than touched.
- **No runtime/CI evidence** — matches the handoff's own framing (a classification/cancellation fix
  over plain functions and a `HashMap`, not something RFC-078's harnesses would add confidence to
  beyond what the unit tests already establish); stated plainly rather than assumed acceptable.

## 8. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
--workspace` (`forskscope-core` +1, `forskscope-ui` +2, all else unchanged and still green),
`cargo xtask css --check`, `cargo xtask i18n` (231 keys, unchanged — no UI text touched), `git diff
--check`. CI dispatched on push (`98dde50`).

## 9. Requested review focus

1. **Is `Result<DigestOutcome>` the right shape**, or would `Option<DigestOutcome>`-without-`Result`
   (folding I/O errors into a fourth variant instead of keeping them as `Err`) read more cleanly at
   the call sites?
2. **Is preserving the pre-existing IO-error-to-`Changed` mapping in `walk_and_merge` correct scope
   discipline**, or should this handoff have fixed it alongside cancellation since it's the same
   line of code?
3. **Is §5's reasoning for not adding a `recursive_diff_with_cancel`-level timing test sound**, or
   does the acceptance criterion ("no longer calls the uncancellable variant") call for more than
   reading the diff?
