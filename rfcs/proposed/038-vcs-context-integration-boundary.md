# RFC 038: VCS Context Integration Boundary

**Status.** Proposed

## Status
Proposed. (Originally proposed in RFC package v0.4.)

## Summary

Define the boundary for optional Git/JJ context integration. ForskScope may use VCS information to improve comparison workflows, but it must not become a VCS client, repository hosting tool, or history editor.

## Motivation

Many users compare files from working trees, branches, and merge conflicts. VCS context can help discover base versions, show modified files, and launch comparisons. However, broad VCS functionality can quickly expand scope and distract from the diff/merge product.

## Goals

- Provide optional read-mostly VCS context.
- Support common Git working-tree workflows.
- Leave room for JJ support without forcing it into v1.
- Avoid modifying repository history.
- Keep VCS integration behind a capability boundary.

## Non-Goals

- Implement commit, rebase, checkout, push, pull, or branch management.
- Become a Git GUI.
- Become a JJ GUI.
- Require VCS for normal file/directory compare.

## External Design

### VCS-Aware Entry Points

```text
File menu:
  Open Files...
  Open Directories...
  Open VCS Changes...        optional if repository detected

Explorer context:
  Compare with HEAD          Git initial scope
  Compare with Base          when merge base is available
  Open Conflict in Merge View
```

### VCS Changes Panel

```text
+---------------------------------------------------+
| Repository Context                                |
+---------------------------------------------------+
| Root: /project                                    |
| System: Git                                       |
| State: working tree has 12 modified files         |
+---------------------------------------------------+
| M src/main.rs       [Compare Working vs HEAD]     |
| A src/new.rs        [View Added]                  |
| D src/old.rs        [View Deleted]                |
| U config/app.toml   [Open Conflict Resolution]    |
+---------------------------------------------------+
```

## Internal Design

### VCS Provider Trait

```rust
pub trait VcsProvider {
    fn detect(root: &Path) -> Option<Self> where Self: Sized;
    fn status(&self) -> Result<Vec<VcsFileStatus>, VcsError>;
    fn read_revision_file(&self, rev: VcsRevision, path: &RelativePath) -> Result<Vec<u8>, VcsError>;
    fn merge_base(&self, left: VcsRevision, right: VcsRevision) -> Result<Option<VcsRevision>, VcsError>;
}
```

### Initial Provider Scope

Git provider:

```text
- detect repository root
- list working-tree status
- read HEAD version of a file
- read merge-base version where available
- detect conflicted paths if command output supports it
```

JJ provider:

```text
- reserved as future provider
- do not block core design on JJ-specific workflow
```

## Security and Safety

- VCS commands must be explicit and bounded.
- No arbitrary shell command execution from UI.
- Repository writes are out of scope.
- File contents read from VCS revisions become normal read-only documents in `forskscope-core`.

## Acceptance Criteria

- ForskScope works fully without VCS.
- Git working-tree status can launch file comparisons.
- VCS-provided base files can create three-way merge sessions.
- No commit/history-changing commands are exposed.
- VCS failures degrade to normal file comparison.

## Dependencies

- RFC 033 — Three-Way Merge Model
- RFC 036 — External Modification Handling
- RFC 041 — v1 Governance
