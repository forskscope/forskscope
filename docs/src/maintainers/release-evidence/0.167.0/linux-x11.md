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
