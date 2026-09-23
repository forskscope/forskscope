# Review request: macOS DMG output cleanup

## Scope

This package addresses the non-blocking follow-up from
`dev-record/reviews/016-macos-release-dmg-workflow-review.md` about
deterministic macOS DMG output when `target/` is cached or reused.

## Files to inspect

- `packaging/macos/build-dmg.sh`

## Change summary

- Added `rm -f "$OUT"` immediately before `create-dmg`.

## Why this changed

The macOS helper already removes and recreates the staging app directory, but it
did not remove an existing `target/forskscope-vX.Y.Z-macos-aarch64.dmg` before
calling `create-dmg`. Removing the output path first avoids relying on
`create-dmg` overwrite behavior and makes reruns deterministic when `target/`
is reused or restored from cache.

## Observed verification

Passed in this thread:

- `bash -n packaging/macos/build-dmg.sh`
- `git diff --check`

## Limitations

- This Linux environment cannot execute the macOS `create-dmg` path or inspect
  a generated DMG.
- No macOS runtime/package verification was observed.

## Reviewer questions

- Is deleting the existing DMG output before `create-dmg` the right deterministic
  behavior for local and workflow reruns?
- Is syntax/whitespace verification sufficient for this one-line macOS helper
  hardening in a Linux-only environment?
