# RFC 033: Three-Way Merge Model

**Status.** Proposed

## Status
Proposed. (Originally proposed in RFC package v0.4.)

## Summary

Introduce a base-aware three-way merge model while preserving the existing two-way comparison and merge workflow. Three-way merge must be explicit, inspectable, and conflict-safe.

## Motivation

Two-way diff/merge is sufficient for many manual comparisons, but users who work with source files, configuration files, and release branches often need base-aware merge. Without a base document, the app cannot distinguish independent edits from conflicting edits.

## Goals

- Support optional base document selection.
- Generate merge results from base, left, and right inputs.
- Produce structured conflict records.
- Preserve two-way behavior as the default simple path.
- Allow future VCS-assisted base discovery without requiring VCS integration.

## Non-Goals

- Become a Git/JJ merge driver in this RFC.
- Hide conflicts by making risky automatic decisions.
- Replace manual two-way comparison.

## External Design

### Entry Points

```text
File menu:
  Open Two-Way Compare...
  Open Three-Way Merge...

Explorer context menu:
  Compare Selected Pair
  Merge with Base...
```

### Three-Way Open Dialog

```text
+-----------------------------------------------------+
| Open Three-Way Merge                                |
+-----------------------------------------------------+
| Base file:  [ /path/base.txt        ] [Browse...]   |
| Left file:  [ /path/left.txt        ] [Browse...]   |
| Right file: [ /path/right.txt       ] [Browse...]   |
| Result:     [ new result buffer     ] [Save As...]  |
|                                                     |
| Options:                                            |
| [x] Preserve original line endings when possible    |
| [x] Stop at unresolved conflicts before save        |
| [ ] Prefer non-conflicting right-side changes       |
|                                                     |
|                         [Cancel] [Open Merge]       |
+-----------------------------------------------------+
```

### Workspace Layout

```text
+--------------------------------------------------------------------------------+
| Three-Way Merge: base.txt / left.txt / right.txt                               |
+-------------------+-------------------+-------------------+--------------------+
| Base              | Left              | Right             | Result             |
| read-only         | read-only/edit    | read-only/edit    | editable           |
| common ancestor   | variant A         | variant B         | merged output      |
+-------------------+-------------------+-------------------+--------------------+
| Conflict Navigator: 12 conflicts | 8 auto-merged | 4 unresolved                |
+--------------------------------------------------------------------------------+
```

For smaller screens, the base pane may be collapsible, but the conflict state must remain visible.

## Internal Design

### Data Model

```rust
pub struct ThreeWayMergeSession {
    pub id: MergeSessionId,
    pub base: DocumentId,
    pub left: DocumentId,
    pub right: DocumentId,
    pub result: DocumentId,
    pub conflicts: Vec<MergeConflict>,
    pub auto_merges: Vec<AutoMergeRecord>,
    pub policy: MergePolicy,
}

pub struct MergeConflict {
    pub id: ConflictId,
    pub base_range: TextRange,
    pub left_range: TextRange,
    pub right_range: TextRange,
    pub result_range: Option<TextRange>,
    pub status: ConflictStatus,
}

pub enum ConflictStatus {
    Unresolved,
    ResolvedLeft,
    ResolvedRight,
    ResolvedBoth,
    ResolvedManual,
    Ignored,
}
```

### Algorithm Boundary

The RFC does not mandate one final merge algorithm. It requires a clean boundary:

```rust
pub trait ThreeWayMergeEngine {
    fn merge(&self, input: ThreeWayMergeInput) -> Result<ThreeWayMergeOutput, MergeError>;
}
```

The initial implementation may use a conservative line-oriented merge. Later implementations may improve matching, move detection, or intra-line conflict analysis.

## Conflict Markers

ForskScope must not silently write conflict markers into user files unless the user explicitly chooses a marker-based export. The primary conflict representation is structured metadata in the session model.

## Save Policy

Default behavior:

```text
if unresolved_conflicts > 0:
  block direct save
  allow Save As Conflict File only with explicit confirmation
else:
  allow normal save through atomic file operation policy
```

## Acceptance Criteria

- Two-way compare remains available without base input.
- Three-way merge creates a result buffer with structured conflict metadata.
- Unresolved conflicts are visible in the UI and block unsafe final save by default.
- Conflict resolution changes are undoable.
- The merge session can be serialized and reopened.

## Dependencies

- RFC 021 — Document and Result Buffer Model
- RFC 032 — Text Editing Operation Model
- RFC 034 — Conflict Resolution Workspace
- RFC 036 — External Modification Handling
