# Review Request: Docs Refresh

## Request

Review the v0.164.0 documentation refresh.

The reviewer can inspect the project directly. Please focus on the files listed
below and compare the docs against the current release-readiness decisions:
security/audit remediation, S-001 network dependency decision, source archive
contract, and CI/release gate alignment.

## Requirements / Intent

The roadmap item was:

- Update `ROADMAP.md`, `docs/src/maintainers/testing.md`,
  `docs/src/maintainers/threat-model.md`, and README release claims so they
  match v0.164.0 and current gate reality.

The implementation also updates two nearby maintainer docs that contained stale
test-count references:

- `docs/src/maintainers/local-dev.md`
- `docs/src/maintainers/gtk-smoke-test.md`

## Implementation Summary

- Updated `ROADMAP.md` from the stale v0.140.0 state to v0.164.0:
  - current phase is release-readiness verification
  - observed headless test total is 930
  - i18n gate covers 202 `t(...)` keys
  - XLSX parsing is fail-closed pending parser dependency remediation
  - Dioxus desktop loopback IPC/network-capable dependency decision is recorded
  - source archive contract and CI/release gates are recorded
  - remaining work is runtime/platform verification.
- Updated maintainer testing docs:
  - current headless gate wording
  - full workspace gates when GTK/WebKitGTK dependencies are available
  - v0.164.0 test-count table.
- Updated threat model:
  - version marker to v0.164.0
  - `.xlsx` fail-closed behavior under file-load controls
  - audit history entries for XLSX dependency removal, Dioxus dependency policy,
    and release/CI gate alignment.
- Updated README release/CI claim:
  - no longer implies verified release builds have already shipped
  - describes the configured CI and draft-release gates.
- Updated local-dev and GTK smoke docs to replace stale 936-test references.
- Split the release checklist version gate into local metadata sync and
  release tag/workspace version validation.

## Files To Inspect

- `README.md`
- `ROADMAP.md`
- `docs/src/maintainers/testing.md`
- `docs/src/maintainers/threat-model.md`
- `docs/src/maintainers/release.md`
- `docs/src/maintainers/local-dev.md`
- `docs/src/maintainers/gtk-smoke-test.md`

## Review Questions

1. Do the docs now match the accepted v0.164.0 gate reality?
2. Are any release claims still stronger than what has actually been verified?
3. Is the 930-test breakdown clear and consistent across maintainer docs?
4. Does the threat model accurately represent XLSX fail-closed behavior and the
   accepted Dioxus loopback WebSocket IPC path?
5. Is it acceptable that README describes configured GitHub Actions gates rather
   than verified release artifacts?

## Observed Local Verification

Passed:

- `cargo test -p forskscope-core -p forskscope-ui-logic`
  - observed total: 930 tests
  - breakdown: 643 core unit, 27 diff corpus, 16 merge corpus, 2 patch apply,
    228 ui-logic unit, 6 CSS integration, 7 core doctests, 1 ui-logic doctest
- `cargo xtask version-sync`
- `cargo xtask i18n`
  - reported 202 UI keys covered
- stale-claim scan for old counts/release wording:
  - no remaining matches for `936`, `v0.135.0`, `code and docs complete`,
    `release builds on tag push`, `GitHub Actions CI/CD`, or stale per-crate
    count comments
- `mdbook build docs`
- `git diff --check`

Generated artifacts removed after verification:

- `docs/book/`
- `rfcs/index.html`
- `xtask/Cargo.lock`

Not locally verified:

- GitHub Actions execution.
- Runtime/platform verification.

## Current Working Tree Scope

Expected modified files:

- `README.md`
- `ROADMAP.md`
- `docs/src/maintainers/gtk-smoke-test.md`
- `docs/src/maintainers/local-dev.md`
- `docs/src/maintainers/release.md`
- `docs/src/maintainers/testing.md`
- `docs/src/maintainers/threat-model.md`

