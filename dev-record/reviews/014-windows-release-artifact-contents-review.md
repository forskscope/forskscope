# Windows Release Artifact Contents Review

Request: `dev-record/review-requests/012-windows-release-artifact-contents.md`

Date: 2026-07-10

## Verdict

Needs changes.

The intended Windows artifact contents are reasonable, and the workflow archive root matches the local helper in the clean-stage case. However, the workflow package step stages into cached `target/` without clearing the stage directory first. Because the Windows job caches `target`, stale files can leak into the release zip on reruns or cache restores.

## Blocking findings

1. `.github/workflows/release.yml:180`-`.github/workflows/release.yml:199` can include stale cached stage files in the Windows release artifact. The job caches `target`, then the package step uses `mkdir -p "$STAGE"` and copies the intended files into the existing directory without `rm -rf "$STAGE"` first. I reproduced this by adding `target/forskscope-v0.164.0-windows-x64/stale.txt`, running the workflow-equivalent snippet, and observing `forskscope-v0.164.0-windows-x64/stale.txt` inside the zip. This breaks the stated goal of matching the local helper's exact staged contents. Fix by removing the stage directory before recreating it, matching `packaging/windows/build-zip.sh:19`-`packaging/windows/build-zip.sh:22`.

## Non-blocking findings

None beyond the blocking staging-cleanliness issue.

## Requirement checks

- `.github/workflows/release.yml:194`-`.github/workflows/release.yml:199` now stages `forskscope.exe`, `README.md`, `LICENSE`, `NOTICE`, and `CHANGELOG.md`, then archives `forskscope-vX.Y.Z-windows-x64/` from `target`.
- `packaging/windows/build-zip.sh:19`-`packaging/windows/build-zip.sh:27` already removes and recreates the local helper stage before archiving, so the workflow should mirror that cleanup behavior.
- In a clean-stage local run, the workflow-equivalent snippet produced a zip rooted at `forskscope-v0.164.0-windows-x64/` with the intended five files.
- In a dirty-stage local run, the same snippet included an extra stale file, confirming the current workflow is not deterministic under the cached `target` path.

## Reviewer questions

- It is acceptable for the Windows GitHub release artifact to include the same README/license/notice/changelog files as the local helper. That is better for distribution and makes local helper verification more representative.
- The workflow archive root matches the local helper only when the stage directory is clean. Add `rm -rf "$STAGE"` before `mkdir -p "$STAGE"` to make this reliable.
- macOS workflow-vs-helper artifact format convergence should remain a separate follow-up. This change is specifically about Windows artifact contents.

## Observed evidence

- `git diff --check` passed.
- Local clean-stage execution of the workflow-equivalent snippet with `VER=0.164.0` and the ignored dummy `target/release/forskscope.exe` passed; `7z` reported `Everything is Ok`.
- `unzip -l forskscope-v0.164.0-windows-x64.zip` and `7z l forskscope-v0.164.0-windows-x64.zip` confirmed the clean-stage archive root and intended files.
- Stale-stage reproduction: after writing `target/forskscope-v0.164.0-windows-x64/stale.txt`, rerunning the workflow-equivalent snippet included `forskscope-v0.164.0-windows-x64/stale.txt` in the zip.
- Generated test artifacts were removed after review.

## Missing evidence

- No live GitHub Actions release workflow run was observed.
- No real Windows build was observed; local snippet execution used an ignored dummy executable.
- No Windows runtime/package verification was observed.
- macOS helper-vs-workflow artifact format convergence was not reviewed here.

## Recommended next action

Update the Windows workflow package step to clear the stage directory before copying files, for example:

```sh
STAGE="target/forskscope-v${VER}-windows-x64"
rm -rf "$STAGE"
mkdir -p "$STAGE"
```

Then rerun the clean-stage and stale-stage local snippet checks and re-request review.
