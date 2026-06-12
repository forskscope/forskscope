# ForskScope RFCs

Lifecycle: [RFC 000](./done/000-rfc-lifecycle-policy.md). Numbers never reused.

> Numbering note: RFCs 043–053 are reserved for the packaging, accessibility,
> documentation, and product-boundary themes sketched in the migration roadmap
> (RFC 042) and the non-goals addendum. New feature work introduced after the
> v0.34 audit is numbered from RFC 054 onward.

## Implemented (39)

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | Implemented |
| 001 | [Core Extraction and Domain Model](./done/001-core-extraction-and-domain-model.md) | v0.23.0 |
| 002 | [Diff Engine: `similar` v3 + Normalized Diff Model](./done/002-similar-v3-diff-engine.md) | v0.23.0 |
| 003 | [Dioxus Application Shell and State Runtime](./done/003-dioxus-application-shell.md) | v0.23.0 |
| 005 | [Explorer Workspace and Paired Directory Workflow](./done/005-explorer-workspace.md) | v0.25.0 |
| 006 | [Diff/Merge Workspace and Merge Transaction Model](./done/006-diff-merge-workspace.md) | v0.26.0 |
| 007 | [Save, Session, and File Safety Policy](./done/007-save-session-file-safety.md) | v0.27.0 |
| 008 | [Directory Comparison and Background Job Model](./done/008-directory-comparison-background-jobs.md) | v0.58.0 + v0.68.0 (runner UI deferred) |
| 009 | [Settings, Theme, Localization, and Accessibility](./done/009-settings-theme-localization-accessibility.md) | v0.60.0 (settings dialog UI deferred) |
| 011 | [Workspace Session Persistence](./done/011-workspace-session-persistence.md) | v0.56.0 (tab list restore deferred to schema v2) |
| 012 | [Text Encoding, Newline, and Binary Policy](./done/012-text-encoding-newline-binary-policy.md) | v0.50.0 + v0.69.0 (footer display UI deferred) |
| 013 | [Large File, Performance, and Virtualization Strategy](./done/013-large-file-performance-virtualization.md) | v0.46.0 + v0.59.0 (row virtualization UI deferred) |
| 014 | [Search, Filter, and Navigation](./done/014-search-filter-navigation.md) | v0.43.0 (explorer filter UI deferred) |
| 015 | [Undo/Redo Transaction Log](./done/015-undo-redo-transaction-log.md) | v0.47.0 (history panel UI deferred) |
| 017 | [Error Taxonomy and Diagnostics UX](./done/017-error-taxonomy-diagnostics-ux.md) | v0.67.0 (diagnostics panel UI deferred) |
| 019 | [Command Registry, Keyboard Shortcuts, and Command Palette](./done/019-command-shortcut-palette-accessibility.md) | v0.63.0 (command palette UI deferred) |
| 020 | [Developer Architecture, CI, and Test Gates](./done/020-developer-architecture-ci-test-gates.md) | v0.48.0 + v0.73.0 |
| 021 | [Document and Result Buffer Model](./done/021-document-and-result-buffer-model.md) | v0.28.0 |
| 022 | [Directory Merge and Batch Operations](./done/022-directory-merge-and-batch-operations.md) | v0.52.0 (batch preview UI deferred) |
| 023 | [Atomic File Operations, Backup, and Restore](./done/023-atomic-file-operations-backup-restore.md) | v0.44.0 (restore picker UI deferred) |
| 024 | [Diff Visual Semantics and Decoration Contract](./done/024-diff-visual-semantics-decoration-contract.md) | v0.61.0 (renderer wiring deferred) |
| 027 | [Report Export and Session Evidence](./done/027-report-export-and-session-evidence.md) | v0.49.0 (HTML export dialog deferred) |
| 028 | [Preferences, Profiles, and Compare Options](./done/028-preferences-profiles-and-compare-options.md) | v0.50.0 + v0.66.0 (toolbar selector UI deferred) |
| 029 | [Integration with External Tools and Open With](./done/029-integration-with-external-tools-and-open-with.md) | v0.55.0 + v0.70.0 (settings UI deferred) |
| 031 | [Release Channel, Migration, and Data Compatibility](./done/031-release-channel-migration-and-data-compatibility.md) | v0.51.0 |
| 032 | [Text Editing Operation Model and Editor Truth Boundary](./done/032-text-editing-operation-model-and-editor-truth-boundary.md) | v0.62.0 (EditBuffer dispatch deferred) |
| 033 | [Three-Way Merge Model](./done/033-three-way-merge-model.md) | v0.40.0 (conflict workspace UI deferred) |
| 034 | [Conflict Resolution Workspace](./done/034-conflict-resolution-workspace.md) | v0.64.0 (workspace UI deferred) |
| 035 | [Scroll Sync, Line Mapping, and Diff Decoration Engine](./done/035-scroll-sync-line-mapping-and-diff-decoration-engine.md) | v0.61.0 (scroll-sync wiring deferred) |
| 036 | [Live Reload, File Watcher, and External Modification Handling](./done/036-live-reload-file-watcher-and-external-modification-handling.md) | v0.53.0 + v0.71.0 (platform watcher deferred) |
| 037 | [Scalable Directory Compare Index and Incremental Refresh](./done/037-scalable-directory-compare-index-and-incremental-refresh.md) | v0.42.0 + v0.58.0 (persistent cache deferred) |
| 038 | [VCS Context Integration Boundary](./done/038-vcs-context-integration-boundary.md) | v0.54.0 (UI panel and JJ provider deferred) |
| 039 | [Patch Export, Apply, and Review Workflow](./done/039-patch-export-apply-and-review-workflow.md) | v0.39.0 (export only; apply deferred) |
| 054 | [Explorer Tree-View and Interaction Model](./done/054-explorer-tree-view-and-interaction-model.md) | v0.36.0 |
| 055 | [Breadcrumb Path Navigation](./done/055-breadcrumb-path-navigation.md) | v0.36.0 |
| 056 | [Ignore Patterns for Files and Directories](./done/056-ignore-patterns-for-files-and-directories.md) | v0.36.0 |
| 057 | [Settings Dialog Layout Refinements](./done/057-settings-dialog-layout-refinements.md) | v0.36.0 |
| 058 | [Spreadsheet (`.xlsx`) Structural Diff and Adapter Contract](./done/058-spreadsheet-xlsx-structural-diff.md) | v0.57.0 (aligned view deferred) |
| 059 | [Explorer and Compare UI/UX Audit Remediation](./done/059-explorer-and-compare-uiux-audit-remediation.md) | v0.41.0 (UI keyboard items deferred) |

## Proposed (9)

All remaining proposed RFCs are editor-adapter track, platform/packaging, or governance/documentation.
RFC-026 and RFC-030 are substantially implemented; their remaining items require GTK or are deferred.

| ID | Title | Category | Progress |
|----|-------|----------|----------|
| 004 | [Editor Adapter and CodeMirror Bridge](./proposed/004-editor-adapter-and-codemirror-bridge.md) | Editor adapter | Not started — requires GTK/WebView |
| 010 | [Packaging, Diagnostics, QA, and Release Gates](./proposed/010-packaging-diagnostics-qa.md) | Platform/packaging | Not started — requires cross-platform CI |
| 016 | [Editor Bridge Security and Contract](./proposed/016-editor-bridge-security-and-contract.md) | Editor adapter | Blocked on RFC-004 |
| 025 | [Editor Adapter Prototype and Kill Switch](./proposed/025-editor-adapter-prototype-and-kill-switch.md) | Editor adapter | Blocked on RFC-004 |
| 026 | [Cross-Platform WebView and Linux Desktop Compatibility](./proposed/026-cross-platform-webview-and-linux-desktop-compatibility.md) | Platform/packaging | **Partially shipped** — PlatformInfo, --diagnostics, troubleshooting.md |
| 030 | [User Documentation, Onboarding, and Help System](./proposed/030-user-documentation-onboarding-and-help-system.md) | Documentation | **Substantially shipped** — 18 doc files; in-app help panel deferred |
| 040 | [Editor Adapter Verification Harness and Golden Corpus](./proposed/040-editor-adapter-verification-harness-and-golden-corpus.md) | Editor adapter | Blocked on RFC-004 |
| 041 | [v1.0 Product Stabilization and RFC Governance](./proposed/041-v1-product-stabilization-and-rfc-governance.md) | Governance | 12/16 checklist items done; 4 require GTK or deferred |
| 042 | [Roadmap and RFC Execution Plan](./proposed/042-roadmap-and-rfc-execution-plan.md) | Governance | Living document — current through v0.113.0 |

## Archive (1)

| ID | Title | Reason |
|----|-------|--------|
| 018 | [Migration Compatibility and Parity Plan](./archive/018-migration-compatibility-parity-plan.md) | Withdrawn — migration complete; superseded by delivered implementation |

## Notes

- [acceptance-test-corpus-plan](./notes/acceptance-test-corpus-plan.md)
- [core-completion-summary-v0.72](./notes/core-completion-summary-v0.72.md)
- [editor-adapter-risk-register](./notes/editor-adapter-risk-register.md)
- [editor-risk-decision-record](./notes/editor-risk-decision-record.md)
- [feature-completion-scope-control](./notes/feature-completion-scope-control.md)
- [implementation-checklist](./notes/implementation-checklist.md)
- [implementation-gate-checklist-v0.2](./notes/implementation-gate-checklist-v0.2.md)
- [modern-diff-merge-tools-feature-candidates](./notes/modern-diff-merge-tools-feature-candidates.md)
- [release-readiness-checklist-v0.3](./notes/release-readiness-checklist-v0.3.md)
- [rfc-cross-reference-matrix](./notes/rfc-cross-reference-matrix.md)
- [rfc-dependency-map-v0.2](./notes/rfc-dependency-map-v0.2.md)
- [roadmap-v0.2-refinement](./notes/roadmap-v0.2-refinement.md)
- [roadmap-v0.3-feature-completion](./notes/roadmap-v0.3-feature-completion.md)
- [roadmap-v0.4-winmerge-class-hardening](./notes/roadmap-v0.4-winmerge-class-hardening.md)
- [v0.4-implementation-gate-checklist](./notes/v0.4-implementation-gate-checklist.md)
- [winmerge-parity-risk-matrix](./notes/winmerge-parity-risk-matrix.md)
