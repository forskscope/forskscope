# ForskScope Dioxus Migration — RFC Cross-Reference Matrix

| Concern | RFC Owner | Notes |
|---|---|---|
| Canonical product state | RFC-001 | Rust core owns file/session/diff/merge/save truth. |
| Diff algorithm and hunk model | RFC-002 | `similar` v3 integration hidden behind normalized model. |
| Desktop shell and commands | RFC-003 | Dioxus application state and command dispatcher. |
| Editable text surface | RFC-004 | Editor adapter boundary; CodeMirror-like bridge. |
| Directory/file browsing | RFC-005 | Two-pane explorer workflow. |
| Merge transactions | RFC-006 | Hunk navigation, merge commands, undo/redo. |
| Data loss prevention | RFC-007 | Dirty state, save conflict, backup. |
| Long-running work | RFC-008 | Background jobs, digest, cancellation. |
| Human factors | RFC-009 | Theme, typography, localization, accessibility. |
| Release readiness | RFC-010 | Packaging, diagnostics, QA gates. |

## High-Risk Dependencies

```text
RFC-004 editor proof must complete before committing to full editable merge UX.
RFC-007 save safety must complete before any release candidate.
RFC-001 core extraction must complete before meaningful Dioxus migration.
```
