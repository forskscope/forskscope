# Testing Strategy

Tests validate **design specifications** (RFC-001 §10, RFC-002 §11), not merely
the written code.  Each test references the behaviour promised by an RFC.

## Core unit tests

Located in `crates/forskscope-core/src/tests/`.
Tests validate **design specifications** (RFC-001 §10, RFC-002 §11), not merely
the written code. Each test references the behaviour promised by an RFC.

| File | Covers | RFC |
|---|---|---|
| `encoding_tests` | UTF-8 detection, legacy decoding, round-trip encode, newline styles. | RFC-002, RFC-012 |
| `document_tests` | File kind classification, load, fingerprint, hex preview. | RFC-021 |
| `editability_tests` | `EditabilityClass` derivation; `NewlinePolicy::resolve`. | RFC-012 |
| `diff_tests` | Hunk kinds, ranges, stable IDs, newline markers, inline Unicode, large-file policy. | RFC-002 |
| `compare_profile_tests` | `CompareProfile` presets, `to_diff_options` mapping, type defaults. | RFC-028 |
| `merge_tests` | Apply, undo, redo, double-apply rejection, dirty state, mark_saved. | RFC-006 |
| `three_way_tests` | `ThreeWayMergeSession`: diff3 engine, conflict resolution, undo/redo, `can_save`. | RFC-033 |
| `transaction_log_tests` | `TransactionLog`: push/undo/redo, revision tracking, dirty state, redo-branch discard. | RFC-015 |
| `save_tests` | Atomic write, `.bak` backup, conflict detection. | RFC-007 |
| `external_state_tests` | `ExternalFileState`; `check_external_state` for clean/dirty/changed/deleted/replaced. | RFC-036 |
| `dir_tests` | Listing sort, file digest equality, recursive directory equality. | RFC-005 |
| `dir_cancel_tests` | `CancellationToken`; `recursive_diff_with_cancel`; symlink reporting. | RFC-037 |
| `batch_tests` | `batch_copy` outcomes, `BatchManifest`, `restore_from_manifest`. | RFC-023 |
| `merge_plan_tests` | `plan_operations`, `execute_plan`, `OperationPlan`, `RiskSummary`. | RFC-022 |
| `ignore_tests` | `IgnoreRules` extension and directory-pattern matching. | RFC-056 |
| `error_tests` | `ErrorSeverity` mapping, `RecoveryHint`, `is_user_recoverable`. | RFC-017 |
| `job_tests` | `JobProgress::fraction`, `JobHandle` cancel propagation, threshold constants. | RFC-013 |
| `persist_tests` | `VersionedEnvelope` round-trip, `MigrationPolicy` decisions, schema names. | RFC-031 |
| `report_tests` | `FileComparisonReport` and `DirComparisonReport` Markdown + JSON, path privacy. | RFC-027 |
| `vcs_tests` | `GitProvider` detect/status/read/merge_base; degrade outside repo. | RFC-038 |
| `patch_tests` | `patch_from_file_diff`, `to_unified`; GNU `patch` round-trip integration. | RFC-039 |
| `xlsx_tests` | `diff_xlsx` structured model; panic isolation on corrupt files. | RFC-058 |

Run all: `cargo test -p forskscope-core`

## UI build verification

`cargo build -p forskscope-ui` is the current UI gate.  Integration and
screenshot tests are planned in RFC-020 and RFC-040.

## Coverage convention note

Most test modules are named `<module>_tests` matching their source module.
`file_kind.rs` is an exception: its `EditabilityClass` type is covered by
`editability_tests` (named for the type, not the file). The convention check
in `rfcs/notes/` documents this intentional divergence.

## UI logic tests

`cargo test -p forskscope-ui-logic` runs tests for the two GTK-free
presentation-logic modules:

| File | Covers |
|---|---|
| `explore/align` | `compute_aligned_rows`: pairing, ordering, recursion. |
| `compare/search_index` | `MatchIndex`: build, next/prev, wrapping, `matching_hunk_ids`. |
