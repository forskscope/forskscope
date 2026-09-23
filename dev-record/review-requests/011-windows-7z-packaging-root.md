# Review request: Windows 7z packaging root

## Scope

This package addresses the non-blocking follow-up from
`dev-record/reviews/012-cross-platform-packaging-helper-names-review.md`
about the Windows helper's `7z` fallback archive root.

## Files to inspect

- `packaging/windows/build-zip.sh`

## Change summary

- Changed the `7z` fallback from:
  - `7z a "$OUT" "$STAGE/"`
- to:
  - `(cd target && 7z a "../$OUT" "$(basename "$STAGE")/")`

This mirrors the existing `zip` branch so both archive from `target` and store
`forskscope-vX.Y.Z-windows-x64/` at archive root instead of potentially storing
`target/forskscope-vX.Y.Z-windows-x64/`.

## Observed verification

Passed in this thread:

- `bash -n packaging/windows/build-zip.sh`
- `bash -n packaging/macos/build-dmg.sh`
- `bash packaging/windows/build-zip.sh`
  - executed with the existing ignored dummy
    `target/x86_64-pc-windows-msvc/release/forskscope.exe`
  - produced `target/forskscope-v0.164.0-windows-x64.zip`
- `env PATH="$PWD/target/nozip-bin" /usr/bin/bash packaging/windows/build-zip.sh`
  - executed the `7z` fallback by exposing `7z` and required core tools while
    omitting `zip` from `PATH`
  - `7z` reported `Everything is Ok`
  - produced `target/forskscope-v0.164.0-windows-x64.zip`
- `unzip -l target/forskscope-v0.164.0-windows-x64.zip`
  - confirmed top-level zip directory
    `forskscope-v0.164.0-windows-x64/`
  - confirmed no `target/` prefix in the archive entries
- `git diff --check`

## Limitations

- The `zip` branch and the forced `7z` fallback were both executed locally.
- The binary used for helper execution is an ignored dummy file, not a real
  Windows build.

## Reviewer questions

- Does the `7z` fallback now mirror the `zip` branch's archive root correctly?
- Is the forced no-`zip` PATH test sufficient evidence for this fallback fix?
