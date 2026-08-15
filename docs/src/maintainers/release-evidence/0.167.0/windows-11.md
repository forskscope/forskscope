# Platform Evidence — `windows-11`

**Artifact filename:** `forskscope-v0.167.0-windows-x64.zip`
**SHA-256:** `bd7c1d9107754f7866639de7d09668fcd0c70ca5669f5cbee15ccdfeca293c1d`
**Source commit:** `cb6f5b6`
**Test date (UTC):** 2026-08-15
**Tester role:** implementer (M5-A), automated via CI; F45's prerequisite
sub-case remains owner-executed and outstanding (see below)
**Host OS and version:** Windows Server-based CI image — **resolved
runner image** `win25-vs2026`, image version `20260810.198.2`, kernel
`Microsoft Windows NT 10.0.26100.0` (recorded per review 057 §4.3's
rolling-label caveat: `windows-latest` itself is not reproducible, this
resolved image is)
**Architecture:** x86_64
**Display server / WebView runtime:** a real interactive Windows desktop
session (not a virtual framebuffer — GitHub's Windows runners provide
one directly, unlike the Linux row's Xvfb stand-in) + WebView2, per
`matrix-plan.md`'s stated CI method for this row
**Install source and prerequisites:** published GitHub Release asset,
downloaded and digest-verified by CI before every case run; no
undocumented runtime prerequisite was needed (contrast Linux's F59) —
`windows-latest` ships the VC++ redistributable and WebView2 already, so
this is not evidence that a bare Windows install needs nothing (see F45
below)

## Cases

| Case | Result | Evidence |
|---|---|---|
| P01 — Install and cold launch | **Pass** (CI) / **Not executed** (F45's prerequisite sub-case — manual only, see below) | CI run [`31853150147`](https://github.com/forskscope/forskscope/actions/runs/31853150147) |
| P02 — CLI file compare | **Pass** | CI run [`31853258477`](https://github.com/forskscope/forskscope/actions/runs/31853258477) |
| P09 — Mergetool | **Pass** | CI run [`31853496997`](https://github.com/forskscope/forskscope/actions/runs/31853496997) |
| P10 — Binary/XLSX fail-closed policy | **Pass** | CI run [`31853362804`](https://github.com/forskscope/forskscope/actions/runs/31853362804) |

Harness: `packaging/evidence/windows_harness.py`, driven by
`.github/workflows/m5-evidence-windows.yml` (`workflow_dispatch`, one
case per run). Every case downloads and digest-verifies the **published**
artifact itself — nothing here was built from source.

This row and `windows-10.md` share the same CI host: `windows-latest`
resolves to one image family, and there is no separate Windows 11 vs.
Windows 10 CI runner available. Per `matrix-plan.md`'s own text for the
`windows-10` row ("a Server-based image ... stands in reasonably"), both
rows record the *same* CI run evidence below rather than duplicating it —
this is not a shortcut invented here, it is what the frozen plan already
specifies for CI method on both rows.

### P01

`--diagnostics` reported `ForskScope 0.167.0`, with `OS: windows` and a
redacted home path (`Home: C:\Users\***`); a plain cold launch produced
a real, non-blank 1044×788 window. No undocumented prerequisite was
needed to reach this — unlike Linux's F59, `windows-latest` already
carries what a real Windows 11 machine with WebView2 and the VC++
redistributable installed would have.

### F45 — the "prerequisites missing" sub-case is not executed here

Per the M5-A handoff §4 and `matrix-plan.md`: `windows-latest` ships the
VC++ redistributable and WebView2 already, so CI cannot exercise "these
are missing on a clean machine" — there is no way to uninstall them on a
hosted runner to observe the failure mode a real first-time user without
either dependency would see. This sub-case is recorded as **owner-executed
and outstanding**, not attempted here, not waived, and not silently
folded into the CI "Pass" above. The CI pass above establishes only that
launch succeeds *given* those prerequisites — a real, but narrower, claim
than "Windows 11 P01 is fully covered."

## Falsifiability

Every case's `--break` mode was run and confirmed to fail for the
expected reason, per handoff §7:

| Case | Break-mode run | Result |
|---|---|---|
| P01 | [`31853199727`](https://github.com/forskscope/forskscope/actions/runs/31853199727) | Fail — `--diagnostics output does not start with 'ForskScope 0.0.0-impossible': ForskScope 0.167.0` |
| P02 | [`31853350190`](https://github.com/forskscope/forskscope/actions/runs/31853350190) | Fail — `compare view never rendered these expected tokens within 60s: ['this-token-cannot-appear-in-real-output']` |
| P09 | [`31853592413`](https://github.com/forskscope/forskscope/actions/runs/31853592413) | Fail — content check correctly rejected a value real Save output can never produce |
| P10 | [`31853491433`](https://github.com/forskscope/forskscope/actions/runs/31853491433) | Fail — `could not find text containing 'this message cannot appear' within 60s` |

Unlike the Linux harness's development history (several iterations were
needed to get P09's button-invocation working reliably under a bare
Xvfb display with no window manager), every case here passed in normal
mode and failed correctly in break mode on the **first** dispatched run
of each. Every break-mode run above is a genuine, observed failure with
the expected message, not an assumption — this is not treated as
evidence the checks are weaker, only that a real interactive Windows
desktop session (unlike Linux's bare Xvfb) gave UIA's Invoke pattern a
straightforward path with no window-manager/focus complications to work
around. One deliberate difference from the Linux harness's readiness
condition: this harness waits for the fixture's distinguishing text
tokens to appear in the UIA tree, not an exact per-pane row count —
`windows_harness.py`'s module docstring has the full reasoning, including
that the exact UIA control type WebView2 maps `role="row"` to was never
empirically confirmed, since the content-based check already worked.

**Known dependency of M5-C, not a limitation of this slice (review 063
§5.3):** P02's content-presence check is sufficient for what P02 itself
needs — early satisfaction of "these tokens exist somewhere" costs
nothing when there's no wrong-answer risk from a partially-rendered tree.
It is not sufficient for **P03 (compare layout and scrolling)**, which
compares row alignment across the whole tree the way F34's Linux check
does — a partial row set there would compare a subset and risk a false
pass, the exact failure shape F34 exists to prevent. Establishing the
UIA control type WebView2 maps `role="row"` to (so an exact row count
becomes possible on Windows, matching Linux's AT-SPI approach) is
something to settle **before** P03 in M5-C, not discover during it.

## Failures and issue links

- **F45** — Windows P01's prerequisite sub-case ("VC++ redistributable /
  WebView2 missing") cannot be exercised on `windows-latest` because both
  are already present on the image. Recorded as owner-executed and
  outstanding, per the handoff.

## Waivers

None.
