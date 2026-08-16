# Platform Evidence — `windows-10`

**Artifact filename:** `forskscope-v0.167.0-windows-x64.zip`
**SHA-256:** `bd7c1d9107754f7866639de7d09668fcd0c70ca5669f5cbee15ccdfeca293c1d`
**Source commit:** `cb6f5b6`
**Test date (UTC):** 2026-08-15
**Tester role:** implementer (M5-A), automated via CI
**Host OS and version:** Windows Server-based CI image — **resolved
runner image** `win25-vs2026`, image version `20260810.198.2`, kernel
`Microsoft Windows NT 10.0.26100.0`. Per `matrix-plan.md`'s row text,
this is not a literal Windows 10 (1809+) install — it is the same
`windows-latest` host `windows-11.md` records, used as the plan's stated
CI stand-in for this row's save/filesystem/WebView2 behavior.
**Architecture:** x86_64
**Display server / WebView runtime:** a real interactive Windows desktop
session + WebView2 (same host as `windows-11.md`)
**Install source and prerequisites:** published GitHub Release asset,
downloaded and digest-verified by CI before every case run

## Row-specific note (matrix-plan.md)

`matrix-plan.md` states this row's verification method plainly:
*"CI — `windows-latest` is a Server-based image, not a literal retail
Win10/11 install, but stands in reasonably for save/filesystem/WebView2
behavior."* There is no separate Windows 10 CI runner available from
GitHub Actions, and no manual Windows 10 host in the owner's stated
execution model (§1a). This row's evidence is therefore identical to
`windows-11.md`'s CI results — the same four cases, the same runs, the
same falsifiability demonstrations — recorded here as its own file per
F56's schema rather than a cross-reference, because the frozen plan
treats this as a distinct row with its own required verification, not as
an alias.

**What this does and doesn't establish:** the `AppxManifest.xml`
`MinVersion` commitment to Windows 10 1809+ (F49b) is a packaging/manifest
claim, not something a `windows-latest` run can verify directly — this
evidence establishes that the CLI/mergetool/fail-closed behaviors work on
a current Windows runtime, standing in for the 1809+ claim to the extent
the plan already accepts. It does not establish that ForskScope actually
launches on a real, period-accurate Windows 10 1809 installation; no such
host was available under this milestone's resourcing (see `matrix-plan.md`
§1a on test-host vs. supported-platform breadth).

## Cases

| Case | Result | Evidence |
|---|---|---|
| P01 — Install and cold launch | **Pass** | CI run [`31853150147`](https://github.com/forskscope/forskscope/actions/runs/31853150147) |
| P02 — CLI file compare | **Pass** | CI run [`31853258477`](https://github.com/forskscope/forskscope/actions/runs/31853258477) |
| P04 — Merge, undo/redo, safe save | **Pass** (mouse path) / **Not executed** (keyboard path — see `windows-11.md`) | CI run [`31878228535`](https://github.com/forskscope/forskscope/actions/runs/31878228535) |
| P05 — External modification | **Pass** | CI run [`31878276750`](https://github.com/forskscope/forskscope/actions/runs/31878276750) |
| P06 — Async identity | **Pass** | CI run [`31879158929`](https://github.com/forskscope/forskscope/actions/runs/31879158929) |
| P08 — Persistence migration and recovery | **Pass** | CI run [`31878490123`](https://github.com/forskscope/forskscope/actions/runs/31878490123) |
| P09 — Mergetool | **Pass** | CI run [`31853496997`](https://github.com/forskscope/forskscope/actions/runs/31853496997) |
| P10 — Binary/XLSX fail-closed policy | **Pass** | CI run [`31853362804`](https://github.com/forskscope/forskscope/actions/runs/31853362804) |
| P12 — Session/settings restart | **Fail — real path not exercised (F61)**; theme/language/font restore itself passes | CI run [`31880416324`](https://github.com/forskscope/forskscope/actions/runs/31880416324) |

Harness: `packaging/evidence/windows_harness.py`, driven by
`.github/workflows/m5-evidence-windows.yml`. See `windows-11.md` for the
per-case narrative (`--diagnostics` output, window dimensions, message
text observed) — reproduced here would be a verbatim duplicate of the
same run's log.

### P01

`--diagnostics` reported `ForskScope 0.167.0`, `OS: windows`, and a
redacted `Home: C:\Users\***`; cold launch produced a real 1044×788
window. No launch-blocking issue was observed — this row does not have a
Linux-style F44/F59 finding.

## M5-B — the interaction cases

P04, P05, P06, P08, P12: same CI runs as `windows-11.md`, since both
rows share one CI host (`windows-latest`) per `matrix-plan.md`. See
`windows-11.md`'s M5-B section for the full per-case narrative
(behavior observed, deviations reported, the config-directory reasoning
for P08/P12) — reproduced here would be a verbatim duplicate of the same
runs' logs. P04's keyboard path is not executed, for the same reason
recorded there (`app.rs`'s Enter-key shortcut has no bound UI element
for any accessibility API to invoke).

## Falsifiability

Every case's `--break` mode was run and confirmed to fail for the
expected reason (same runs as `windows-11.md`, since both rows share one
CI host):

| Case | Break-mode run | Result |
|---|---|---|
| P01 | [`31853199727`](https://github.com/forskscope/forskscope/actions/runs/31853199727) | Fail — `--diagnostics output does not start with 'ForskScope 0.0.0-impossible': ForskScope 0.167.0` |
| P02 | [`31853350190`](https://github.com/forskscope/forskscope/actions/runs/31853350190) | Fail — `compare view never rendered these expected tokens within 60s: ['this-token-cannot-appear-in-real-output']` |
| P09 | [`31853592413`](https://github.com/forskscope/forskscope/actions/runs/31853592413) | Fail — content check correctly rejected a value real Save output can never produce |
| P10 | [`31853491433`](https://github.com/forskscope/forskscope/actions/runs/31853491433) | Fail — `could not find text containing 'this message cannot appear' within 60s` |
| P04 | [`31878264147`](https://github.com/forskscope/forskscope/actions/runs/31878264147) | Fail — saved content check correctly rejected a value real Save output can never produce |
| P05 | [`31878529324`](https://github.com/forskscope/forskscope/actions/runs/31878529324) | Fail — `.bak` content check correctly rejected a value this harness never externally writes |
| P06 | [`31878786557`](https://github.com/forskscope/forskscope/actions/runs/31878786557) | Fail — process B correctly never showed process A's token |
| P08 | [`31878790194`](https://github.com/forskscope/forskscope/actions/runs/31878790194) | Fail — §6's specifically-flagged Exit risk, demonstrated: requiring the impossible (process still running) correctly fails |
| P12 | [`31880463459`](https://github.com/forskscope/forskscope/actions/runs/31880463459) | Fail — header button never showed the impossible required label after restart |

M5-B's cases needed real iteration against actual CI output before
passing (P06, P08, and especially P12) — see `windows-11.md`'s
Falsifiability section for the full explanation; the same CI runs are
recorded here since both rows share one host.

## Failures and issue links

None specific to this row. F45 (Windows P01's prerequisite sub-case)
is recorded under `windows-11.md` since it is that row's Required
sub-case, not this row's. P04's keyboard path, the candidate product
defect found while building P12 (a 2-arg CLI-launched compare's tab is
never persisted to `session.json` — see `windows-11.md`'s Failures
section for the full detail, not fixed here), and the two other
discovered (not invented) app behaviors noted in `windows-11.md`'s M5-B
section (Toolbar hidden during `TabState::Loading`; tabs referencing
nonexistent paths pruned by `restore_tabs` + auto-save) apply identically
here, since this row runs the identical CI evidence — recorded under
`windows-11.md` to avoid duplication, not omitted.

## M5-C — P03, P07, P11

Same CI runs as `windows-11.md`, since both rows share one CI host
(`windows-latest`) per `matrix-plan.md`. See `windows-11.md`'s M5-C
section for the full per-case narrative (Prerequisite B resolution,
falsifiability demonstrations, and both candidate defects) — reproduced
here would be a verbatim duplicate of the same runs' logs.

| Case | Result | Evidence |
|---|---|---|
| P03 — Compare layout and scrolling | **Pass** | CI run [`31937225763`](https://github.com/forskscope/forskscope/actions/runs/31937225763) |
| P07 — Explorer and directory report | **Fail — candidate product defect, blocks the whole case (see `windows-11.md`)** | CI run [`31938459272`](https://github.com/forskscope/forskscope/actions/runs/31938459272) |
| P11 — Keyboard and modal safety | **Fail — candidate product defect (see `windows-11.md`)** | CI run [`31938755692`](https://github.com/forskscope/forskscope/actions/runs/31938755692) |

## M5-C falsifiability

Same runs as `windows-11.md`, since both rows share one CI host:

| Case | Break-mode run | Result |
|---|---|---|
| P03 | [`31937274686`](https://github.com/forskscope/forskscope/actions/runs/31937274686) | Fail (expected) — row geometry required an impossible misalignment, correctly found none |
| P07 | Not reached — the case fails before any `--break`-gated assertion | N/A |
| P11 | [`31938879519`](https://github.com/forskscope/forskscope/actions/runs/31938879519) | Fail — same real-defect reason as normal mode, not `--break`'s own branch — see `windows-11.md` |

## M5-C failures and issue links

Both candidate defects noted under `windows-11.md`'s M5-C section (the
Explorer directory-listing blocker, and the `autofocus` focus-position
defect) apply identically here, since this row runs the identical CI
evidence — recorded under `windows-11.md` to avoid duplication, not
omitted.

## M5-C waivers

None.

## Waivers

None.
