# Review Request: CI / Release Gate Alignment

## Request

Review the CI and release gate alignment implementation.

The reviewer can inspect the project directly. Please focus on the files listed below and their interactions; the whole codebase does not need to be passed into the prompt.

## Requirements / Intent

The release-readiness roadmap identified missing CI/release gates after the earlier security, network-dependency, and source-archive contract work.

Required alignment:

- CI should enforce the same release-relevant gates documented for maintainers:
  - `cargo fmt --check`
  - `cargo xtask css --check`
  - `cargo audit`
  - `cargo xtask audit-deps`
  - `cargo test -p forskscope-core -p forskscope-ui-logic`
  - `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings`
  - version metadata sync
  - i18n coverage
- The tag release workflow should fail before artifact creation when release gates fail.
- Source archive layout verification should be reusable from local tooling and the release workflow.
- Keep changes small and repo-local; do not add new Rust dependencies.

## Implementation Summary

- Added new `xtask` commands:
  - `cargo xtask version-sync`
    - checks workspace version against `xtask/Cargo.toml`, `packaging/linux/PKGBUILD`, `packaging/windows/AppxManifest.xml`, `CHANGELOG.md`, and local package entries in `Cargo.lock`.
  - `cargo xtask i18n`
    - scans UI `t(...)` calls for string-literal translation keys and verifies Japanese translations exist in `crates/forskscope-ui/src/i18n.rs`.
  - `cargo xtask archive-layout [archive]`
    - verifies a source archive has root `Cargo.toml`, no `forskscope-vX.Y.Z/` parent directory, and no generated/local-only paths such as `.git-exclude/`, `.git/`, `target/`, or archive self-entry.
- Updated CI workflow to run:
  - fmt
  - CSS check
  - version sync
  - i18n audit
  - cargo audit
  - dependency-path audit
  - headless tests
  - headless clippy
  - existing workspace test, workspace clippy, and UI build smoke.
- Added a release `preflight` job that runs the release gates before the `source` job and platform artifact jobs.
- Replaced inline shell archive-layout checks in the release workflow with `cargo xtask archive-layout`.
- Updated maintainer testing/release docs to mention the new gates.

## Files To Inspect

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `xtask/src/main.rs`
- `crates/forskscope-ui/src/state/compare.rs`
- `crates/forskscope-ui/src/ui/component/notice.rs`
- `crates/forskscope-ui/src/ui/view/diff.rs`
- `crates/forskscope-ui/src/ui/view/diff_actions.rs`
- `crates/forskscope-ui/src/ui/view/dir_pane.rs`
- `docs/src/maintainers/testing.md`
- `docs/src/maintainers/release.md`

## Review Questions

1. Are the CI and release workflow gates correctly aligned with the current release prerequisites?
2. Is adding `cargo install cargo-audit --locked` in CI/release acceptable, or should the project use a pinned action/tool-cache strategy instead?
3. Is the `xtask i18n` string-literal scanner strict enough for the current UI pattern without creating avoidable false positives?
4. Is the `xtask version-sync` scope correct for this release, or should additional files be included/excluded?
5. Is the release workflow dependency graph correct, especially `source` depending on `preflight` before platform artifacts proceed?

## Observed Local Verification

Passed:

- `cargo fmt --check`
- `cargo xtask version-sync`
- `cargo xtask i18n`
- `cargo xtask css --check`
- Created a tracked-file archive under `target/forskscope-v0.164.0.tar.gz`
- `cargo xtask archive-layout target/forskscope-v0.164.0.tar.gz`
- `cargo audit`
  - exited successfully under checked-in policy
  - reported allowed warning advisories
- `cargo xtask audit-deps`
- `cargo test -p forskscope-core -p forskscope-ui-logic`
- `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings`
- `cargo clippy --workspace -- -D warnings`
- `git diff --check`

Not locally verified:

- GitHub Actions execution.
- Workflow YAML parser validation. `ruby` was not installed and Python `yaml` was not available in this environment.

## Current Working Tree Scope

Expected modified files:

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `crates/forskscope-ui/src/state/compare.rs`
- `crates/forskscope-ui/src/ui/component/notice.rs`
- `crates/forskscope-ui/src/ui/view/diff.rs`
- `crates/forskscope-ui/src/ui/view/diff_actions.rs`
- `crates/forskscope-ui/src/ui/view/dir_pane.rs`
- `docs/src/maintainers/release.md`
- `docs/src/maintainers/testing.md`
- `xtask/src/main.rs`
