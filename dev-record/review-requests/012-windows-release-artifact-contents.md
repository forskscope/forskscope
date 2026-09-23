# Review request: Windows release artifact contents

## Scope

This package addresses the remaining non-blocking follow-up from
`dev-record/reviews/012-cross-platform-packaging-helper-names-review.md`:
the local Windows helper stages documentation/license files, while the GitHub
release workflow previously archived only `forskscope.exe`.

## Files to inspect

- `.github/workflows/release.yml`

## Change summary

- Updated the Windows release workflow package step to stage:
  - `forskscope.exe`
  - `README.md`
  - `LICENSE`
  - `NOTICE`
  - `CHANGELOG.md`
- The workflow removes the staging directory before recreating it, matching the
  local Windows helper and preventing stale files from cached `target/` state.
- The workflow now archives the staged
  `forskscope-vX.Y.Z-windows-x64/` directory from `target`, matching the local
  `packaging/windows/build-zip.sh` artifact contents and archive root.

## Why this changed

The local Windows helper and GitHub release workflow used the same artifact
name convention, but not the same contents. Converging the workflow on the
helper's fuller archive makes local packaging verification more representative
of the release artifact and includes license/notice material next to the
Windows executable.

## Observed verification

Passed in this thread:

- Local execution of the workflow-equivalent package snippet with
  `VER=0.164.0` and an ignored dummy `target/release/forskscope.exe`.
- `7z` reported `Everything is Ok`.
- `unzip -l forskscope-v0.164.0-windows-x64.zip`
  - confirmed top-level directory
    `forskscope-v0.164.0-windows-x64/`
  - confirmed included files:
    `CHANGELOG.md`, `LICENSE`, `NOTICE`, `README.md`, `forskscope.exe`
- `7z l forskscope-v0.164.0-windows-x64.zip`
  - independently confirmed the same archive root and files
- Stale-stage reproduction:
  - created `target/forskscope-v0.164.0-windows-x64/stale.txt`
  - reran the workflow-equivalent package snippet with `rm -rf "$STAGE"`
  - `7z` reported `Everything is Ok`
  - `unzip -l` and `7z l` showed only the intended staged files
  - `unzip -l ... | rg 'stale\.txt'` returned no match
- `git diff --check`

## Limitations

- The workflow itself was not run on GitHub Actions.
- The local command used an ignored dummy `target/release/forskscope.exe`, not a
  real Windows build.
- This does not close Windows runtime/package verification for release.
- macOS workflow-vs-helper artifact format remains a separate policy question:
  the workflow emits a zip while the local helper emits a DMG.

## Reviewer questions

- Is it acceptable for the Windows GitHub release artifact to include the same
  README/license/notice/changelog files as the local helper?
- Does the workflow archive root now match the local helper's
  `forskscope-vX.Y.Z-windows-x64/` root?
- Should macOS workflow-vs-helper artifact format convergence be handled as a
  separate follow-up?
