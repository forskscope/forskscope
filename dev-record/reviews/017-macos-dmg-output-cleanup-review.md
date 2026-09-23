# macOS DMG Output Cleanup Review

Request: `dev-record/review-requests/014-macos-dmg-output-cleanup.md`

Date: 2026-07-10

## Verdict

Accept with notes.

The one-line helper hardening is correct. `packaging/macos/build-dmg.sh` now removes the existing DMG output before invoking `create-dmg`, so local and workflow reruns no longer rely on unknown `create-dmg` overwrite behavior when `target/` is reused or restored from cache.

This resolves the non-blocking deterministic-output note from `dev-record/reviews/016-macos-release-dmg-workflow-review.md`. It still does not close macOS runtime/package verification because the DMG build path was not executed in a macOS environment.

## Blocking findings

None.

## Non-blocking findings

None for this scoped helper cleanup.

## Requirement checks

- `packaging/macos/build-dmg.sh:13` defines `OUT="target/forskscope-v$VER-macos-aarch64.dmg"`.
- `packaging/macos/build-dmg.sh:20`-`packaging/macos/build-dmg.sh:22` already clears and recreates the app staging directory.
- `packaging/macos/build-dmg.sh:39` now removes the existing DMG output with `rm -f "$OUT"` immediately before `create-dmg`.
- `.github/workflows/release.yml:167` runs `bash packaging/macos/build-dmg.sh`, so the release workflow receives the same deterministic-output behavior.

## Reviewer questions

- Yes, deleting the existing DMG output before `create-dmg` is the right deterministic behavior for local and workflow reruns.
- For this one-line macOS-specific hardening, syntax and whitespace verification are sufficient in a Linux-only review environment, provided the remaining macOS execution gap stays explicit.

## Observed evidence

- `bash -n packaging/macos/build-dmg.sh` passed.
- `git diff --check` passed.
- The diff is limited to adding `rm -f "$OUT"` immediately before `create-dmg`.

## Missing evidence

- No macOS execution of `packaging/macos/build-dmg.sh` was observed.
- No `create-dmg` run or generated DMG inspection was observed.
- No live GitHub Actions release workflow run was observed.
- No macOS runtime/package verification, signing, or notarization was observed.

## Recommended next action

Proceed with this helper cleanup. Keep macOS release readiness open for a real macOS workflow/helper run, DMG inspection, and runtime smoke.
