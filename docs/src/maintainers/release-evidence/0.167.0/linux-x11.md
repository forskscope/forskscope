# Platform Evidence — `linux-x11`

**Artifact filename:** `forskscope-v0.167.0-linux-x86_64.tar.gz`
**SHA-256:** `e17baa26abbb91e5e8e046d3812b08203f0d1ddfd6f8dc9fb9182326ed04bf09`
**Source commit:** `cb6f5b6`
**Test date (UTC):** 2026-08-14 / 2026-08-15
**Tester role:** implementer (M5-A), automated via CI + one manual sub-check on a real host
**Host OS and version:** Ubuntu 24.04.4 LTS — **resolved runner image**
`ubuntu-24.04`, image version `20260810.271.1` (recorded per review 057
§4.3's rolling-label caveat: `ubuntu-latest` itself is not reproducible,
this resolved image is)
**Architecture:** x86_64
**Display server / WebView runtime:** Xvfb (X11-family virtual
framebuffer, `xvfb-run --auto-servernum`) + WebKitGTK 4.1, per
`matrix-plan.md`'s stated stand-in for this row
**Install source and prerequisites:** published GitHub Release asset,
downloaded and digest-verified by CI before every case run; see
"Prerequisite finding" below for exactly what was and wasn't required

## Cases

| Case | Result | Evidence |
|---|---|---|
| P01 — Install and cold launch | **Pass** (CI, ubuntu-latest) / **Fail** (a real libxdo-4 host — see below) | See "P01" and "F44" below |
| P02 — CLI file compare | **Pass** | CI run [`31852380216`](https://github.com/forskscope/forskscope/actions/runs/31852380216) |
| P09 — Mergetool | **Pass** | CI run [`31852248989`](https://github.com/forskscope/forskscope/actions/runs/31852248989) |
| P10 — Binary/XLSX fail-closed policy | **Pass** | CI run [`31852396676`](https://github.com/forskscope/forskscope/actions/runs/31852396676) |

Harness: `packaging/evidence/linux_harness.py`, driven by
`.github/workflows/m5-evidence-linux.yml` (`workflow_dispatch`, one case
per run). Every case downloads and digest-verifies the **published**
artifact itself — nothing here was built from source.

### P01 — two distinct sub-results, not one

**On `ubuntu-latest` (Debian/Ubuntu-family, CI):** Pass, but only after
installing an undocumented prerequisite — see F59 below.
`--diagnostics` reported `ForskScope 0.167.0` with a redacted home path; a
plain cold launch produced a real, non-blank 1180×760 frame. CI run
[`31852372407`](https://github.com/forskscope/forskscope/actions/runs/31852372407).

**On a real libxdo-4 host (F44): Fail, exactly as expected.** Reproduced
directly on a real, currently-used Arch-family Linux desktop (not a CI
container) — the implementer's own development machine, sanitized per
RFC-078's schema (no home paths, hostnames, or other host-identifying
detail beyond what's needed to establish the distribution family):

```text
Host OS: CachyOS Linux (Arch-family, ID_LIKE=arch), rolling release
$ ./forskscope --diagnostics
./forskscope: error while loading shared libraries: libxdo.so.3: cannot
open shared object file: No such file or directory
$ pacman -Q xdotool
xdotool 4.20260303.1-1.1   (provides /usr/lib/libxdo.so.4, not .so.3)
$ readelf -d ./forskscope | grep libxdo
 0x0000000000000001 (NEEDED)  Shared library: [libxdo.so.3]
```

Per handoff §4 and `matrix-plan.md` §3: because Linux support is
confirmed unqualified (no per-distribution floor), this is **not**
satisfied by the CI pass above — a libxdo-4 distribution is a supported
platform, and this failure is recorded as the real, expected P01 outcome
for it. Not waivable (RFC-078's Waiver policy: "inability to launch on a
claimed supported platform"). Tracked as **F44**, un-waivable per review
061 §3.1 — this is a schedule dependency on the upstream `dioxus-desktop`
release (`DioxusLabs/dioxus#5749`, merged, not yet released), not
something this evidence pass can close.

### Prerequisite finding (F59) — recorded, not silently worked around

A fresh `ubuntu-latest` host with **exactly** `installation.md`'s
documented runtime prerequisites (`libwebkit2gtk-4.1-0`, `libgtk-3-0`)
installed — nothing else — **cannot launch the binary at all**:

```text
$ ./forskscope --diagnostics
./forskscope: error while loading shared libraries: libxdo.so.3: cannot
open shared object file: No such file or directory
```

CI runs [`31850560177`](https://github.com/forskscope/forskscope/actions/runs/31850560177)
and [`31850683293`](https://github.com/forskscope/forskscope/actions/runs/31850683293)
confirm this directly, plus a dedicated evidence step in
`m5-evidence-linux.yml` that attempts the launch against documented-only
prerequisites on every subsequent run before installing anything else.
`libxdo3` (confirmed via `apt-cache search libxdo`) is a real, separate,
installable package neither `libwebkit2gtk-4.1-0` nor `libgtk-3-0` pulls
in transitively, and it is not mentioned anywhere in the docs. Distinct
from F44 (a libxdo.so.**4** distribution family issue with no simple fix)
— this is libxdo.so.**3** itself missing from a supposedly-compatible
Debian/Ubuntu host, fixable with one documentation line. Registered as
**F59**, not fixed in this evidence-gathering pass per the M5-A handoff's
explicit constraint. The P01 "Pass" above was obtained only after
installing `libxdo3` in addition to the documented set — a real user
following the current docs exactly does not get this.

## Falsifiability

Every case's `--break` mode was run and confirmed to fail for the
expected reason, per handoff §7:

| Case | Break-mode run | Result |
|---|---|---|
| P01 | [`31852376324`](https://github.com/forskscope/forskscope/actions/runs/31852376324) | Fail — `--diagnostics output does not start with 'ForskScope 0.0.0-impossible': ForskScope 0.167.0` |
| P02 | [`31852384290`](https://github.com/forskscope/forskscope/actions/runs/31852384290) | Fail — `compare view did not reach 99 rows per pane within 45s` |
| P09 | [`31852312861`](https://github.com/forskscope/forskscope/actions/runs/31852312861) | Fail — content check correctly rejected a value real Save output can never produce |
| P10 | [`31852400902`](https://github.com/forskscope/forskscope/actions/runs/31852400902) | Fail — `could not find text containing 'this message cannot appear' within 45s` |

## Failures and issue links

- **F44** — Linux P01 fails on libxdo-4 distributions. Un-waivable
  (review 061 §3.1). Schedule dependency on an upstream `dioxus-desktop`
  release.
- **F59** — `installation.md`'s documented Debian/Ubuntu prerequisites
  are missing `libxdo3`. Not a schedule dependency; a one-line doc fix.

## Waivers

None. F44 is explicitly not waivable per RFC-078's Waiver policy.

## M5-B — P04, P05, P06, P08, P12

**Test date (UTC):** 2026-08-15. Same published `0.167.0` artifact and
digest as M5-A above; no new download, no source build. Harness extended
in place (`packaging/evidence/linux_harness.py`, same
`m5-evidence-linux.yml` dispatch entry point) — see the M5-B review
request for the full account of what each case's mechanism turned out to
require and why.

### §3 — P04's "keyboard and mouse"

Resolved before any evidence was gathered under it (handoff §3 required
this). The mouse path — the "Use this change" button — is invoked via
`Atspi.Action.do_action`, the same accessibility-action route M5-A
established for Save/Undo/Redo; this is not laundering, since it fires
the identical `onclick` handler a real click would. The keyboard path
(`app.rs`'s global `Key::Enter` listener) is architecturally different: a
raw `onkeydown` handler bound to no actionable UI element, so **no
accessibility action exists to invoke it on any platform** — this is not
the same problem M5-A had with Ctrl+S (where the delivery mechanism
existed but was unreliable). Recorded as not-executed/manual-outstanding,
mirroring F45's existing shape for P01's prerequisite sub-case, not
amended into RFC-078's text — this narrows CI-verification scope the same
way F45 already does, not the case's actual requirement.

| Case | Result | Evidence |
|---|---|---|
| P04 — Apply/Undo/Redo/Save (mouse path only; keyboard: not executed, see §3 above) | **Pass** | CI run [`31877812540`](https://github.com/forskscope/forskscope/actions/runs/31877812540) |
| P05 — External modification (Cancel/Overwrite/Save As) | **Pass** | CI run [`31878357564`](https://github.com/forskscope/forskscope/actions/runs/31878357564) |
| P06 — Async identity | **Pass** | CI run [`31912063479`](https://github.com/forskscope/forskscope/actions/runs/31912063479) |
| P08 — Persistence migration (legacy v0, Exit, Continue, Reset) | **Pass** | CI run [`31879673483`](https://github.com/forskscope/forskscope/actions/runs/31879673483) |
| P12 — Session/settings restart | **Fail — real product defect, see F61** | CI run [`31894750441`](https://github.com/forskscope/forskscope/actions/runs/31894750441) |

### P06 — falsifiability risk addressed (handoff §6)

The handoff flagged "a check that passes whenever two tabs merely exist"
as a vacuous-pass risk. The harness opens two genuinely large (20,000-line)
synthetic fixture pairs with distinct content markers, closes the
lower-index tab while it may still be loading (exercising RFC-065's
dirty-check bypass for loading tabs), and asserts the surviving tab shows
the *other* pair's marker specifically — the break-mode run below requires
the closed tab's marker instead, and correctly fails to find it.

Two mechanism notes, not defects: Explorer's file-tree rows (`role="row"`)
fire their real `on_select` handler reliably via `Atspi.Action.do_action`;
the permanent "Explorer" tab (a plain `div` with no ARIA role) does not,
so switching to it uses a real X11 click instead — a third, distinct
instance of the shadow-AT-SPI-state family P12 also hit (below). Separately,
reloading the 20,000-line fixture twice in quick succession occasionally
produced a transient `GLib.GError: "The application no longer exists"` —
confirmed via `proc.poll()` (polled for up to 2s, not checked once) that
the *harness's own launched process* stayed alive throughout; WebKitGTK
runs actual web content in a separate process from the one launched, and
that content process appears to drop off the accessibility bus briefly
under load without the parent exiting. The harness retries through this
rather than treating it as fatal.

### P12 — falsifiability, and a real product defect (F61)

Sub-test 1 (theme/language restore) is genuinely verified via UI reads,
but its *change* half is seeded directly rather than driven through the
Settings dialog's `<select>` elements — three distinct mechanisms
(`Atspi.Selection.select_child`, `Atspi.Action.do_action` on the menu
item, and a real X11 click+arrow+Enter on the combo's native popup) were
tried and none reliably changed the value under this harness's
bare-Xvfb-no-window-manager CI environment; the first two report success
and even flip the AT-SPI SELECTED state without firing the real Dioxus
`onchange` handler, and the third either had no effect with the process
alive, or (see P06 above) coincided with the same transient AT-SPI drop.
Documented as a real, reportable limitation rather than silently worked
around — see `packaging/evidence/linux_harness.py`'s `p12` docstring for
the full account. The *restore* half is unaffected and is what the
break-mode run below demonstrates.

Sub-test 2 (tab restore across a CLI-launched-then-terminated session)
fails for real: `forskscope <left> <right>`, terminate, relaunch with no
args does not restore the tab. This is not a harness defect — Windows and
macOS's independent M5-B harnesses hit the identical failure via entirely
different code, and macOS's harness traced the actual cause: `app.rs`'s
reactive `use_effect` on tab changes never fires a real `session.json`
write for a CLI-opened tab, while an explicit call site like `close_tab`
does persist correctly. Registered as **F61**, not fixed (M5-B's
constraints forbid product changes).

## M5-B falsifiability

| Case | Break-mode run | Result |
|---|---|---|
| P04 | [`31878367904`](https://github.com/forskscope/forskscope/actions/runs/31878367904) | Fail — saved content hash did not equal the required impossible value |
| P05 | [`31878360350`](https://github.com/forskscope/forskscope/actions/runs/31878360350) | Fail — `.bak` content did not equal a value that can never be the real backup |
| P06 | [`31911967700`](https://github.com/forskscope/forskscope/actions/runs/31911967700) | Fail — surviving tab did not show the closed (impossible) tab's marker |
| P08 | [`31879245189`](https://github.com/forskscope/forskscope/actions/runs/31879245189) | Fail — required the process to still be running after Exit, which it correctly is not |
| P12 | [`31894752487`](https://github.com/forskscope/forskscope/actions/runs/31894752487) | Fail — restored Language selection correctly read back as `'日本語'`, not the impossible required value |

## M5-B failures and issue links

- **F61** — a tab opened via CLI startup args is not reliably persisted
  to `session.json`, confirmed independently on Linux, Windows, and
  macOS. Not fixed here; registered for owner triage before Gate D.

## M5-B waivers

None.
