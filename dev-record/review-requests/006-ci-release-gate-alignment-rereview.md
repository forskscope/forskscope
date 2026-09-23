# Re-review Request: CI / Release Gate Alignment

## Context

This re-review addresses the blocking finding in:

- `dev-record/reviews/007-ci-release-gate-alignment-review.md`

The prior review accepted the CI/release gate alignment overall, but found that
the release workflow did not verify that the pushed tag version matches the
workspace/package version before naming and publishing artifacts.

## Blocking Fix

- Extended `cargo xtask version-sync` to accept an optional expected version:
  - `cargo xtask version-sync`
  - `cargo xtask version-sync 0.164.0`
- When an expected version is provided, the command now fails if it differs from
  `[workspace.package] version` in `Cargo.toml`.
- Updated the release preflight job to call:

```sh
cargo xtask version-sync "${GITHUB_REF_NAME#v}"
```

This makes a tag such as `v0.165.0` fail preflight if the checked-out workspace
still declares version `0.164.0`.

## Files To Inspect

Focused blocker fix:

- `xtask/src/main.rs`
- `.github/workflows/release.yml`
- `docs/src/maintainers/release.md`

Previously reviewed files still in the implementation scope:

- `.github/workflows/ci.yml`
- `crates/forskscope-ui/src/state/compare.rs`
- `crates/forskscope-ui/src/ui/component/notice.rs`
- `crates/forskscope-ui/src/ui/view/diff.rs`
- `crates/forskscope-ui/src/ui/view/diff_actions.rs`
- `crates/forskscope-ui/src/ui/view/dir_pane.rs`
- `docs/src/maintainers/testing.md`

## Review Questions

1. Does the release preflight now block tag/workspace version mismatches before any artifact is created?
2. Is the optional expected-version argument on `cargo xtask version-sync` a suitable local/release interface?
3. Are the docs clear enough about the difference between normal local version sync and release tag validation?

## Verification To Run

Recommended focused checks:

- `cargo fmt --check`
- `cargo xtask version-sync`
- `cargo xtask version-sync 0.164.0`
- `cargo xtask version-sync 0.165.0` should fail with a version mismatch
- `cargo clippy --workspace -- -D warnings`
- `git diff --check`

## Observed Local Verification

Passed:

- `cargo xtask version-sync`
- `cargo xtask version-sync 0.164.0`
- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `git diff --check`

Expected failure observed:

- `cargo xtask version-sync 0.165.0`
  - failed with: `release version mismatch: expected 0.165.0, but [workspace.package] version is 0.164.0`

Not locally verified:

- GitHub Actions execution.
- Workflow YAML parser validation.
