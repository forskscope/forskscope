# RFC 039: Patch Export, Apply, and Review Workflow

**Status.** Implemented (v0.39.0) — export only; guarded apply deferred

## Status
Implemented (v0.39.0). The *export* half of this RFC shipped in v0.39.0:
`patch_from_file_diff`, `patch_from_directories`, and `to_unified` are
available in `forskscope-core::patch`. The guarded *apply* workflow
(preflight, atomic write, backup-protected application) is intentionally
deferred — it depends on RFC-023 (Atomic File Operations) and RFC-027
(Report Export) which are still proposed. When those ship, a follow-up
release will close the apply gap without renumbering this RFC.

## Summary

Define patch export, patch preview, and guarded patch application workflows for ForskScope.

## Motivation

Diff/merge tools are often used to review and exchange changes. Patch support gives users a portable artifact without requiring VCS integration. However, patch application can modify many files and must be guarded by preview, backup, and clear errors.

## Goals

- Export reviewable patches from file and directory comparisons.
- Preview patch contents before applying.
- Apply patches only with explicit confirmation.
- Integrate with backup/restore policy.
- Keep patch support independent from VCS.

## Non-Goals

- Implement every patch format variant.
- Become a full package manager or deployment tool.
- Apply patches silently.

## External Design

### Export Patch

```text
Toolbar:
  [Export Report] [Export Patch]

Export Patch Dialog:
  Format: [Unified Diff]
  Scope:  [Selected files only | All modified files]
  Paths:  [Relative to left root | Absolute paths disabled]
  Options:
    [x] Include file creation/deletion markers
    [ ] Include binary file notices
    [x] Write patch summary header
```

### Patch Review

```text
+-------------------------------------------------------------------+
| Patch Review                                                      |
+-------------------------------------------------------------------+
| Summary: 12 files, 180 additions, 44 deletions                    |
|                                                                   |
| files:                                                            |
|   M src/main.rs        32 additions, 8 deletions                  |
|   A src/new.rs         44 additions                               |
|   D src/old.rs         12 deletions                               |
|                                                                   |
| [Open File Diff] [Save Patch] [Cancel]                            |
+-------------------------------------------------------------------+
```

### Apply Patch

```text
+-------------------------------------------------------------------+
| Apply Patch                                                       |
+-------------------------------------------------------------------+
| Target directory: /project                                        |
| Patch file:       changes.patch                                   |
|                                                                   |
| Preflight:                                                         |
|   ✓ 10 files match expected context                               |
|   ! 2 files have fuzzy or failed context                          |
|                                                                   |
| [Show Details] [Cancel] [Apply with Backup]                       |
+-------------------------------------------------------------------+
```

## Internal Design

### Patch Model

```rust
pub struct PatchDocument {
    pub format: PatchFormat,
    pub files: Vec<PatchFileChange>,
    pub summary: PatchSummary,
}

pub enum PatchFileChange {
    Modify { path: RelativePath, hunks: Vec<PatchHunk> },
    Add { path: RelativePath, content: Vec<u8> },
    Delete { path: RelativePath, expected_digest: Option<ContentDigest> },
    BinaryNotice { path: RelativePath },
}
```

### Apply Preflight

```rust
pub struct PatchApplyPlan {
    pub target_root: PathBuf,
    pub operations: Vec<PatchApplyOperation>,
    pub warnings: Vec<PatchWarning>,
    pub blockers: Vec<PatchBlocker>,
}
```

Patch application must use the same atomic write and backup policy as normal save.

## Acceptance Criteria

- Users can export a unified diff for selected changes.
- Users can preview patch application before writing files.
- Failed context blocks or clearly marks unsafe apply.
- Patch apply creates backups when overwriting files.
- Patch export does not require Git/JJ.

## Dependencies

- RFC 002 — similar v3 Diff Engine
- RFC 023 — Atomic File Operations
- RFC 027 — Report Export and Session Evidence
- RFC 037 — Scalable Directory Compare
