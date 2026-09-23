# Review Request: XLSX Security and MSRV Requirements Change

**Date:** 2026-07-09
**Reviewer stance:** requirements/product/security review
**Repository state:** inspect the current working tree directly

The reviewer should inspect the project directly. This request records the
requirements change and does not include the full source tree.

## Summary

Review the requirement change introduced by the security remediation:

- XLSX comparison is temporarily disabled for security.
- XLSX files remain recognized by file-type detection.
- Runtime workbook XML parsing must not occur until a safe parser path is
  available.
- Minimum supported Rust version is raised from `1.85` to `1.91`.

## Previous Requirement

Before this change, the application treated `.xlsx` as a supported comparison
format and parsed workbook content through the transitive dependency path:

```text
forskscope-core -> sheets-diff -> calamine -> quick-xml
```

This conflicted with the release-blocking audit finding because `quick-xml`
advisories affected local, user-supplied XLSX inputs.

## New Requirement

`.xlsx` behavior:

- `.xlsx` remains a recognized file kind.
- XLSX comparison is disabled until a non-vulnerable parser path is available.
- Attempted XLSX comparison fails closed.
- User-facing copy must say spreadsheet comparison is temporarily disabled for
  security.
- Docs must avoid claiming XLSX comparison currently works.

Dependency/security behavior:

- `sheets-diff` and `calamine` must not be present in the runtime dependency
  graph.
- Remaining `quick-xml` advisories may be accepted only for the documented
  `wayland-scanner` path.
- Audit exceptions must stay narrow and documented.

MSRV behavior:

- The app MSRV is now Rust `1.91`.
- This is intentional because the project is an application, not a public
  library crate, and dependency remediation requires a newer compiler than the
  previous `1.85` baseline.
- The project is not moving to latest stable by default, to reduce contributor
  friction.

## Review Questions

1. Is disabling XLSX comparison acceptable for the release-readiness objective?
2. Should the product docs describe XLSX as "recognized but disabled" or remove
   XLSX from supported-format tables entirely?
3. Is Rust `1.91` the right MSRV compromise for an application with this
   dependency graph?
4. Is the accepted `quick-xml` residual risk correctly scoped to
   `wayland-scanner` and unrelated to user-controlled workbook XML?
5. Should restoration of XLSX comparison be tracked as a separate release
   blocker or a post-release feature restoration task?

## Acceptance Criteria For This Review

- The reviewer accepts or revises the new XLSX requirement.
- The reviewer accepts or revises Rust `1.91` as the MSRV.
- The reviewer confirms whether the current docs wording is suitable for users.
- The reviewer confirms whether the residual audit policy is acceptable for the
  release gate.

## Follow-Up Candidate

If the requirements change is accepted, create a future task to restore XLSX
comparison after selecting one of:

- a `sheets-diff`/`calamine` upgrade path that reaches fixed `quick-xml`;
- an alternative XLSX parser with acceptable audit posture;
- an app-owned minimal XLSX extraction path with explicit security review.
