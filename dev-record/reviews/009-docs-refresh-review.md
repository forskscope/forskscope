# Docs Refresh Review

Request: `dev-record/review-requests/007-docs-refresh.md`

Date: 2026-07-09

## Verdict

Accept with notes.

The documentation refresh is consistent with the current v0.164.0 release-readiness decisions for security/audit remediation, S-001 network dependency handling, source archive layout, CI/release gate alignment, and remaining runtime/platform verification. I found no blocking issues.

## Blocking findings

None.

## Non-blocking findings

1. `README.md:23` still says ForskScope is "packaged for Linux, macOS, and Windows", while `ROADMAP.md:36`-`ROADMAP.md:38` correctly says packaging verification on Linux, macOS, and Windows remains. This is not a blocker because the roadmap and maintainer docs preserve the release-readiness boundary, but the README could be softened before v1.0 to avoid implying that all platform packages have already been verified.

## Requirement checks

- Release status claims are no longer stale: `ROADMAP.md:3`-`ROADMAP.md:4` identifies v0.164.0 and says runtime/platform verification remains.
- The 930-test headless gate is internally consistent across `ROADMAP.md:10`-`ROADMAP.md:13`, `docs/src/maintainers/testing.md:41`-`docs/src/maintainers/testing.md:55`, `docs/src/maintainers/local-dev.md:16`-`docs/src/maintainers/local-dev.md:19`, and `docs/src/maintainers/gtk-smoke-test.md:20`-`docs/src/maintainers/gtk-smoke-test.md:24`.
- The README no longer claims completed release builds on tag push. `README.md:84` describes configured GitHub Actions gates instead of verified release artifacts.
- The threat model now matches the S-001 decision: it states no app-authored external network behavior, documents the Dioxus loopback WebSocket exception, and records the reviewed `tungstenite`/`native-tls` path at `docs/src/maintainers/threat-model.md:153`-`docs/src/maintainers/threat-model.md:205`.
- The XLSX security posture is reflected in the threat model: `.xlsx` comparison fails closed while the vulnerable parser path is disabled, with the remaining `quick-xml` path limited to `wayland-scanner` at `docs/src/maintainers/threat-model.md:208`-`docs/src/maintainers/threat-model.md:223`.
- The release archive contract is documented as no top-level parent directory and covered by `cargo xtask archive-layout` in `docs/src/maintainers/release.md:22`-`docs/src/maintainers/release.md:53`.
- The release checklist includes the aligned gates for format, audit policy, dependency-path audit, CSS freshness, version sync, tag version match, i18n, and archive layout at `docs/src/maintainers/release.md:5`-`docs/src/maintainers/release.md:14`.

## Observed evidence

- `cargo test -p forskscope-core -p forskscope-ui-logic` passed during this review thread. The observed suite total matches the documented 930-test breakdown.
- `cargo xtask version-sync` passed with `version sync passed for v0.164.0.`
- `cargo xtask i18n` passed with `i18n audit passed: 202 UI keys are covered by Japanese translations.`
- Stale-claim scan over `README.md`, `ROADMAP.md`, and `docs/src` found no matches for the reviewed stale claims (`936`, old version markers, old release-build wording, old test/i18n counts).
- `mdbook build docs` passed.
- `git diff --check` passed.

## Missing evidence

- I did not observe a live GitHub Actions run for these docs changes.
- I did not observe GTK/WebKit runtime smoke tests or Linux/macOS/Windows packaging verification in this review.
- I did not rerun full-workspace GUI-dependent gates in a GTK environment.

## Recommended next action

Proceed with the docs refresh. Optionally soften the README platform-packaging phrase before final release so it reads as a packaging target rather than already-verified platform package status.
