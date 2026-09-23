# Review Request: Linux Wayland CLI Render Smoke Evidence

## Context

Runtime/platform verification remains the main release-readiness lane. This package adds fresh Linux Wayland evidence for CLI-open two-way file compare rendering under Niri.

This does not close the full RFC-041 runtime/platform lane because keyboard navigation, hunk apply, undo, save, search, directory workflow, macOS verification, and Windows verification remain unobserved here.

## Evidence Collected

- Launched release binary under the active Wayland session:
  - `env WEBKIT_DISABLE_DMABUF_RENDERER=1 ./target/release/forskscope tests/fixtures/text/left_function.txt tests/fixtures/text/right_function.txt`
- Niri reported a ForskScope window:
  - window id `28`
  - title `ForskScope`
  - app id `forskscope`
  - window size `1042x1148`
- Captured fresh window screenshot:
  - `.git-exclude/runtime-smoke/linux-wayland-cli-open-20260710.png`
- Inspected the screenshot visually.
- Confirmed screenshot file metadata:
  - `PNG image data, 1298 x 1426, 8-bit/color RGBA, non-interlaced`
- Ran release binary diagnostics:
  - `./target/release/forskscope --diagnostics`

## Screenshot Inspection Result

The screenshot shows:

- ForskScope window rendered under Wayland/Niri.
- CLI-open diff tab for `left_function.txt` and `right_function.txt`.
- Two-pane diff layout with left/old and right/new content.
- Focused changed hunk with gutter markers and the inline `Use` merge control.
- Status bar showing local-only mode.
- No default Dioxus/framework devtools menu visible.

## Files To Inspect

- `.git-exclude/runtime-smoke/linux-wayland-cli-open-20260710.png`
- `docs/src/maintainers/gtk-smoke-test.md`
- `dev-record/reviews/010-runtime-platform-verification-review.md`
- `dev-record/reviews/011-linux-wayland-smoke-docs-review.md`

## Verification Observed

- `command -v niri`
- `test -x target/release/forskscope`
- `niri msg -j windows`
- `niri msg action screenshot-window --id 28 --path /home/<user>/Desktop/forskscope/forskscope-git/.git-exclude/runtime-smoke/linux-wayland-cli-open-20260710.png`
- `file .git-exclude/runtime-smoke/linux-wayland-cli-open-20260710.png`
- `./target/release/forskscope --diagnostics`
- `git status --short`

## Known Limits

- This is visual render/startup evidence only.
- No keyboard navigation, hunk apply, undo, save, search, or directory workflow execution was observed.
- `wtype`, `ydotool`, and `dotool` were not installed, so I could not synthesize Wayland keyboard input.
- `niri msg action focus-window --id 28` returned success, but `niri msg -j focused-window` continued to report the editor window as focused, so I did not record focused-window evidence for ForskScope.
- No macOS runtime/package verification was observed.
- No Windows runtime/package verification was observed.
- No live GitHub Actions run was observed.

## Recommended Next Action

Use this as partial Linux Wayland render evidence. The next runtime/platform step should be a manual Linux keyboard/apply/save smoke pass, or installation of a Wayland input tool such as `wtype`/`ydotool` followed by an automated keyboard smoke attempt.
