# Linux Wayland Smoke Docs Review

Request: `dev-record/review-requests/009-linux-wayland-smoke-docs.md`

Date: 2026-07-10

## Verdict

Accept with notes.

The maintainer-doc correction is valid and scoped. The old CLI smoke-test fixture paths pointed at files that do not exist, while the new `.txt` paths exist and match the captured Linux Wayland visual smoke evidence. The Niri/Wayland note is acceptable as optional compositor-specific evidence guidance in the GTK smoke checklist.

This reduces the Linux runtime verification gap by adding real visual evidence for CLI-open diff rendering, but it does not close the runtime/platform lane. Keyboard navigation, hunk apply, save behavior, full manual workflow coverage, and macOS/Windows runtime/package verification remain outstanding.

## Blocking findings

None.

## Non-blocking findings

1. The Niri instructions are useful where Niri is the active compositor, but they should remain optional evidence guidance rather than a required Linux smoke procedure. The checklist still needs to support manual verification on non-Niri Wayland and X11 sessions.

2. The screenshot evidence is stored under `.git-exclude/runtime-smoke/`, so it is local review evidence rather than durable tracked release documentation. That is fine for this review package, but the release owner should decide whether final runtime evidence belongs in an ignored handoff path or in tracked release notes.

## Requirement checks

- `docs/src/maintainers/gtk-smoke-test.md:20`-`docs/src/maintainers/gtk-smoke-test.md:25` adds Niri/Wayland commands for listing windows and capturing a focused window screenshot.
- `docs/src/maintainers/gtk-smoke-test.md:41`-`docs/src/maintainers/gtk-smoke-test.md:42` now references `tests/fixtures/text/left_function.txt` and `tests/fixtures/text/right_function.txt`.
- `tests/fixtures/text/left_function.txt` and `tests/fixtures/text/right_function.txt` exist.
- The old `tests/fixtures/text/left_function.rs` and `tests/fixtures/text/right_function.rs` paths do not exist, confirming the correction is necessary.
- The referenced screenshot `.git-exclude/runtime-smoke/linux-wayland-cli-open.png` is a valid PNG and visually shows a two-pane CLI-open diff with one focused hunk, visible controls, local-only status, and no default Dioxus/framework devtools menu.

## Reviewer questions

- The fixture-path correction is sufficient for the maintainer checklist.
- The Niri/Wayland screenshot note is acceptable in `gtk-smoke-test.md` because it is directly tied to collecting visual smoke evidence. It does not need to move to troubleshooting/local-dev docs unless the section grows into broader compositor troubleshooting.
- The partial visual evidence reduces the Linux runtime verification gap, but final release should remain blocked on manual keyboard/apply/save workflow evidence and macOS/Windows verification.

## Observed evidence

- `file .git-exclude/runtime-smoke/linux-wayland-cli-open.png` reported `PNG image data, 1250 x 1378, 8-bit/color RGBA, non-interlaced`.
- I visually inspected `.git-exclude/runtime-smoke/linux-wayland-cli-open.png` and confirmed the two-pane diff view and absence of the default Dioxus menu.
- `git diff --check` passed.
- `mdbook build docs` passed.
- A stale-path scan for `left_function.rs` and `right_function.rs` across `docs`, `README.md`, `ROADMAP.md`, `tests`, and `crates` found no matches.

Request-provided but not reobserved during this review:

- `niri msg -j windows` showing ForskScope window id `60`.
- `niri msg action screenshot-window --id 60 --path ...`.

## Missing Evidence

- No keyboard navigation, hunk apply, undo, save, or search workflow execution was observed in this review.
- No completed RFC-041 manual smoke pass was observed.
- No macOS runtime/package verification was observed.
- No Windows runtime/package verification was observed.
- No live GitHub Actions run was observed.

## Recommended Next Action

Proceed with this documentation correction. Keep runtime/platform verification open, with the next Linux step focused on manual keyboard/apply/save workflow evidence in a real desktop session, then macOS and Windows package/runtime verification or explicit release-owner waivers.
