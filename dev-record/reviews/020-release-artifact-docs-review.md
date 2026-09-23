# Release Artifact Docs Review

Request: `dev-record/review-requests/016-release-artifact-docs.md`

Date: 2026-07-10

## Verdict

Needs changes.

The new release artifact table accurately reflects the current source, Linux, macOS, and Windows artifact names. However, the changed post-artifact instructions still describe uploading artifacts before tagging the commit, while the release workflow is triggered by a tag push and creates/uploads the draft release artifacts after that tag exists. That leaves the maintainer release process internally inconsistent.

## Blocking findings

1. `docs/src/maintainers/release.md:79`-`docs/src/maintainers/release.md:82` says to upload the source and platform artifacts to the release page before tagging the commit. The current release workflow is tag-triggered at `.github/workflows/release.yml:3`-`.github/workflows/release.yml:6`, then downloads job artifacts and creates the draft release at `.github/workflows/release.yml:216`-`.github/workflows/release.yml:229`. For the documented workflow path, the tag must exist before the release artifacts are built and attached. Adjust the post-build section so the maintainer order is tag first, then inspect the workflow-created draft release/artifacts, or explicitly separate a manual local-upload path from the tag-triggered workflow path.

## Non-blocking findings

None.

## Requirement checks

- `docs/src/maintainers/release.md:57`-`docs/src/maintainers/release.md:64` now lists:
  - `forskscope-vX.Y.Z.tar.gz`
  - `forskscope-vX.Y.Z-linux-x86_64.tar.gz`
  - `forskscope-vX.Y.Z-macos-aarch64.dmg`
  - `forskscope-vX.Y.Z-windows-x64.zip`
- `.github/workflows/release.yml:133`-`.github/workflows/release.yml:139` produces/uploads the Linux x86_64 tarball with the same name pattern.
- `.github/workflows/release.yml:163`-`.github/workflows/release.yml:172` builds/runs the macOS DMG helper and uploads `target/forskscope-v*-macos-aarch64.dmg`.
- `.github/workflows/release.yml:197`-`.github/workflows/release.yml:208` stages the Windows executable plus README/license/notice/changelog and uploads `forskscope-v*-windows-x64.zip`.
- `.github/workflows/release.yml:225`-`.github/workflows/release.yml:229` includes the same platform artifact globs in the draft GitHub release.
- The Windows table description is specific enough for this table; it names the key bundled docs plus executable without overloading the row.

## Reviewer questions

- The artifact table now accurately reflects the workflow outputs.
- The Windows artifact contents description is specific enough.
- The post-artifact release process order still needs correction before accepting this docs package.

## Observed evidence

- `git diff --check` passed.
- `mdbook build docs` passed.
- A release-artifact reference scan across `docs/src/maintainers/release.md`, `packaging/README.md`, and `.github/workflows/release.yml` confirmed the new table names match the workflow/helper artifact names.
- Generated `docs/book/` and `rfcs/index.html` outputs from the docs build were removed after review.

## Missing evidence

- No live GitHub Actions release workflow was observed.
- No platform runtime/package verification was performed for this documentation review.

## Recommended next action

Revise `docs/src/maintainers/release.md` so the post-artifact instructions match the tag-triggered workflow. After that, re-run `git diff --check`, `mdbook build docs`, and the release-artifact reference scan.
