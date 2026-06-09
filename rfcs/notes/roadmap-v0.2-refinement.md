# Roadmap v0.2 Refinement — Implementation Sequencing After the First RFC Batch

## 1. Purpose

The first RFC package established the migration direction. This roadmap refinement converts that direction into a stricter execution sequence that reduces the two largest risks:

1. building a Dioxus UI before the core model is reliable; and
2. embedding a web editor before the editor-to-core contract is testable.

This document does not replace RFC-042 (the roadmap RFC, originally numbered RFC-000). It extends RFC-042 by defining the next batch of implementation gates.

## 2. Updated Roadmap Philosophy

The migration must not be a visual port of the existing application. It must be a product-state migration first.

The current app already demonstrates the important product idea: two file/directory sides, explorer-driven selection, text diff, inline character diff, Excel/binary handling, and preliminary merge actions. However, the next architecture must treat those features as domain behavior, not as UI state.

The desired evolution is:

```text
Current behavior preserved
  ↓
Core truth extracted
  ↓
Diff/merge sessions become model-backed
  ↓
Dioxus shell becomes a state presenter
  ↓
CodeMirror bridge becomes an editor adapter
  ↓
Save/session safety becomes explicit
  ↓
Large-file and directory workflows become cancellable jobs
```

## 3. Phase Model

### Phase A — Core and Parity Foundation

Relevant RFCs:

- RFC-001 Core extraction and canonical domain model
- RFC-002 `similar` v3 diff engine
- RFC-018 Migration compatibility and parity plan
- RFC-020 Developer architecture, CI, and test gates

Expected result:

```text
A command-line or test-only core can load two paths, classify them, decode text, produce normalized diff hunks, and compare outputs against the old app for representative fixtures.
```

No Dioxus screen is required to finish this phase.

### Phase B — Session, Encoding, and Save Safety

Relevant RFCs:

- RFC-007 Save, session, and file safety
- RFC-011 Workspace session persistence
- RFC-012 Text encoding, newline, and binary policy
- RFC-015 Undo/redo transaction log

Expected result:

```text
A comparison session can survive UI redraws, can track dirty state, can apply merge commands transactionally, and can calculate a safe save plan before writing files.
```

The important deliverable is not visual polish. The important deliverable is trustworthiness.

### Phase C — Dioxus Shell and Editor Adapter

Relevant RFCs:

- RFC-003 Dioxus application shell
- RFC-004 Editor adapter and CodeMirror bridge
- RFC-016 Editor bridge security and contract
- RFC-019 Command, shortcut, palette, and accessibility

Expected result:

```text
The Dioxus app can open a session, mount an editor pair through a stable adapter API, show decorations from the core model, send edit events back to the core, and execute commands through one command registry.
```

CodeMirror or any other editor implementation is replaceable only if it satisfies the adapter contract.

### Phase D — Workspaces and Navigation

Relevant RFCs:

- RFC-005 Explorer workspace
- RFC-006 Diff/Merge workspace
- RFC-014 Search, filter, and navigation
- RFC-017 Error taxonomy and diagnostics UX

Expected result:

```text
Users can open directories, compare pairs, navigate hunks, filter explorer rows, search text, and understand all errors through consistent dialogs and status surfaces.
```

### Phase E — Performance, Background Jobs, and Release Readiness

Relevant RFCs:

- RFC-008 Directory comparison background jobs
- RFC-010 Packaging, diagnostics, QA
- RFC-013 Large-file, performance, and virtualization
- RFC-020 Developer architecture, CI, and test gates

Expected result:

```text
The app can handle realistic directory trees and large text files without freezing the UI. Releases are blocked unless parity, safety, performance, and cross-platform smoke gates pass.
```

## 4. Milestone Plan

### Milestone M1 — Core Parity Slice

Scope:

- Read two paths.
- Classify text/binary/Excel/unsupported.
- Decode text with explicit metadata.
- Produce normalized line hunks.
- Produce inline segments on demand.
- Run parity fixtures against the existing behavior.

Exit criteria:

- Core test fixture suite exists.
- No GUI dependency exists in the core crate.
- Old/new fixture outputs are explainably equal or intentionally changed.

### Milestone M2 — Model-Backed Merge Slice

Scope:

- Create a `ComparisonSession`.
- Apply hunk copy operations.
- Track dirty state.
- Maintain undo/redo transaction history.
- Build save plans without writing.

Exit criteria:

- Merge operations are deterministic.
- Transaction history can replay from the original session.
- Save preflight detects unchanged external state and changed external state.

### Milestone M3 — Dioxus Shell Slice

Scope:

- Create Dioxus window shell.
- Implement app-level route/workspace state.
- Mount placeholder explorer and diff tabs.
- Wire command registry.
- Display errors/toasts/modals.

Exit criteria:

- App can load a sample session from core.
- All visible commands go through the command registry.
- No editor-specific logic leaks into shell state.

### Milestone M4 — Editor Adapter Slice

Scope:

- Mount editor pair.
- Push text from core to editor.
- Pull edits as transactions or snapshots.
- Apply line and inline decorations.
- Synchronize scrolling.
- Handle editor disposal and tab switching.

Exit criteria:

- Editor can be replaced by a test mock.
- The core remains the source of truth.
- Dioxus component redraws do not lose editor state.

### Milestone M5 — Explorer and Directory Slice

Scope:

- Directory opening.
- Paired tree/list view.
- Background digest jobs.
- Filtering and status badges.
- Cancellation.

Exit criteria:

- UI stays responsive during directory comparison.
- Cancellation is observable and safe.
- File pairs can open diff sessions.

### Milestone M6 — Save and Release Candidate Slice

Scope:

- Save current side.
- Save as.
- Backup policy.
- External modification conflict handling.
- Crash-safe write strategy.
- Diagnostics bundle.
- Cross-platform smoke tests.

Exit criteria:

- The app can be trusted with real files under documented conditions.
- Release gates block unsafe or untested builds.

## 5. Do-Not-Start-Yet List

The following items should not begin before M4 is stable:

- rich visual theming beyond the minimum required for diff readability;
- complex plugin architecture;
- cloud, sharing, collaboration, or project management features;
- generalized code editor ambitions unrelated to diff/merge;
- a second UI backend such as Iced.

## 6. RFC Batch Priority

Recommended order for this v0.2 batch:

```text
1. RFC-018 Migration Compatibility and Parity Plan
2. RFC-012 Text Encoding, Newline, and Binary Policy
3. RFC-011 Workspace Session Persistence
4. RFC-015 Undo/Redo Transaction Log
5. RFC-016 Editor Bridge Security and Contract
6. RFC-013 Large File and Performance Virtualization
7. RFC-014 Search, Filter, and Navigation
8. RFC-017 Error Taxonomy and Diagnostics UX
9. RFC-019 Command/Shortcut/Palette/Accessibility
10. RFC-020 Developer Architecture, CI, and Test Gates
```

This order deliberately starts with testability and data correctness before UI convenience.

## 7. Roadmap Risks

| Risk | Impact | Mitigation |
|---|---:|---|
| Editor bridge becomes product state | Critical | Enforce adapter contract and core ownership |
| Dioxus UI is built before core truth | High | Complete M1 and M2 before editor UI |
| Save appears before safety is real | Critical | Gate save behind RFC-007/RFC-012/RFC-015 |
| Large files freeze UI | High | Introduce virtualization policy before release candidate |
| Migration changes behavior silently | High | Parity fixtures and intentional-change register |
| Keyboard shortcuts conflict with editor | Medium | Central command registry with editor precedence rules |

## 8. Decision Summary

The project should proceed with Dioxus, but only under a strict core-first architecture.

The editor risk that made Iced concerning is real. Dioxus reduces that risk by allowing a mature web editor surface. However, that convenience must not create a new architecture where the editor DOM becomes the source of truth.
