# Pin cargo-audit Version Review

Request: `dev-record/review-requests/017-pin-cargo-audit-version.md`

Date: 2026-07-10

## Verdict

Accept with notes.

Pinning `cargo-audit` to `0.22.2` in both CI and release preflight is an appropriate process-hardening change. It resolves the moving-tool-install note from `dev-record/reviews/008-ci-release-gate-alignment-rereview.md` while leaving the checked-in audit policy unchanged.

## Blocking findings

None.

## Non-blocking findings

1. The pin should be revisited periodically so the audit tool itself does not become stale. A formal cadence is not necessary for this patch, but the release owner should treat future cargo-audit upgrades as intentional review items.

## Requirement checks

- `.github/workflows/ci.yml:42`-`.github/workflows/ci.yml:43` installs `cargo-audit` with `cargo install cargo-audit --version 0.22.2 --locked`.
- `.github/workflows/release.yml:43`-`.github/workflows/release.yml:44` installs the same pinned version in release preflight.
- Both workflows still run `cargo audit` after installation, so the audit gate behavior remains in place.
- The locally installed `cargo-audit` version used for current verification is `0.22.2`.

## Reviewer questions

- Yes, pinning `cargo-audit` to `0.22.2` is acceptable for CI and release preflight.
- Review-driven updates are sufficient for now. A regular cadence can be added later if release maintenance becomes less frequent or if security tooling updates need stricter governance.

## Observed evidence

- `cargo audit --version` printed `cargo-audit-audit 0.22.2`.
- `cargo audit` passed under the checked-in policy and reported the expected 14 allowed warnings.
- `rg -n "cargo install cargo-audit" .github/workflows -g '*.yml'` showed both workflow install commands pinned to `0.22.2`.
- `git diff --check` passed.

## Missing evidence

- The pinned `cargo install cargo-audit --version 0.22.2 --locked` command was not executed in this environment.
- No live GitHub Actions run was observed.

## Recommended next action

Proceed with this workflow pinning change. Track future `cargo-audit` version bumps as explicit maintenance changes.
