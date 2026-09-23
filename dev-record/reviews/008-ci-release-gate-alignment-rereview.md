# CI / release gate alignment re-review

Reviewed:

- `dev-record/review-requests/006-ci-release-gate-alignment-rereview.md`
- Prior review: `dev-record/reviews/007-ci-release-gate-alignment-review.md`
- `.github/workflows/release.yml`
- `.github/workflows/ci.yml`
- `xtask/src/main.rs`
- `docs/src/maintainers/release.md`
- `docs/src/maintainers/testing.md`
- UI files still in the implementation scope from the previous review

## Verdict

Accept with notes.

The blocking tag/workspace version mismatch finding from the previous review is
fixed. Release preflight now calls `cargo xtask version-sync "${GITHUB_REF_NAME#v}"`,
and `cargo xtask version-sync` now accepts an optional expected version and
fails when that value differs from `[workspace.package] version`.

## Blocking findings

None.

## Non-blocking findings

1. The i18n gate still covers `t(...)` translation keys, not every hardcoded
   user-facing UI string.

   This is acceptable if the intended release gate is "all `t(...)` keys have
   Japanese coverage." It is not a complete "all UI strings are localized" gate.
   The hardcoded binary badge tooltip at
   `crates/forskscope-ui/src/ui/view/dir_pane.rs:281` remains outside the
   scanner.

2. `cargo install cargo-audit --locked` remains a moving tool install.

   `.github/workflows/ci.yml:42` and `.github/workflows/release.yml:43` install
   the current resolvable `cargo-audit` release. This is acceptable for a small,
   repo-local gate, but a stricter release process should pin a cargo-audit
   version or a maintained action revision.

3. CI still intentionally duplicates some expensive gates.

   CI runs both the headless release-gate test/clippy pair and workspace
   test/clippy. This is defensible for release readiness, but it may become a CI
   runtime concern.

## Requirement checks

- Release preflight blocks tag/workspace version mismatches before artifacts:
  pass. `.github/workflows/release.yml:64` calls
  `cargo xtask version-sync "${GITHUB_REF_NAME#v}"`, and `source` still has
  `needs: preflight` at `.github/workflows/release.yml:74`.
- Optional expected-version interface: pass. `xtask/src/main.rs:31` accepts at
  most one optional argument for `version-sync`, and `xtask/src/main.rs:266`
  fails if that expected value differs from the workspace version.
- Local version-sync behavior remains intact: pass. No-argument version sync
  still validates repository metadata without requiring a release tag value.
- Docs explain local sync and release tag validation: pass with minor polish.
  `docs/src/maintainers/release.md:11` lists both forms, though it is dense
  enough that a separate release-tag bullet would be easier to scan.
- Release workflow dependency graph: pass. `source` depends on `preflight`, and
  platform jobs depend on `source`, so preflight gates run before artifact
  creation.

## Observed evidence

Commands observed during this re-review pass:

- `cargo xtask version-sync` - passed.
- `cargo xtask version-sync 0.164.0` - passed.
- `cargo xtask version-sync 0.165.0` - failed as expected with:
  `release version mismatch: expected 0.165.0, but [workspace.package] version is 0.164.0`
- `cargo fmt --check` - passed.
- `cargo clippy --workspace -- -D warnings` - passed.
- `git diff --check` - passed.

Generated artifact removed after verification:

- `xtask/Cargo.lock`

## Missing evidence

- GitHub Actions execution was not observed.
- Workflow YAML parser validation was not observed.
- The full release preflight job was not executed end to end in Actions.

## Recommended next action

Accept this gate-alignment fix. Before final release polish, consider splitting
the release version checklist item into separate local metadata and tag-match
bullets, and decide whether the i18n and cargo-audit tool-pinning notes should
be handled before v1.0 or tracked as follow-up process hardening.
