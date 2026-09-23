# Security Audit Remediation Review

**Reviewed requests:**

- `dev-record/review-requests/001-security-audit-remediation-implementation.md`
- `dev-record/review-requests/002-xlsx-msrv-requirements-change.md`

**Date:** 2026-07-09
**Verdict:** Needs changes

## Summary

The remediation is directionally correct: the runtime XLSX parser path through
`sheets-diff -> calamine -> quick-xml` is removed, `.xlsx` remains classified,
UI compare attempts fail closed with a clear message, and the current audit
passes under the checked-in policy.

Two issues remain before accepting this as release-ready:

1. Directory patch export still treats `ExcelXlsx` as textual.
2. The `quick-xml` audit exception is documented but not enforceably narrow.

## Blocking Findings

### 1. Directory patch export does not fail closed for XLSX

`crates/forskscope-core/src/patch/directory.rs:160` still treats
`FileKind::ExcelXlsx` as textual:

```rust
fn is_textual(kind: &FileKind) -> bool {
    matches!(kind, FileKind::Text | FileKind::ExcelXlsx)
}
```

After the remediation, `crates/forskscope-core/src/xlsx.rs:208` loads XLSX as
metadata only and sets `text: None`. That means directory patch export can
silently omit changed, added, or deleted `.xlsx` files rather than surfacing an
unsupported/binary notice.

Required action:

- Remove `ExcelXlsx` from `is_textual`.
- Add tests for changed, right-only, and left-only XLSX entries in
  `patch_from_directories`.

### 2. `quick-xml` audit exception is not enforceably narrow

`.cargo/audit.toml` globally ignores:

- `RUSTSEC-2026-0194`
- `RUSTSEC-2026-0195`

The rationale is specific and currently true: `cargo tree -i quick-xml` shows
`quick-xml v0.39.4` only under `wayland-scanner`, through the Dioxus/rfd
Wayland path. However, a global audit ignore would also allow a future runtime
`quick-xml 0.39` path to pass unnoticed.

Required action:

- Add a release/CI check that asserts `cargo tree -i quick-xml` contains only
  the accepted `wayland-scanner` path.
- Assert `sheets-diff` and `calamine` remain absent.

## Non-Blocking Findings

- Some source/UI wording still says spreadsheet comparison is read-only rather
  than disabled, for example `crates/forskscope-core/src/file_kind.rs` and
  the readonly notice in `crates/forskscope-ui/src/ui/view/diff.rs`. The main
  UI compare path returns the correct disabled error, so this is cleanup rather
  than a behavior blocker.
- Historical RFC/notes still describe XLSX support as implemented. That is
  acceptable as history, but the requirements-change document should become
  the current authority or be linked from RFC-058/README.

## Positive Evidence Observed

- `cargo tree -i sheets-diff` reported no matching package.
- `cargo tree -i calamine` reported no matching package.
- `cargo tree -i quick-xml` showed only the `wayland-scanner` path.
- `cargo tree -i time` showed `time v0.3.47`.
- `cargo tree -i crossbeam-epoch` showed `crossbeam-epoch v0.9.20`.
- `cargo fmt --check` passed.
- `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings`
  passed.
- `cargo test -p forskscope-core -p forskscope-ui-logic` passed on the current
  toolchain.
- `cargo xtask css --check` passed.
- `cargo audit` passed under `.cargo/audit.toml`, with 14 allowed warnings.
- `git diff --check` passed.
- `cargo +1.91 check -p forskscope-ui` passed.

## Missing Evidence

- Full `cargo +1.91 test -p forskscope-core -p forskscope-ui-logic` was not
  cleanly observed in this sandbox. The first run failed because `/tmp` is
  read-only for rustc temp files. Rerunning with `TMPDIR` inside the repository
  invalidated VCS tests that expect to run outside any Git repository.
- GUI/platform smoke tests were not part of this review.

## Requirements Decision

Disabling XLSX comparison is acceptable for the release-readiness objective
because it removes a vulnerable local user-input parser path and fails closed.
Docs should describe XLSX as "recognized but temporarily disabled"; removing
it entirely from file-type tables would obscure the intentional classification
behavior.

Rust `1.91` is an acceptable MSRV for this application, provided it is verified
in a normal environment before release.

Restoring XLSX comparison should be tracked as a separate post-remediation task
unless the owner wants XLSX support to remain a v1 release blocker.

