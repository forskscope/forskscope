# Review request: macOS release DMG workflow

## Scope

This package addresses the macOS workflow-vs-helper artifact format follow-up
left open by the Windows release artifact reviews.

## Files to inspect

- `.github/workflows/release.yml`

## Change summary

- Updated the macOS release job to install `create-dmg`.
- Configured `dtolnay/rust-toolchain` with the explicit
  `aarch64-apple-darwin` target.
- Changed the macOS release build from host `target/release/forskscope` to:
  - `cargo build --release --locked --target aarch64-apple-darwin -p forskscope-ui`
- Changed the macOS package step to run:
  - `bash packaging/macos/build-dmg.sh`
- Changed the uploaded macOS artifact path from:
  - `forskscope-v*-macos-aarch64.zip`
  - to `target/forskscope-v*-macos-aarch64.dmg`
- Changed the GitHub release file glob from:
  - `forskscope-v*-macos-aarch64.zip`
  - to `forskscope-v*-macos-aarch64.dmg`

## Why this changed

The local macOS packaging helper and `packaging/README.md` define the macOS
release artifact as `target/forskscope-vX.Y.Z-macos-aarch64.dmg`, but the
GitHub release workflow previously uploaded a bare-binary zip. Aligning the
workflow with the helper makes the documented local packaging path and the
published release artifact format consistent.

## Observed verification

Passed in this thread:

- `bash -n packaging/macos/build-dmg.sh`
- `rg -n "macos-aarch64\\.zip|macos-aarch64\\.dmg|create-dmg|build-dmg" .github/workflows/release.yml packaging docs README.md -g '*.yml' -g '*.md' -g '*.sh'`
  - found no remaining `macos-aarch64.zip` references
  - found `.dmg` references in the workflow, packaging README, and helper
- `git diff --check`

## Limitations

- The macOS workflow was not run on GitHub Actions.
- This Linux environment cannot execute the macOS build, `sed -i ''`, or
  `create-dmg` path.
- No real macOS runtime/package verification was observed.
- No signing or notarization is added; existing docs still state those are not
  automated.
- No local `actionlint`, `yamllint`, `ruby`, or `yq` executable was available
  for workflow/YAML validation.

## Reviewer questions

- Is it acceptable for the GitHub release workflow to publish the same DMG
  artifact format documented for the local macOS helper?
- Is using the explicit `aarch64-apple-darwin` target in the workflow correct
  for the helper's expected binary path?
- Should `brew install create-dmg` remain in the release workflow, or should the
  workflow continue publishing a zip while DMG creation remains manual?
