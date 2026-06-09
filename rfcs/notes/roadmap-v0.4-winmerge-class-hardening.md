# Roadmap v0.4: WinMerge-Class Hardening

## Purpose

The v0.4 roadmap converts the migration from a functional Dioxus rewrite into a credible diff/merge workstation product. The central concern is no longer simply rendering diffs; it is ensuring that user edits, merge decisions, file writes, external changes, and directory-scale operations remain understandable, recoverable, and testable.

## Roadmap Layers

```text
Layer 1: Editor Truth and Operations
  RFC 032, RFC 035, RFC 040

Layer 2: Merge Semantics
  RFC 033, RFC 034

Layer 3: File-System Safety
  RFC 036

Layer 4: Scale and Batch Work
  RFC 037, RFC 039

Layer 5: Context and Product Boundary
  RFC 038, RFC 041
```

## Milestone M4.1 — Editor Operation Safety

### Objective
Define a strict contract for editable text operations so that Dioxus/CodeMirror is treated as an editor surface, not the authoritative source of merge truth.

### RFCs
- RFC 032 — Text Editing Operation Model and Editor Truth Boundary
- RFC 035 — Scroll Sync, Line Mapping, and Diff Decoration Engine
- RFC 040 — Editor Adapter Verification Harness and Golden Corpus

### Exit Criteria
- Every edit from the editor is represented as a typed operation.
- The core can replay, reject, validate, and serialize operations.
- Diff decorations are derived from core state, never manually patched as primary truth.
- A golden corpus covers line endings, Unicode, long lines, large files, binary detection, and IME-like edit sequences.

## Milestone M4.2 — Merge Semantics and Conflict UX

### Objective
Support a principled path from two-way merge to three-way merge while keeping unresolved conflicts visible and safe.

### RFCs
- RFC 033 — Three-Way Merge Model
- RFC 034 — Conflict Resolution Workspace

### Exit Criteria
- Two-way merge remains simple and stable.
- Three-way merge uses an explicit base document model.
- Conflicts have durable IDs and clear statuses.
- Users can resolve conflicts by choosing left, right, both, manual edit, or custom result.

## Milestone M4.3 — File System Reconciliation

### Objective
Prevent silent data loss when files change outside ForskScope.

### RFC
- RFC 036 — Live Reload, File Watcher, and External Modification Handling

### Exit Criteria
- External modifications are detected.
- Save is blocked or mediated when the on-disk file differs from the loaded snapshot.
- The user can reload, keep session, compare with external version, or save as a new file.

## Milestone M4.4 — Scalable Comparison and Change Exchange

### Objective
Support large directory comparisons and reviewable patch workflows without making the app feel heavy.

### RFCs
- RFC 037 — Scalable Directory Compare Index and Incremental Refresh
- RFC 039 — Patch Export, Apply, and Review Workflow

### Exit Criteria
- Directory comparison supports incremental refresh and cancellation.
- Batch operations are previewed before execution.
- Patch export is deterministic and reviewable.
- Patch apply is guarded by preview and backup policy.

## Milestone M4.5 — Context Integration and v1 Governance

### Objective
Add useful project context without turning ForskScope into a VCS client or IDE.

### RFCs
- RFC 038 — VCS Context Integration Boundary
- RFC 041 — v1.0 Product Stabilization and RFC Governance

### Exit Criteria
- Git/JJ integration is optional and read-mostly at first.
- ForskScope does not own repository history.
- v1.0 freezes session schema, command IDs, safety prompts, and core operation contracts.

## v0.4 Strategic Decision

The v0.4 roadmap confirms Dioxus as the implementation target because editor capability is central to the product. Iced remains strategically valuable, but an Iced migration should be reconsidered only after:

1. `forskscope-core` owns all document and merge truth;
2. the editor operation model is GUI-independent;
3. the Dioxus editor adapter has validated the required behavior;
4. the team can estimate the cost of reimplementing or adopting an Iced editor widget with real evidence.
