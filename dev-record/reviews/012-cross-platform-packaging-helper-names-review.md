# Cross-Platform Packaging Helper Names Review

Request: `dev-record/review-requests/010-cross-platform-packaging-helper-names.md`

Date: 2026-07-10

## Verdict

Accept with notes.

The helper-script naming alignment is correct: the macOS DMG and Windows zip helpers now use the documented `forskscope-vX.Y.Z-*` release artifact convention, and both scripts now derive the version from `[workspace.package]` instead of the first top-level-looking `version` line in `Cargo.toml`.

This is packaging-helper alignment only. It does not close real macOS or Windows runtime/package verification.

## Blocking findings

None.

## Non-blocking findings

1. `packaging/windows/build-zip.sh:26`-`packaging/windows/build-zip.sh:27` has an unverified `7z` fallback that likely archives the path as `target/forskscope-vX.Y.Z-windows-x64/` because it invokes `7z a "$OUT" "$STAGE/"` from the repository root. The observed verification used the `zip` branch, which correctly runs from `target` and stores `forskscope-vX.Y.Z-windows-x64/` at archive root. If the helper is expected to work on Windows machines with `7z` but no `zip`, make the `7z` branch mirror the `zip` branch's archive root.

2. The Windows helper stages `README.md`, `LICENSE`, `NOTICE`, and `CHANGELOG.md`, while `.github/workflows/release.yml:191`-`.github/workflows/release.yml:195` currently uploads a zip containing only `target/release/forskscope.exe`. This is acceptable for a local helper, but the release owner should decide whether workflow artifacts and local helper artifacts are intentionally different or should converge.

## Requirement checks

- `packaging/macos/build-dmg.sh:9` now extracts the workspace package version with `[workspace.package]`-scoped `awk`, matching the release docs' guidance to avoid `grep '^version'`.
- `packaging/macos/build-dmg.sh:13` now emits `target/forskscope-v$VER-macos-aarch64.dmg`, matching `packaging/README.md:37`-`packaging/README.md:42`.
- `packaging/windows/build-zip.sh:9` now extracts the workspace package version with the same scoped `awk` pattern.
- `packaging/windows/build-zip.sh:11`-`packaging/windows/build-zip.sh:12` now uses `target/forskscope-v$VER-windows-x64.zip` and `target/forskscope-v$VER-windows-x64`.
- `.github/workflows/release.yml:163`-`.github/workflows/release.yml:169` and `.github/workflows/release.yml:194`-`.github/workflows/release.yml:200` use the same `forskscope-v${VER}-macos-aarch64` and `forskscope-v${VER}-windows-x64` artifact naming convention.

## Reviewer questions

- Aligning local helper artifact names to the documented `vX.Y.Z` convention is acceptable and reduces release artifact ambiguity.
- The `[workspace.package]`-scoped version extraction is appropriate and is safer than `grep '^version' Cargo.toml`.
- The Windows helper may continue staging README/LICENSE/NOTICE/CHANGELOG if it is intentionally a fuller local distribution archive. If local helper artifacts are meant to be byte/layout comparable to GitHub workflow artifacts, then the workflow or helper should be adjusted in a follow-up.

## Observed evidence

- `bash -n packaging/macos/build-dmg.sh` passed.
- `bash -n packaging/windows/build-zip.sh` passed.
- The workspace-version extraction command returned `0.164.0`.
- `bash packaging/windows/build-zip.sh` passed using the existing ignored dummy `target/x86_64-pc-windows-msvc/release/forskscope.exe` and produced `target/forskscope-v0.164.0-windows-x64.zip`.
- `unzip -l target/forskscope-v0.164.0-windows-x64.zip` showed top-level directory `forskscope-v0.164.0-windows-x64/` and included `forskscope.exe`, `README.md`, `LICENSE`, `NOTICE`, and `CHANGELOG.md`.
- `git diff --check` passed.
- A stale-pattern scan found no remaining old `forskscope-$VER-*` helper names or `grep '^version'` packaging-version extraction patterns in the reviewed docs/workflows/scripts.

## Missing evidence

- No real Windows build was observed; the Windows helper execution used an ignored dummy binary and verified naming/layout only.
- The Windows `7z` fallback path was not executed because `7z` is not available in this environment.
- The macOS helper was syntax-checked only; no macOS build, `create-dmg` run, signing, notarization, or runtime smoke was observed.
- No live GitHub Actions release workflow run was observed.

## Recommended next action

Proceed with this helper-name alignment. Track the Windows `7z` fallback layout and the helper-vs-workflow artifact-content difference as follow-ups, while keeping macOS/Windows runtime and packaging verification open for the release.
