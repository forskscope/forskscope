# Review Request 080: F84 — consult the large-file guard before loading

**Governing task.** `dev-record/handoffs/011-f84-wire-load-guard.md`
**Register.** F84. Wires one of F75(b)'s four KEEP modules.
**Baseline.** `main` at `9c44883` (docs: F84 and F52 handed off)
**Commit.** `8242a61`

## 1. §3 observation — required, and leading, per the handoff

Before writing any code: built `cargo build --release -p forskscope-ui --bin forskscope`, generated two 150 MB text fixtures well past the 64 MiB "very large" threshold (`yes "the quick brown fox jumps over the lazy dog, line filler content for a large text file test" | head -c 150000000 > left.txt`, and a similarly generated `right.txt` with different filler text, so essentially every line differs — a plausible but somewhat adversarial fixture shape, disclosed rather than presented as typical), launched `forskscope left.txt right.txt` (two-arg CLI compare mode) in the background, and watched it with `ps`, `/proc/<pid>/status` (VmRSS), and `niri msg action screenshot-window`.

**Result: it never finished.**

| Elapsed | Observation |
|---|---|
| ~20s | Window appears (App ID `forskscope`) |
| 45s | Screenshot shows `⟳ Loading left.txt ↔ right.txt...`, spinner visibly rotating (compositor/UI thread still responsive) |
| 135s | Still "Loading..." |
| ~4–5 min | RSS climbs to and **plateaus around 19.36 GB** (19,359,572–19,559,212 kB) — **~65× the 300 MB combined raw input** |
| after plateau | CPU state moves from sustained `R` (running) to mostly `S` (sleeping) with occasional `R` blips — the heavy computation appears to have stopped, but the UI never leaves "Loading" |
| 340s, 545s | Screenshots still show "Loading left.txt ↔ right.txt...", no rendered diff |
| **567s (~9.5 min)** | **Killed manually** — no sign it would ever complete |

System had 59 GB total / 11 GB free at the time — not in immediate OOM danger, but this is a severe, effectively-unbounded resource and time cost for exactly the pair this guard exists to intercept before it starts. Fixture files (300 MB) cleaned up afterward.

**This is worse than the handoff's neutral "expected, not observed" framing implied.** F84 was not overstating the risk; if anything the actual behavior (an apparent multi-gigabyte, multi-minute hang with no forward progress and no error) is worse than "slow or produces an approximate result" reads.

## 2. Falsification 1 — guard evaluation is reachable

Temporarily replaced `decide_load`'s body with an unconditional `return LoadDecision::Go { opts, banner: None }` (before the real `guard_for_sizes` call), ran `cargo test -p forskscope-ui state::compare::tests --lib`:

```
test state::compare::tests::confirm_prompt_suppresses_inline_diff_on_the_resumed_options ... FAILED
test state::compare::tests::decide_load_dispatches_through_the_real_guard_for_every_size_class ... FAILED
test state::compare::tests::both_load_call_sites_stop_at_the_guard_for_a_large_pair ... FAILED
...
test result: FAILED. 16 passed; 3 failed
```
`decide_load_dispatches_through_the_real_guard_for_every_size_class` panicked with: `a 5 MiB file must block on confirmation, not proceed`. Restored; all 19 pass again.

## 3. Falsification 2 — `suppress_inline` reaches the diff

Named precisely: `suppress_inline` turns off `DiffOptions.inline_mode`, forcing it to `InlineMode::None` (character-level inline diffing) on the `opts` that get resumed — this is the flag most likely to be half-wired, since the banner text is visible and this is not.

Temporarily changed the suppression line to `if false && guard.suppress_inline() { adjusted.inline_mode = InlineMode::None; }`, ran the same test target:

```
test state::compare::tests::confirm_prompt_suppresses_inline_diff_on_the_resumed_options ... FAILED
...
thread '...confirm_prompt_suppresses_inline_diff_on_the_resumed_options' panicked at ...:550:13:
assertion `left == right` failed: ConfirmPrompt always implies suppress_inline() — the resumed options must reflect it, not just the banner text
  left: Lazy
 right: None
test result: FAILED. 18 passed; 1 failed
```
Exactly the one test targeting this failed, nothing else. Restored; all 19 pass again.

## 4. Both call sites are covered by a test

Yes, genuinely — not just asserted in prose. `both_load_call_sites_stop_at_the_guard_for_a_large_pair` (`state/compare/tests.rs`) uses `with_test_store` to drive both real entry points against a real 5 MiB fixture on disk:

- `open_compare_request`: asserts `store.tabs` stays empty (no tab allocated) and `store.modal` becomes `Modal::ConfirmLargeLoad` with `LargeLoadTarget::Open(_)`.
- `reload_tab`: pushes a real tab, overwrites its paths with the large pair, calls `reload_tab`, and asserts `tab.state` is still `Loading` (untouched) and `store.modal` becomes `Modal::ConfirmLargeLoad` with `LargeLoadTarget::Reload(0)`.

Both call sites additionally route through the same pure `decide_load`, which the two falsifications above directly exercise — so a regression in the shared logic fails on both real call sites and the pure decision layer simultaneously, not just one.

## 5. `LoadGuard` field not used, and why

`ConfirmPrompt`'s `too_large: bool` is threaded all the way through — `decide_load` → `LoadDecision::Confirm` → `LargeLoadPrompt.too_large` — and stored on `Modal::ConfirmLargeLoad`, but **`LargeLoadModal` never reads it.** The modal renders `title`/`body`/`confirm_label` as given; the distinction `too_large` captures (diffable-but-slow vs. too-large-for-text-diff-entirely) is already fully conveyed through the differing `title`/`body`/`confirm_label` text `guard_for_sizes` produces for `Large` vs. `VeryLarge` (confirmed by `load_guard.rs`'s own `very_large_prompt_has_different_confirm_label_than_large` test), so there is no separate UI behavior keyed on the boolean today. I left the field on the struct rather than dropping it — a future change that wants to special-case "too large to diff at all" (e.g. disabling the confirm button, or routing to a binary/metadata-only view instead of a full load) has it available without another round trip through `load_guard`.

## 6. Scope discipline

- `PerformanceLimits` thresholds: untouched.
- `palette_view`, `conflict_nav_view`, `save_error`: untouched (`save_error` is handoff 012/review 081).
- No no-allowlist gate added.
- `ROADMAP.md`: not touched by this commit.

## 7. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo test --workspace` (all green, including the 3 new F84 tests), `cargo xtask css --check`, `cargo xtask version-sync`, `cargo xtask i18n` (236 keys, unchanged), `cargo xtask rfc-sync`, `git diff --check`. Pushed as `8242a61`; CI green (run 33076894343).
