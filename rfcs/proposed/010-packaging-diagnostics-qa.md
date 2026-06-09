# RFC-010 — Packaging, Diagnostics, QA, and Release Gates

**Status.** Proposed

---toml
project = "ForskScope"
rfc = "010"
title = "Packaging, Diagnostics, QA, and Release Gates"
status = "proposed"
phase = "M10"
depends_on = ["RFC-007", "RFC-008", "RFC-009"]
---

## 1. Summary

Define the packaging, diagnostics, QA matrix, and release gates for the Dioxus migration. This RFC ensures that the migration produces a practical cross-platform desktop app rather than only a development build.

## 2. Goals

- Package ForskScope for Linux, Windows, and macOS where feasible.
- Provide diagnostics useful for bug reports.
- Define smoke tests and regression tests.
- Validate migration behavior against current v0.22.13 representative workflows.
- Ensure file safety features pass before release.

## 3. Non-Goals

- Guarantee every Linux desktop environment behaves identically.
- Solve paid code-signing distribution in this RFC.
- Implement auto-update in first Dioxus release.
- Provide enterprise deployment tooling.

## 4. Packaging Targets

| Platform | Target Artifacts | Notes |
|---|---|---|
| Linux | tarball/AppImage/deb/rpm or selected subset | Unix/Linux workers are a primary audience. |
| Windows | zip/msi or installer-free package | WinMerge alternative positioning requires Windows smoke test too. |
| macOS | app bundle/dmg or zip | Gatekeeper/signing strategy may be documented separately. |

The exact artifact set can be narrowed by release engineering capacity.

## 5. Diagnostics Panel

```text
┌──────────────────────────────────────────────────────────────┐
│ Diagnostics                                                  │
├──────────────────────────────────────────────────────────────┤
│ App version: 0.xx                                            │
│ Core version: 0.xx                                           │
│ Dioxus version: pinned                                       │
│ similar version: pinned                                      │
│ OS: Linux x86_64                                             │
│ WebView backend: detected                                    │
│ Config path: ~/.config/forskscope/...                        │
│ Log path: ~/.local/state/forskscope/...                      │
│                                                            │
│ [Copy Diagnostics] [Open Log Folder] [Export Report]         │
└──────────────────────────────────────────────────────────────┘
```

## 6. Logging Policy

- Logs must avoid dumping full file contents by default.
- Paths may appear in diagnostics unless privacy mode is enabled.
- Errors should include operation, path if allowed, and core error code.
- A user-copyable diagnostic report should be available.

## 7. QA Matrix

### 7.1 Functional Matrix

| Area | Required Checks |
|---|---|
| Startup | no args, one arg, two args, invalid args |
| File load | UTF-8, non-UTF-8, binary, empty, missing, permission denied |
| Diff | equal, insert, delete, replace, Unicode, newline differences |
| Merge | copy left→right, copy right→left, undo, redo, stale hunk |
| Save | save, save as, conflict, backup, permission denied |
| Explorer | browse, pair files, missing one side, digest running, cancel |
| Settings | theme, font size, locale, corrupt settings fallback |
| Accessibility | keyboard-only core workflow, focus trap, non-color diff semantics |

### 7.2 Platform Matrix

| Platform | Minimum Gate |
|---|---|
| Linux Wayland | Launch, open files, diff, merge, save |
| Linux X11 | Launch, open files, explorer, save |
| Windows 11 | Launch, open files, diff, save |
| macOS current supported version | Launch, open files, save, close dirty tab |

## 8. Migration Regression Suite

Use representative fixtures from the current app:

```text
fixtures/
  text/
    equal.txt
    changed_ascii.txt
    changed_japanese.txt
    crlf_vs_lf.txt
  binary/
    small.bin
  excel/
    small-a.xlsx
    small-b.xlsx
  dirs/
    left/
    right/
```

The new app should preserve user-visible comparison meaning even if internal JSON differs.

## 9. Release Gates

### Gate A — Core Gate

- Core tests pass.
- Golden diff tests pass.
- No GUI dependencies in core.

### Gate B — Editor Gate

- Editor adapter proof passes.
- Text change events are revision-checked.
- Decorations apply reliably.

### Gate C — Save Safety Gate

- External modification conflict test passes.
- Dirty close dialog test passes.
- Backup test passes.

### Gate D — Platform Gate

- App launches on target platforms.
- File open/diff/save smoke tests pass.
- Diagnostics report is copyable.

## 10. CI Strategy

Recommended CI jobs:

```text
cargo fmt
cargo clippy
cargo test -p forskscope-core
cargo test -p forskscope-editor-adapter
cargo test -p forskscope-ui-dioxus --no-run
fixture regression tests
packaging dry-run per platform where practical
```

UI automation may start with smoke tests and grow over time.

## 11. Acceptance Criteria

- Release artifacts can be produced for selected platforms.
- Diagnostics panel is implemented.
- QA matrix is documented and partially automated.
- Migration regression suite exists.
- No release candidate can pass without save safety tests.
- Known platform limitations are documented in release notes.

## 12. Risks

| Risk | Mitigation |
|---|---|
| Packaging consumes too much time | Start with tarball/zip before installers. |
| WebView differences surface late | Add early platform smoke tests. |
| Logs expose sensitive paths/content | Provide privacy mode and no content dumps. |
| Release criteria are too broad | Define minimum gate and stretch gate. |

## 13. Open Questions

- Which Linux package formats are mandatory for the first Dioxus release?
- Should unsigned macOS/Windows distribution be accepted initially?
- Should a portable app mode be supported?
- How much UI automation is realistic in the first release?
