# Release Evidence — 0.167.0 (M5-A)

**Candidate:** `0.167.0`, published 2026-08-14, source commit `cb6f5b6`.
**Slice:** M5-A — P01 (Install and cold launch), P02 (CLI file compare),
P09 (Mergetool), P10 (Binary/XLSX fail-closed policy) only. P03–P08,
P11–P12 and the two owner manual passes are later M5 slices; see
`matrix-plan.md` for the full frozen plan this run executes against.

## Verdict for this slice

**This evidence cannot support a Gate D pass, and nothing below changes
that.** F44 fails Linux P01 un-waivably on a supported platform
(libxdo-4 distributions) — reproduced directly on a real host, not
simulated. RFC-078's Waiver policy has no waiver for "inability to launch
on a claimed supported platform," and `matrix-plan.md`/review 061 §3.1
already settled this as a schedule dependency on an upstream
`dioxus-desktop` release, not something any further evidence-gathering
closes. While F44 is open, v1 cannot go regardless of how clean the rest
of this evidence is.

**Within that constraint, every case this slice covers passes on every
CI-verified row.** This slice's job was to gather accurate evidence, not
to produce a passing matrix — see the M5-A handoff §1: "A case that
fails, recorded accurately with its cause, is a successful outcome for
this milestone." Three further findings surfaced along the way, all
recorded rather than fixed or laundered, per the handoff's explicit
constraint:

- **F45** (already known, confirmed structurally invisible to CI as
  expected) — Windows P01's "prerequisites missing" sub-case cannot run
  on `windows-latest`, which already ships both dependencies. Owner-
  executed, outstanding.
- **F46** (already known, confirmed structurally invisible to CI as
  expected) — macOS Gatekeeper cannot engage against a `gh release
  download`ed artifact; no manual macOS host exists to test it for real.
  Recorded as Blocked, not Pass.
- **F59** (new, found while building this slice) — `installation.md`'s
  documented Debian/Ubuntu runtime prerequisites are missing `libxdo3`,
  distinct from F44. A one-line documentation fix, not a schedule
  dependency; registered, and — now unblocked per review 063 — fixed in
  `installation.md` directly.
- **F60** (found by review 063, not by this slice): the declared Windows
  floor (`AppxManifest.xml`'s `MinVersion` 1809, matching
  `installation.md`) has no runtime evidence and none is planned — the
  `windows-10` row's CI host is NT 10.0.26100, seven years newer, and no
  Windows 10 host exists anywhere in the execution model. Not a defect in
  this slice; a Gate D input the owner needs to see before the go/no-go,
  not discover while reading a verdict.

## Rows in this evidence set

| Row | Verification method (per `matrix-plan.md`) | This slice's cases | File |
|---|---|---|---|
| `linux-x11` | CI (`ubuntu-latest` + Xvfb) | P01 (two sub-results), P02, P09, P10 — all Pass on CI | `linux-x11.md` |
| `linux-wayland` | Manual (owner) | Not executed in this slice | `linux-wayland.md` |
| `windows-11` | CI (`windows-latest`), F45 sub-case manual | P01 (CI Pass; F45 sub-case outstanding), P02, P09, P10 — all Pass | `windows-11.md` |
| `windows-10` | CI (`windows-latest`, same host as `windows-11`) | Same four cases, same runs, per the plan's own text | `windows-10.md` |
| `macos-aarch64` | CI (`macos-latest`); F46 unverifiable at all | P01, P02, P09, P10 — all Pass; Gatekeeper Blocked | `macos-aarch64.md` |

## Harnesses

Each CI row has its own on-demand `workflow_dispatch` entry point,
matching the shape F34's `render-check.yml` established — every case
runnable without a tag, and every case has a `--break` mode proving the
assertion isn't vacuous (handoff §7; all 24 combinations — 4 cases × 2
modes × 3 CI rows — were run for real and confirmed, not just written and
trusted):

- **Linux:** `packaging/evidence/linux_harness.py` +
  `.github/workflows/m5-evidence-linux.yml`. AT-SPI (`Atspi.Action.do_action`)
  for button invocation — no synthetic X11 input events, after several
  approaches that don't work reliably under a bare Xvfb display with no
  window manager (see the commit history on this file for exactly what
  didn't work and why).
- **Windows:** `packaging/evidence/windows_harness.py` +
  `.github/workflows/m5-evidence-windows.yml`. `pywinauto`'s UIA Invoke
  pattern — the same "invoke the accessible action directly" approach as
  Linux, adapted to Windows's accessibility API.
- **macOS:** `packaging/evidence/macos_harness.py` +
  `packaging/evidence/macos_ui.applescript` +
  `.github/workflows/m5-evidence-macos.yml`. AppleScript/System Events
  (`AXPress`) — no pre-installed Python Accessibility binding on
  `macos-latest`; System Events UI scripting worked with no permission
  wall on every run in this pass, though that isn't proven robust against
  a future runner-image change (see `macos-aarch64.md`).

All three harnesses download and digest-verify the **published** 0.167.0
artifact before every case — see `artifacts.md`. None were built from
source.

## Known limitations of this slice specifically

- `linux-wayland` (the owner's manual row) was not executed — outside
  what CI automation can cover.
- F45's and F46's manual/unverifiable sub-cases remain exactly as open as
  `matrix-plan.md` already recorded before this slice; nothing here
  changes their disposition, only confirms the CI-observable portions
  around them.
- P03–P08, P11, P12 are later M5 slices, per the M5-A handoff's explicit
  scope boundary — not started here.
- **Dependency for M5-C, not a limitation here (review 063 §5.3):**
  Windows's P02 readiness check uses content-token presence, not an
  exact row count like Linux's. Sufficient for P02; P03 (compare layout
  and scrolling) needs row-level precision the same way F34 does on
  Linux, so the UIA control-type mapping this slice didn't need to
  establish should be settled before M5-C's P03 work starts — see
  `windows-11.md`.
