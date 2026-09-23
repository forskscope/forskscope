# Review Request 065: M5-C Linux row — P03, P07, P11

**Governing RFC.** [RFC-078](../../rfcs/proposed/078-platform-runtime-acceptance.md), under [RFC-074](../../rfcs/proposed/074-v1-release-stabilization-program.md)
**Governing handoff.** [`rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md`](../../rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md)
**Scope of this request.** The **Linux row only** (`linux-x11`) — P03, P07, P11,
extending `packaging/evidence/linux_harness.py` and hardening the shared
`packaging/render_check.py` infrastructure every M5 row depends on. The handoff
also covers Windows, macOS, and full evidence assembly (README.md verdict, Gate
D input list, artifacts.md); Windows is already reviewed (review 067, request
064), macOS is still in progress, and assembly is deferred until both other
rows land — not this request's scope.
**Candidate.** `0.167.0`, same published artifact/digest as M5-A/M5-B.
**Baseline.** `main` at `b27545d` at handoff time; Prerequisite A (below) landed
first, before any P03/P07/P11 evidence-gathering.

## 1. Prerequisite A's outcome — resolved before any evidence was gathered

`render_check.py`'s `check_pane` has two branches: an accessible-child-count
comparison (already demonstrable via `reintroduce_f32_defect.py`) and a
content-cell x-origin comparison that review 055 tried to demonstrate twice via
CSS padding/margin mutations and never observed firing.

**Finding:** a `display:table-cell` div's own AT-SPI accessible box position
comes from the table-layout algorithm's column placement, not its CSS
padding/margin — padding moves content *inside* the cell's box, not the box
`Atspi.Component.get_extents` reports. `transform: translateX()` does move it,
since it affects the painted position directly. Confirmed empirically against a
locally-built binary: `padding-left` on `.hunk-rep .cell` produced zero change
in `extents(cell).x` across all 7 rows; `transform: translateX(20px)` shifted
exactly one row (the mutated one) while the other 6 stayed at baseline.

`packaging/inject_geometry_defect.py` (new) applies that transform to
`.hunk-rep .cell` only, wired into `render-check.yml` as a second
`workflow_dispatch` input matching `inject_f32_defect`'s existing shape —
checkout-only, never committed, never the default. Verified end-to-end locally:
injected, rebuilt, ran `render_check.py`, confirmed it correctly failed with "a
row's content starts at x=67, other rows start at x=47"; reverted, confirmed
clean pass again.

**Outcome:** F34's geometry branch is now demonstrably real, not vacuous — the
same falsifiability standard every other case in this program meets.

## 2. Cases executed, with results

| Case | Result | Evidence (CI run) |
|---|---|---|
| P03 — Visual/navigation | **Pass** | [`31937038496`](https://github.com/forskscope/forskscope/actions/runs/31937038496) |
| P07 — Explorer and directory report | **Pass** | [`31937164577`](https://github.com/forskscope/forskscope/actions/runs/31937164577) |
| P11 — Keyboard interface (CI-verifiable sub-item only) | **Pass** | [`31937229160`](https://github.com/forskscope/forskscope/actions/runs/31937229160) |

Full narrative for each case is in
`docs/src/maintainers/release-evidence/0.167.0/linux-x11.md`'s new "M5-C"
section.

**P03** — five sub-checks against `left_all_hunk_kinds.txt`/`right_*` and
`left_long_line.txt`/`right_long_line.txt`: full-width rows regardless of
content length; action-button (`Use this change`) alignment against real
rows (the act column itself is not AT-SPI-row-readable, confirmed by direct
inspection); F34's geometry check (Prerequisite A, reused directly); horizontal
scroll mirroring with a settling window; word wrap keeps the 7-row shape.

**P07** — `DeepCompareView`'s summary line checked for an exact match against a
real directory pair (Equal/Changed/LeftOnly/RightOnly, one file each); filter
buttons don't break the view; batch copy (`Copy to right N`) verified against
the actual files on disk, the actual `.bak` backup's byte-for-byte content, and
the actual manifest JSON (F62's lesson: verified state, not a reported
"success"); a light navigation-history check on the Up/Home buttons.

**P11** — RFC-078's keyboard coverage decomposes into four items; only one is
CI-verifiable without synthesizing keystrokes (see §3). Applies a hunk to
genuinely dirty a tab (what `toolbar.rs` actually gates `ConfirmSwap` behind,
not a stub), opens Swap sides, reads AT-SPI's own `FOCUSED` state on the
modal's Cancel and Discard-and-Swap controls with no input synthesized for the
focus check itself (`autofocus: true` sets DOM focus on mount; this only
observes that outcome).

## 3. P11's decomposition, and the keyboard-coverage statement

| Item | CI-verifiable? | What happened |
|---|---|---|
| (1) Keyboard checklist | **No** — manual | Not attempted; manual-outstanding, mirroring F45's shape |
| (2) Modal focus starts on safe/cancel | **Yes** — this is what P11 checks | Executed. **Pass** — Cancel focused, not the destructive control |
| (3) Global shortcuts inert behind a modal | **No** — needs a real keystroke | Not attempted; manual-outstanding |
| (4) Escape behaviour | **No** — needs a real keystroke | Not attempted; manual-outstanding |

Same structural gap M5-B already established for P04's Enter-shortcut path: no
accessibility action can invoke a global `onkeydown` listener bound to no UI
element. Consistent with M5-C Windows's own P11 decomposition (review 067),
reached independently.

**On Windows, this same item (2) check found a real defect** (F69 — focus
never moves into the modal at all on WebView2). **On Linux/WebKitGTK it
passes** — Cancel genuinely receives `FOCUSED`. Review 067 §2 independently
reproduced this exact Linux result on a real desktop as part of verifying F69,
so this row's P11 result is already externally corroborated, not merely
self-reported.

## 4. Falsifiability demonstrations

| Case | Break-mode run | Result |
|---|---|---|
| P03 | [`31937227134`](https://github.com/forskscope/forskscope/actions/runs/31937227134) | Fail (expected) — `check_pane` found no misalignment against an unmodified build, required a mismatch that cannot be real |
| P07 | [`31937314306`](https://github.com/forskscope/forskscope/actions/runs/31937314306) | Fail (expected) — deep-compare summary line never showed the required impossible counts |
| P11 | [`31937230438`](https://github.com/forskscope/forskscope/actions/runs/31937230438) | Fail (expected) — required the destructive control focused, which is false against the real build |

All six runs (3 cases × 2 modes) were dispatched for real via
`gh workflow run` and read from the actual run log via `gh run view --log` —
none inferred from reading the harness code alone.

**Horizontal scroll mirroring needed a fallback chain, not one assumed
method** — worth spelling out since it parallels Windows's own scroll-mirror
investigation (review 067 §5.2) despite using an entirely different mechanism.
CI run [`31936716874`](https://github.com/forskscope/forskscope/actions/runs/31936716874)
confirmed button-7 (the GTK horizontal-wheel convention) silently did nothing
against Xvfb's default virtual pointer (commonly only 5 buttons defined,
swallowing 6/7 with no error — not a guess, a real observed "did not move").
`shift+button-4` (shifted vertical wheel, the more portable GTK/WebKit
convention) is what actually works, confirmed on the next dispatch
([`31937038496`](https://github.com/forskscope/forskscope/actions/runs/31937038496),
`OK: ... horizontal scroll mirrors (via shift+button-5 (shifted vertical
wheel)) ...` — the harness tries all three methods in order per run and this
particular run's underlying mechanics landed on shift+button-5 specifically;
both are the same shifted-wheel family). The harness now tries button-7, then
shift+button-4, then shift+button-5 in sequence and records whichever one
moved the pane, rather than asserting one in advance. This sandbox's own local
X11 input synthesis is confirmed broken for everything (even a plain vertical
scroll no-ops here locally), so none of this could be validated except by real
CI dispatch.

## 5. Harness bugs found and fixed, all via real CI dispatch (not local)

This sandbox's local X11 synthesis is broken for every interaction (confirmed:
even a plain vertical mouse-wheel scroll no-ops here), so every one of these
was found and confirmed exclusively through real CI runs, not local
iteration:

1. **`render_check.py`'s `find_by_role`/`collect_rows` crashed on a stale
   AT-SPI node mid-render-mutation** — CI run
   [`31936218182`](https://github.com/forskscope/forskscope/actions/runs/31936218182),
   `GLib.GError: Object does not exist at path ...`. F57 already handles a
   tree caught mid-*render* (a partial-but-consistent row set, fixed by
   waiting for the pinned shape); it did not anticipate a tree caught
   mid-*mutation*, where a proxy for an already-torn-down node raises `GError`
   the instant any walk touches it, crashing the whole script instead of
   retrying like every other not-ready-yet state `wait_for_ready` already
   tolerated. Fixed by treating a `GError`'d node as simply absent from that
   walk — **this is shared infrastructure every case in every M5 slice uses**,
   so the fix benefits all of them retroactively, not just P03.
2. **P07's `navigate_pane_to` call crashed with "expected 2 'Edit path'
   buttons, found 0"** — CI run
   [`31936719013`](https://github.com/forskscope/forskscope/actions/runs/31936719013).
   `navigate_pane_to`'s button lookup is a single unretried tree walk, and the
   window registers on the a11y bus before the WebView paints (the F57 race,
   a new symptom). P06 (`navigate_pane_to`'s other caller) already polls for
   an Explorer-rendered marker before calling it; P07 launches straight into
   the Explorer view (no tab click needed) but was missing the same wait.
3. **P07's Up/Home navigation buttons were searched for by the wrong
   string** — CI run
   [`31937039794`](https://github.com/forskscope/forskscope/actions/runs/31937039794),
   exhausted its full 45s retry budget and never found "Go up one directory"/
   "Home directory". Root cause, found by reading `dir_pane.rs`: these strings
   are only the buttons' `title` tooltip, never an `aria_label` — their real
   AT-SPI accessible name is their glyph text content (`"↑"`/`"⌂"`), the same
   pattern already established for this exact PathBar's Edit-path button
   (`"✎"`, not "Edit path"). I should have checked the source before inventing
   the search strings rather than after CI caught it.
4. **`find_text_containing` crashed the same stale-node way as #1, but only
   under `--break` mode's long retry** — CI run
   [`31937228151`](https://github.com/forskscope/forskscope/actions/runs/31937228151),
   `GLib.GError: The application no longer exists`. Break mode's search for an
   impossible summary line is the only caller that exhausts this function's
   full retry budget against a live, mutating tree; every other caller finds
   a real string quickly. Hardened the same way as #1.

Each fix was confirmed by re-dispatching the exact failing case to CI and
observing it pass (or correctly fail, for `--break`) before moving on — no fix
here is asserted from reading the diff alone.

## 6. Product defects found

**None, in this row.** Unlike Windows (F68/F69/F70, review 067), Linux's P03,
P07, and P11 all pass cleanly against the real product once the harness bugs
above were fixed. Every failure encountered while building this row was a
harness defect, confirmed and fixed as such — not a product behavior recorded
as a defect and then explained away.

## 7. Differences from the handoff, RFC-078, or the frozen plan

- `matrix-plan.md` was not edited. No difference in scope or case-to-row
  mapping.
- Prerequisite A required a new file (`packaging/inject_geometry_defect.py`)
  and a `render-check.yml` input — additive to the existing
  `inject_f32_defect` mechanism, not a change to it.
- No product code was touched by this row (harness/CI-workflow/docs only,
  aside from `ROADMAP.md`).

## 8. Executed gates

No Rust source was touched by this row specifically (the F61 Rust changes in
the same commit range are already covered by request 063 / review 066). Full
CI (fmt, clippy, test, i18n, actionlint over every changed workflow) ran green
on the final commit,
[`16817f6`](https://github.com/forskscope/forskscope/actions/runs/31937420282) —
confirmed via `gh run view`, not assumed from a green badge. `mdbook build
docs` ran clean locally after the evidence-doc changes. Python syntax checked
(`python3 -m py_compile`) before every commit touching `linux_harness.py` or
`render_check.py`.

## 9. Unresolved issues

- **P03's scroll-mirror fallback chain has not been reduced to a single
  documented mechanism** — the harness tries three methods per run and reports
  whichever worked; it has not been run enough times to say whether the
  working method is stable across CI images or coincidental to this
  particular runner generation. Worth revisiting if a future CI image change
  makes it flip.
- **This sandbox's local X11 input synthesis is confirmed broken for
  everything**, not narrowly for scroll or typing as earlier sessions assumed
  — every fix in §5 needed a real CI round-trip to validate, none could be
  iterated locally first. Worth stating plainly for whoever picks up Linux
  harness work next, so they don't spend time trying to reproduce a failure
  locally that can only be seen in CI.
- **F61's fix is not yet in a published artifact** (review 066 §6) — this
  row's P03/P07/P11 evidence is against the same 0.167.0 artifact as
  M5-A/M5-B, so it says nothing new about F61/P12; a new candidate build is
  still needed before P12 can be re-run.

## 10. Requested review focus

1. **Prerequisite A's methodology** — is demonstrating the geometry branch via
   `transform: translateX()` (rather than padding/margin, which review 055
   found doesn't move the accessible box) a sound falsifiability proof, or
   does it risk being too narrow a mutation to represent F32's real defect
   class?
2. **The scroll-mirror fallback chain (§4/§9)** — is trying multiple X11
   synthesis conventions in sequence and reporting whichever worked an
   acceptable Gate D input, given the alternative (asserting one method in
   advance) already failed once on real CI?
3. **Whether the harness bugs in §5 warrant any standing-practice change**,
   the way review 067 adopted F71 from the Windows row's concurrency
   incident — in particular, whether `render_check.py`'s stale-node hardening
   should be applied more broadly to the other recursive walkers in
   `linux_harness.py` pre-emptively, or left as-is until one of them actually
   crashes (the approach taken here).
4. **This row is ready to fold into full M5-C evidence assembly** once
   macOS lands — nothing here is contingent on further Linux-specific work.
