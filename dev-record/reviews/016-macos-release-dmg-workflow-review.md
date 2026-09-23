# macOS Release DMG Workflow Review

Request: `dev-record/review-requests/013-macos-release-dmg-workflow.md`

Date: 2026-07-10

## Verdict

Accept with notes.

The macOS release workflow now aligns with the local macOS helper and packaging documentation: it installs `create-dmg`, builds the explicit `aarch64-apple-darwin` target expected by `packaging/macos/build-dmg.sh`, runs the helper, uploads the generated DMG, and updates the GitHub release file glob from `.zip` to `.dmg`.

This accepts the workflow alignment change. It does not close macOS runtime/package verification because the workflow and DMG creation path were not executed in a macOS environment.

## Blocking findings

None.

## Non-blocking findings

1. The workflow uploads `target/forskscope-v*-macos-aarch64.dmg` while the release job glob is `forskscope-v*-macos-aarch64.dmg`. This appears acceptable for `actions/upload-artifact` wildcard behavior because path hierarchy before the first wildcard is flattened, and the first wildcard is in the file name. Still, the final confirmation needs a live workflow run because the artifact download/release handoff was not observed locally.

2. `packaging/macos/build-dmg.sh` does not remove an existing `target/forskscope-vX.Y.Z-macos-aarch64.dmg` before calling `create-dmg`. I did not confirm whether `create-dmg` overwrites existing output. Because the workflow caches `target`, a rerun with the same version could restore an existing DMG if the cache was saved after packaging. This is not a blocker for the current alignment review, but adding `rm -f "$OUT"` before `create-dmg` would make the helper and workflow more deterministic.

## Requirement checks

- `.github/workflows/release.yml:148`-`.github/workflows/release.yml:153` configures the Rust toolchain with `aarch64-apple-darwin` and installs `create-dmg`.
- `.github/workflows/release.yml:163`-`.github/workflows/release.yml:167` builds `cargo build --release --locked --target aarch64-apple-darwin -p forskscope-ui` and runs `bash packaging/macos/build-dmg.sh`.
- `.github/workflows/release.yml:169`-`.github/workflows/release.yml:172` uploads `target/forskscope-v*-macos-aarch64.dmg`.
- `.github/workflows/release.yml:225`-`.github/workflows/release.yml:229` includes `forskscope-v*-macos-aarch64.dmg` in the draft release files.
- `packaging/macos/build-dmg.sh:9`-`packaging/macos/build-dmg.sh:13` expects the same target binary path and emits `target/forskscope-v$VER-macos-aarch64.dmg`.
- `packaging/README.md:37`-`packaging/README.md:42` documents the same local helper and DMG output path.

## Reviewer questions

- Yes, it is acceptable for the GitHub release workflow to publish the same DMG artifact format documented for the local macOS helper.
- Yes, using the explicit `aarch64-apple-darwin` target is correct for the helper's expected `target/aarch64-apple-darwin/release/forskscope` binary path.
- `brew install create-dmg` is appropriate if the project wants the release workflow to produce the documented DMG. Keeping a workflow zip while the helper produces a DMG would preserve the previous inconsistency.

## Observed evidence

- `bash -n packaging/macos/build-dmg.sh` passed.
- `git diff --check` passed.
- A reference scan found no remaining `macos-aarch64.zip` references in the reviewed workflow, packaging docs, maintainer docs, README, or scripts.
- The same scan found `.dmg` references in `.github/workflows/release.yml`, `packaging/README.md`, and `packaging/macos/build-dmg.sh`.
- I checked the `actions/upload-artifact` documentation for wildcard path behavior while reviewing the `target/...dmg` upload path and release-job download handoff.

## Missing evidence

- No live GitHub Actions run was observed.
- No macOS `cargo build --target aarch64-apple-darwin` run was observed in this Linux environment.
- No `create-dmg` execution was observed.
- No DMG inspection, signing, notarization, or macOS runtime smoke was observed.
- No local `actionlint`, `yamllint`, `ruby`, or `yq` executable was available for independent workflow/YAML validation.

## Recommended next action

Proceed with this workflow alignment. Track a live macOS release-workflow run or macOS-machine package verification as the remaining evidence, and consider adding `rm -f "$OUT"` to `packaging/macos/build-dmg.sh` before final release hardening.
