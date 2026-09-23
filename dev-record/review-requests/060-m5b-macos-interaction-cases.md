# Review Request: M5-B (macOS row) — The Interaction Cases

**Date:** 2026-08-15
**Reviewer stance:** §3's resolution and P08's Exit process-state evidence
are the highest-stakes parts of this slice; the newly-found session-
persistence defect is the highest-value finding
**Repository baseline:** `2558cb3` (per the handoff)
**Governing document:** `rfcs/handoffs/078-platform-runtime-acceptance/m5b-interaction-cases-handoff.md`
**Scope:** `macos-aarch64` row only (P04, P05, P06, P08, P12) — Linux/Windows
rows for this same milestone are a separate effort, not covered here.

## 1. §3's resolution — P04's "keyboard and mouse", settled before execution

Per the handoff's explicit instruction to settle this before running P04:
**the mouse path is CI-verifiable and was executed; the keyboard path is
structurally not CI-verifiable, on any platform, and was not attempted.**

The hunk's "Use this change" button (`aria_label: "Use this change (apply
left to right)"`, `crates/forskscope-ui/src/ui/view/hunk.rs`) has a real
`onclick` handler. Invoking it via `AXPress` calls the exact same handler
a real mouse click would — the same equivalent-path argument M5-A already
used successfully for the Save button. The Enter-key shortcut
(`Key::Enter => apply_focused_hunk(...)` in `crates/forskscope-ui/src/
app.rs`'s global `onkeydown`) is a raw keydown listener bound to no
actionable UI element at all — there is no accessible "action" to invoke
for it via any accessibility API, on any platform. This is a different
situation in kind from M5-A's Linux input-synthesis struggles (where the
*mechanism* to deliver a real key event existed but was unreliable under
bare Xvfb); here no accessibility mechanism exists at all to exercise a
global keydown handler.

This is option 1 from the handoff's own framing: the case's intent is
"both invocation paths reach the same handler", demonstrated via the
mouse path, with the keyboard path recorded as manual-outstanding
(mirroring F45's shape) rather than silently skipped or falsely claimed
covered. RFC-078's text was not amended — this reading doesn't require
it, per the handoff's own note that it's the same kind of CI-scope
narrowing F45 already uses.

## 2. Implementation summary — how each case runs on demand

Extends M5-A's existing files, per the handoff's instruction not to
restructure them:

- `packaging/evidence/macos_harness.py` — new case functions
  (`p04`, `p05`, `p06`, `p08_exit`, `p08_continue`, `p08_reset`, `p08_fs`,
  `p12`), new shared polling helpers (`poll_ui`, `wait_rows`,
  `click_wait`, `find_wait`, `pid_target`), all added alongside M5-A's
  four case functions, which are untouched.
- `packaging/evidence/macos_ui.applescript` — new commands
  (`click_row`, `click_row_side`, `click_any`, `click_button_exact`,
  `get_value`, `set_value`, `perform_action`, `select_popup_item`,
  `type_into`, `close_window`, plus a `dump_roles` debugging aid that was
  ultimately abandoned — see §10), a `safeContents`/`flatWalk` fallback
  and a bounded retry wrapper around the whole dispatcher, added
  alongside M5-A's four commands, which are untouched.
- `.github/workflows/m5-evidence-macos.yml` — new `case` dropdown entries
  for the above, plus several `recon_*` diagnostic-only entries used
  during development (not scored cases; documented in §5 and §9, left in
  place as they cost nothing and may help future debugging).

Run any case on demand:

```sh
gh workflow run "M5 Evidence - macOS" --ref main -f case=p08_exit -f break_case=false
gh workflow run "M5 Evidence - macOS" --ref main -f case=p12 -f break_case=true
```

## 3. Cases executed, per case, with results

| Case | Normal-mode result | Break-mode result |
|---|---|---|
| P04 | **Pass** — [`31879443866`](https://github.com/forskscope/forskscope/actions/runs/31879443866) | **Fail (expected)** — [`31880080196`](https://github.com/forskscope/forskscope/actions/runs/31880080196) |
| P05 | **Pass** — [`31879848097`](https://github.com/forskscope/forskscope/actions/runs/31879848097) | **Fail (expected)** — [`31880082912`](https://github.com/forskscope/forskscope/actions/runs/31880082912) |
| P06 | **Pass** (reduced scope — see §4) — [`31883446824`](https://github.com/forskscope/forskscope/actions/runs/31883446824) | **Fail (expected)** — [`31883530193`](https://github.com/forskscope/forskscope/actions/runs/31883530193) |
| P08 — Exit | **Pass** — [`31879559456`](https://github.com/forskscope/forskscope/actions/runs/31879559456) | **Fail (expected)** — [`31880085680`](https://github.com/forskscope/forskscope/actions/runs/31880085680) |
| P08 — Continue with defaults | **Pass** — [`31879563832`](https://github.com/forskscope/forskscope/actions/runs/31879563832) | **Fail (expected)** — [`31880089230`](https://github.com/forskscope/forskscope/actions/runs/31880089230) |
| P08 — Reset and back up | **Pass** — [`31879567945`](https://github.com/forskscope/forskscope/actions/runs/31879567945) | **Fail (expected)** — [`31880092086`](https://github.com/forskscope/forskscope/actions/runs/31880092086) |
| P08 — legacy migration (fs-only) | **Pass** — [`31879767594`](https://github.com/forskscope/forskscope/actions/runs/31879767594) | **Fail (expected)** — [`31880096494`](https://github.com/forskscope/forskscope/actions/runs/31880096494) |
| P12 | **Fail — real product defect (§7)** — [`31881394791`](https://github.com/forskscope/forskscope/actions/runs/31881394791) | **Fail (expected, on the settings-restore assertion)** — [`31883545108`](https://github.com/forskscope/forskscope/actions/runs/31883545108) |

Full narrative for every case is in
`docs/src/maintainers/release-evidence/0.167.0/macos-aarch64.md`'s new
M5-B sections — this request summarizes, that file is the record.

## 4. P06 — a real deviation from RFC-078's description, disclosed plainly

RFC-078 describes P06 as opening a second compare *in-app* while the
first is still loading, in one process. **That is not what the passing
run does.** Three shapes were tried against real CI, in order:

1. **In-app** (the literal description): switch to Explorer while tab 0
   loads, pick a second pair there, open it as tab 1, close tab 0
   mid-load. Hung 45s+ in accessibility queries against Explorer whenever
   another tab's load was in progress, reproducibly, across every
   fixture size tried.
2. **Two concurrent processes** (the handoff's own pre-approved
   fallback: "launch a second `<binary>` process... while the first is
   mid-load"). Still unreliable even after ruling out process-identity
   ambiguity outright via unambiguous PID-based addressing (`"pid:<n>"`
   in `macos_ui.applescript`) — "by pid" and "by name" queries returned
   identical (wrong) results.
3. **What the passing run actually does**: two launches that never
   overlap in time. Process A launches, gets a short real load window,
   and is terminated before process B ever starts; process B is then
   interacted with alone.

The real root cause of (1) and (2)'s failures was found only after
retreating to (3), via `recon_generated_pair_alone` (an isolated
diagnostic, not a scored case): a generated diff pair's content never
reached the accessibility tree once the file crossed a size threshold
between 30 and 100 lines — binary-searched with nothing else running (10
and 30 lines: renders correctly; 100 and 400: does not, consistently,
even after 60s+ of polling). This was never a process-concurrency problem
at all; (1) and (2) were almost certainly failing for this same reason.
**Revisiting the in-app design with correctly-sized (≤30-line) fixtures
is a reasonable, not-yet-attempted follow-up** — flagged in §9.

What the passing (3) actually verifies: process A exits cleanly after a
real load window; process B, launched only afterward, shows its own
correct content (the falsifiable check per handoff §6 — not a bare
"still running" check); two rapid reloads within process B end with the
second reload's content displayed, not the first's (`reload_tab`'s
`LoadToken` exercised for real). It **cannot** exercise concurrent-process
coexistence or in-process async-task-identity confusion. Recorded as
**Pass for this reduced scope**, not for RFC-078's original description —
flagging this explicitly per the instruction not to weaken a case
silently. Per review 056 §5.3 (referenced in the handoff), this should be
treated as informative context, not a clean pass, for any future P06
spot-check on this platform.

## 5. P08's three-choice evidence, including process state (handoff §4)

All three recovery-dialog choices exercised, each on a fresh, isolated
`$HOME` (`$HOME/Library/Application Support/forskscope`, confirmed
against `dirs-next`'s own source — the `HOME` env var, not a native
macOS API call, so overriding it for the launched subprocess genuinely
isolates config resolution):

| Choice | Fixture | Expected state | Confirmed how |
|---|---|---|---|
| Exit | future-schema session (`Incompatible`) | fully exited, no orphan | `proc.poll()` (direct, non-racy child-reap) returned a real return code; corroborated by `os.kill(pid, 0)` reporting no such process |
| Continue with defaults | same future-schema fixture | running normally, nothing written | app still running post-click; `session.json` bytes confirmed byte-for-byte unchanged |
| Reset and back up | corrupt session (`CorruptPreserved`) | file reset, original backed up | `session.json` no longer holds the corrupt bytes; `session.json.reset.bak` confirmed to hold the *original* corrupt bytes exactly |

**Exit's process-state mechanism, precisely** (per handoff §4's explicit
warning that "the dialog closed" alone is a vacuous check): the harness
holds the `subprocess.Popen` handle for the launched app and, after
clicking Exit, polls `proc.poll()` — which reads this exact child
process's own wait status via this Python process's own `waitpid` call,
so there is no PID-reuse ambiguity a separate `ps`/`kill -0` query
against an arbitrary PID could have. `proc.poll()` returned a real,
non-`None` return code (`rc=0`) within the poll window. `os.kill(pid, 0)`
was additionally checked immediately afterward (before any PID reuse
could plausibly occur) and confirmed `OSError` (no such process) —
recorded as corroborating, not primary, evidence. The `--break` run
asserted the (impossible) opposite — that the process would still be
running after Exit — and correctly failed, since real behaviour has it
exit: [`31880085680`](https://github.com/forskscope/forskscope/actions/runs/31880085680),
`FAIL (expected, --break): process (pid 27125) exited with returncode 0
within 20s of clicking Exit`.

## 6. Falsifiability demonstrations with observed output

Every case's `--break` mode was run and confirmed to fail for the
expected reason — full table in
`docs/src/maintainers/release-evidence/0.167.0/macos-aarch64.md`'s
extended Falsifiability section; the two the handoff specifically flagged
as easy to write vacuously:

- **P06**: `--break` asserts the *first* (stale) reload's sentinel is
  what's displayed, instead of the second's — false under real
  (last-reload-wins) behaviour. Observed:
  `FAIL — expected final sentinel 'ASYNC-IDENTITY-SENTINEL-PAIR-B-RELOAD-V1' never appeared`
  (the impossible expectation was correctly not satisfied; real behaviour
  shows the second reload's content).
- **P08 — Exit**: see §5 above. Observed:
  `FAIL (expected, --break): process (pid 27125) exited with returncode 0
  within 20s of clicking Exit - the real behaviour --break's impossible
  expectation ('still running') was checked against.`

## 7. Product defect found, registered, not fixed

**Session state opened via CLI startup is never persisted if the tab is
never closed before the app quits.** Root cause: `app.rs`'s tabs-changed
`use_effect` (meant to auto-persist the session on every `store.tabs`
mutation) does not actually write, while direct (non-reactive) save call
sites — `close_tab` in `crates/forskscope-ui/src/state/session.rs` — do,
in the identical environment.

Isolated via `recon_session_save` (a dedicated, not-scored diagnostic
added to `macos_harness.py`):

1. `forskscope <left> <right>` opens a tab. `session.json` confirmed
   **absent** 1.5s after the tab fully renders.
2. Clicking the tab's Close button (`close_tab`, a direct call, not the
   reactive effect) — `session.json` appears correctly right afterward,
   same process, same environment.

Two hypotheses were investigated and ruled out first (both real findings
in their own right, kept in the report for anyone revisiting this):

- Missing config directory (`dirs_next::config_dir()/forskscope` is never
  created by anything in the shipped binary — confirmed by grepping the
  whole crate) — pre-creating it did not fix the symptom, ruling this out
  as the *sole* cause. Independently real and worth fixing: both
  `persist_session` (`state/session.rs`) and `persist_settings`
  (`ui/view/settings.rs`) discard `repo.save(...)`'s `Result` with
  `let _ =`, so any write failure is silent by construction, directory-
  related or not.
- App Sandbox entitlements — ruled out via `codesign -d --entitlements`
  (empty; unsigned, no sandbox).

**Practical impact:** `forskscope <left> <right>` from the command line,
quit without touching the tab list — the single most ordinary CLI/
`git difftool`-style workflow — loses that session silently. Settings
persistence (a separate, synchronous, non-reactive call path) is
unaffected.

**Not fixed in this pass**, per the handoff's explicit instruction.

## 8. Created and changed files

- `packaging/evidence/macos_harness.py` (extended)
- `packaging/evidence/macos_ui.applescript` (extended)
- `.github/workflows/m5-evidence-macos.yml` (extended — new `case` options)
- `docs/src/maintainers/release-evidence/0.167.0/macos-aarch64.md` (M5-A's
  header fields and P01/P02/P09/P10 sections untouched; new M5-B `## Cases`
  rows, `### PXX` sections, Falsifiability rows, and clearly-labeled new
  `## M5-B — ...` sections added)
- `rfcs/handoffs/078-platform-runtime-acceptance/m5b-interaction-cases-handoff.md`
  (read only, not modified)
- `matrix-plan.md` — **not touched**, per the handoff's freeze

## 9. Differences from the handoff, RFC-078, or the frozen plan

- **P06** runs a materially reduced scope (sequential, non-overlapping
  launches) instead of RFC-078's described concurrent/in-app design — see
  §4. This is the one deviation that goes beyond what the handoff itself
  anticipated (it pre-approved a two-*concurrent*-process fallback, not a
  sequential one); flagging it explicitly for review rather than treating
  the handoff's pre-approval as covering this narrower variant too.
- `matrix-plan.md` was not touched, per the freeze.
- No dependency was added, removed, or version-changed.
- No product behaviour was changed — the defect in §7 was registered, not
  fixed.

## 10. Executed gates with observed output

- `cargo fmt --check` — clean (no Rust files were touched this slice; the
  command was still run to confirm, output empty).
- `actionlint` — not installed on this machine; the workflow YAML's
  validity was instead confirmed the hard way, by real dispatch: every
  case and `recon_*` diagnostic in this report was actually invoked via
  `gh workflow run` and observed to parse and execute (dozens of real
  dispatches across this slice's development — see the git log for the
  iteration history).
- Every case above was dispatched in **both** normal and `--break` mode
  on real `macos-latest` CI runs, and results were read from the actual
  run logs, not inferred — per the handoff's explicit instruction not to
  trust code-by-inspection.

## 11. Unresolved issues and known limitations

- **P06's in-app/concurrent-process design is not restored.** Now that
  the real root cause (fixture size, not process concurrency) is known,
  revisiting either the in-app or two-concurrent-process shape with
  correctly-sized (≤30-line) fixtures could plausibly restore full
  fidelity to RFC-078's description. Not attempted in this pass given the
  time already spent finding the root cause.
- **`dump_roles`** (a full accessible-tree-dump debugging aid added early
  in this slice) was never made reliable — it hit "AppleEvent handler
  failed (-10000)" under several different hypotheses (properties-of-e
  bulk fetch, an unconditional `enabled of e` query, and others), none of
  which fully explained it. Abandoned in favour of targeted `find_text`/
  `click_button`-family probes, which remained reliable throughout. Left
  in `macos_ui.applescript` with its investigation documented in comments
  rather than deleted, in case a working variant is found later.
- **P12's explicit-CLI-args-does-not-restore-old-tabs check** was
  implemented but not independently re-verified after the tab-restore
  defect was found (§7) — there was nothing in the session file to
  *not*-restore in that run. Flagged as an open item.
- **The `let _ =` discard pattern** in `persist_session`/
  `persist_settings` (§7) makes *any* persistence write failure silent,
  not just the specific defect found here. Worth its own fix, separate
  from the specific reactive-effect gap.
- The macOS-specific accessibility findings in §9 of
  `macos-aarch64.md` ("M5-B — accessibility technique findings") are
  recorded for reuse by any future macOS case work, but several remain
  not-fully-explained (why text vs. number `<input>`s behave differently
  for a direct `AXValue` write, in particular).

## 12. Requested review focus

1. §1's §3 resolution — is the mouse-only reading of "keyboard and mouse"
   correctly applied, and is the manual-outstanding treatment of the
   keyboard path adequate, or does this warrant an RFC-078 amendment
   after all?
2. §4's P06 deviation — is a sequential, non-overlapping two-launch
   variant an acceptable evidence artifact for this milestone, or does it
   need to be redone with correctly-sized fixtures (now that the real
   fixture-size root cause is known) before Gate D can rely on it?
3. §7's defect — severity/priority assessment for the session-persistence
   gap, and whether the `let _ =` discard pattern warrants its own
   tracked issue independent of this specific reactive-effect bug.
4. Whether P12's overall row result (Fail, due to §7) is the right
   Gate D input, or whether the passing half (Theme/Language restore)
   should be recorded/weighted separately somehow.
