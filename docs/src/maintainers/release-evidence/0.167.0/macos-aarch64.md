# Platform Evidence — `macos-aarch64`

**Artifact filename:** `forskscope-v0.167.0-macos-aarch64.dmg`
**SHA-256:** `2d66f125f0325adfef36cdf9bbb643a8deed50112db77707f7e6c9970ca25099`
**Source commit:** `cb6f5b6`
**Test date (UTC):** 2026-08-15
**Tester role:** implementer (M5-A), automated via CI (`macos-latest`) — no
manual macOS host exists in the owner's stated execution model
(`matrix-plan.md` §1)
**Host OS and version:** macOS 26.5.2 — **resolved runner image**
`macos26`, image version `20260728.0273.1` (recorded per review 057
§4.3's rolling-label caveat: `macos-latest` itself is not reproducible,
this resolved image is; `Darwin ... RELEASE_ARM64_VMAPPLE arm64` confirms
the runner is aarch64, matching this row's architecture)
**Architecture:** aarch64
**Display server / WebView runtime:** real macOS window server (the
runner is a full GUI-session VM, not headless) + WKWebView
**Install source and prerequisites:** published GitHub Release asset,
downloaded and digest-verified by CI before every case run; `.dmg`
mounted with `hdiutil attach`, `ForskScope.app` copied out, its
`Contents/MacOS/forskscope` binary launched directly

## Cases

| Case | Result | Evidence |
|---|---|---|
| P01 — Install and cold launch | **Pass** | CI run [`31853094639`](https://github.com/forskscope/forskscope/actions/runs/31853094639) |
| P02 — CLI file compare | **Pass** | CI run [`31853203173`](https://github.com/forskscope/forskscope/actions/runs/31853203173) |
| P09 — Mergetool | **Pass** | CI run [`31853204009`](https://github.com/forskscope/forskscope/actions/runs/31853204009) |
| P10 — Binary/XLSX fail-closed policy | **Pass** | CI run [`31853205096`](https://github.com/forskscope/forskscope/actions/runs/31853205096) |

Harness: `packaging/evidence/macos_harness.py` + `macos_ui.applescript`,
driven by `.github/workflows/m5-evidence-macos.yml` (`workflow_dispatch`,
one case per run). Every case downloads and digest-verifies the
**published** artifact itself — nothing here was built from source.

### P01

`--diagnostics` exited 0 and reported `ForskScope 0.167.0`; the harness's
own assertions (not just a printed summary line) confirmed the `OS:` line
contains `macos` and, when a `Home:` line is present, that it is redacted
to `***` rather than a literal path — the harness's stdout only echoes a
one-line summary of these checks, not the full diagnostic report text
verbatim, so this record states what was *asserted and passed*, not a
literal captured string. A plain cold launch (no args) produced a real,
non-blank 1024×677 window, confirmed via macOS's Accessibility API
(System Events).

### P02

`forskscope <left> <right>` rendered the F34 fixture pair
(`left_all_hunk_kinds.txt`/`right_all_hunk_kinds.txt`) as 14 `AXRow`
accessible elements — `hunk.rs`'s `RowLeft`/`RowRight` each independently
carry `role="row"`, so the pinned 7-rows-per-pane fixture produces 14
total once both panes render. This asserts rendered content, not merely a
zero exit code.

### P09

Applied the fixture's first hunk via the "Use this change" button
(clicked through System Events' `AXPress`, addressed by its accessible
description, not synthetic keyboard/mouse input), then clicked the
toolbar's "Save merge result" button the same way. `<merged>`, pre-seeded
with an unmistakable placeholder, was confirmed overwritten with real
content (46 bytes) afterward.

### P10

`forskscope <left.xlsx> <right.xlsx>` (arbitrary bytes; classification is
by extension only, `core::file_kind::classify`) produced the fail-closed
message **"Spreadsheet comparison is temporarily disabled for security."**,
found by walking the accessible tree for that exact text — the message
reaching the user was asserted directly, not merely "no crash occurred."

## Accessibility approach — a deviation from the Linux harness worth noting

The Linux harness (`linux_harness.py`) drives AT-SPI's `Action.do_action`
via `pyobjc`-equivalent Python GTK bindings (`gi.repository.Atspi`), which
are pre-installed via `apt`. macOS has no equivalent pre-installed Python
binding for the Accessibility API (`pyobjc` is not on the `macos-latest`
image and would need a `pip install` step — a new tooling dependency to
manage, and still needs its own accessibility grant separate from
whatever System Events already has). This harness instead drives macOS's
built-in `osascript`/AppleScript against System Events, invoking the same
kind of direct accessible action (`click`/`AXPress`) that AT-SPI's
`do_action` provides on Linux — see `macos_ui.applescript`'s header
comment for the full reasoning. This worked without any special CI setup:
`macos-latest`'s System Events UI scripting was already usable with no
permission wall encountered on any of the eight runs in this pass (see
"Falsifiability" below) — worth flagging as **not fully proven robust**
for a wider ongoing use, since it was observed to work on this specific
resolved image (`macos26`/`20260728.0273.1`) only.

## Falsifiability

Every case's `--break` mode was run and confirmed to fail for the
expected reason, per handoff §7:

| Case | Break-mode run | Result |
|---|---|---|
| P01 | [`31853183079`](https://github.com/forskscope/forskscope/actions/runs/31853183079) | Fail — `--diagnostics output does not start with 'ForskScope 0.0.0-impossible': ForskScope 0.167.0` |
| P02 | [`31853264441`](https://github.com/forskscope/forskscope/actions/runs/31853264441) | Fail — `compare view showed 14 AXRow elements, expected 99, within 45s` |
| P09 | [`31853265706`](https://github.com/forskscope/forskscope/actions/runs/31853265706) | Fail — real merge content (`alpha\nold-line\ngamma\nepsilon\nzeta\ninsert-line\n`) correctly rejected against a value real Save output can never produce |
| P10 | [`31853267021`](https://github.com/forskscope/forskscope/actions/runs/31853267021) | Fail — `could not find text containing 'this message cannot appear' within 45s` |

## F46 — Gatekeeper: Blocked, not Pass, not Waived

Per `matrix-plan.md` §3 and the M5-A handoff, out of scope for this slice
to resolve and not attempted here. `gh release download` and CI's
checkout/download path never apply the quarantine extended attribute a
real browser/`curl`-to-Finder download would, so Gatekeeper never engages
regardless of what this workflow does — the `.dmg` mounted and the
binary launched cleanly in every run above, which is expected and
uninformative about signing/notarization posture, not evidence Gatekeeper
was satisfied. **No manual macOS host exists in the current execution
model, so F46 has no resolution path under current resourcing** — recorded
here as an explicit, unresolved Gate D input, exactly as `matrix-plan.md`
directs.

## Failures and issue links

None found. All four cases pass functionally on `macos-latest`
(`macos26`/`20260728.0273.1`, aarch64) — no P01/P02/P09/P10 product
defect was observed in this pass. F46 (Gatekeeper) remains open per
above, and is not a P01-vs-Pass/Fail case result — it is a separate,
structurally-unverifiable posture question.

## Waivers

None.
