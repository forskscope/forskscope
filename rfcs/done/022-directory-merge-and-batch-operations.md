# RFC 022 — Directory Merge and Batch Operations

**Status.** Implemented (v0.52.0) — core complete; batch preview dialog UI and deletion actions (elevated confirmation) deferred to UI layer

## Status
Partially implemented in v0.52.0. `forskscope-core::dir::merge_plan` ships:

- **`DirectoryMergeAction`** (`CopyLeftToRight | CopyRightToLeft | Skip`)
- **`CopyDirection`**, **`EntrySelection`** (AllNonEqual / ChangedOnly / SourceOnlyEntries)
- **`OperationPreflight`** — per-file pre-execution checks: `target_exists`, `target_writable`, `backup_required`, `estimated_bytes`
- **`RiskSummary`** — `total_files`, `new_files`, `overwrites`, `estimated_bytes`, `permission_blocks`; used for the batch preview dialog
- **`OperationPlan`** — `plan_operations(entries, left, right, direction, selection)` builds a previewable plan from `Vec<RecEntry>`; `is_safe_to_execute()` predicate
- **`execute_plan(plan, backup, failure_policy)`** — executes via `batch_copy`, returns `PlanExecutionReport` with `succeeded`, `failed`, `skipped`, per-file `FileOutcome`
- **15 tests** covering all RFC-022 acceptance criteria.

Remaining open: the batch preview dialog UI, deletion actions (require elevated confirmation, deferred), TypeMismatch and PermissionDifferent status variants from RFC-022 §"Directory comparison states" (await richer scan metadata), and the operation plan export to JSON/Markdown (RFC-027 integration).

## Summary

Define actionable directory comparison and merge behavior. Directory comparison should not only show differences; it should allow safe, previewable, recoverable batch operations such as copying added/changed files from one side to the other.

## Goals

- Turn directory comparison into a review-and-action workflow.
- Support per-file and batch actions.
- Provide operation preview before writes.
- Reuse atomic file operation policy.
- Preserve user trust through clear summaries and backups.

## Non-goals

- Recursive VCS integration.
- Three-way directory merge.
- Conflict auto-resolution using semantic file content.
- Remote directory synchronization.

## User workflow

```text
1. User opens left directory and right directory.
2. App scans both sides in background.
3. App displays categorized results.
4. User filters to added/modified/deleted/conflict items.
5. User opens individual file diff when needed.
6. User selects one or more operations.
7. App displays operation preview.
8. User confirms.
9. App executes operations with backup/restore metadata.
10. App displays completion summary.
```

## Directory comparison states

```rust
pub enum DirectoryEntryStatus {
    Equal,
    AddedLeft,
    AddedRight,
    Modified,
    DeletedLeft,
    DeletedRight,
    TypeMismatch,
    BinaryDifferent,
    PermissionDifferent,
    Error,
}
```

## Directory merge actions

```rust
pub enum DirectoryMergeAction {
    CopyLeftToRight { relative_path: RelativePath },
    CopyRightToLeft { relative_path: RelativePath },
    DeleteLeft { relative_path: RelativePath },
    DeleteRight { relative_path: RelativePath },
    CreateDirectoryLeft { relative_path: RelativePath },
    CreateDirectoryRight { relative_path: RelativePath },
    Skip { relative_path: RelativePath },
}
```

Deletion actions must be disabled by default or require a stronger confirmation than copy actions.

## Main directory comparison wireframe

```text
+--------------------------------------------------------------------------------+
| Directory Compare: /left/project  ↔  /right/project                            |
| Filter: [All] [Modified] [Only Left] [Only Right] [Conflicts] [Errors]          |
+----------------------+-----------------------------+---------------------------+
| Summary              | File list                   | Action preview            |
| Equal: 2031          | [M] src/main.rs             | Selected: 3 files         |
| Modified: 14         | [L] docs/new.md             | - copy L→R docs/new.md    |
| Only Left: 8         | [R] tests/case.rs           | - copy R→L tests/case.rs  |
| Only Right: 2        | [!] assets/logo.png         | - open diff src/main.rs   |
| Errors: 1            |                             |                           |
+----------------------+-----------------------------+---------------------------+
| [Open File Diff] [Copy Selected →] [Copy Selected ←] [Batch Review]             |
+--------------------------------------------------------------------------------+
```

## Batch review dialog

```text
+--------------------------------------------------------------+
| Batch Operation Preview                                      |
+--------------------------------------------------------------+
| Operation set: Copy selected left-to-right                   |
| Files affected: 8                                            |
| Bytes affected: 123.4 KiB                                    |
| Backups: enabled                                             |
|                                                              |
| [ ] docs/new.md       create on right                        |
| [ ] src/lib.rs        overwrite right, backup first          |
| [ ] assets/icon.png   binary overwrite, backup first         |
|                                                              |
| Risk: 2 overwrites, 1 binary file                             |
+--------------------------------------------------------------+
| [Cancel] [Export Plan] [Execute]                             |
+--------------------------------------------------------------+
```

## Operation plan model

```rust
pub struct OperationPlan {
    pub id: OperationPlanId,
    pub created_at: SystemTime,
    pub base_left: PathBuf,
    pub base_right: PathBuf,
    pub actions: Vec<PlannedFileOperation>,
    pub risk_summary: RiskSummary,
}

pub struct PlannedFileOperation {
    pub action: DirectoryMergeAction,
    pub source: Option<PathBuf>,
    pub target: Option<PathBuf>,
    pub preflight: OperationPreflight,
}

pub struct OperationPreflight {
    pub target_exists: bool,
    pub target_writable: bool,
    pub backup_required: bool,
    pub external_change_detected: bool,
    pub estimated_bytes: u64,
}
```

## Safety rules

- Batch operations must always be previewed.
- Overwrites must require backup unless explicitly disabled in settings.
- Deletions must require typed or elevated confirmation.
- Symlink behavior must be explicit and conservative.
- Read-only or permission-denied targets must fail before partial execution where possible.
- Partial success must produce a recovery report.

## Acceptance criteria

- Directory scan categorizes equal, modified, added, deleted, binary, and error items.
- User can open a file-level diff from directory results.
- User can create a batch copy plan.
- Batch plan preview shows overwrites, binary files, and backup requirements.
- Batch execution uses atomic file operation policy.
- Completion summary lists success, skipped, and failed operations.
- Errors do not leave the UI in an ambiguous state.

## Test strategy

- Test corpus directories for each status.
- Unit tests for action planning.
- Integration tests using temporary directories.
- Permission/read-only tests on supported platforms.
- Recovery tests for partial failure.

## Dependencies

- RFC 008 Directory comparison background jobs.
- RFC 017 Error taxonomy.
- RFC 023 Atomic file operations.
- RFC 027 Report/export.
