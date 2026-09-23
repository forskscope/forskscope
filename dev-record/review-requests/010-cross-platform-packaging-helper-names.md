# Review request: cross-platform packaging helper names

## Scope

This package covers a small packaging-script alignment found while continuing
runtime/platform release verification.

## Files to inspect

- `packaging/macos/build-dmg.sh`
- `packaging/windows/build-zip.sh`

## Change summary

- Updated macOS DMG output from `target/forskscope-$VER-macos-aarch64.dmg` to
  `target/forskscope-v$VER-macos-aarch64.dmg`.
- Updated Windows zip output and staging directory from
  `forskscope-$VER-windows-x64` to `forskscope-v$VER-windows-x64`.
- Replaced `grep '^version' Cargo.toml` version extraction with the same
  `[workspace.package]`-scoped `awk` extraction style used by the main release
  archive docs/script.

## Why this changed

`packaging/README.md` and `.github/workflows/release.yml` use
`forskscope-vX.Y.Z-*` artifact names, but the macOS and Windows helper scripts
used `forskscope-X.Y.Z-*` names. That mismatch makes local helper artifacts
look different from documented and workflow-created release artifacts.

## Observed verification

Passed in this thread:

- `bash -n packaging/macos/build-dmg.sh`
- `bash -n packaging/windows/build-zip.sh`
- `bash packaging/windows/build-zip.sh`
  - executed on Linux using a dummy ignored
    `target/x86_64-pc-windows-msvc/release/forskscope.exe`
  - produced `target/forskscope-v0.164.0-windows-x64.zip`
- `unzip -l target/forskscope-v0.164.0-windows-x64.zip`
  - confirmed top-level zip directory
    `forskscope-v0.164.0-windows-x64/`
  - confirmed included files:
    `forskscope.exe`, `README.md`, `LICENSE`, `NOTICE`, `CHANGELOG.md`
- `git diff --check`

## Limitations

- The Windows helper execution used a dummy ignored binary to verify naming and
  archive layout only; it was not a real Windows runtime build.
- The macOS helper was syntax-checked only in this Linux environment because
  `create-dmg` and macOS runtime packaging are unavailable locally.
- This does not close macOS/Windows runtime/package verification for release.

## Reviewer questions

- Is aligning local helper artifact names to the documented `vX.Y.Z` convention
  acceptable?
- Is the `[workspace.package]`-scoped version extraction appropriate for these
  helper scripts?
- Should the Windows helper continue staging README/LICENSE/NOTICE/CHANGELOG in
  the zip while the GitHub release workflow currently uploads only the binary?
