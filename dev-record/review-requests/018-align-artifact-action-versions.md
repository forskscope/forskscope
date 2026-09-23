# Review request: align artifact action versions

## Scope

This package follows the project owner's GitHub Actions `uses:` version update
and aligns the remaining artifact upload/download refs in the release workflow.

## Files to inspect

- `.github/workflows/release.yml`

## Change summary

- Changed the Windows artifact upload step from:
  - `actions/upload-artifact@v4`
  - to `actions/upload-artifact@v7`
- Changed the release job artifact download step from:
  - `actions/download-artifact@v4`
  - to `actions/download-artifact@v7`

After this change, all release-workflow artifact upload steps use
`actions/upload-artifact@v7`, and the release job uses
`actions/download-artifact@v7`.

## Why this changed

The workflow had already been updated to newer action majors for checkout,
cache, most artifact uploads, and the GitHub release action. The Windows upload
and release download steps were still on older artifact actions, leaving the
release workflow internally inconsistent.

## Upstream tag verification

Checked upstream GitHub release pages in this thread:

- `actions/checkout` has `v7.0.0`.
- `actions/cache` has `v5.0.0` and `v5.0.1`.
- `actions/upload-artifact` has `v7.0.0`.
- `actions/download-artifact` has `v7.0.0`.
- `softprops/action-gh-release` has `v3.0.0` and `v3.0.1`.

The upstream pages also note that several Node 24 action versions require
Actions Runner `2.327.1`; this is expected to be satisfied by GitHub-hosted
runners, but should be considered if self-hosted runners are introduced.

## Observed verification

Passed in this thread:

- `rg -n "actions/(upload-artifact|download-artifact)@" .github/workflows/release.yml`
  - confirmed all upload steps are `actions/upload-artifact@v7`
  - confirmed the release download step is `actions/download-artifact@v7`
- `git diff --check`

## Limitations

- No live GitHub Actions run was observed.
- No local `actionlint`, `yamllint`, or `yq` executable was available for
  workflow/YAML validation.
- The upstream action tag checks used GitHub release pages, not a workflow run.

## Reviewer questions

- Is aligning all release artifact upload/download steps to `v7` acceptable?
- Should the repository document a minimum self-hosted runner version if it ever
  supports self-hosted Actions runners?
