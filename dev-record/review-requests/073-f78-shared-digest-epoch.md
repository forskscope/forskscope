# Review Request 073: F78 — one shared comparison mechanism for both views

**Governing task.** `dev-record/handoffs/005-f78-shared-digest-epoch.md`
**Register.** F78 (fixed here). Also closes F77's recorded regression exposure (review 074 §2).
**Baseline.** `main` at `981ac99`
**Commit.** `7c58fb2`

## The falsifications, first

### §8.2 — `DigestEpoch`'s own guard

```
thread 'ui::view::digest_epoch::tests::restart_invalidates_a_stamp_taken_before_it' panicked at crates/forskscope-ui/src/ui/view/digest_epoch.rs:103:9:
restart() must invalidate stamps taken before it
```

Broken by commenting out `self.generation += 1;` in `restart()`. Restored; the suite is green.

### §8.1 — Deep Compare's stale-stamp apply guard

```
thread 'ui::view::deep_compare::tests::a_stale_stamp_result_does_not_mutate_deep_compares_entries' panicked at crates/forskscope-ui/src/ui/view/deep_compare.rs:609:9:
assertion `left == right` failed: a stale-stamp result must not mutate the entry
  left: Equal
 right: Computing
```

Broken by short-circuiting `apply_epoch_result`'s `if !epoch.is_current(stamp) { return; }` (deep_compare.rs)
so it always applies. Restored; the suite is green.

### §8.3 — review 074's two F77 falsifications, re-run against the converted `explorer.rs`

**Generation guard, at the real (converted) call site:**

```
thread 'ui::view::explorer::tests::a_stale_stamp_result_does_not_mutate_the_map' panicked at crates/forskscope-ui/src/ui/view/explorer.rs:661:9:
a stale-stamp result must not mutate the map
```

Broken by short-circuiting `apply_epoch_result`'s `if epoch.is_current(stamp)` in explorer.rs to always insert.
Restored; the suite is green.

**Core's in-loop cancellation poll (core untouched by this handoff — confirming it still fails correctly):**

```
thread 'tests::dir_cancel_tests::file_digest_equal_with_cancel_stops_early_and_says_so' panicked at crates/forskscope-core/src/tests/dir_cancel_tests.rs:165:5:
assertion `left == right` failed: a cancelled comparison must report Cancelled, not a completed verdict
  left: Equal
 right: Cancelled
```

Broken by commenting out the `if token.is_cancelled() { return Ok(DigestOutcome::Cancelled); }` check in
`file_digest_equal_with_cancel`'s read loop. Restored; the suite is green.

Both of §8.3's re-runs still fail correctly — the conversion did not weaken F77's coverage.

## 1. `DigestEpoch` (§7a) — `digest_epoch.rs`, new

Owns exactly three things: `generation: u64`, `token: CancellationToken`, `semaphore: Arc<Semaphore>`.

- `EpochStamp(u64)` — private field, no public constructor, no `From<u64>`. Only `begin_task()` can
  mint one. This is what makes the acceptance criterion literal rather than aspirational: there is no
  second value of type `EpochStamp` a careless call site could substitute for a captured one, and no
  way to pass a raw generation number where a stamp belongs — it doesn't compile, not "a test would
  catch it."
- `restart()` — cancels the outgoing token, installs a fresh one, bumps generation. Called exactly
  where each view's roots change.
- `begin_task()` — `#[must_use]`, returns `(EpochStamp, CancellationToken, Arc<Semaphore>)`.
- `is_current(stamp)` — the guard.
- The semaphore is created once in `new()` and never replaced by `restart()` — verified by a
  dedicated test (`the_semaphore_is_the_same_instance_across_restart`, `Arc::ptr_eq`), since this is
  the one property that's easy to silently break by writing `restart()` as "just build a fresh
  `DigestEpoch`."

Both views construct it as `DigestEpoch::new(forskscope_core::DIGEST_CONCURRENCY_LIMIT)` — the same
constant `deep_compare.rs` already used for its own `Semaphore`.

## 2. `explorer.rs` conversion (§7b)

`digest_generation`, `digest_token`, and `apply_digest_result` are gone — replaced by
`digest_epoch: Signal<DigestEpoch>`. The roots-changed block now calls `digest_epoch.write().restart()`
in place of the old cancel/replace-token/bump-generation sequence. The `NeedsDigest` spawn arm calls
`digest_epoch.read().begin_task()` for the stamp, token, and semaphore handle; the permit is acquired
*inside* the spawned task (`sem.acquire_owned().await`), after the `begin_task()` read guard has
already been dropped — never held across an await. This is the concurrency bound the Explorer did not
have before: previously one `spawn_blocking` per common file, unbounded.

`apply_digest_result` is replaced by a new, differently-shaped `apply_epoch_result` (map + key + state
+ `EpochStamp` + `&DigestEpoch`, vs. the old map + key + state + two `u64`s) — same reason as F77's
version (a test can drive it without a Dioxus runtime), now backed by `DigestEpoch::is_current` instead
of a raw integer compare.

## 3. `deep_compare.rs` conversion (§7c) — the defect fix

- **New `restart()`-on-root-change block.** This view previously had none — its `use_effect` ran phase
  1/phase 2 unconditionally on every effect execution. Added a `scan_roots: Signal<Option<(PathBuf,
  PathBuf)>>` to detect an actual root change (vs. an effect re-run for some other reason), and on a
  real change: `digest_epoch.write().restart()`, then synchronously `scan.set(true)`, `comp.set(0)`,
  `tc.set(0)` before spawning phase 1. A run whose effect executes without a root change now returns
  early instead of spawning a redundant second scan on top of one in flight.
- **Phase 1 is now cancellable**: `list_recursive_for_display_with_cancel` instead of the uncancellable
  variant, using the epoch's token (discarding the stamp — phase 1 applies nothing itself, it only
  needs to know whether it was superseded). If cancelled, the spawned task returns without touching
  `entries`/`scanning` — the run that superseded it already reset those for its own roots.
- **Phase 2 is now cancellable and stamp-guarded**: `file_digest_equal_with_cancel` in place of
  `file_digest_equal`; each per-file task calls `begin_task()` for its own stamp/token/permit (acquired
  inside the task, same pattern as Explorer); the result is applied through a new `apply_epoch_result`
  (entries slice + rel path + status + stamp + epoch) instead of the old unconditional
  `iter_mut().find(...)`.
- **Semaphore replaced**: the old per-effect-run `Arc::new(Semaphore::new(DIGEST_CONCURRENCY_LIMIT))`
  is gone; both phases now go through the epoch's persistent one.

**§14's known risk — a permanently-stuck "Scanning…" — checked by reasoning through the sequence,
not just by inspection:** every `restart()` is immediately followed, in the same synchronous block,
by `scan.set(true)`. A superseded run's phase-1 task, on observing its own (now-cancelled) token,
returns without touching `scan` at all — it neither confirms nor denies anything, it just leaves
whatever the newer run already set. So `scan` only ever goes `false` when a *non-superseded* phase-1
completes, and every root change unconditionally sets it back to `true` synchronously before any
`await` point exists for a race to land in. I did not write a test for this — it's a sequencing
argument about signal writes relative to `await` points, not something a unit test without a Dioxus
runtime can exercise, and a timing test is explicitly out per handoff 004 §14.

## 4. I/O-error handling — unchanged, both views

`Err(_) => RecStatus::Changed` / `DigestState::Different`, matching the pre-existing (F76-territory)
collapse. Deep Compare's join-error fallback is now `Ok(DigestOutcome::Different)` before the match,
same effective behavior as the old `.unwrap_or(false)` → `Changed`. Not touched beyond making it
match the already-established `explorer.rs`/F77 idiom instead of `.ok().and_then(...).unwrap_or(false)`.

## 5. Scope discipline

- No RFC-080 work — no tiers, no size threshold, no new status vocabulary.
- `ROADMAP.md` not touched.
- F74/F75/F76 untouched.
- `DigestEpoch` lives in `forskscope-ui`, not `forskscope-ui-logic` — it depends on
  `tokio::sync::Semaphore`.
- No test for the concurrency bound, per §8's explicit instruction — stated here rather than implied:
  the bound is enforced only by `begin_task()` being the sole way to obtain a stamp, and its
  `#[must_use]` return handing you the semaphore alongside it.

## 6. Your three required questions, answered plainly

**Did anything from `explorer.rs`'s F77 mechanism survive?** No live code. `digest_generation`,
`digest_token`, and `apply_digest_result` no longer exist — confirmed by grep, zero matches outside
historical comments referencing F77 by name. The *shape* of the guard (a small `apply_*` helper taking
the collection and epoch by value so it's testable without a Dioxus runtime) survived, because it's the
same shape `deep_compare.rs` needed independently — that's reuse of a pattern, not two mechanisms.

**Is the concurrency bound enforced anywhere other than the API shape?** No. There is no runtime
assertion or test that counts in-flight permits. It is enforced exactly by `begin_task()` being the
only way to get a stamp, and by both views' spawn sites always acquiring a permit from the handle it
returns before doing any blocking work. If a future call site got a token without going through
`begin_task()`, nothing would stop it — but nothing in this codebase does that today, and `EpochStamp`'s
opacity means it couldn't construct a valid stamp to pair with such a token anyway.

**What, if anything, got added to `DigestEpoch` beyond the three concerns?** Nothing. The struct is
exactly `generation: u64`, `token: CancellationToken`, `semaphore: Arc<Semaphore>`. `EpochStamp` is a
struct, not a fourth field on `DigestEpoch` — it's the epoch's own generation value, opaquely wrapped,
handed back to the caller. I didn't add a job registry, a result channel, or a callback — the only
API surface is `new`, `restart`, `begin_task`, `is_current`.

## 7. Changed files

- `crates/forskscope-ui/src/ui/view/digest_epoch.rs` — new: `DigestEpoch`, `EpochStamp`, 4 tests.
- `crates/forskscope-ui/src/ui/view.rs` — `pub mod digest_epoch;`.
- `crates/forskscope-ui/src/ui/view/explorer.rs` — converted per §7b; 2 tests replaced (same intent,
  new shape).
- `crates/forskscope-ui/src/ui/view/deep_compare.rs` — converted per §7c; 1 new test
  (`apply_epoch_result`'s guard) plus a new `entry_with_status` test helper.

`forskscope-core` unchanged (§12 confirmed correct — nothing needed there).

## 8. Unresolved issues

- None carried forward from this handoff. F76 (I/O-error collapse) remains open in both views, as
  instructed.

## 9. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
(`forskscope-core` 689 unchanged, `forskscope-ui` lib 69 — was 64 before this handoff: `explorer.rs`
nets 0 (2 removed, 2 added, same intent/new shape), `deep_compare.rs` +1, `digest_epoch.rs` +4),
`cargo xtask css --check`, `cargo xtask i18n` (231 keys, unchanged — no UI text touched), `git diff
--check`. Pushed as `7c58fb2`; CI dispatched on push.

## 10. Requested review focus

1. Does `apply_epoch_result` existing in both `explorer.rs` and `deep_compare.rs` (same shape,
   different collection type — `HashMap` vs. entry slice) read as acceptable duplication, or should it
   have been a fourth thing `DigestEpoch` itself exposes (e.g. a generic `apply_if_current` helper)? I
   judged that as scope creep into "a general job scheduler" (§11), since the two call sites apply to
   structurally different collections — but I'd rather you tell me if that reasoning is wrong than
   leave it implicit.
2. Is the `scan_roots` signal the right way to detect Deep Compare's root changes, or would you have
   preferred keying off something already available (e.g. comparing against a value read once per
   render) instead of a new tracking signal?
3. Is the §14 stuck-scanning reasoning in §3 above (a sequencing argument, not a test) sufficient
   evidence, or does this need a real integration exercise before you'd consider it closed?
