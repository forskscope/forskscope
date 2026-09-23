# Handoff 037 — F109 follow-ups: compile Windows code in CI, and prove the detection on real Windows

**From:** architect. **Date:** 2026-09-17. **Release:** 0.172.0. **This
blocks the cut.**

**Why:** `dev-record/reviews/109-f109-webview2-startup-check-review.md`
(§3–§5). This file says what to do.

There are three parts. Put A and B in separate commits; C may ride with either.

## A. Compile the Windows code paths on every push (`ci.yml`)

1. **Fix the two Windows-only `unused_mut` warnings** at
   `crates/forskscope-core/src/save.rs:179` and `:298`.
   - The mutation of `builder` evidently happens only in a non-Windows branch.
     Restructure so the binding is mutable only where it is mutated, for
     example with a `cfg`-gated rebinding or a small helper.
   - **Do not add `#[allow(unused_mut)]`.** A lint silenced on one platform
     hides the next real one there.
   - Behaviour on every platform must be unchanged.
2. **Add steps to `ci.yml`'s existing `test` job** on Ubuntu, not a new
   Windows runner:
   ```
   rustup target add x86_64-pc-windows-gnu
   cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings
   ```
   - This is a **check**: nothing links, so no MinGW toolchain should be
     needed. If a dependency's build script needs one after all, say so.
     Do not install a toolchain silently.
   - Say in a comment what it covers: `cfg(windows)` code type-checks and
     lints. And what it does not: the MSVC target, linking and behaviour,
     which the release build and part B cover.
3. **Falsification.** In a scratch commit on a branch, not `main`, add a
   `#[cfg(windows)]` type error to `crates/forskscope-ui/src/main.rs`:
   - Show the new step **failing** on it, and the old Linux-only steps still
     passing.
   - Delete the branch afterwards.
   - A plain branch push is fine. **Do not push a tag.**
4. Report how many seconds the step adds to the job.

## B. Prove WebView2 detection on real Windows (a new dispatchable workflow)

1. **Remove** the `Confirm WebView2 detection (F109)` step from
   `.github/workflows/release.yml`. The release workflow is not where a check
   first runs.
2. **Add `.github/workflows/windows-check.yml`,** with `workflow_dispatch`
   only (no push trigger). Model it on `release.yml`'s `windows` job,
   `windows-latest`:
   - build `forskscope.exe` in release mode, as `release.yml` does;
   - run `forskscope.exe --diagnostics` and **assert** that its output has a
     line matching `^WebView2 runtime: \d`, meaning a version;
   - run it again with `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` set to a fresh
     empty directory, and **assert** a line starting
     `WebView2 runtime: not found (`;
   - print both outputs whatever the result.
3. **Before the assertions, check that stdout is actually captured.** The
   release binary is a GUI-subsystem executable (`windows_subsystem =
   "windows"`), so confirm that its `println!` output reaches the step log.
   - If it does not, the assertions would test nothing. Stop there and report
     it. Do not work around it by weakening the assertions.
4. **Dispatch it** against the head of `main` with `gh workflow run`, and give
   the run ID and both outputs.
5. **If the environment variable does not make the check fail:**
   - keep only the first assertion;
   - say so plainly, quoting what happened;
   - do not invent another way to simulate a missing runtime in the product.

   I will then change the owner's pre-publish check.
6. Add one line to `docs/src/maintainers/release.md`'s pre-release checklist:
   dispatch `windows-check.yml` for the release commit and confirm it is
   green.

## C. The Japanese text is really used (`crates/forskscope-ui/src/webview2.rs`)

In `absent_stops_with_a_non_empty_bilingual_message`, add an assertion that
the `Lang::Ja` dialog text differs from the `Lang::En` one.

**Falsification:** temporarily remove the Japanese entry for the dialog string
from `i18n.rs`. Show the test failing, then restore the entry.

## Scope

- **In:**
  - `crates/forskscope-core/src/save.rs`: the two bindings only
  - `.github/workflows/ci.yml`: new steps
  - `.github/workflows/release.yml`: removing the F109 step
  - `.github/workflows/windows-check.yml`: new
  - `docs/src/maintainers/release.md`: one checklist line
  - `crates/forskscope-ui/src/webview2.rs`: the test
- **Out:**
  - any change to F109's product behaviour
  - `m5-evidence-windows.yml` and the harness
  - `CHANGELOG.md` and `ROADMAP.md`
  - RFC-080 and F107

**No rehearsal tags.** Review 108 §5 still applies.

## Gates

- `cargo fmt --check`.
- `cargo clippy --workspace --all-targets -- -D warnings`, **and** the new
  Windows-target clippy, run locally too.
- `cargo test --workspace`.
- `cargo xtask` with `i18n`, `ui-logic-connectivity`, `ui-logic-docs`,
  `rfc-sync`, `audit-deps` and `version-sync`.
- `actionlint` on all three workflow files.
- `mdbook build docs`.
- A green CI run for the final commit, and the `windows-check.yml` run ID.

Reply in `dev-record/review-requests/109-f109-windows-checks.md`.
