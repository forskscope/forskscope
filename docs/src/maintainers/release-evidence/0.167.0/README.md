# Release Evidence — 0.167.0 (M5-A + M5-B)

**Candidate:** `0.167.0`, published 2026-08-14, source commit `cb6f5b6`.
**Slices:** M5-A — P01 (Install and cold launch), P02 (CLI file compare),
P09 (Mergetool), P10 (Binary/XLSX fail-closed policy). M5-B — P04
(Merge/undo/redo/safe save), P05 (External modification), P06 (Async
identity), P08 (Persistence migration), P12 (Session/settings restart).
P03, P07, P11, and the two owner manual passes are M5-C, not started
here; see `matrix-plan.md` for the full frozen plan this run executes
against.

## Verdict for this evidence

**This evidence cannot support a Gate D pass, and nothing below changes
that — for two independent, un-waivable reasons.**

**F44** fails Linux P01 un-waivably on a supported platform (libxdo-4
distributions) — reproduced directly on a real host, not simulated.
RFC-078's Waiver policy has no waiver for "inability to launch on a
claimed supported platform," and `matrix-plan.md`/review 061 §3.1 already
settled this as a schedule dependency on an upstream `dioxus-desktop`
release, not something any further evidence-gathering closes.

**F61** — a tab opened via CLI startup args is not reliably persisted to
`session.json` (`forskscope <left> <right>`, the ordinary `git difftool`
workflow, silently loses that session on the next launch) — is **silent
settings/session loss**, one of the five things RFC-078's Waiver policy
names explicitly as un-waivable into a release pass. Confirmed
independently by three genuinely different M5-B harness implementations
(Linux, Windows, macOS) and reproduced a fourth time directly on a real
desktop during review 064. Unlike F44, **this is a defect this project
can fix** — it is not a schedule dependency on anything external, and its
fix is now on the v1 critical path as its own slice (review 064 §7).

While either is open, v1 cannot go regardless of how clean the rest of
this evidence is.

**Within that constraint, every case both slices cover passes on every
CI-verified row it was exercised on, except P12 — which fails for real,
on every row, because of F61** (Windows's P12 row initially read Pass by
seeding around the CLI-launch path rather than exercising it; corrected
to Fail per review 064 §5.4, since a passing row would misrepresent a
cross-platform defect as platform-specific). Recording an accurately-
failing case is a successful outcome for this program, not a gap in it —
see the M5-A handoff §1: "A case that fails, recorded accurately with its
cause, is a successful outcome for this milestone." Further findings
surfaced along the way, all recorded rather than fixed or laundered, per
both slices' explicit constraint:

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
- **F61** (new, found independently by all three M5-B harnesses,
  reproduced a fourth time in review 064) — see the verdict above. A
  second un-waivable Gate D blocker, and one consequence worth stating
  plainly: `README.md`'s own top-level feature list claims "Session
  persistence — open tabs are restored on next launch," which is false
  for a CLI-opened tab. F16's earlier feature-claim audit passed all
  seventeen bullets correctly — its method (auditing against the UI
  crate) could not have caught a claim that holds on one path and fails
  on another; not a mistake in that slice, a real limit of it. Also
  found in the same area: `persist_session`/`persist_settings` discard
  their save `Result` with `let _ =`, making any write failure —
  F61's or a future one — silent by construction; tracked separately
  (see `ROADMAP.md`).
- **Enter-to-apply has no automated runtime coverage on any platform**
  (review 064 §5.1) — P04's keyboard shortcut (`app.rs`'s global
  `Key::Enter` handler) is bound to no actionable UI element, so no
  accessibility API on any platform has anything to invoke for it. The
  mouse path is fully CI-verified on all three rows; the keyboard path is
  manual-outstanding, mirroring F45's shape. Keyboard operability is an
  accessibility claim this project makes, so this belongs among the Gate
  D inputs, not only in each row's fine print.
- **macOS's P06 fixture-size accessibility question is unresolved, not
  dismissed** (review 064 §5.3): building macOS's M5-B evidence found
  that a generated diff pair's content stops reaching the accessibility
  tree once the file crosses a size threshold between 30 and 100 lines,
  which is why P06 there runs a reduced (sequential, non-overlapping)
  scope instead of RFC-078's concurrent design. Whether this is a real
  macOS accessibility defect in the product (the class RFC-061/RFC-019
  exist for) or a harness artifact (a timeout, a lazily-rendered virtual
  list) is not yet known — tracked separately (see `ROADMAP.md`) so it
  doesn't stay a footnote to a fixture-sizing choice.

## Rows in this evidence set

| Row | Verification method (per `matrix-plan.md`) | M5-A cases | M5-B cases | File |
|---|---|---|---|---|
| `linux-x11` | CI (`ubuntu-latest` + Xvfb) | P01 (two sub-results), P02, P09, P10 — all Pass | P04, P05, P06, P08 — Pass; P12 — **Fail (F61)** | `linux-x11.md` |
| `linux-wayland` | Manual (owner) | Not executed | Not executed | `linux-wayland.md` |
| `windows-11` | CI (`windows-latest`), F45 sub-case manual | P01 (CI Pass; F45 sub-case outstanding), P02, P09, P10 — all Pass | P04, P05, P06, P08 — Pass; P12 — **Fail (F61), real path not exercised** | `windows-11.md` |
| `windows-10` | CI (`windows-latest`, same host as `windows-11`) | Same four cases, same runs | Same five cases, same runs | `windows-10.md` |
| `macos-aarch64` | CI (`macos-latest`); F46 unverifiable at all | P01, P02, P09, P10 — all Pass; Gatekeeper Blocked | P04, P05, P08 — Pass; P06 — Pass (reduced scope, see below); P12 — **Fail (F61)** | `macos-aarch64.md` |

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

## Known limitations of this evidence

- `linux-wayland` (the owner's manual row) was not executed — outside
  what CI automation can cover.
- F45's and F46's manual/unverifiable sub-cases remain exactly as open as
  `matrix-plan.md` already recorded before M5-A; nothing here changes
  their disposition, only confirms the CI-observable portions around
  them.
- P03, P07, P11, and the two owner manual passes are M5-C, per the
  handoffs' explicit scope boundaries — not started here.
- P04's Enter-key path (see F61's bullet above) has no automated coverage
  on any of the three CI rows — manual-outstanding, not executed.
- macOS's P06 runs a reduced (sequential, non-overlapping) scope, not
  RFC-078's concurrent design — see the fixture-size finding above.
- **Dependency for M5-C, not a limitation here (review 063 §5.3):**
  Windows's P02 readiness check uses content-token presence, not an
  exact row count like Linux's. Sufficient for P02; P03 (compare layout
  and scrolling) needs row-level precision the same way F34 does on
  Linux, so the UIA control-type mapping this slice didn't need to
  establish should be settled before M5-C's P03 work starts — see
  `windows-11.md`.
