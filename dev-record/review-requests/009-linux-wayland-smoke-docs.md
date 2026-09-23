# Review request: Linux Wayland smoke-test docs

## Scope

This package covers a small maintainer-doc correction found while continuing
runtime/platform verification after the desktop release-build fix.

## Files to inspect

- `docs/src/maintainers/gtk-smoke-test.md`

## Change summary

- Corrected the CLI smoke-test fixture paths from non-existent
  `left_function.rs` / `right_function.rs` to the existing
  `left_function.txt` / `right_function.txt`.
- Added a Niri/Wayland note showing how to use `niri msg` to list windows and
  capture a focused window screenshot as visual evidence.

## Observed verification

Passed in this thread:

- `niri msg -j windows`
  - observed ForskScope Wayland window id `60`, title `ForskScope`, app id
    `Forskscope`, size `1042x1148`.
- `niri msg action screenshot-window --id 60 --path /home/<user>/Desktop/forskscope/forskscope-git/.git-exclude/runtime-smoke/linux-wayland-cli-open.png`
- Visual inspection of `.git-exclude/runtime-smoke/linux-wayland-cli-open.png`
  confirmed:
  - valid two-pane diff view opened from
    `tests/fixtures/text/left_function.txt` and
    `tests/fixtures/text/right_function.txt`
  - changed line rendered on both panes
  - focused hunk outline visible
  - hunk count displayed as `1/1`
  - Save/Save As/Undo controls visible
  - Local-only status visible
  - no default Dioxus/framework devtools menu visible
- `file .git-exclude/runtime-smoke/linux-wayland-cli-open.png`
  - output: `PNG image data, 1250 x 1378, 8-bit/color RGBA, non-interlaced`
- `git diff --check`
- `mdbook build docs`

## Limitations

- This is partial Linux visual smoke evidence, not a complete RFC-041 manual
  workflow pass.
- This environment has Niri/Wayland and no installed Wayland keyboard injection
  tool such as `wtype`, `ydotool`, or `dotool`; keyboard navigation/apply/save
  workflow checks still require manual execution or a separate input automation
  setup.
- macOS and Windows runtime/package verification remain unobserved.

## Reviewer questions

- Is the fixture-path correction sufficient for the maintainer checklist?
- Is the Niri/Wayland screenshot note acceptable as optional visual-evidence
  guidance, or should it be moved to troubleshooting/local-dev docs instead?
- Does this partial visual evidence reduce the Linux runtime verification gap,
  while keeping final release blocked on manual keyboard/save workflow and
  macOS/Windows verification?
