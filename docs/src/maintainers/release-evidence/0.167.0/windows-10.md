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
| P09 — Mergetool | **Pass** | CI run [`31853496997`](https://github.com/forskscope/forskscope/actions/runs/31853496997) |
| P10 — Binary/XLSX fail-closed policy | **Pass** | CI run [`31853362804`](https://github.com/forskscope/forskscope/actions/runs/31853362804) |

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

## Failures and issue links

None specific to this row. F45 (Windows P01's prerequisite sub-case)
is recorded under `windows-11.md` since it is that row's Required
sub-case, not this row's.

## Waivers

None.
