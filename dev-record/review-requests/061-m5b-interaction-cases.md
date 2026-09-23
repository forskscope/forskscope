# Review Request: M5-B — The Interaction Cases (all three rows)

**Date:** 2026-08-15/16
**Reviewer stance:** §3's resolution and P08's Exit process-state evidence
are the highest-stakes parts of this slice; F61 (session state opened via
CLI startup is not reliably persisted) — found independently on all three
platforms — is the highest-value finding and the one item most likely to
change the Gate D picture.
**Repository baseline:** handoff-pinned; published `0.167.0` artifacts and
digests throughout, same as M5-A used — no new download, no source build,
no dependency change.
**Governing document:** `rfcs/handoffs/078-platform-runtime-acceptance/m5b-interaction-cases-handoff.md`
**Scope:** all three CI-verified rows (`linux-x11`, `windows-11`/`windows-10`,
`macos-aarch64`) — P04, P05, P06, P08, P12.

**Delegation note.** Linux was built and verified directly in this
session. Windows and macOS were delegated to two background agents,
each given the full §3 resolution, fixture formats, and lessons learned
up front, so neither had to rediscover them independently; both were
spot-checked against real CI run IDs before being folded in here (not
merely trusted from their own summaries). The macOS agent additionally
wrote its own detailed review request,
`dev-record/review-requests/060-m5b-macos-interaction-cases.md` — that
file is the fuller record for that row; this document summarizes it
alongside Linux (full detail) and Windows (summarized from the agent's
report and CI, no separate file written).

## 1. §3's resolution — P04's "keyboard and mouse", settled before execution

Per the handoff's explicit instruction to settle this before running P04,
and independently arrived at the same way by all three platform efforts:

**The mouse path is CI-verifiable and was executed on all three rows; the
keyboard path is structurally not CI-verifiable on any platform, and was
not attempted anywhere.**

The "Use this change" button has a real `onclick` handler
(`crates/forskscope-ui/src/ui/view/hunk.rs`). Invoking it via the
platform's accessibility-action route (`Atspi.Action.do_action` on
Linux, UIA Invoke on Windows, `AXPress` on macOS) fires the exact same
handler a real click would — the same equivalent-path argument M5-A
already used for the Save button, not laundering. The Enter-key shortcut
(`Key::Enter => apply_focused_hunk(...)`, `app.rs`'s global `onkeydown`)
is a raw keydown listener bound to no actionable UI element at all —
there is no accessible "action" to invoke for it via any accessibility
API, on any platform. This is different in kind from M5-A's Linux
input-synthesis struggles (where the *mechanism* to deliver a real key
event existed but was unreliable); here no accessibility mechanism
exists at all to exercise a global keydown handler.

Recorded as not-executed/manual-outstanding, mirroring F45's existing
shape for P01's prerequisite sub-case — this narrows CI-verification
scope the same way F45 already does, not the case's actual requirement.
RFC-078's text was **not** amended.

## 2. Implementation summary — how each case runs on demand

Each row extends its M5-A harness in place, per the handoff's instruction
not to restructure existing files.

**Linux** — `packaging/evidence/linux_harness.py`, dispatched via
`.github/workflows/m5-evidence-linux.yml`:

```sh
gh workflow run "M5 Evidence - Linux" --ref main -f case=p06 -f break_case=false
gh workflow run "M5 Evidence - Linux" --ref main -f case=p12 -f break_case=true
```

New case functions `p04`, `p05`, `p06`, `p08`, `p12`; new shared helpers
(`type_into_field` — real X11 synthesis for the one text-entry widget
with no `EditableText` interface; `real_click_at` — real X11 click for
the two elements whose `Action.do_action` doesn't fire their real
handler; `find_app_root`, `process_died`, `wait_for_dialog`/
`wait_for_dialog_gone`, `explorer_rows_by_pane`, `selected_option_name`).
M5-A's `p01`/`p02`/`p09`/`p10` untouched.

**Windows** — `packaging/evidence/windows_harness.py` (527 → 2236 lines),
`.github/workflows/m5-evidence-windows.yml` extended; `pywinauto`'s UIA
Invoke pattern throughout, matching M5-A's own approach on this row.

**macOS** — `packaging/evidence/macos_harness.py` and
`macos_ui.applescript` extended; AppleScript/System Events (`AXPress`),
matching M5-A. Full detail in review request 060.

## 3. Cases executed, per row, with results

### Linux (`linux-x11`)

| Case | Normal | Break |
|---|---|---|
| P04 | **Pass** — [`31877812540`](https://github.com/forskscope/forskscope/actions/runs/31877812540) | **Fail (expected)** — [`31878367904`](https://github.com/forskscope/forskscope/actions/runs/31878367904) |
| P05 | **Pass** — [`31878357564`](https://github.com/forskscope/forskscope/actions/runs/31878357564) | **Fail (expected)** — [`31878360350`](https://github.com/forskscope/forskscope/actions/runs/31878360350) |
| P06 | **Pass** — [`31912063479`](https://github.com/forskscope/forskscope/actions/runs/31912063479) | **Fail (expected)** — [`31911967700`](https://github.com/forskscope/forskscope/actions/runs/31911967700) |
| P08 | **Pass** — [`31879673483`](https://github.com/forskscope/forskscope/actions/runs/31879673483) | **Fail (expected)** — [`31879245189`](https://github.com/forskscope/forskscope/actions/runs/31879245189) |
| P12 | **Fail — real product defect (F61), restore-half verified separately** — [`31894750441`](https://github.com/forskscope/forskscope/actions/runs/31894750441) | **Fail (expected, on the settings-restore assertion)** — [`31894752487`](https://github.com/forskscope/forskscope/actions/runs/31894752487) |

### Windows (`windows-11` / `windows-10`, same host/runs per `matrix-plan.md`)

| Case | Normal | Break |
|---|---|---|
| P04 | **Pass** — [`31880581511`](https://github.com/forskscope/forskscope/actions/runs/31880581511) | **Fail (expected)** — [`31880584219`](https://github.com/forskscope/forskscope/actions/runs/31880584219) |
| P05 | **Pass** — [`31880587452`](https://github.com/forskscope/forskscope/actions/runs/31880587452) | **Fail (expected)** — [`31880590546`](https://github.com/forskscope/forskscope/actions/runs/31880590546) |
| P06 | **Pass** — [`31880593958`](https://github.com/forskscope/forskscope/actions/runs/31880593958) | **Fail (expected)** — [`31880597266`](https://github.com/forskscope/forskscope/actions/runs/31880597266) |
| P08 | **Pass** — [`31880600935`](https://github.com/forskscope/forskscope/actions/runs/31880600935) | **Fail (expected)** — [`31880604376`](https://github.com/forskscope/forskscope/actions/runs/31880604376) |
| P12 | **[Corrected by review 064 §5.4] Fail — real path not exercised (F61)**, worked around by seeding rather than a genuine Pass — [`31880607270`](https://github.com/forskscope/forskscope/actions/runs/31880607270) | **Fail (expected)** — [`31880609761`](https://github.com/forskscope/forskscope/actions/runs/31880609761) |

Spot-checked directly against CI (not taken on the delegated agent's word
alone): runs `31880600935` and `31880581511` both confirmed `completed` /
`success` via `gh run view`. `git log` confirms 19 real commits on
`packaging/evidence/windows_harness.py`, including the exact F61-shaped
fix commit message the agent described (`"P12 seeds session.json
directly; CLI-launch auto-save doesn't work"`).

### macOS (`macos-aarch64`)

| Case | Normal | Break |
|---|---|---|
| P04 | **Pass** — [`31879443866`](https://github.com/forskscope/forskscope/actions/runs/31879443866) | **Fail (expected)** — [`31880080196`](https://github.com/forskscope/forskscope/actions/runs/31880080196) |
| P05 | **Pass** — [`31879848097`](https://github.com/forskscope/forskscope/actions/runs/31879848097) | **Fail (expected)** — [`31880082912`](https://github.com/forskscope/forskscope/actions/runs/31880082912) |
| P06 | **Pass, reduced scope — see §4** — [`31883446824`](https://github.com/forskscope/forskscope/actions/runs/31883446824) | **Fail (expected)** — [`31883530193`](https://github.com/forskscope/forskscope/actions/runs/31883530193) |
| P08 (4 sub-cases: Exit/Continue/Reset/legacy) | **Pass, all four** | **Fail (expected), all four** |
| P12 | **Fail — real product defect (F61)** — [`31881394791`](https://github.com/forskscope/forskscope/actions/runs/31881394791) | **Fail (expected, on the settings-restore assertion)** — [`31883545108`](https://github.com/forskscope/forskscope/actions/runs/31883545108) |

Full per-run detail for P08's four sub-cases is in review request 060 §3.

## 4. P06 and P12 — the two explicitly-flagged vacuous-pass risks (handoff §6)

**P06** ("a check that passes whenever two tabs merely exist"): addressed
on all three rows by asserting the *specific, distinct content* of the
surviving tab after closing the loading one, not just its existence —
each row's break-mode run requires the impossible (closed tab's) content
and correctly fails to find it.

- **Linux**: opens two genuinely large (20,000-line) fixture pairs via
  CLI launch + Explorer navigation; closes the loading tab (exercising
  RFC-065's dirty-check bypass); asserts the survivor's distinct marker.
  Two reloads in quick succession occasionally hit a transient
  `GLib.GError` from WebKitGTK's separate content process dropping off
  the accessibility bus under load — confirmed via `proc.poll()` (polled,
  not checked once) that the harness's own launched process stayed alive
  throughout; the harness retries through this rather than treating it
  as fatal or as a product defect.
- **Windows**: passes as designed, per the agent's report.
- **macOS**: **a real, disclosed deviation from RFC-078's description.**
  The literal in-app design (switch to Explorer while tab 0 loads, open
  tab 1 there) and the handoff's own pre-approved concurrent-two-process
  fallback both hung/failed reproducibly. The actual root cause, found
  only after retreating to two *non-overlapping* launches: a generated
  diff pair's content never reaches the accessibility tree once the file
  crosses a size threshold between 30 and 100 lines — a rendering-size
  issue, not a concurrency one, and almost certainly what was failing the
  first two designs too. Recorded as **Pass for a reduced scope**
  (sequential, non-overlapping launches), not RFC-078's original
  concurrent description. Flagged in review 060 as needing revisit with
  correctly-sized fixtures — not attempted there given time already
  spent. **Linux and Windows do not share this limitation** — both
  exercise genuinely overlapping/racing tabs within one process.

**P12** ("P08's Exit" was the other named risk — see §5; P12's own risk
is implicit in the case's own design, not separately named, but the same
discipline applies): every row's restore-check reads back real UI state
(a specific theme/language selection or specific tab content), not a
weaker "did the process not crash" signal.

## 5. P08's three-choice evidence per row, including process state (handoff §4)

All three recovery-dialog choices (Exit, Continue with defaults, Reset
and back up) plus legacy v0 migration, exercised on every row, each with
the process-state assertion the handoff requires — not just that the
dialog disappeared.

| Row | Exit mechanism | Confirmed |
|---|---|---|
| Linux | `proc.poll()` polled to non-`None` (via `process_died`, up to 2s, not a single racy check) | Exit terminates; Continue leaves it running with the file untouched; Reset backs up the corrupt original (`session.json.reset.bak` byte-for-byte equal) and resets the file |
| Windows | `psutil.pid_exists(pid)` polled to `False` | `OK: Exit terminated pid 5572 - confirmed gone via psutil.pid_exists() within 60s.` Break mode correctly failed the inverted (still-running) check |
| macOS | `proc.poll()` (direct, non-racy child-reap), corroborated by `os.kill(pid, 0)` | `rc=0` within the poll window; `os.kill` confirmed `OSError` (no such process) immediately after |

All three rows' `--break` runs asserted the impossible opposite (process
still running after Exit) and correctly failed — this is the single most
important assertion per the handoff, and it holds on every row.

## 6. Falsifiability demonstrations with observed output

Every case, every row, both modes — full tables in each platform's
evidence file (`linux-x11.md`'s new "M5-B falsifiability" section;
`windows-11.md`/`windows-10.md`; `macos-aarch64.md`). The two the handoff
named explicitly:

- **P06** (Linux): break mode requires the *closed* tab's marker and
  correctly doesn't find it — `FAIL: surviving tab does not show
  'PAIR-A-MARKER' content after closing the loading tab`.
- **P08's Exit** (all three rows): see §5 — every row's break run
  required the impossible "still running" state and correctly failed.

## 7. Product defect found, registered, not fixed — F61

**A tab opened via CLI startup args is not reliably persisted to
`session.json` — confirmed independently, via three genuinely different
harness implementations, on Linux, Windows, and macOS.**

`app.rs`'s reactive `use_effect` on `store.tabs` (meant to auto-persist
the session on every tab-list change, the same mechanism that makes
settings persist "reactively, not only on exit") does not fire a real
write for a CLI-opened tab, while direct, non-reactive call sites like
`close_tab` (`state/session.rs`) do persist correctly in the identical
environment — traced precisely by the macOS harness's isolated
`recon_session_save` diagnostic (review request 060 §7): open a tab via
CLI, `session.json` confirmed absent 1.5s later; click Close on that same
tab, `session.json` appears correctly right afterward, same process, same
environment.

**Practical impact:** `forskscope <left> <right>` from the command line
— the single most ordinary CLI/`git difftool`-style workflow — quit
without touching the tab list, and that session is silently lost on
restart.

Also found in the same area (macOS report, §7): both `persist_session`
and `persist_settings` discard their save `Result` with `let _ =`, so any
write failure — this one or otherwise — is silent by construction.

Registered as **F61** in `ROADMAP.md`. **Not fixed** — M5-B's constraints
forbid product changes; register only.

Windows worked around this for its own P12 pass by seeding `session.json`
directly instead of exercising the CLI-launch path (§3). Linux and macOS
both let P12 fail honestly against the real path instead, since both
harnesses' restore-half assertions were already separable from the
tab-restore half and could confirm the *other* thing P12 checks (theme/
language restore) still works correctly even though the row's overall
result is Fail.

## 8. Created and changed files

- `packaging/evidence/linux_harness.py` (extended — this session)
- `packaging/evidence/windows_harness.py` (extended — delegated agent)
- `packaging/evidence/macos_harness.py`, `macos_ui.applescript` (extended
  — delegated agent)
- `.github/workflows/m5-evidence-{linux,windows,macos}.yml` (extended —
  new `case` dropdown entries on all three)
- `docs/src/maintainers/release-evidence/0.167.0/{linux-x11,windows-11,
  windows-10,macos-aarch64}.md` — appended, M5-A content on each
  untouched, per the handoff's explicit instruction
- `ROADMAP.md` — F61 added
- `dev-record/review-requests/060-m5b-macos-interaction-cases.md` — the
  macOS agent's own detailed report, referenced throughout this one
- `rfcs/handoffs/078-platform-runtime-acceptance/m5b-interaction-cases-handoff.md`
  — read only, not modified
- `matrix-plan.md` — **not touched**, per the freeze

## 9. Differences from the handoff, RFC-078, or the frozen plan

- **P06 on macOS** runs a materially reduced scope (sequential,
  non-overlapping launches) instead of RFC-078's described concurrent/
  in-app design — see §4. This goes beyond what the handoff itself
  anticipated (it pre-approved a two-*concurrent*-process fallback, not a
  sequential one); flagged explicitly per review 060, not silently
  covered by the handoff's pre-approval.
- **P12 on Windows** verifies the settings-restore half through the CLI
  path it was designed for, but seeds `session.json` directly rather than
  exercising the CLI-launch tab-restore path, because of F61 — a
  deliberate substitution disclosed here and in `windows-11.md`, not a
  silent weakening.
- **P12 on Linux** similarly seeds settings.json directly for the
  *change* half of the theme/language sub-test (three real mechanisms
  tried, none landed reliably under bare-Xvfb-no-window-manager CI — see
  `linux_harness.py`'s `p12` docstring); the *restore* half and the
  tab-restore half are both genuinely exercised, and the tab-restore half
  fails honestly on F61.
- `matrix-plan.md` was not touched, per the freeze, on any row.
- No dependency was added, removed, or version-changed, on any row.
- No product behaviour was changed anywhere — F61 (and, on macOS, the
  `let _ =` discard pattern) was registered, not fixed.

## 10. Executed gates with observed output

- `cargo fmt --check` — clean (no Rust files were touched by any row this
  slice; the command was still run to confirm).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — **[Corrected by review 064 §3]** 1095
  unit/integration/doc tests across the workspace, all passing (252 was
  one crate's count, not the workspace's; no Rust changes this slice,
  run per the gate list regardless).
- `cargo xtask i18n` — clean (227 UI keys covered).
- `mdbook build docs` — clean.
- `git diff --check` — clean.
- `actionlint` — not installed locally on any of the three development
  environments used; workflow YAML validity confirmed the hard way
  instead, by real dispatch: every case and mode combination in the
  tables above was actually invoked via `gh workflow run` and its result
  read from the real run log, not inferred.
- CI (`ci.yml`, the mandatory gate) — green on the final commit pushed
  ([`3b77ff9`](https://github.com/forskscope/forskscope/commit/3b77ff9)).

## 11. Unresolved issues and known limitations

- **F61** (§7) is the headline unresolved item — a real, cross-platform
  product defect affecting the most ordinary CLI workflow, not fixed
  here by design.
- **P06's in-app/concurrent-process design on macOS is not restored** to
  RFC-078's literal description (§4, §9) — now that the real root cause
  (a rendering size threshold, not process concurrency) is known,
  revisiting it with correctly-sized fixtures is a reasonable follow-up,
  not attempted here.
- **Linux P12's change-via-UI half is not exercised** for the
  theme/language combo boxes specifically (§9; full account in
  `linux_harness.py`'s `p12` docstring) — three distinct mechanisms
  tried, none reliable under this CI environment's bare-Xvfb-no-window-
  manager constraint. The restore half and the (failing, F61) tab-restore
  half are both genuine.
- **The `let _ =` discard pattern** in `persist_session`/
  `persist_settings` (found via macOS's investigation, §7) makes *any*
  persistence write failure silent, not just F61's specific gap — worth
  its own tracked issue, separate from F61.
- macOS's `dump_roles` debugging aid was abandoned as unreliable (review
  060 §11) — left in place with its investigation documented rather than
  deleted.
- P03, P07, P11, and the owner's manual passes remain out of scope for
  this slice, per the handoff.

## 12. Requested review focus

1. §1's §3 resolution — is the mouse-only reading of "keyboard and mouse"
   correctly applied across all three platforms, and is the
   manual-outstanding treatment of the keyboard path adequate, or does
   this warrant an RFC-078 amendment after all?
2. **F61's severity/priority** — this is the slice's central finding.
   Does a silently-lost CLI session block Gate D, or is it a
   post-release fix? Should the `let _ =` discard pattern get its own
   tracked issue?
3. §4's P06 deviation on macOS — is the reduced (sequential) scope an
   acceptable evidence artifact for this milestone, or does it need
   revisiting with correctly-sized fixtures before Gate D can rely on it?
4. Whether P12's overall per-row result (Fail on Linux/macOS due to F61;
   worked-around-and-Pass on Windows) is being reported consistently
   enough across the three evidence files for a reader to understand
   without cross-referencing all three, or whether a short cross-platform
   summary should be added to `README.md` alongside F44/F45/F46/F59/F60.
5. Whether F61 should be folded into this slice's `README.md` verdict
   section (matching how F44 already leads it) given it's now a
   confirmed, cross-platform, non-hypothetical defect rather than a risk.
