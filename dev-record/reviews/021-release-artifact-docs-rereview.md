# Release Artifact Docs Re-review

Request: `dev-record/review-requests/016-release-artifact-docs.md`

Date: 2026-07-10

## Verdict

Accept.

The previous process-order blocker is resolved. The release docs now list the current source and platform artifacts, and the post-check instructions match the tag-triggered workflow: tag the commit, push the tag, then inspect the draft GitHub release artifacts created by the workflow.

## Blocking findings

None.

## Non-blocking findings

None.

## Requirement checks

- `docs/src/maintainers/release.md:57`-`docs/src/maintainers/release.md:64` lists the source archive, Linux binary tarball, macOS DMG, and Windows zip.
- `docs/src/maintainers/release.md:79`-`docs/src/maintainers/release.md:85` now uses "After local artifact checks" and places tag creation plus tag push before draft release artifact inspection.
- `.github/workflows/release.yml:3`-`.github/workflows/release.yml:6` confirms the release workflow is tag-triggered.
- `.github/workflows/release.yml:216`-`.github/workflows/release.yml:229` confirms the workflow downloads artifacts and creates the draft release with the matching platform artifact globs.
- The Windows artifact table description remains specific enough without overloading the table row.

## Reviewer questions

- Yes, the release artifact table accurately reflects the workflow outputs.
- Yes, the Windows artifact contents description is specific enough for maintainer release documentation.

## Observed evidence

- `git diff --check` passed.
- `mdbook build docs` passed.
- A release-artifact reference scan across `docs/src/maintainers/release.md`, `packaging/README.md`, and `.github/workflows/release.yml` confirmed the documented artifact names match the workflow/helper names.
- A process-order scan found no remaining `Upload the source and platform artifacts`, `Upload the archive`, `After the artifacts`, or `After the archive` wording in `docs/src/maintainers/release.md`.
- Generated `docs/book/` and `rfcs/index.html` outputs from the docs build were removed after review.

## Missing evidence

- No live GitHub Actions release workflow was observed.
- No platform runtime/package verification was performed for this documentation review.

## Recommended next action

Proceed with this release documentation update. Keep live release-workflow execution and platform runtime/package verification tracked separately.
