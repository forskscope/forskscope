# Platform Evidence — `linux-x11`

**Artifact filename:** `forskscope-v0.167.0-linux-x86_64.tar.gz`
**SHA-256:** `e17baa26abbb91e5e8e046d3812b08203f0d1ddfd6f8dc9fb9182326ed04bf09`
**Source commit:** `cb6f5b6`
**Test date (UTC):** 2026-08-14 / 2026-08-15 / 2026-08-16
**Tester role:** implementer (M5-A/B/C), automated via CI + one manual sub-check on a real host
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

## M5-C — P03, P07, P11

| Case | Result | Evidence |
|---|---|---|
| P03 — Visual/navigation (full-width rows, action alignment, geometry, horizontal scroll mirror, word wrap) | **Pass** | CI run [`31937038496`](https://github.com/forskscope/forskscope/actions/runs/31937038496) |
| P07 — Explorer and directory report (status classification, filters, batch copy, navigation) | **Pass** | CI run [`31937164577`](https://github.com/forskscope/forskscope/actions/runs/31937164577) |
| P11 — Keyboard interface (CI-verifiable sub-item only — see below) | **Pass** | CI run [`31937229160`](https://github.com/forskscope/forskscope/actions/runs/31937229160) |

Harness: `packaging/evidence/linux_harness.py` (same file as M5-A/M5-B),
same `workflow_dispatch` mechanism.

### P03 — five sub-checks, one required a fallback chain

1. **Full-width rows** — every row in a pane reports the same on-screen
   width regardless of its own text length, checked against
   `left_all_hunk_kinds.txt`/`right_*` (a real mix of short and long
   lines, not a synthetic one-line fixture).
2. **Action-row alignment** — every "Use this change" button's y-center
   falls within one row-height of a real left-pane row's y-center and a
   real right-pane row's. The act column's own rows are not exposed as
   `role="row"` at all (confirmed by direct inspection), so this checks
   the actionable buttons themselves instead.
3. **Vertical/geometry alignment** — F34's `check_pane` (child-count and
   content-cell x-origin), reused directly against a genuine multi-hunk
   fixture.
4. **Horizontal scroll mirroring, with settling** — real X11 wheel-scroll
   synthesis on the left pane, then the right pane's content-cell x is
   sampled three times over a settling window and required to have moved
   and be stable across all three samples. **Needed a fallback chain, not
   one assumed method**: button-7 (the GTK horizontal-wheel convention)
   silently did nothing on Xvfb's default virtual pointer (commonly only
   5 buttons defined, swallowing 6/7 with no error) — CI run
   [`31936716874`](https://github.com/forskscope/forskscope/actions/runs/31936716874)
   confirmed this with a real "did not move" result, not a guess.
   `shift+button-4` (shifted vertical wheel, the more portable
   convention) is what actually works; the harness now tries all three in
   order and records which one moved the pane. This sandbox's own local
   X11 input synthesis is confirmed broken for everything (even a plain
   vertical scroll no-ops here), so none of this could be validated
   locally — every iteration here needed a real CI dispatch.
5. **Word wrap** — toggling wrap on does not lose the 7-row shape.

`--break`: sub-check 3 requires the impossible baseline x from
`inject_geometry_defect.py`'s own falsifiability convention.

### P07 — status classification via the summary line, batch copy verified against real files

`DeepCompareView`'s own summary line ("N different · N equal · N left
only · N right only") is checked for an exact match against a real
directory pair (one file each of Equal, Changed, LeftOnly, RightOnly) —
`DeepRow`'s row `div`s carry no explicit `role="row"`, so individual rows
aren't AT-SPI-readable the way the diff view's are; the aggregate summary
line is. Filter buttons ("Different"/"All"/"Equal only") are each clicked
and confirmed not to break the view. Batch copy ("Copy to right N") is
verified against the actual files on disk, the actual `.bak` backup's
byte-for-byte content, and the actual manifest JSON (two `Copied`
outcomes, exactly one with a non-null `backup_path`) — not just that the
operation reported success (F62's lesson). A light navigation-history
check confirms the Up/Home buttons are clickable via a real `do_action`
return.

**Two harness bugs found and fixed by CI itself**, not by local testing
(this sandbox's own X11 synthesis cannot validate any of this):

- `navigate_pane_to`'s "Edit path" (✎) button lookup is a single
  unretried tree walk, and the window registers on the a11y bus before
  the WebView paints — the F57 race, a new symptom ("expected 2 buttons,
  found 0" instead of a silently-partial row set). P07 launches straight
  into the Explorer view (no tab click needed) but was missing the same
  readiness poll P06 (`navigate_pane_to`'s other caller) already has.
- The Up/Home buttons (`dir_pane.rs`) carry only a `title` tooltip
  ("Go up one directory"/"Home directory"), never an `aria_label` — their
  real AT-SPI accessible name is their glyph text content ("↑"/"⌂"), the
  same pattern already established for this exact PathBar's Edit-path
  button ("✎", not "Edit path"). The harness searched for the tooltip
  strings and never found them.

A third bug, found via `--break` mode's long retry search for an
impossible summary line: `find_text_containing` crashed on a stale AT-SPI
node mid-render-mutation (`GLib.GError: The application no longer
exists`) — the same class of race as `render_check.py`'s fix below, never
hit here before because no other caller polls this function as long as an
impossible-string search does.

"Focused-pane keyboard behaviour" (arrow-key navigation within Explorer)
is not exercised — no accessibility API can synthesize the needed
keystrokes; recorded as manual-outstanding, mirroring F45's shape.

### P11 — the one RFC-078 keyboard item this program can verify without synthesizing keystrokes

RFC-078's keyboard coverage decomposes into four items. Three need real
keyboard input no accessibility action can produce (the manual checklist,
global shortcuts staying inert behind an open modal, Escape closing a
modal) — recorded as manual-outstanding, mirroring F45's shape, not
attempted here. The fourth — **a destructive operation's confirmation
modal starts focus on the safe/cancel control, not the destructive one**
— is CI-verifiable with no input synthesis at all: `autofocus: true` sets
DOM focus on mount, and reading AT-SPI's own `FOCUSED` state observes
that real post-mount outcome directly.

Applies a hunk via "Use this change" to genuinely dirty a tab (what
`toolbar.rs` actually gates `ConfirmSwap` behind, not a stub), opens
"Swap sides" through the advanced disclosure panel, then reads `FOCUSED`
on the modal's Cancel and "Discard and Swap" controls: Cancel is focused,
the destructive control is not.

### Harness hardening found by this row, benefiting every case

CI run [`31936218182`](https://github.com/forskscope/forskscope/actions/runs/31936218182)
(P03's first dispatch) crashed with an uncaught `GLib.GError` inside
`render_check.py`'s `find_by_role`, called from every case's
`wait_for_ready`. F57 already handles a tree caught mid-*render* (a
partial-but-consistent row set, fixed by waiting for the pinned shape)
but not a tree caught mid-*mutation*: a proxy for a node the DOM has
already torn down raises `GError` the instant any walk touches it,
crashing the whole script instead of retrying like every other
not-ready-yet state `wait_for_ready` already tolerated. `find_by_role`
and `collect_rows` now treat a `GError`'d node as simply absent from that
walk, letting the existing 0.5s poll loop retry once the tree settles —
this is shared infrastructure, so every case in every M5 slice benefits
from this fix retroactively, not just M5-C's own three.

## M5-C falsifiability

| Case | Break-mode run | Result |
|---|---|---|
| P03 | [`31937227134`](https://github.com/forskscope/forskscope/actions/runs/31937227134) | Fail (expected) — `check_pane` found no misalignment against an unmodified build, required a mismatch that cannot be real |
| P07 | [`31937314306`](https://github.com/forskscope/forskscope/actions/runs/31937314306) | Fail (expected) — deep-compare summary line never showed the required impossible counts |
| P11 | [`31937230438`](https://github.com/forskscope/forskscope/actions/runs/31937230438) | Fail (expected) — modal focus state required the destructive control focused, which is false against the real build (Cancel is focused) |

## M5-C failures and issue links

None — no new product defects found in this row. (F61's product defect
from M5-B carries forward unchanged; P12 needs re-running against a new
candidate build once the real fix, already merged to `main`, is in a
published artifact — see ROADMAP.md.)

## M5-C waivers

None.
