# Review Request: M5-A — Evidence Harness and the Launch/CLI Cases

**Date:** 2026-08-15
**Reviewer stance:** F44's recorded outcome and the falsifiability demonstrations are the highest-stakes parts — this evidence feeds a v1 go/no-go
**Repository baseline:** `6746080`
**Governing document:** `rfcs/handoffs/078-platform-runtime-acceptance/m5a-harness-and-launch-cases-handoff.md`

## 1. Implementation summary — the harness, and how to run each case on demand

Three platform-specific harnesses, one per CI-verified row family, each
following `render-check.yml`'s established shape (`workflow_dispatch`
only, no tag/release side effects, one case per dispatch):

- **Linux** (`linux-x11`): `packaging/evidence/linux_harness.py` +
  `.github/workflows/m5-evidence-linux.yml`. Built directly by me,
  validated locally first (a real desktop AT-SPI bus) before every CI
  dispatch, exactly like F57's development pattern.
- **Windows** (`windows-11`/`windows-10`, one shared host): `packaging/evidence/windows_harness.py`
  + `.github/workflows/m5-evidence-windows.yml`. Built by a delegated
  agent, independently spot-verified by me afterward (see §9).
- **macOS** (`macos-aarch64`): `packaging/evidence/macos_harness.py` +
  `packaging/evidence/macos_ui.applescript` +
  `.github/workflows/m5-evidence-macos.yml`. Also delegated and
  independently spot-verified.

Run any case on demand:

```sh
gh workflow run "M5 Evidence - Linux"   --ref main -f case=p01 -f break_case=false
gh workflow run "M5 Evidence - Windows" --ref main -f case=p09 -f break_case=true
gh workflow run "M5 Evidence - macOS"   --ref main -f case=p10 -f break_case=false
```

All three delegated/direct harnesses converged independently on the same
core design decision: **invoke the platform's native accessibility action
directly** (AT-SPI's `Action.do_action`, Windows UIA's Invoke pattern,
macOS's `AXPress` via System Events) **rather than synthesizing keyboard
or mouse input**. On Linux this was discovered the hard way — see §9's
account of ~10 failed iterations trying X11 input synthesis under a bare
Xvfb display before landing on direct action invocation — and that lesson
was passed to the Windows/macOS work up front, which is likely why both
of those passed on their first real CI attempt with no comparable
struggle.

## 2. Cases executed, per row, with results

All four cases (P01, P02, P09, P10) executed on all three CI-verified
rows. Full per-case narrative, exact commands, and evidence schema fields
are in `docs/src/maintainers/release-evidence/0.167.0/{README,linux-x11,windows-11,windows-10,macos-aarch64}.md`
— summarized here:

| Row | P01 | P02 | P09 | P10 |
|---|---|---|---|---|
| `linux-x11` | **Pass** (CI) / **Fail** (real libxdo-4 host — F44) | Pass | Pass | Pass |
| `windows-11` | **Pass** (CI) / F45 sub-case not executed (owner) | Pass | Pass | Pass |
| `windows-10` | Pass (same CI run as `windows-11`, per the plan's own text) | Pass | Pass | Pass |
| `macos-aarch64` | Pass | Pass | Pass | Pass |

`linux-wayland` (owner's manual row) was not executed in this slice —
`docs/src/maintainers/release-evidence/0.167.0/linux-wayland.md` records
it as outstanding rather than silently omitting it.

## 3. Falsifiability demonstrations, with observed output

All 24 combinations (4 cases × 2 modes × 3 CI rows) were dispatched for
real and their logs read, not just written and trusted. Full tables with
run links are in each row's evidence file; representative samples:

**Linux P09, break mode** ([`31852312861`](https://github.com/forskscope/forskscope/actions/runs/31852312861)):
```
FAIL: .../merged.txt's content 'alpha\nold-line\ngamma\nepsilon\nzeta\ninsert-line\n' != required 'this exact string can never appear in real merge output'
```
(This also incidentally confirms P09's normal-mode result is *real* merge
output, not a placeholder that happened to differ — the same string shows
up as the accepted value in the normal-mode run.)

**Windows P02, break mode** ([`31853350190`](https://github.com/forskscope/forskscope/actions/runs/31853350190)):
```
FAIL: compare view never rendered these expected tokens within 60s: ['this-token-cannot-appear-in-real-output']
```

**macOS P10, break mode** ([`31853267021`](https://github.com/forskscope/forskscope/actions/runs/31853267021)):
```
FAIL: could not find text containing 'this message cannot appear' within 45s
```

No break-mode run passed by accident, and no normal-mode run's pass
depended on the break-mode assertion being trivially satisfiable — each
`--break` targets a value the real, unmodified app cannot produce
(impossible version string, impossible row/token count, wrong file
content, unfindable message text), per handoff §7.

## 4. F44/F45/F46's recorded outcomes, exact wording

**F44 (Linux, libxdo-4):** recorded as **Fail**, not waived, not
narrowed to a compatible host. `linux-x11.md`:

> Per handoff §4 and `matrix-plan.md` §3: because Linux support is
> confirmed unqualified (no per-distribution floor), this is **not**
> satisfied by the CI pass above — a libxdo-4 distribution is a supported
> platform, and this failure is recorded as the real, expected P01
> outcome for it. Not waivable... Tracked as **F44**, un-waivable per
> review 061 §3.1 — this is a schedule dependency on the upstream
> `dioxus-desktop` release..., not something this evidence pass can close.

Reproduced directly on a real Arch-family host (my own machine), not
simulated — exact `readelf`/`pacman -Q` output is in the evidence file.

**F45 (Windows, prerequisites):** recorded as **owner-executed and
outstanding**, `windows-11.md`:

> This sub-case is recorded as **owner-executed and outstanding**, not
> attempted here, not waived, and not silently folded into the CI "Pass"
> above. The CI pass above establishes only that launch succeeds *given*
> those prerequisites — a real, but narrower, claim than "Windows 11 P01
> is fully covered."

**F46 (macOS, Gatekeeper):** recorded as **Blocked**, `macos-aarch64.md`:

> Per `matrix-plan.md` §3 and the M5-A handoff, out of scope for this
> slice to resolve and not attempted here... the `.dmg` mounted and the
> binary launched cleanly in every run above, which is expected and
> uninformative about signing/notarization posture, not evidence
> Gatekeeper was satisfied... recorded here as an explicit, unresolved
> Gate D input, exactly as `matrix-plan.md` directs.

## 5. Digest verification results

All three published assets downloaded fresh (`gh release download
0.167.0`) and hashed independently — matched the handoff §5 values and
each other exactly, no mismatch observed at any point:

| File | SHA-256 |
|---|---|
| `forskscope-v0.167.0-linux-x86_64.tar.gz` | `e17baa26abbb91e5e8e046d3812b08203f0d1ddfd6f8dc9fb9182326ed04bf09` |
| `forskscope-v0.167.0-macos-aarch64.dmg` | `2d66f125f0325adfef36cdf9bbb643a8deed50112db77707f7e6c9970ca25099` |
| `forskscope-v0.167.0-windows-x64.zip` | `bd7c1d9107754f7866639de7d09668fcd0c70ca5669f5cbee15ccdfeca293c1d` |

## 6. Created and changed files

```
.github/workflows/m5-evidence-linux.yml     (new)
.github/workflows/m5-evidence-macos.yml     (new)
.github/workflows/m5-evidence-windows.yml   (new)
packaging/evidence/linux_harness.py         (new)
packaging/evidence/windows_harness.py       (new)
packaging/evidence/macos_harness.py         (new)
packaging/evidence/macos_ui.applescript     (new)
docs/src/maintainers/release-evidence/0.167.0/README.md          (new)
docs/src/maintainers/release-evidence/0.167.0/artifacts.md       (new)
docs/src/maintainers/release-evidence/0.167.0/linux-x11.md       (new)
docs/src/maintainers/release-evidence/0.167.0/linux-wayland.md   (new)
docs/src/maintainers/release-evidence/0.167.0/windows-11.md      (new)
docs/src/maintainers/release-evidence/0.167.0/windows-10.md      (new)
docs/src/maintainers/release-evidence/0.167.0/macos-aarch64.md   (new)
ROADMAP.md                                   (F59 registered)
```

No file outside `docs/`, `.github/workflows/`, `packaging/evidence/`, and
one `ROADMAP.md` line was touched. No Rust source changed — confirmed by
`cargo fmt --check`/`clippy`/`test --workspace` all passing unchanged
throughout (1095 tests, same as before this slice).

## 7. Product defects found — registered, not fixed

**F59** (new): `installation.md`'s documented Debian/Ubuntu runtime
prerequisites (`libwebkit2gtk-4.1-0`, `libgtk-3-0`) are incomplete —
`libxdo.so.3` is also required, is not pulled in by either package, and
is not mentioned in the docs. Confirmed directly: a fresh `ubuntu-latest`
host with only the documented packages fails to launch the published
binary at all (`error while loading shared libraries: libxdo.so.3`).
Distinct from F44 (libxdo.so.**4**-family distros, no simple fix) — this
is a one-line doc fix, registered in `ROADMAP.md`, not applied here per
the handoff's explicit constraint against fixing product/doc content
mid-evidence-gathering.

No other product defect was found on any platform. Windows and macOS in
particular needed no undocumented prerequisite or workaround of any
kind — both passed P01 cleanly against exactly `windows-latest`/`macos-latest`'s
stock state.

## 8. Difference from the handoff, RFC-078, or the frozen plan

None to the frozen plan itself — `matrix-plan.md` was not edited by me
or by either delegated agent (both were explicitly instructed not to,
and I confirmed this in the final diff). One process choice worth
flagging: I delegated the Windows and macOS harness-building to two
background agents running in parallel while I built Linux directly and
handled evidence assembly — the handoff doesn't address *how* the work
gets done, only what's required, so I judged this within scope, but
naming it since it's a departure from every prior slice in this program
being done by one continuous thread of work. See §9 for how I verified
the delegated work rather than taking it on faith.

## 9. Executed gates, with observed output

```text
cargo fmt --check                                              pass
cargo clippy --workspace --all-targets -- -D warnings           pass
cargo test --workspace                                          pass — 1095, unchanged (no Rust touched)
cargo xtask i18n                                                 pass — 227 keys, unchanged
mdbook build docs                                                pass
git diff --check                                                 pass
actionlint .github/workflows/*.yml (v1.7.12, matching ci.yml's pin) pass
```

**Independent verification of the delegated Windows/macOS work** (not
just trusting the agents' self-reports, per this project's own "trust
but verify" standard): after each agent reported completion, I pulled
the pushed commits and, for one representative run per platform,
independently re-ran `gh run view <id> --json conclusion` and `gh run
view <id> --log` myself and confirmed the exact `OK:`/`FAIL:` text
matched what was reported — for example Windows P09 normal
([`31853496997`](https://github.com/forskscope/forskscope/actions/runs/31853496997))
and macOS P10 normal
([`31853205096`](https://github.com/forskscope/forskscope/actions/runs/31853205096)),
both confirmed to print exactly the claimed output. I also read both
evidence documents in full and found and fixed one real issue myself: a
dangling cross-reference in `windows-11.md` pointing at a "What went
right" section in an "M5-A Windows report" that only ever existed as the
delegated agent's own summary message to me, not anything in the repo a
reader of the evidence file could follow (fixed in `c3de953`).

**Linux harness development history**, since it's the most instructive
part of this slice for anyone building the next platform harness: P09's
"click Save" step failed on five consecutive CI iterations before
landing on the working approach —

1. `Atspi.generate_keyboard_event` (AT-SPI's key-synthesis API): no
   effect — a bare Xvfb display has no window manager, so nothing ever
   gives the newly launched window ambient X11 focus for the synthesized
   event to reach.
2. `xdotool windowactivate`: failed outright — needs a window manager to
   answer an EWMH `_NET_ACTIVE_WINDOW` request that never comes under
   bare Xvfb.
3. `xdotool windowfocus` (`XSetInputFocus` directly, no WM needed): raced
   `BadMatch` against window-viewable timing — fixed with a short retry
   loop.
4. `xdotool key --window <id> ctrl+s`: silently had no effect — uses
   `XSendEvent`, which GTK (like most modern toolkits) ignores as
   synthetic input.
5. `xdotool key ctrl+s` (no `--window`, real XTEST event to whatever now
   has genuine focus from step 3): still no effect — X11-level window
   focus doesn't guarantee the embedded WebView widget itself has
   internal focus.

What actually worked, and what I'd recommend as the default approach for
any future platform harness rather than a last resort: the toolbar
already exposes a real "Save merge result (Ctrl+S)" button with an
accessible name, and AT-SPI's `Action.do_action()` invokes its `onclick`
handler directly through the accessibility bridge — no OS-level input
synthesis, window manager, or focus state involved at all. This is the
approach both delegated agents used from the start (briefed on this
history up front), which is almost certainly why they didn't hit the
same wall.

## 10. Unresolved issues and known limitations

- **F44 remains open** — a schedule dependency on an upstream release,
  not something any amount of further evidence-gathering closes.
- **F45/F46's manual/unverifiable sub-cases** remain exactly as open as
  `matrix-plan.md` already recorded — this slice only confirmed the
  CI-observable portions around them, per the frozen plan's own design.
- **`linux-wayland` was not executed** — the owner's manual row, out of
  this slice's (CI-automation) scope entirely.
- **Windows's readiness condition differs structurally from Linux's**:
  Linux waits for an exact row count via AT-SPI's "table row" role;
  Windows waits for the fixture's distinguishing text tokens to appear
  anywhere in the UIA tree, because the exact UIA control type WebView2
  maps a table-less `role="row"` div to was never empirically confirmed
  (the token-based check worked, so that investigation wasn't forced).
  Still a real, fixture-specific content assertion, not a vacuous
  "something rendered" check, but not row-count parity with Linux.
- **macOS's System Events UI scripting worked with zero permission
  friction** — flagged as not proven robust against a future
  `macos-latest` image revoking that default, only confirmed to work on
  the specific resolved image (`macos26`/`20260728.0273.1`) this pass
  observed.
- **P01's `--diagnostics` output is asserted programmatically but not
  captured verbatim** in the Windows/macOS evidence docs (the harnesses
  print a one-line summary, not the full report text) — both docs are
  worded to say what was asserted and passed, not to imply a literal
  captured transcript exists.

## 11. Requested review focus

1. Whether F44's recorded outcome (Fail, un-waivable, schedule
   dependency) is stated with enough force in `README.md`/`linux-x11.md`
   that a reader skimming only the top-level verdict can't mistake the
   otherwise-clean pass rate for "this candidate is close to ready."
2. Whether delegating the Windows/macOS harness construction to
   background agents (§8) was the right call for a slice whose output
   feeds a v1 go/no-go, or whether that kind of evidence-gathering work
   should be done by one continuous thread even at higher wall-clock
   cost, for reasons beyond what I've already checked in §9.
3. Windows's token-based readiness condition (§10) — whether it needs to
   be strengthened toward Linux's exact-row-count parity before this
   evidence is trusted as equivalent in rigor across rows, or whether
   it's already sufficient for what P02 needs to prove.
