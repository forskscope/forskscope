# Review request: pin cargo-audit workflow version

## Scope

This package addresses the non-blocking process-hardening note from
`dev-record/reviews/008-ci-release-gate-alignment-rereview.md` that
`cargo install cargo-audit --locked` was still a moving tool install in CI and
release preflight.

## Files to inspect

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

## Change summary

- Changed both workflow install commands from:
  - `cargo install cargo-audit --locked`
- to:
  - `cargo install cargo-audit --version 0.22.2 --locked`

## Why this changed

The security advisory gate should not silently change tool versions just
because Cargo resolves a newer `cargo-audit` release. Pinning the version makes
CI and release preflight behavior more reproducible while keeping the existing
checked-in audit policy unchanged.

`0.22.2` matches the locally installed `cargo-audit` version used for current
verification.

## Observed verification

Passed in this thread:

- `cargo audit --version`
  - output: `cargo-audit-audit 0.22.2`
- `cargo audit`
  - passed under the checked-in policy
  - reported the expected 14 allowed warnings
- `rg -n "cargo install cargo-audit" .github/workflows -g '*.yml'`
  - confirmed both workflow install commands are pinned to `0.22.2`
- `git diff --check`

## Limitations

- The pinned `cargo install` command itself was not executed in this environment
  because it would reinstall/download tooling.
- No live GitHub Actions run was observed.

## Reviewer questions

- Is pinning `cargo-audit` to `0.22.2` acceptable for CI and release preflight?
- Should the project document a regular cadence for updating this pinned tool
  version, or is review-driven updating sufficient for now?
