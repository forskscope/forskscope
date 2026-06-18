# RFC 062: Safe Batch Copy UX and Restore Manifest Integration

**Status.** Proposed
**Tracks.** Directory-report copy safety; binding the UI to the core batch
manifest/restore model; explicit copy direction; recovery-facing result UX.
**Touches.** `crates/forskscope-ui/src/ui/modals.rs` (batch copy modal),
`crates/forskscope-ui/src/ui/deep_compare.rs` (per-row copy actions),
`crates/forskscope-core/src/dir/batch.rs` (existing batch model — consume, do
not duplicate), plus result/restore UI.

## Summary

The core already has a batch copy model with operation IDs, per-entry outcomes,
and restore manifests (`forskscope-core::dir::batch`). The directory-report UI
does **not** use it: the batch-copy modal loops over `copy_file` directly, and
per-row copy actions are asymmetric (left→right only). This means a file-writing
operation — among the most dangerous a diff/merge tool performs — runs without
the manifest-backed audit and recovery the core was built to provide, and with
directional ambiguity that invites accidental overwrites.

This RFC binds the UI to the core batch model and makes copy direction explicit
and symmetric, with confirmation dialogs that name the exact source and
destination and the backup behaviour.

## Motivation

The UI/UX architect source review flagged two related findings:

- **P0-4 (High):** the batch-copy UI bypasses the core batch manifest/restore
  model, so users cannot confidently undo or audit a batch copy.
- **P0-5 (High):** per-row directory copy is directionally ambiguous —
  `Copy →` / `← Copy` require the user to infer which side is source and which
  is destination, for an operation that overwrites files.

Directory copy overwrites real files on disk. Direction ambiguity and missing
recovery are exactly the failure modes the product's "no hidden destructive
actions" and "forgiving design" principles exist to prevent.

## Design

### B1 — Route all copies through the core batch model

Replace the direct `copy_file` loop in the batch-copy modal with the core
`batch_copy` operation. The UI passes the set of (source, destination) entries;
the core returns an operation result containing per-entry outcomes (succeeded /
failed / skipped) and a restore manifest path.

Per-row single-file copies should also produce a (single-entry) manifest so the
recovery story is uniform — a one-file copy is just a batch of one.

### B2 — Result summary instead of silent close

After a batch copy, do not immediately dismiss the modal. Show a result
summary:

- counts of succeeded / failed / skipped
- the restore manifest path
- a "How to restore" affordance (display the restore command or offer a restore
  action)

If any entry failed, the modal stays open so the user sees what happened.

### B3 — Explicit, symmetric copy direction

Per-row and batch actions use written-direction labels, never arrows alone:

- `Copy to right` / `Copy to left`
- when the action overwrites an existing file: `Replace right with left` /
  `Replace left with right`

When a file exists only on one side, only the valid direction is offered.

### B4 — Confirmation names source, destination, backup

Before any copy that writes or overwrites, a confirmation dialog states the
full source path, the full destination path, and that a backup will be created
first. Per RFC-060/the destructive-modal policy in RFC-063, **Cancel is the
default focused button**.

```
Copy this file?

From:
  /project-old/src/main.rs

To:
  /project-new/src/main.rs

A backup will be created first.

[Cancel]  [Copy file]
```

## Relationship to other findings

- The directional-label wording overlaps with RFC-063 (clarity hardening);
  the *labels* are defined here because they are copy-safety-critical; RFC-063
  references them rather than redefining.
- The Cancel-default destructive-modal rule is defined in RFC-063 and applied
  here.

## Non-goals

- This RFC does not add directory-tree merge (whole-subtree reconciliation);
  that is RFC-022's territory. This is per-file and explicit-batch copy only.
- It does not change the core batch model; it consumes the existing one.

## Acceptance criteria

- No directory copy runs through a direct `copy_file` loop in the UI; all go
  through the core batch model and produce a manifest.
- Every copy action states its direction in words.
- Both directions are available when both sides exist; only the valid direction
  when one side is missing.
- Copy confirmation names full source, full destination, and backup behaviour,
  with Cancel focused by default.
- A batch result summary (succeeded/failed/skipped + manifest path + restore
  affordance) is shown; the modal does not auto-close on failure.

## Cross-references

- `forskscope-core::dir::batch` — the manifest/restore model to consume.
- RFC-022 — directory merge and batch operations (core semantics).
- RFC-023 — atomic file operations, backup/restore (backup guarantee).
- RFC-063 — destructive-modal focus policy and clarity wording.

## Open questions

- Should "How to restore" execute a restore in-app, or only display the
  manifest path + command for the user to run deliberately? In-app restore is
  friendlier; manifest-only is safer/auditable. Lean toward displaying the
  manifest path plus an explicit, separately-confirmed "Restore this batch"
  action.
