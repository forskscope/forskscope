# RFC 027 — Report Export and Session Evidence

**Status.** Implemented (v0.49.0) — Markdown and JSON report core; HTML and export dialog open

## Status
Implemented (v0.49.0). The `forskscope-core::report` module ships:

- **`FileComparisonReport::from_diff`** — builds from a `DiffDocument` +
  optional `TransactionLog` + optional file paths; renders to
  `to_markdown()` and `to_json()`.
- **`DirComparisonReport::from_entries`** — builds from `Vec<RecEntry>` +
  optional `BatchManifest` + optional roots; renders to `to_markdown()` and
  `to_json()`.
- **`ReportPathMode`** (NameOnly / Relative / Absolute) — privacy-safe path
  display. Default is `NameOnly` (basename only, no directory prefix).
- **`ReportOptions`** — include_hunks, include_history, include_options,
  include_warnings, include_sizes, path_mode.
- **JSON schema version 1** with `schema_version`, `app_version`, `kind`,
  `summary`, `options`, `warnings`, `hunks`, `history` / `files` fields.
- **20 tests** covering all RFC-027 acceptance criteria.

Remaining open: HTML report format (deferred per RFC-027 §"Future formats"),
the export dialog UI, CSV directory summary, and PDF (both deferred to post-v1
per RFC-027 §"Non-goals").

## Summary

Define exportable comparison reports and session evidence artifacts. Reports help users review, share, and archive comparison outcomes without requiring the app to be open.

## Goals

- Export file and directory comparison summaries.
- Include compare options and environment metadata.
- Include operation plans and results for batch merges.
- Support Markdown and JSON initially.
- Avoid leaking unnecessary local sensitive information by default.

## Non-goals

- Cloud sharing.
- PDF export in the first implementation.
- Cryptographic attestation.
- Full audit compliance framework.

## Export formats

Initial formats:

```text
Markdown report: human-readable
JSON report: machine-readable and testable
```

Future formats:

```text
HTML report
PDF report
unified diff patches
CSV directory summary
```

## File comparison report

Markdown report outline:

```markdown
# ForskScope File Comparison Report

## Summary

- Left: old.txt
- Right: new.txt
- Status: different
- Hunks: 12
- Added lines: 20
- Deleted lines: 8
- Modified lines: 4

## Options

- Whitespace: significant
- Newlines: preserve
- Encoding: UTF-8
- Inline diff: enabled

## Hunk Summary

| Hunk | Lines | Type | Merge State |
|---|---:|---|---|
| 1 | 10-14 | modified | unresolved |

## Operation History

- 2026-06-08 10:30:12 Copy hunk 3 left-to-right
- 2026-06-08 10:31:04 Save right buffer with backup
```

## Directory comparison report

```markdown
# ForskScope Directory Comparison Report

## Summary

- Left root: /left/project
- Right root: /right/project
- Equal: 2031
- Modified: 14
- Only left: 8
- Only right: 2
- Errors: 1

## Changed Files

| Path | Status | Size Left | Size Right | Action |
|---|---|---:|---:|---|
| src/main.rs | modified | 10 KiB | 11 KiB | reviewed |
| docs/new.md | only left | 2 KiB | - | copy left-to-right |

## Batch Operation Result

- Operation id: op-20260608-103012-a1b2c3
- Completed: 7
- Failed: 1
- Backups: enabled
```

## Privacy policy

Default reports should not include full absolute paths unless the user enables them.

Options:

```text
Report path mode:
  ( ) relative paths only
  ( ) display names only
  ( ) include absolute paths
```

Content inclusion:

```text
[ ] Include changed line snippets
[ ] Include full unified diff
[ ] Include operation history
[ ] Include diagnostics metadata
```

## Export dialog wireframe

```text
+----------------------------------------------------------+
| Export Report                                            |
+----------------------------------------------------------+
| Format: ( ) Markdown  ( ) JSON  ( ) HTML later           |
| Path mode: ( ) Relative  ( ) Names only  ( ) Absolute    |
|                                                          |
| Include:                                                 |
| [x] Summary                                              |
| [x] Compare options                                      |
| [x] Hunk/file list                                       |
| [ ] Changed line snippets                                |
| [ ] Full diff content                                    |
| [x] Operation history                                    |
|                                                          |
| Destination: /tmp/forskscope-report.md                   |
+----------------------------------------------------------+
| [Cancel] [Export]                                        |
+----------------------------------------------------------+
```

## JSON schema sketch

```json
{
  "schema_version": 1,
  "app": { "name": "ForskScope", "version": "3.0.0-preview" },
  "kind": "file_comparison",
  "created_at": "2026-06-08T10:30:12Z",
  "options": {},
  "summary": {},
  "hunks": [],
  "operations": []
}
```

## Acceptance criteria

- User can export Markdown report from file comparison.
- User can export JSON report from file comparison.
- User can export Markdown report from directory comparison.
- Reports include compare options.
- Reports can omit absolute paths by default.
- Export failure shows actionable error.

## Test strategy

- Snapshot tests for Markdown output.
- Schema tests for JSON output.
- Privacy tests for path redaction.
- Manual export tests on all Tier 1 platforms.

## Dependencies

- RFC 022 Directory merge.
- RFC 023 Atomic file operations.
- RFC 028 Compare options.
- RFC 031 Data compatibility.
