# Roadmap v0.3 — Feature Completion and Release Readiness

## 1. Purpose

This roadmap refines the Dioxus migration plan after the foundational RFCs. It assumes the following decisions are already accepted:

- Dioxus is adopted as the near-term UI platform.
- Iced remains a respected future-native option, not the current target.
- ForskScope must extract a GUI-independent Rust core before UI behavior is expanded.
- Text editor complexity is a primary risk.
- A CodeMirror-like editor surface may be used through a controlled adapter boundary.
- Product truth must live in Rust models, not in DOM state.

The purpose of v0.3 is to move the implementation plan from “can migrate” to “can ship a credible diff/merge workstation tool.”

## 2. Roadmap phase map

```text
Phase A — Foundation already covered
  001 Core extraction
  002 similar v3 diff engine
  003 Dioxus shell
  004 Editor adapter
  005 Explorer
  006 Diff/merge workspace
  007 Save/session safety
  008 Directory comparison jobs
  009 Settings/theme/accessibility
  010 Packaging/diagnostics/QA

Phase B — Deepening already covered
  011 Workspace persistence
  012 Encoding/newline/binary policy
  013 Large-file performance
  014 Search/filter/navigation
  015 Undo/redo transaction log
  016 Editor bridge security contract
  017 Error taxonomy and diagnostics UX
  018 Migration compatibility parity
  019 Command/shortcut palette
  020 Developer architecture and CI gates

Phase C — This package
  021 Document/result-buffer model
  022 Directory merge and batch operations
  023 Atomic file operations, backup, restore
  024 Diff visual semantics
  025 Editor-adapter prototype and kill switch
  026 Cross-platform WebView/Linux compatibility
  027 Report/export/session evidence
  028 Preferences, profiles, compare options
  029 External tools and open-with integration
  030 User documentation, onboarding, help
  031 Release channel, migration, data compatibility
```

## 3. Milestone plan

### Milestone M1 — Core-safe document editing

Target RFCs:

- RFC 021
- RFC 023
- RFC 024
- RFC 025

Outcome:

The application can open two files, build a diff, expose an editable result buffer, apply merge actions as transactions, render stable diff decorations, and save only through an atomic write policy.

Key gate:

```text
No editor operation may bypass the Rust document model.
```

### Milestone M2 — Product-grade comparison options

Target RFCs:

- RFC 028
- parts of RFC 024

Outcome:

Users can control comparison behavior with stable, understandable options such as whitespace handling, newline treatment, encoding fallback, case sensitivity, and binary limits.

Key gate:

```text
Every compare option must be reflected in session metadata and report/export output.
```

### Milestone M3 — Directory merge workflow

Target RFCs:

- RFC 022
- RFC 023
- RFC 027

Outcome:

Directory comparison becomes actionable. Users can review changed/added/deleted files, open file-level diffs, copy selected files, perform batch merge actions, and receive a safe operation summary before writes occur.

Key gate:

```text
Batch operations must be previewable, cancellable, and recoverable.
```

### Milestone M4 — Cross-platform hardening

Target RFCs:

- RFC 026
- RFC 029
- RFC 031

Outcome:

The app is tested across Linux, Windows, and macOS with explicit handling for WebView availability, Wayland/X11 behavior, file dialogs, external tool launching, and release-channel compatibility.

Key gate:

```text
The app must fail with actionable diagnostics, not silent UI breakage.
```

### Milestone M5 — Release usability

Target RFCs:

- RFC 027
- RFC 030
- RFC 031

Outcome:

Users can understand the app without developer guidance. The app provides onboarding, in-app help, visible shortcut reference, exportable comparison evidence, and clear release upgrade behavior.

Key gate:

```text
A new user should be able to complete a two-file compare, one hunk merge, and safe save within five minutes.
```

## 4. RFC completion criteria

Each RFC in this package should be considered complete only when it includes:

- explicit goals and non-goals,
- user-visible behavior,
- model or data contract where relevant,
- UI/UX implications,
- failure modes,
- acceptance criteria,
- test strategy,
- dependencies.

## 5. Scope-control rule

ForskScope is not trying to become a full IDE. Features that resemble IDE behavior must be admitted only if they directly support diff/merge work.

Accepted:

- search within compared files,
- syntax-like line rendering if cheap,
- external tool launch,
- compare profiles,
- report export.

Rejected for this phase:

- language server integration,
- project indexing,
- Git repository management,
- collaborative editing,
- cloud synchronization,
- real-time multi-user merge.
