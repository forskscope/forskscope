# Review Request: Security Audit Remediation Implementation

**Date:** 2026-07-09
**Reviewer stance:** implementation/security review
**Repository state:** inspect the current working tree directly

The reviewer should inspect the project directly. This request intentionally does
not include the whole codebase or full diff.

## Summary

Review the implementation that mitigates the release-blocking `cargo audit`
failures affecting user-supplied XLSX input.

The implementation removes the runtime XLSX parser dependency path:

```text
sheets-diff -> calamine -> quick-xml
```

Because a compatible fixed parser path was not available through the current
dependency stack, XLSX comparison is temporarily disabled and fails closed.

## Scope Followed

- Removed `sheets-diff` and `calamine` from the dependency graph.
- Kept `.xlsx` as a recognized file kind.
- Changed XLSX comparison to return an unsupported/fail-closed result.
- Added a user-facing UI error for disabled spreadsheet comparison.
- Added Japanese i18n for the new UI error.
- Added `.cargo/audit.toml` documenting the remaining accepted `quick-xml`
  advisories.
- Updated tests to assert fail-closed XLSX behavior.
- Updated docs and threat model to match the new security posture.
- Raised MSRV to Rust `1.91`.
- Updated `time` to `0.3.47` and `crossbeam-epoch` to `0.9.20`.
- Applied clippy cleanups required by the current toolchain.

## Primary Files To Inspect

- `Cargo.toml`
- `Cargo.lock`
- `.cargo/audit.toml`
- `crates/forskscope-core/Cargo.toml`
- `crates/forskscope-core/src/xlsx.rs`
- `crates/forskscope-core/src/document.rs`
- `crates/forskscope-core/src/tests/xlsx_tests.rs`
- `crates/forskscope-ui/src/state/compare.rs`
- `crates/forskscope-ui/src/i18n.rs`
- `docs/src/maintainers/threat-model.md`
- `docs/src/maintainers/testing.md`
- `docs/src/maintainers/release.md`
- `docs/src/intermediate/file-types.md`
- `docs/src/users/features.md`
- `docs/src/users/faq.md`
- `docs/src/users/known-limitations.md`

## Review Questions

1. Is workbook XML content now unreachable from the runtime comparison path?
2. Is the fail-closed XLSX behavior implemented consistently across core and UI?
3. Is the audit policy narrow enough for `RUSTSEC-2026-0194` and
   `RUSTSEC-2026-0195`?
4. Is the remaining `quick-xml` path accurately limited to `wayland-scanner`
   through the Dioxus desktop stack?
5. Are the docs and threat model clear that XLSX comparison is temporarily
   disabled, not partially supported?
6. Are there any stale tests, docs, or UI messages that still imply XLSX
   comparison works?

## Observed Evidence

The following commands were observed passing after the implementation:

```text
cargo fmt --check
cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings
cargo test -p forskscope-core -p forskscope-ui-logic
cargo xtask css --check
cargo audit
git diff --check
```

Dependency checks observed:

```text
cargo tree -i sheets-diff
```

reported no matching package.

```text
cargo tree -i calamine
```

reported no matching package.

```text
cargo tree -i quick-xml
```

showed `quick-xml v0.39.4` only under `wayland-scanner v0.31.10
(proc-macro)`, through the Dioxus desktop/rfd Wayland path.

```text
cargo tree -i crossbeam-epoch
```

showed `crossbeam-epoch v0.9.20`.

```text
cargo tree -i time
```

showed `time v0.3.47`.

## Known Limitations

- XLSX comparison is unavailable until a secure parser dependency path is
  selected.
- `cargo audit` exits successfully under `.cargo/audit.toml`, but still reports
  14 allowed warnings, mostly GTK3 maintenance advisories and other transitive
  warnings.
- GUI/platform smoke tests were not part of this implementation pass.

## Acceptance Criteria For This Review

- Confirm that `sheets-diff` and `calamine` are absent from `Cargo.lock`.
- Confirm that user-supplied XLSX workbook content is not parsed.
- Confirm that XLSX attempts fail closed with clear user-facing behavior.
- Confirm that the audit exception rationale is specific and defensible.
- Confirm that docs and release/testing guidance match the implemented behavior.
