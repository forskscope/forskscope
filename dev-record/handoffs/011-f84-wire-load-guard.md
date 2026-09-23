# Developer Handoff 011 — F84: consult the large-file guard before loading

**From:** architect
**Date:** 2026-08-27
**Register:** F84. Wires one of F75(b)'s four KEEP modules.
**Gate:** Not a Gate D blocker (needs a deliberately large input, and degrades
rather than corrupting).

---

## 1. Task title

Call `guard_for_sizes` before a comparison loads, and honour all three of its
outcomes.

## 2. Purpose

`forskscope-ui-logic::compare::load_guard` returns proceed / warn / confirm for a
pair of file sizes. It has **19 tests and zero call sites.** So opening a pair of
very large files goes straight to reading and diffing them, with no size check
anywhere between the picker and the diff engine.

RFC-013 (*Large File Performance and Virtualization*) is in `rfcs/done/`. The
guard it specifies exists and is never consulted — **a protection that was built,
tested, and then bypassed**, which is a different thing from a feature that was
never written.

## 3. Measure before you fix — this is required, and it comes first

F84 records the consequence as **expected, not observed**. Nobody has actually
opened a multi-gigabyte pair and watched what happens.

**Do that first**, before writing any code:

- Build a pair well past the 64 MiB large threshold (a generated text file is
  fine — `yes` piped into `head -c`, two of them differing).
- Open them in a real desktop build.
- Record what happens: how long until the window responds, whether it responds at
  all, memory if you can see it.

**Report that observation in your review request whichever way it comes out.**
If the app handles it gracefully, say so — that materially changes how urgent this
is, and it would be worth knowing that F84 overstated the risk. **Do not skip this
because the fix seems obviously right.** A fix that cannot be shown to change
anything is the shape this program keeps finding.

## 4. What the guard returns

`guard_for_sizes(left_bytes, right_bytes) -> LoadGuard`, using
`PerformanceLimits::default()` — **4 MiB** medium, **64 MiB** large
(`core/src/job/limits.rs`).

| Variant | Meaning | Required behaviour |
|---|---|---|
| `Proceed` | Under thresholds | Load as today. No change. |
| `WarnBanner { message, suppress_inline }` | Large but workable | **Load, and show `message`** as a non-blocking notice. If `suppress_inline` is set, the diff must run **without** character-level inline diffing. |
| `ConfirmPrompt { title, body, confirm_label, too_large }` | Big enough to ask first | **Do not load.** Show a modal built from these fields. Load only on confirm. |

`suppress_inline()` is also `true` for `ConfirmPrompt`, so a confirmed load must
suppress inline diffing too.

## 5. Where it goes, and why not where it looks like it goes

**Not inside `load_and_diff`.** That function runs in
`tokio::task::spawn_blocking` (`state/compare.rs:117` and `:224`) and returns
`Result<PreparedCompare, String>`. A `ConfirmPrompt` needs to ask the user
*before* any load, and a blocking task cannot.

**The guard is evaluated on the UI side, before the spawn**, at **both** call
sites — the open path and the reload path. Missing one leaves a hole exactly where
a user re-opens the file they were warned about.

Sizes come from `fs::metadata(path).len()` per side, taken before `load_path`.
**A side may legitimately be absent** (`LoadOptions { allow_missing: true }`), so
treat a failed `metadata` as zero bytes and let the existing missing-file handling
do its job — do not turn a missing file into a guard failure.

## 6. Explicit non-change scope

- **Do not change the thresholds.** They are `PerformanceLimits::default()`'s and
  are not yours or mine to retune in a wiring change.
- **Do not touch the other three KEEP modules** (`palette_view`,
  `conflict_nav_view`, `save_error`). `save_error` is handoff 012.
- **Do not add the no-allowlist gate.**
- Do not edit `ROADMAP.md`.

## 7. Required tests

1. **Guard evaluation is reachable from a test.** Extract the decision — sizes in,
   `LoadGuard` out, plus whatever the caller needs to act on — so it can be driven
   without a Dioxus runtime, the way `classify_entry` and `apply_epoch_result`
   already are in this codebase. **Falsify by making it always return `Proceed`.**
2. **`suppress_inline` actually reaches the diff.** Assert that a guard demanding
   suppression produces diff options with inline/character-level diffing off.
   **Falsify by dropping the flag on the way through** — this is the one most
   likely to be wired half-way, because the banner is visible and the flag is not.
3. **Both call sites are covered.** If your extraction lets one path skip the
   guard, a test must fail. If that is not expressible, say so plainly rather than
   implying coverage.

## 8. Acceptance criteria

- A pair over the large threshold does **not** load until confirmed.
- A pair over the medium threshold loads, warns, and suppresses inline diffing.
- A small pair behaves exactly as today.
- A missing file behaves exactly as today.
- Removing the guard call, or the `suppress_inline` plumbing, fails a test.
- The §3 observation is reported.
- Gates green: `fmt`, `clippy --workspace --all-targets -- -D warnings`,
  `test --workspace`, `xtask css --check`, `xtask version-sync`, `xtask i18n`,
  `xtask rfc-sync`, `git diff --check`.

## 9. Prohibited shortcuts

- **Do not skip §3** because the fix is obviously right.
- **Do not show the banner and drop `suppress_inline`.** Half of this guard is
  invisible, and the invisible half is the one that protects the diff engine.
- **Do not guard only the open path.**
- **Do not report a falsification you did not run.**

## 10. Required review-request format

Lead with the §3 observation — what actually happened when you opened a very
large pair, before the fix.

Then the falsifications. State plainly:
- whether both call sites are covered by a test, or only one;
- what `suppress_inline` turns off, named precisely;
- any `LoadGuard` field you did not use, and why.
