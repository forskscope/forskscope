# Release Evidence — 0.167.0 (M5-A + M5-B + M5-C)

**Candidate:** `0.167.0`, published 2026-08-14, source commit `cb6f5b6`.
**Slices:** M5-A — P01 (Install and cold launch), P02 (CLI file compare),
P09 (Mergetool), P10 (Binary/XLSX fail-closed policy). M5-B — P04
(Merge/undo/redo/safe save), P05 (External modification), P06 (Async
identity), P08 (Persistence migration), P12 (Session/settings restart).
M5-C — P03 (Visual/navigation), P07 (Explorer and directory report), P11
(Keyboard interface, CI-verifiable item only). All three slices' CI-verified
cases are now complete on every automated row; the two owner manual passes
(`linux-wayland`, and the manual sub-cases within F45/F46) remain outstanding
— see the **Gate D input list** (`gate-d-input-list.md`) for their status.
See `matrix-plan.md` for the full frozen plan this run executes against.

## Verdict for this evidence

**This evidence cannot support a Gate D pass, and nothing below changes
that — for three independent, un-waivable reasons.**

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
desktop during review 064. **Fixed on `main` for real** (review 066,
2026-08-16) — but the fix is **not in the published `0.167.0` candidate**;
a new candidate build is required before M5's P12 rows can be re-run
against it and this input clears.

**F73** — `DeepRow`'s per-row copy buttons in the Directory Report silently
write to the wrong location: they source their destination from Explorer's
remembered pane directory instead of the deep-compare view's actual compare
root, with no error surfaced. Confirmed with a real backup and overwrite,
not a reported "success" — this is **wrong-file/stale-load behavior**, one
of the same five un-waivable categories F61 falls under. Found
**independently on both Windows and macOS** during M5-C's P07 work
(review 068 §4) — the same underlying code, not a platform quirk. **Not
fixed in this candidate.** Shares its root cause with F68, so one fix
(passing the compare roots into `DeepRow`) closes both.

**While any of the three is open, v1 cannot go regardless of how clean the
rest of this evidence is.** This is not a gap in M5's evidence-gathering —
per the M5-A handoff §1, a case or input that fails, recorded accurately
with its cause, is a successful outcome for this milestone.

**Within that constraint, every CI-verified case across all three slices
passes on every row it was exercised on, except:**

- **P12** (Session/settings restart) — fails on every row, for real,
  because of F61 (Windows's P12 row initially read Pass by seeding around
  the CLI-launch path rather than exercising it; corrected to Fail per
  review 064 §5.4, since a passing row would misrepresent a cross-platform
  defect as platform-specific).
- **P07** (Explorer and directory report) — fails outright on Windows
  because of F70 (Explorer never renders a single directory row on that
  CI environment — product defect or CI-environment limitation is
  undetermined). Passes as a case on Linux and macOS, where the assertions
  genuinely hold — F68/F72/F73 are registered separately from that Pass,
  not folded into it, per review 068 §4's distinction between what a case
  result measures and what a finding means for Gate D.
- **P11** (Keyboard interface, CI-verifiable item only) — fails on Windows
  because of F69 (a destructive modal's `autofocus` never moves keyboard
  focus into the modal at all on WebView2; Linux/WebKitGTK is confirmed
  correct for the same modal pattern). Passes on Linux and macOS.

**The full list of everything bearing on the go/no-go — the three blockers
above plus every other open input, each with its current status — is the
[Gate D input list](gate-d-input-list.md).** That is the document Gate D is
actually assessed against; this section is a summary, not a substitute for
it.

## Rows in this evidence set

| Row | Verification method (per `matrix-plan.md`) | M5-A cases | M5-B cases | M5-C cases | File |
|---|---|---|---|---|---|
| `linux-x11` | CI (`ubuntu-latest` + Xvfb) | P01 (two sub-results), P02, P09, P10 — all Pass | P04, P05, P06, P08 — Pass; P12 — **Fail (F61)** | P03, P07, P11 — all Pass | `linux-x11.md` |
| `linux-wayland` | Manual (owner) | Not executed | Not executed | Not executed | `linux-wayland.md` |
| `windows-11` | CI (`windows-latest`), F45 sub-case manual | P01 (CI Pass; F45 sub-case outstanding), P02, P09, P10 — all Pass | P04, P05, P06, P08 — Pass; P12 — **Fail (F61), real path not exercised** | P03 — Pass; P07 — **Fail (F70)**; P11 — **Fail (F69)** | `windows-11.md` |
| `windows-10` | CI (`windows-latest`, same host as `windows-11`) | Same four cases, same runs | Same five cases, same runs | Same three cases, same runs | `windows-10.md` |
| `macos-aarch64` | CI (`macos-latest`); F46 unverifiable at all | P01, P02, P09, P10 — all Pass; Gatekeeper Blocked | P04, P05, P08 — Pass; P06 — Pass (reduced scope, see below); P12 — **Fail (F61)** | P03, P07, P11 — all Pass (P07 carries F68/F72/F73, registered not fixed) | `macos-aarch64.md` |

## Harnesses

Each CI row has its own on-demand `workflow_dispatch` entry point,
matching the shape F34's `render-check.yml` established — every case
runnable without a tag, and every case has a `--break` mode proving the
assertion isn't vacuous. Across M5-A/B/C, every case's normal and `--break`
modes were run for real and their results read from the actual CI run log —
none inferred from reading harness code alone. Two exceptions on Windows,
both noted in their own row file: **P07's `--break` is not reached at all**
(the case fails before any `--break`-gated assertion, on the same F70
initial-listing blocker as normal mode); **P11's `--break` fails for the
same real-defect reason as normal mode**, not its own impossible-value
branch, so it cannot currently demonstrate falsifiability in isolation
while F69 persists.

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
- **The documented keyboard interface has no automated runtime coverage on
  any platform** — P04's Enter-apply path, P06's double-reload, and three
  of P11's four items all need a real synthesized keystroke no
  accessibility API on any of the three platforms can produce. See the
  Gate D input list's "Keyboard interface" row for the full statement.
- macOS's P06 runs a reduced (sequential, non-overlapping) scope, not
  RFC-078's concurrent design. **This is now known to be unnecessary**
  (F63 closed as a harness artifact, review 068 §3) but was not retrofitted
  here — out of scope for M5-C, and `matrix-plan.md`'s freeze does not
  reopen a completed row mid-matrix.
- **Linux's local development sandbox cannot validate any X11-dependent
  harness behavior** — even a plain vertical mouse-wheel scroll no-ops
  there. Every M5-C Linux harness fix needed a real CI round-trip to
  confirm; nothing here was iterated locally first. Worth knowing before
  attempting to reproduce a Linux harness failure outside CI.
- **P03's horizontal-scroll-mirror check on Linux tries three X11 wheel
  conventions in sequence** (button-7, then two shift-modified fallbacks)
  and records whichever one moved the pane, rather than asserting one in
  advance — button-7 (the GTK convention) was confirmed silently swallowed
  by Xvfb's default virtual pointer on the first real dispatch. Whether the
  working method stays stable across future CI runner-image changes is not
  yet known.
- **P03's horizontal-scroll-mirror check is not executed on macOS at all**
  — a disclosed, evidenced platform/technique limitation (no
  accessibility-exposed scroll-position property was found for this
  content on macOS), not a skipped check.
- **The right Explorer pane's controls do not respond to macOS's
  `click`-technique for navigation** — a harness/technique limitation, not
  confirmed as a product defect; see `macos-aarch64.md`'s Finding 3 for the
  open question this leaves about real VoiceOver operability.
