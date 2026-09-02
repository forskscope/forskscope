# Platform Evidence — `windows-11` (P01 only)

**Scope.** This is a **targeted P01 record closing F60**, not a Gate D matrix
run for `0.167.2`. Only P01 was executed. Every other case for this row remains
as recorded for `0.167.0`.

**Artifact filename:** `forskscope-v0.167.2-windows-x64.zip`
**SHA-256:** `6be8b8661539e60aa2d5ec05528edf68e51c1ea25ca17b8703c8c6d49328100c`
**Source commit:** `36e8ee2`
**Test date (UTC):** 2026-09-02
**Tester role:** owner, manual
**Host OS and version:** Windows 11, `10.0.26200.0` — a **retail Windows client
install on the owner's own hardware**, not a CI image. This is the distinction
that matters: every prior Windows row on this project, `windows-11` included,
was executed on a Server-based GitHub image (`win25-vs2026`, kernel
`NT 10.0.26100`).
**Architecture:** x86_64
**Display server / WebView runtime:** the host's own interactive desktop
session; WebView2 as preinstalled by Windows 11, nothing added for the test
**Install source and prerequisites:** published GitHub Release asset,
downloaded with `curl.exe` and digest-verified on the host with `Get-FileHash`
before extraction. No source checkout, no developer toolchain, no prerequisite
installed to make it run.

## Cases

| Case | Result | Evidence |
|---|---|---|
| P01 — Install and cold launch | **Pass** | Digest verified on host; launched from the extracted zip; Explorer panes rendered in both sides; Settings → Diagnostics reported `0.167.2`, matching the artifact |

P02–P12: **not executed in this record.** See `0.167.0/windows-11.md`.

## What this settles

**F60.** The finding was that the oldest Windows the project *claimed to
support* had never been observed running the application. Two things close it
together: the user-facing support claim is corrected to name Windows 11 as the
tested platform (`installation.md`), and this record supplies that
platform's first evidence from a real client OS rather than a Server image.

It also removes the objection recorded against the owner's 2026-08-18 manual
pass — that it ran against the superseded `0.167.0`. This one is against the
current published artifact, with the digest verified on the host.

**`MinVersion` is unchanged** and stays `10.0.17763.0`. It is an installability
floor, not a support claim; F49b (2026-08-13) decided that correctly and is
confirmed.

## Observation — not a P01 failure

Diagnostics reported **`Rust: unknown`** where a toolchain version is expected.
Version, OS, architecture and CPU count were all populated correctly, and P01
requires only that diagnostics report the expected app version and redact home
data — both of which held, so **this does not affect the Pass**.

But diagnostics exists to make bug reports useful, and if released builds always
report `unknown` here, every report from a real user is missing the toolchain
that built the binary. **Recorded as a follow-up to verify against a local
build; not investigated as part of this record.**

## Limitations of this record

Manual and unharnessed, like the 2026-08-18 pass. That is appropriate for P01 —
which is specifically *"launch without a source checkout or developer
environment"*, something a CI harness cannot honestly demonstrate about itself —
and it is **not** a basis for granting any other case a pass.

**Windows 10 was not tested and will not be** (owner decision, 2026-09-02:
limited time and resource). The artifact remains installable on 1809 or later
per the manifest floor; the documentation states that it is untested there
rather than claiming support.
