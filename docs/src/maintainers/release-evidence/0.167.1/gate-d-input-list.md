# Gate D Input List — 0.167.1

**This supersedes `0.167.0`'s blocker list for the current candidate.** It is
not a full M5 re-run — `0.167.0`'s [`gate-d-input-list.md`](../0.167.0/gate-d-input-list.md)
carries the full M5-A/B/C detail and remains the historical record of what
was true for `0.167.0` specifically. This file records only what changed:
the architect's instruction to re-run M5's affected rows (P12/F61, P07/F73,
P11/F69) against the newly published `0.167.1` artifacts, and what that
re-run found.

**Assembled:** 2026-08-17, after re-dispatching the affected cases against
`0.167.1` on all three CI-verified platforms (Linux, Windows, macOS).

## Un-waivable blockers

Three at `0.167.0`. **One at `0.167.1`.**

| Input | Status |
|---|---|
| **F44** | **Fail, un-waivable — unchanged.** The published Linux artifact still does not launch on any libxdo-4 distribution. Upstream schedule dependency, not addressed by this fix release (owner decision: wait for an upstream `dioxus-desktop` release rather than fork). **This is now the only blocker nobody here can act on.** |

**F61 and F73 are resolved, not listed here** — see below. Gate D still
cannot pass (F44 remains), but the blocker count dropped from three to one.

## Resolved this candidate, verified for real

| Input | Status |
|---|---|
| **F61** | **Resolved, verified against `0.167.1` on all three platforms.** Linux CI run [`32004014491`](https://github.com/forskscope/forskscope/actions/runs/32004014491) (after also fixing an independent harness bug — P12's own sub-test 2 was silently dropping its scratch `XDG_CONFIG_HOME`, not a product regression), Windows run [`32002963437`](https://github.com/forskscope/forskscope/actions/runs/32002963437) (`"a CLI-opened tab restored automatically on a no-args relaunch"`), macOS run [`32002978390`](https://github.com/forskscope/forskscope/actions/runs/32002978390). `--break` mode correctly failed on all three, confirming the checks aren't vacuous. |
| **F73 / F68** | **Resolved, verified against `0.167.1` on macOS** (the only platform where P07 reaches the per-row copy check — Windows P07 remains blocked by F70, unrelated; Linux P07 was never affected). Run [`32003819705`](https://github.com/forskscope/forskscope/actions/runs/32003819705): with `last_right_dir` still seeded to the wrong directory (the exact condition that produced the original defect), a per-row copy correctly landed at the compare root with a real backup, and the old wrong location stayed untouched. `--break` run [`32003821373`](https://github.com/forskscope/forskscope/actions/runs/32003821373) correctly failed. F68 (the buttons vanishing with `remember_explorer_dirs` off) shares the identical fix and is resolved alongside it. |
| **F69** | **Resolved, verified against `0.167.1` — the specific evidence this Gate D input was waiting on.** Windows CI run [`32002971000`](https://github.com/forskscope/forskscope/actions/runs/32002971000): `"OK: modal focus starts on 'Cancel' (the safe action), not 'Overwrite' (the destructive one)"` — the P11 item-2 check has flipped Fail → Pass for real. `--break` run [`32002972760`](https://github.com/forskscope/forskscope/actions/runs/32002972760) correctly rejected the impossible requirement. Linux (`32002939138`/`32002940976`) and macOS (`32002985498`/`32002987450`) both confirmed to still pass in both directions. |
| **F72** | **Resolved, verified against `0.167.1` on macOS** (run [`32003819705`](https://github.com/forskscope/forskscope/actions/runs/32003819705): Forward correctly stays available after Back; `--break` run [`32003821373`](https://github.com/forskscope/forskscope/actions/runs/32003821373) correctly failed). Not independently re-exercised via Linux/Windows P07 — shared, not platform-specific, code, backed by a dedicated unit test confirmed to fail before the fix. Was not previously listed as un-waivable (a navigation-history defect, not one of RFC-078's five named categories), so this closes a non-blocking input rather than a blocker. |

## Unchanged from `0.167.0` — not re-tested this pass

Everything else in `0.167.0/gate-d-input-list.md`'s "Other Gate D inputs"
section stands as-is: **F45** (manual, outstanding), **F46** (blocked,
unverifiable), **F60** (Windows floor unevidenced), **F70** (Windows
Explorer listing — still open, still blocks Windows P07, not touched by
this fix release), the **keyboard-coverage gap** (unchanged — F69's fix
addresses item (2) of P11 specifically; items (1)/(3)/(4) remain
structurally not CI-verifiable on any platform), and **`linux-wayland`**
(owner's manual row, still outstanding). F63 remains resolved (harness
artifact, not a product defect).

## What this re-run does not change

- **Gate D still cannot pass.** F44 alone is enough to block it, regardless
  of everything else here.
- **The matrix is still not formally complete** — `linux-wayland` and F45's
  Windows manual sub-case remain outstanding, exactly as before.
- **This was a targeted re-run of three cases on three platforms (18
  dispatches total), not a full M5-A/B/C re-execution.** Every other case
  result recorded against `0.167.0` is assumed to still hold against
  `0.167.1` (the same Rust source outside the four fixed defects) but was
  not re-dispatched to confirm.
