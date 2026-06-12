# RFC 026 — Cross-Platform WebView and Linux Desktop Compatibility

**Status.** Proposed — core diagnostics shipped (v0.93.0, v0.100.0, v0.112.0); platform smoke tests and packaging QA deferred to RFC-010.

## Status

Partially implemented:

- **`PlatformInfo::collect()`** (v0.93.0) — runtime diagnostic snapshot: OS, arch, CPUs, app version, Rust version, home dir (redacted), config dir. 9 tests.
- **`PlatformInfo` wired to About panel** (v0.100.0) — About modal uses `PlatformInfo` and `to_report()` for the "Copy diagnostics" button.
- **`--diagnostics` CLI flag** (v0.112.0) — `forskscope --diagnostics` prints `PlatformInfo::to_report()` and exits without launching the UI. Documented in `main.rs` and in `docs/src/users/troubleshooting.md`.
- **`docs/src/users/troubleshooting.md`** (v0.112.0) — WebView/Linux troubleshooting guide including the `--diagnostics` flag, WebKitGTK installation instructions for major distributions, NVIDIA DMA-BUF workaround, Wayland/X11 fallback, macOS Gatekeeper fix, Windows WebView2 installation, session restoration troubleshooting, and bug report instructions.

**Remaining (requires GTK / cross-platform CI):**
- Platform smoke tests (Wayland session, X11 session, file dialogs).
- Blank-window detection and structured error message.
- `--safe-editor` flag (editor adapter track, RFC-004 dependency).
- Compatibility settings UI (`Settings → Advanced → Compatibility`).

## Summary

Define compatibility requirements, diagnostics, and fallback behavior for Dioxus Desktop/WebView deployment across Linux, Windows, and macOS, with special attention to Unix/Linux workers.

## Goals

- Make WebView dependency explicit and diagnosable.
- Support Linux Wayland and X11 as practical targets.
- Provide useful error messages when WebView/runtime dependencies are missing.
- Define platform smoke tests.
- Avoid silent blank-window failures.

## Non-goals

- Removing WebView in this migration.
- Supporting every old distribution.
- Building a full native Iced UI fallback.

## Platform support matrix

```text
Platform        Tier    Requirement
Linux Wayland   1      primary Unix/Linux target
Linux X11       1      supported fallback/session target
Windows 10/11   1      supported desktop target
macOS current   1      supported desktop target
Older Linux     2      best effort with diagnostics
BSD desktops    3      future/best effort only
```

## Startup diagnostics

Before showing the main window where possible, the app should detect and log:

- OS and desktop session type,
- WebView backend availability,
- graphics/session environment,
- app version,
- safe-editor mode status,
- relevant environment variables.

The app should avoid collecting or transmitting diagnostics automatically. Diagnostics are local and user-exported only.

## Blank-window prevention

If the UI fails to initialize, show a minimal native/error fallback if technically possible. If not possible, write a clear log file and exit with a non-zero code.

Recommended user message:

```text
ForskScope could not start the desktop WebView.

Possible causes:
- missing WebView runtime,
- unsupported Linux desktop session,
- graphics driver issue,
- sandbox restriction.

Try:
- launching with --safe-editor,
- launching from a terminal to view logs,
- installing the platform WebView runtime,
- switching between Wayland and X11 if available.
```

## Linux-specific considerations

The first release should explicitly test:

- Wayland session,
- X11 session,
- file dialogs,
- drag and drop,
- clipboard,
- IME input,
- high DPI scaling,
- dark/light appearance,
- external file opening.

## Compatibility settings

```text
Settings → Advanced → Compatibility
  [ ] Start in safe editor mode
  [ ] Disable drag-and-drop file opening
  [ ] Prefer simple file dialogs
  [ ] Reduce visual effects
  [ ] Enable verbose local diagnostics
```

## CLI options

```text
forskscope --safe-editor
forskscope --diagnostics
forskscope --no-dnd
forskscope --reset-window-state
forskscope --open-log-dir
```

## Acceptance criteria

- Platform support matrix is published.
- Linux Wayland and X11 smoke scripts exist.
- Startup diagnostics are written locally.
- Safe-editor mode can be used as compatibility fallback.
- Missing runtime failure has actionable messaging where possible.
- CI or manual release checklist covers all Tier 1 targets.

## Test strategy

- Manual smoke tests on Tier 1 platforms.
- Automated CLI startup tests where possible.
- Log generation tests.
- Window-state reset tests.
- Safe-editor launch tests.

## Dependencies

- RFC 003 Dioxus application shell.
- RFC 010 Packaging/diagnostics/QA.
- RFC 017 Error taxonomy.
- RFC 025 Editor prototype and kill switch.
