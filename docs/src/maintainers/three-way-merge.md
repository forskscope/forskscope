# Three-way merge model

`forskscope-core::merge::ThreeWayMergeSession` implements the base-aware
merge model defined in [RFC-033](../../../rfcs/done/033-three-way-merge-model.md).
It lives entirely in the core crate and is independent of any UI.

## Algorithm

The engine is a conservative line-oriented **diff3**:

1. Base, left, and right texts are split into terminator-preserving lines
   (the same splitting rules as the two-way diff engine, so the two views
   agree line-for-line).
2. Left and right are each aligned against base by LCS matching (Myers).
   Base lines kept by *both* variants are common synchronization anchors.
3. The three sequences are walked in lockstep between anchors. Each region
   between anchors is classified by comparing each variant against base.

## Region classification

| Left vs base | Right vs base | Result | Output |
|---|---|---|---|
| unchanged | unchanged | `Stable` | base |
| changed | unchanged | `LeftChanged` | left (auto) |
| unchanged | changed | `RightChanged` | right (auto) |
| changed | changed, identical | `BothSame` | either (auto) |
| changed | changed, divergent | `Conflict` | none until resolved |

Only the last row produces a conflict. No risky automatic decision is made
for divergent two-sided edits.

## Conflicts

Each conflict is a `MergeConflict` with:

- a durable `ConflictId` (`hash(ordinal, base, left, right)`), stable across
  resolution operations;
- the base, left, and right line content;
- a `ConflictStatus`: `Unresolved`, `ResolvedLeft`, `ResolvedRight`,
  `ResolvedBoth`, `ResolvedManual`, or `Ignored`.

Conflicts are **metadata**. The model never writes conflict markers into the
result. Marker-based export is a separate, opt-in feature (future work).

## Resolution and undo

`resolve_left`, `resolve_right`, `resolve_both`, `resolve_manual`, `ignore`,
and `reset` change a conflict's status. Every change pushes a reversible
transaction; `undo` / `redo` restore exact prior state, mirroring the
two-way `MergeSession` discipline.

## Result and save policy

`result_text()` reconstructs the merged output, splicing each segment's
content (fixed auto-merged content, or the current resolution of a conflict)
with original line terminators preserved. `can_save()` returns `false` while
any conflict is unresolved — the save layer consults it before writing, per
the RFC-033 save policy.

## Scope

Shipped in v0.40.0: the model above. Deferred: the conflict-resolution
workspace UI (RFC-034), editor-driven manual conflict edits (RFC-032), and
VCS-assisted base discovery.
