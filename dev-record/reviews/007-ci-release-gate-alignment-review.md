# CI / release gate alignment review

Reviewed:

- `dev-record/review-requests/005-ci-release-gate-alignment.md`
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

## Verdict

Needs changes.

The CI workflow and most reusable local gates are aligned with the documented
release checklist. The release workflow now has a preflight job before artifact
creation, and `cargo xtask archive-layout` correctly centralizes source archive
layout verification. However, the release workflow still does not verify that
the Git tag version matches the workspace version before artifacts are named and
published.

## Blocking findings

1. The release workflow can publish artifacts named from a tag that does not
   match the workspace/package version.

   `.github/workflows/release.yml:82` derives `version` from
   `GITHUB_REF_NAME#v`, and artifact names later use that tag-derived value
   (`.github/workflows/release.yml:88`, `.github/workflows/release.yml:132`,
   `.github/workflows/release.yml:163`, `.github/workflows/release.yml:194`).
   The preflight job runs `cargo xtask version-sync`
   (`.github/workflows/release.yml:64`), but `run_version_sync()` only compares
   workspace metadata against files inside the checkout: `xtask/Cargo.toml`,
   `packaging/linux/PKGBUILD`, `packaging/windows/AppxManifest.xml`,
   `CHANGELOG.md`, and local `Cargo.lock` package entries
   (`xtask/src/main.rs:260` through `xtask/src/main.rs:299`).

   There is no check that the release tag equals `[workspace.package] version`.
   A pushed tag such as `v0.165.0` on a commit whose workspace version is still
   `0.164.0` would pass preflight version-sync, create source and binary
   artifacts named `forskscope-v0.165.0...`, and ship content whose Cargo
   metadata still says `0.164.0`.

   Fix by adding a release-only gate before artifact creation, for example:
   compare `${GITHUB_REF_NAME#v}` with the workspace version, or extend
   `cargo xtask version-sync` to accept an optional expected version and call it
   from the release workflow.

## Non-blocking findings

1. The i18n gate covers `t(...)` keys, not all user-facing UI strings.

   `cargo xtask i18n` passed and is useful for the current translation-key
   convention. It scans string literals inside `t(...)` invocations
   (`xtask/src/main.rs:191`) and checks them against Japanese match keys. It
   does not catch hardcoded user-facing strings that are not wrapped in `t(...)`.
   One current example is the binary badge tooltip in
   `crates/forskscope-ui/src/ui/view/dir_pane.rs:281`:

   ```text
   Binary file. Binary comparison is off — enable it in Settings → Advanced.
   ```

   If the intended release gate is "all UI-facing strings are localized", this
   should become stricter or the remaining hardcoded strings should be
   intentionally documented. If the intended gate is only "all `t(...)` keys have
   Japanese coverage", the current implementation is acceptable.

2. `cargo install cargo-audit --locked` is acceptable for immediate CI coverage
   but is not a pinned toolchain strategy.

   Both workflows install whatever current `cargo-audit` release Cargo resolves
   at run time (`.github/workflows/ci.yml:42`,
   `.github/workflows/release.yml:43`). That is simple and repo-local, but
   release gates can shift when a new `cargo-audit` version is published. For a
   stricter release process, pin the install with `--version`, use a maintained
   action with a pinned revision, or cache/tool-install it in a controlled
   preflight image.

3. CI has some duplicate expensive gates.

   `.github/workflows/ci.yml:63` and `.github/workflows/ci.yml:66` run the
   headless release-gate test/clippy pair, then `.github/workflows/ci.yml:69`
   and `.github/workflows/ci.yml:72` run workspace test/clippy. That is
   defensible for release-readiness, but expect longer CI time. If runtime
   becomes a problem, keep the explicit release-gate names and make the broader
   workspace gates conditional or separate.

## Requirement checks

- CI enforces documented release-relevant gates: mostly pass. CI now includes
  fmt, CSS, version-sync, i18n, cargo audit, audit-deps, headless tests, and
  headless clippy. It also retains workspace test/clippy and UI build smoke.
- Release workflow fails before artifact creation when most gates fail: partial.
  `source` depends on `preflight`, and platform jobs depend on `source`, so gate
  ordering is correct. The missing tag-vs-workspace version check leaves a
  release-version mismatch path.
- Source archive layout verification reusable from local tooling and release
  workflow: pass. `cargo xtask archive-layout [archive]` is used by the release
  workflow and passed against a generated tracked-file archive locally.
- No new Rust dependencies: pass. The new functionality is implemented in
  `xtask` using the standard library and external commands already used by the
  repository.
- `xtask version-sync` scope: partial. It covers the repository metadata listed
  in the request, but release workflow usage also needs tag-version validation.
- `xtask i18n` strictness: acceptable for `t(...)` key coverage; not sufficient
  for full UI-string localization enforcement.
- Release workflow dependency graph: pass except for the missing version gate.
  `source` has `needs: preflight`, and linux/macos/windows jobs need `source`.

## Observed evidence

Commands observed during this review pass:

- `cargo xtask version-sync` - passed.
- `cargo xtask i18n` - passed; reported 202 UI keys covered.
- `cargo xtask css --check` - passed.
- `cargo fmt --check` - passed.
- Created a tracked-file source archive under `target/forskscope-v0.164.0.tar.gz`.
- `cargo xtask archive-layout target/forskscope-v0.164.0.tar.gz` - passed.
- `cargo xtask audit-deps` - passed, including absence checks for
  `sheets-diff`, `calamine`, `reqwest`, `hyper`, and `ureq`, inactive
  `dioxus-devtools`, reviewed `quick-xml`, and reviewed network-capable paths.
- `cargo audit` - passed under checked-in policy and reported allowed warnings.
- `cargo test -p forskscope-core -p forskscope-ui-logic` - passed.
- `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` -
  passed.
- `git diff --check` - passed.

Generated artifacts from review verification were removed:

- `target/forskscope-v0.164.0.tar.gz`
- `xtask/Cargo.lock`

## Missing evidence

- GitHub Actions execution was not observed.
- Workflow YAML parser validation was not observed.
- `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and
  `cargo build -p forskscope-ui` were not independently rerun in this review
  pass.

## Recommended next action

Add a release preflight check that the tag version equals the workspace version,
then re-run the CI/release gate review. Keep the current preflight dependency
graph and reusable `archive-layout` command; those parts are moving in the right
direction.
