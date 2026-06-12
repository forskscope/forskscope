# RFC 030 — User Documentation, Onboarding, and Help System

**Status.** Proposed — substantially implemented v0.96.0–v0.98.0; remaining items are in-app help and troubleshooting depth.

## Status

Substantially implemented. The documentation set defined below has shipped:

**Shipped (v0.96.0–v0.98.0):**
- `docs/src/users/quick-start.md` — install + first comparison workflow.
- `docs/src/users/comparing-files.md` — opening methods, diff view, search.
- `docs/src/users/diff-workflow.md` — pane layout, navigation, inline diff, options.
- `docs/src/users/merging.md` — merge model, apply/undo/redo, save workflow.
- `docs/src/users/explorer.md` — directory navigation, file selection, digest icons.
- `docs/src/users/directory-compare.md` — directory comparison and filtering.
- `docs/src/users/settings.md` — all settings fields documented.
- `docs/src/users/features.md` — complete feature overview.
- `docs/src/users/faq.md` — common questions.
- `docs/src/users/known-limitations.md` — current limitations and workarounds.
- `docs/src/intermediate/keyboard.md` — all keyboard shortcuts.
- `docs/src/intermediate/diff-options.md` — compare options reference.
- `docs/src/intermediate/file-types.md` — type classification, encoding, large files.
- `docs/src/intermediate/git-integration.md` — git mergetool setup.
- `docs/src/intermediate/patch-export.md` — unified diff export.
- `docs/src/intermediate/cli.md` — command-line usage.
- `CONTRIBUTING.md` — contributor guide.

**Remaining (deferred):**
- In-app help panel (first-run screen, contextual help links).
- Troubleshooting page for WebView/Linux dependency issues.
- Known-limitations page improvements (automated from issue tracker).

## Summary

Define the user-facing documentation and in-app help required for a credible v3 Dioxus release. ForskScope should be usable by Unix/Linux workers who need a practical alternative to WinMerge, not only by developers who read the source code.

## Goals

- Provide first-run onboarding.
- Provide quick-start workflows.
- Provide in-app command/shortcut help.
- Document safe save, backup, and restore behavior.
- Document directory merge behavior.
- Document known limitations and troubleshooting.

## Non-goals

- Full book-length manual in the first release.
- Video tutorials.
- Online-only help.
- Community documentation portal.

## Documentation set

```text
docs/user/
  quick-start.md
  compare-two-files.md
  merge-text-files.md
  compare-directories.md
  batch-directory-merge.md
  save-backup-restore.md
  compare-options.md
  shortcuts.md
  troubleshooting.md
  known-limitations.md
```

## In-app help model

Help should be available offline from the app.

```text
Help menu
  Quick Start
  Keyboard Shortcuts
  Safe Save and Backups
  Directory Merge Guide
  Troubleshooting
  About ForskScope
```

## First-run onboarding

```text
+--------------------------------------------------------------------------------+
| Welcome to ForskScope                                                           |
+--------------------------------------------------------------------------------+
| Compare and merge files/directories locally.                                    |
|                                                                                |
| Start here:                                                                     |
| [Compare Two Files] [Compare Two Directories] [Open Recent Session]              |
|                                                                                |
| Safety defaults:                                                                |
| ✓ backups before overwrite                                                      |
| ✓ explicit confirmation before batch operations                                 |
| ✓ local-only diagnostics                                                        |
|                                                                                |
| [Open Quick Start] [Continue]                                                    |
+--------------------------------------------------------------------------------+
```

## Contextual empty states

No files open:

```text
Drop two files here, choose files from the toolbar, or pass paths on the command line.
```

Directory scan empty:

```text
The selected directories are identical under the current compare profile.
Change compare options or choose different directories.
```

Unsupported file:

```text
This file cannot be safely displayed as text.
You can compare metadata, copy the file, open it externally, or change encoding/binary options.
```

## Shortcut help

Shortcut help should be generated from the command registry, not duplicated manually.

```text
Command                    Shortcut
Open files                 Ctrl+O
Open directories           Ctrl+Shift+O
Next difference            F7
Previous difference        Shift+F7
Copy left to right         Alt+Right
Copy right to left         Alt+Left
Save                       Ctrl+S
Command palette            Ctrl+Shift+P
```

## Troubleshooting topics

- Blank or failed window startup.
- Missing WebView runtime.
- Wayland/X11 behavior.
- File dialog issues.
- Clipboard/IME issues.
- Permission denied during save.
- External file modification warning.
- Large-file safe mode.
- Encoding ambiguity.
- Backup restore.

## Help implementation

Recommended approach:

- Markdown files stored in the repository.
- Build process embeds or bundles the docs.
- Dioxus help page renders Markdown safely.
- External links open in browser only after explicit click.

## Acceptance criteria

- Quick start is accessible from first-run screen.
- Shortcut help reflects command registry.
- Safe save/backup documentation exists.
- Directory merge documentation exists.
- Troubleshooting covers WebView/Linux issues.
- Help works offline.

## Test strategy

- Documentation link tests.
- Snapshot tests for command registry shortcut table.
- Manual first-run test.
- Manual help navigation test.

## Dependencies

- RFC 019 Command/shortcut palette.
- RFC 023 Atomic file operations.
- RFC 026 Cross-platform compatibility.
- RFC 031 Release channel/data compatibility.
