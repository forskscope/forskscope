# Windows Release Artifact Contents Re-review

Request: `dev-record/review-requests/012-windows-release-artifact-contents.md`

Date: 2026-07-10

## Verdict

Accept with notes.

The previous blocking finding is resolved. The Windows release workflow now clears the staging directory before recreating it, stages the same README/license/notice/changelog payload as the local Windows helper, and archives the staged `forskscope-vX.Y.Z-windows-x64/` directory from `target`.

This accepts the workflow packaging change. It still does not close real Windows runtime/package verification because the local evidence used a dummy ignored executable and the GitHub Actions workflow itself was not run.

## Blocking findings

None.

## Non-blocking findings

None for this scoped Windows artifact-content convergence.

The macOS workflow-vs-helper artifact format difference remains a separate policy question, as called out in the request.

## Requirement checks

- `.github/workflows/release.yml:194`-`.github/workflows/release.yml:200` now sets `STAGE`, removes it with `rm -rf "$STAGE"`, recreates it, copies `target/release/forskscope.exe`, stages `README.md`, `LICENSE`, `NOTICE`, and `CHANGELOG.md`, then archives `$(basename "$STAGE")/` from `target`.
- The cleanup mirrors `packaging/windows/build-zip.sh:19`-`packaging/windows/build-zip.sh:22`, resolving the stale cached-stage risk from `dev-record/reviews/014-windows-release-artifact-contents-review.md`.
- In clean-stage verification, the archive root was `forskscope-v0.164.0-windows-x64/` and the archive contained only the intended files.
- In stale-stage verification, a preexisting `target/forskscope-v0.164.0-windows-x64/stale.txt` did not appear in the final zip after the workflow-equivalent snippet ran.

## Reviewer questions

- Yes, it is acceptable for the Windows GitHub release artifact to include the same README/license/notice/changelog files as the local helper.
- Yes, the workflow archive root now matches the local helper's `forskscope-vX.Y.Z-windows-x64/` root.
- Yes, macOS workflow-vs-helper artifact format convergence should remain a separate follow-up.

## Observed evidence

- `git diff --check` passed.
- Clean-stage workflow-equivalent snippet with `VER=0.164.0` and the ignored dummy `target/release/forskscope.exe` passed; `7z` reported `Everything is Ok`.
- `unzip -l forskscope-v0.164.0-windows-x64.zip` and `7z l forskscope-v0.164.0-windows-x64.zip` confirmed the clean-stage archive root and intended files.
- Stale-stage reproduction passed: after creating `target/forskscope-v0.164.0-windows-x64/stale.txt`, rerunning the workflow-equivalent snippet with `rm -rf "$STAGE"` produced an archive containing only `CHANGELOG.md`, `LICENSE`, `NOTICE`, `README.md`, and `forskscope.exe`.
- `unzip -l forskscope-v0.164.0-windows-x64.zip | rg 'stale\.txt'` returned no match, which is the expected result for the stale-file exclusion check.
- Generated root zip artifacts were removed after review.

## Missing evidence

- No live GitHub Actions release workflow run was observed.
- No real Windows build was observed; local snippet execution used an ignored dummy executable.
- No Windows runtime/package verification was observed.
- macOS helper-vs-workflow artifact format convergence was not reviewed here.

## Recommended next action

Proceed with this Windows release artifact-content change. Keep Windows release readiness open for a real workflow or Windows-machine build, package inspection, and runtime smoke.
