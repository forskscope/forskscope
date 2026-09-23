# Security audit remediation re-review

Reviewed:

- `dev-record/reviews/002-security-audit-remediation-review.md`
- `dev-record/review-requests/001-security-audit-remediation-implementation.md`
- `dev-record/review-requests/002-xlsx-msrv-requirements-change.md`
- Current working tree changes relevant to XLSX disablement, audit policy, CI, docs, and tests.

## Verdict

Accept with notes.

The two blocking findings from the previous review have been addressed:

- Directory patch export now treats `FileKind::ExcelXlsx` as non-textual, so XLSX entries produce binary notices instead of empty/silent text patch output.
- The `quick-xml` audit exception is now paired with an enforceable dependency-path check that fails if `sheets-diff` or `calamine` return, or if `quick-xml` is reachable through anything other than the reviewed `wayland-scanner` path.

## Blocking findings

None.

## Non-blocking findings

1. Stale XLSX wording remains in two code-facing UI/core descriptions.

   - `crates/forskscope-core/src/file_kind.rs:4` still says `.xlsx` goes through the spreadsheet adapter.
   - `crates/forskscope-core/src/file_kind.rs:19` still says Excel workbooks are compared through the spreadsheet adapter.
   - `crates/forskscope-ui/src/ui/view/diff.rs:350` still maps XLSX to `Spreadsheet - read-only comparison.`

   Runtime behavior is fail-closed through `crates/forskscope-ui/src/state/compare.rs:208` and `crates/forskscope-core/src/xlsx.rs:111`, so this is not a security blocker. It should still be cleaned up before release documentation and UI polish are considered done.

2. CI now enforces the reviewed dependency path with `cargo xtask audit-deps`, but does not appear to run `cargo audit` itself.

   `.github/workflows/ci.yml:51` adds `cargo xtask audit-deps`. That closes the prior path-enforcement gap. Release docs require `cargo audit` separately in `docs/src/maintainers/release.md:8`; adding an audit job to CI or a release workflow would reduce manual gate drift.

3. MSRV verification is still partly manual.

   The implementation raises MSRV to Rust 1.91, and a UI check on Rust 1.91 was observed in the earlier review pass. Full MSRV tests were not cleanly observed in this sandbox because tempdir/VCS tests interact badly with the sandboxed filesystem layout. Run the MSRV test command in a normal checkout before tagging.

## Requirement checks

- XLSX remains recognized but disabled: pass. Classification still returns `FileKind::ExcelXlsx`; comparison returns a security-disabled error before normal diffing.
- Runtime workbook XML parsing path removed: pass. `sheets-diff` and `calamine` are absent, and `diff_xlsx` returns `Unsupported` without parsing workbook XML.
- Fail-closed behavior is consistent: pass. UI comparison returns a localized disabled error, core XLSX diff returns `Unsupported`, and directory patch output now emits binary notices for XLSX changes.
- `quick-xml` residual path reviewed and enforced: pass. `.cargo/audit.toml` documents the exception and `cargo xtask audit-deps` enforces the path.
- Docs updated for the temporary disablement: pass with notes. User and maintainer docs now describe XLSX comparison as temporarily disabled; the stale code-facing strings above remain.

## Observed evidence

Commands observed during this review pass:

- `cargo xtask audit-deps` - passed.
  - Reported `sheets-diff is absent.`
  - Reported `calamine is absent.`
  - Reported `quick-xml path is limited to wayland-scanner.`
  - Reported `security dependency path check passed.`
- `cargo fmt --check` - passed.
- `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` - passed.
- `git diff --check` - passed.
- `cargo test -p forskscope-core -p forskscope-ui-logic` - passed.
- `cargo audit` - passed under `.cargo/audit.toml`, with allowed advisory warnings still printed.
- `cargo xtask css --check` - passed.

Key code evidence:

- `crates/forskscope-core/src/patch/directory.rs:160` now restricts textual directory patch output to `FileKind::Text`.
- `crates/forskscope-core/src/tests/patch_tests.rs:223` covers changed, added, and deleted `.xlsx` files as binary notices.
- `xtask/src/main.rs:109` adds the `audit-deps` release check.
- `xtask/src/main.rs:116` verifies absence of `sheets-diff` and `calamine`.
- `xtask/src/main.rs:134` verifies the remaining `quick-xml` path.
- `.github/workflows/ci.yml:51` runs the security dependency path check in CI.

## Missing evidence

- Full `cargo +1.91 test -p forskscope-core -p forskscope-ui-logic` in a normal, non-sandboxed checkout.
- End-to-end release workflow evidence that includes both `cargo audit` and `cargo xtask audit-deps`.

## Recommended next action

Merge/accept this remediation after the owner is comfortable with the residual audit warnings policy. Before v1.0 release, clean up the stale XLSX wording, run full MSRV tests outside the sandbox, and align the release gate so both `cargo audit` and `cargo xtask audit-deps` are enforced consistently.
