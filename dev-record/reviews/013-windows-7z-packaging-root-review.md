# Windows 7z Packaging Root Review

Request: `dev-record/review-requests/011-windows-7z-packaging-root.md`

Date: 2026-07-10

## Verdict

Accept.

The `7z` fallback now mirrors the `zip` branch's archive root behavior. Both branches archive from `target` and pass `$(basename "$STAGE")/`, so the zip entries are rooted at `forskscope-vX.Y.Z-windows-x64/` rather than `target/forskscope-vX.Y.Z-windows-x64/`.

## Blocking findings

None.

## Non-blocking findings

None for this scoped fallback fix.

The broader packaging limitation from the previous review still applies: this verifies helper naming/layout with a dummy ignored binary, not a real Windows build or runtime smoke.

## Requirement checks

- `packaging/windows/build-zip.sh:24`-`packaging/windows/build-zip.sh:27` now uses the same archive-root strategy for both branches:
  - `zip`: `(cd target && zip -r "../$OUT" "$(basename "$STAGE")/")`
  - `7z`: `(cd target && 7z a "../$OUT" "$(basename "$STAGE")/")`
- The implementation directly addresses the non-blocking finding in `dev-record/reviews/012-cross-platform-packaging-helper-names-review.md` about the `7z` fallback potentially storing a `target/` prefix.

## Reviewer questions

- Yes, the `7z` fallback now mirrors the `zip` branch's archive root correctly.
- Yes, the forced no-`zip` PATH test is sufficient evidence for this fallback fix because it exercised the actual `7z` branch and the resulting archive entries were inspected.

## Observed evidence

- `bash -n packaging/windows/build-zip.sh` passed.
- `git diff --check` passed.
- `bash packaging/windows/build-zip.sh` passed through the normal `zip` branch using the existing ignored dummy `target/x86_64-pc-windows-msvc/release/forskscope.exe`.
- `env PATH="$PWD/target/nozip-bin" /usr/bin/bash packaging/windows/build-zip.sh` passed through the forced `7z` branch and reported `Everything is Ok`.
- `unzip -l target/forskscope-v0.164.0-windows-x64.zip` showed entries rooted at `forskscope-v0.164.0-windows-x64/`, including `forskscope.exe`, `README.md`, `LICENSE`, `NOTICE`, and `CHANGELOG.md`.
- `7z l target/forskscope-v0.164.0-windows-x64.zip` independently showed the same root without a `target/` prefix.

## Missing evidence

- No real Windows build was observed; the helper execution used a dummy ignored executable.
- No Windows runtime/package verification was observed.
- No live GitHub Actions release workflow run was observed.

## Recommended next action

Proceed with this fallback fix. Keep Windows release verification open for a real `x86_64-pc-windows-msvc` build, package inspection, and runtime smoke on Windows.
