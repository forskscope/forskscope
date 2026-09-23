# Align Artifact Action Versions Review

Request: `dev-record/review-requests/018-align-artifact-action-versions.md`

Date: 2026-07-10

## Verdict

Accept with notes.

The release workflow artifact actions are now internally aligned: every artifact upload uses `actions/upload-artifact@v7`, and the release job uses `actions/download-artifact@v7`. This is consistent with the surrounding workflow's newer action major versions.

## Blocking findings

None.

## Non-blocking findings

1. `actions/download-artifact@v7` documents a Node.js 24 runtime and a minimum Actions Runner version of `2.327.1` for self-hosted runners. `actions/cache@v5` documents the same runner minimum. This is acceptable for GitHub-hosted runners, but if the project introduces self-hosted runners, document and enforce that minimum runner version.

## Requirement checks

- `.github/workflows/release.yml:97`, `:136`, `:169`, and `:205` all use `actions/upload-artifact@v7`.
- `.github/workflows/release.yml:216` uses `actions/download-artifact@v7`.
- A scan for `actions/(upload-artifact|download-artifact)@v[0-6]` in `.github/workflows` found no matches.
- Upstream release pages exist for `actions/upload-artifact` `v7.0.0` and `actions/download-artifact` `v7.0.0`.
- The reviewed workflow still keeps the existing artifact names and release file globs; this change only aligns action versions.

## Reviewer questions

- Yes, aligning all release artifact upload/download steps to `v7` is acceptable.
- A self-hosted runner minimum does not need to be documented now if the project only uses GitHub-hosted runners. Add it before supporting self-hosted Actions runners.

## Observed evidence

- `rg -n "actions/(upload-artifact|download-artifact)@" .github/workflows/release.yml` showed four `actions/upload-artifact@v7` uses and one `actions/download-artifact@v7` use.
- `rg -n "actions/(upload-artifact|download-artifact)@v[0-6]\b" .github/workflows/release.yml .github/workflows/ci.yml` found no matches.
- `git diff --check` passed.
- No local `actionlint`, `yamllint`, or `yq` executable was available for workflow/YAML validation.
- I checked GitHub release pages for the updated artifact action tags and related action major versions.

## Missing evidence

- No live GitHub Actions run was observed.
- No local workflow/YAML parser validation was observed.

## Recommended next action

Proceed with this workflow version alignment. Keep runner-version documentation as a future requirement only if self-hosted runners are introduced.

## Sources

- `actions/upload-artifact` v7.0.0: https://github.com/actions/upload-artifact/releases/tag/v7.0.0
- `actions/download-artifact` v7.0.0: https://github.com/actions/download-artifact/releases/tag/v7.0.0
- `actions/cache` v5.0.1: https://github.com/actions/cache/releases/tag/v5.0.1
