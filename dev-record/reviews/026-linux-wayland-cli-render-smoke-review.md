# Linux Wayland CLI Render Smoke Review

Request: `dev-record/review-requests/021-linux-wayland-cli-render-smoke.md`

Date: 2026-07-10

## Verdict

Accept with notes.

The screenshot and supporting local checks provide useful partial Linux Wayland runtime evidence. They show that the release binary can launch under the active Niri/Wayland environment, that CLI-open file compare renders a nonblank two-pane diff, and that the default Dioxus/framework devtools menu is not visible.

This does not close the full runtime/platform verification lane. Keyboard navigation, hunk apply/undo/save/search, directory workflow, macOS verification, and Windows verification remain unobserved.

## Blocking findings

None for this evidence package.

## Non-blocking findings

1. This should be recorded as partial Linux render evidence only. It reduces the Linux visual-rendering gap, but it is not a completed RFC-041 smoke pass because no keyboard, apply, undo, save, search, or directory workflow execution was observed.

## Requirement checks

- `.git-exclude/runtime-smoke/linux-wayland-cli-open-20260710.png` is a valid PNG: `1298 x 1426`, 8-bit RGBA, non-interlaced.
- Visual inspection confirms a ForskScope window with a CLI-open diff tab for `left_function.txt` and `right_function.txt`.
- The screenshot shows left/old and right/new panes, a focused changed hunk, gutter markers, the inline `Use` control, and local-only status.
- No default Dioxus/framework devtools menu is visible in the screenshot.
- `docs/src/maintainers/gtk-smoke-test.md:40`-`docs/src/maintainers/gtk-smoke-test.md:46` documents the same fixture pair and expected CLI-open rendering.
- Prior reviews `dev-record/reviews/010-runtime-platform-verification-review.md` and `dev-record/reviews/011-linux-wayland-smoke-docs-review.md` already classify this kind of evidence as useful but partial.

## Observed evidence

- I visually inspected `.git-exclude/runtime-smoke/linux-wayland-cli-open-20260710.png`.
- `file .git-exclude/runtime-smoke/linux-wayland-cli-open-20260710.png` reported `PNG image data, 1298 x 1426, 8-bit/color RGBA, non-interlaced`.
- `./target/release/forskscope --diagnostics` ran successfully and printed ForskScope 0.164.0 Linux x86_64 diagnostics.
- `command -v niri` found `/usr/bin/niri`.
- `test -x target/release/forskscope` passed.
- `ls -l` confirmed the screenshot, release binary, and `left_function.txt` / `right_function.txt` fixtures are present.
- `command -v wtype`, `command -v ydotool`, and `command -v dotool` produced no tool paths in this environment.
- `git status --short` showed no tracked worktree changes from this evidence-only review.

Request-provided but not reobserved during this review:

- `niri msg -j windows` reporting ForskScope window id `28`.
- `niri msg action screenshot-window --id 28 --path ...`.
- `niri msg action focus-window --id 28` behavior.

## Missing evidence

- No keyboard navigation smoke was observed.
- No hunk apply, undo, save, or search workflow was observed.
- No directory compare workflow was observed.
- No completed RFC-041 Linux manual smoke pass was observed.
- No macOS runtime/package verification was observed.
- No Windows runtime/package verification was observed.
- No live GitHub Actions run was observed.

## Recommended next action

Accept this as partial Linux Wayland CLI-render evidence. The next runtime/platform candidate should be a manual Linux keyboard/apply/save smoke pass, or an automated Wayland keyboard smoke attempt after installing an input tool such as `wtype`, `ydotool`, or `dotool`.
