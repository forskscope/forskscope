# RFC-078 M5-A Developer Handoff: Evidence Harness and the Launch/CLI Cases

**Governing RFC.** [RFC-078](../../proposed/078-platform-runtime-acceptance.md), under [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M5-A — the first slice of platform acceptance
**Cases.** P01, P02, P09, P10
**Candidate.** `0.167.0`, published 2026-08-14
**Baseline.** `main` at `f63a054` (`0.167.1` in development)

This handoff directs execution of one slice. It does not redefine RFC-078. The
`matrix-plan.md` is **frozen** — if execution shows a row or case needs to
change, stop and report; amending a frozen row invalidates evidence gathered
under it.

## 1. What M5 is, and what it is not

M5 gathers the runtime evidence that closes — or fails to close — audit blocker
**B4**. Gate D then decides whether v1 can proceed. It is the last substantive
milestone before the go/no-go.

**It is not a release gate for `0.167.0`.** That is already published. M5 tests
those published artifacts because they are what users actually have.

**The deliverable is evidence, not a pass.** A case that fails, recorded
accurately with its cause, is a successful outcome for this milestone. A case
that passes because it was narrowed until it could is a failure of the
milestone, whatever the evidence file says. This is the point where every gate
lesson of the past two months either holds or doesn't.

## 2. Why this is a harness-building slice

The frozen plan makes four of five rows **CI-verified**. So M5 is mostly
*building the automation that executes the cases*, then running it — not
walking a checklist by hand. Budget accordingly.

You already have the pattern: `packaging/render_check.py` downloads/builds an
artifact, runs it under `xvfb-run dbus-run-session`, drives it through AT-SPI,
and asserts. Every Linux case is a variation on that. `render-check.yml`'s
`workflow_dispatch` entry point is the shape the harness should keep — every
case must be runnable on demand, not only in a release run.

## 3. Scope — P01, P02, P09, P10

These four are grouped because none needs in-app interaction beyond launch and
command-line arguments, which makes them the cheapest to automate and the
highest-value to have first. P01 in particular is where F44, F45 and F46 live.

- **P01 — Install and cold launch.** Install from the published artifact the way
  a user would, then start it. Not a build-and-run: **download the published
  asset**, verify its SHA-256 against §5, install/extract, launch.
- **P02 — CLI file compare.** `forskscope <left> <right>` opens a diff of the
  two files. Assert the rendered content, not just a zero exit.
- **P09 — Mergetool.** `forskscope <local> <remote> <merged>`; Save writes to
  `<merged>`. RFC-077's cross-platform guarantee for `persist_noclobber` depends
  on this running on every primary platform, so it is Required on every row.
- **P10 — Binary/XLSX fail-closed policy.** A `.xlsx` is recognised and
  comparison fails closed with a user-visible error. Assert the message reaches
  the user, not merely that no crash occurred.

Out of scope for this slice: P03, P04, P05, P06, P07, P08, P11, P12 — later M5
slices. Do not start them.

## 4. Known outcomes — record, do not work around

Three results are already known. They are why the evidence matters.

- **F44 — Linux P01 fails on libxdo-4 distributions.** The published binary does
  not start there. **Record it as Fail.** RFC-078's waiver policy forbids
  waiving inability to launch on a claimed supported platform, and Linux is
  supported unqualified. Do **not** satisfy Linux P01 by testing only a
  Debian/Ubuntu-family host — that is the laundering this milestone exists to
  prevent. If `dioxus-desktop` releases the fix mid-slice, stop and report
  rather than bumping: a new artifact means new digests and re-run cases.
- **F45 — Windows P01's prerequisite sub-case is manual-only.** CI runners ship
  the VC++ redistributable and WebView2, so CI cannot exercise "these are
  missing." Record what CI *did* establish and mark the sub-case as owner-executed
  and outstanding.
- **F46 — macOS Gatekeeper cannot be verified at all** under current resourcing.
  A CI checkout carries no quarantine attribute, so Gatekeeper never engages.
  Record it as **Blocked**, not Pass and not Waived, with the reason.

## 5. Artifacts and digests

Test the **published** `0.167.0` assets. Record each filename with its SHA-256
in `artifacts.md`, and verify the hash after download rather than trusting the
release page:

```text
forskscope-v0.167.0-linux-x86_64.tar.gz   e17baa26abbb91e5e8e046d3812b08203f0d1ddfd6f8dc9fb9182326ed04bf09
forskscope-v0.167.0-macos-aarch64.dmg     2d66f125f0325adfef36cdf9bbb643a8deed50112db77707f7e6c9970ca25099
forskscope-v0.167.0-windows-x64.zip       bd7c1d9107754f7866639de7d09668fcd0c70ca5669f5cbee15ccdfeca293c1d
```

Source commit is `cb6f5b6`. If any downloaded digest disagrees with the above,
**stop and report** — do not proceed with testing.

## 6. Evidence layout and schema

Under `docs/src/maintainers/release-evidence/0.167.0/`, per F56's structure:

```text
README.md      # what this run is, candidate, dates, status
artifacts.md   # filenames, digests, source commit
linux-wayland.md, linux-x11.md, windows-11.md, windows-10.md, macos-aarch64.md
```

Every platform record carries RFC-078's schema — artifact filename, SHA-256,
source commit, test date (UTC), tester role, host OS and version, architecture,
display server / WebView runtime, install source and prerequisites, per-case
result with an evidence note, failures with links, and any waivers.

Two constraints from the RFC:

- **Record the resolved runner image version**, not the rolling label. A row
  saying `macos-latest` is not reproducible; `macos-14.7.1` is. This is the
  caveat added at review 057 §4.3.
- **No operating-system usernames, real home paths, signing identities, device
  serials, or customer data.** A project role or public handle, not a personal
  identifier.

This slice fills in P01, P02, P09 and P10 rows only; later slices append.

## 7. Falsifiability — the standard still applies

A case that has never been observed failing has not been shown to test anything.
For each of the four cases, demonstrate the assertion failing against a
deliberately broken condition, then revert. Examples, not prescriptions: a
`.xlsx` that *is* parsed would break P10; a mergetool save landing in the wrong
file would break P09.

This matters more here than anywhere previously. A green matrix is the input to
a v1 go/no-go, and F34 has already shown once that a check can be written,
reviewed, and still not detect anything on a real runner.

## 8. Constraints

- `0.165.0`, `0.166.0` and `0.167.0` are published and immutable.
- No dependency is added, removed, or version-changed — **including
  `dioxus-desktop`**, per §4.
- No product behaviour changes. If a case reveals a product defect, **register
  and report it**; fixing it is separate work against a new candidate.
- `matrix-plan.md` is frozen. Report rather than amend.
- Do not weaken a case to make it pass (§1).

## 9. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary — the harness, and how to run each case on demand;
2. cases executed, per row, with results;
3. **the four falsifiability demonstrations, with observed output** (§7);
4. **F44/F45/F46's recorded outcomes** (§4) and the exact wording used;
5. digest verification results (§5);
6. created and changed files;
7. any product defect found, registered not fixed;
8. any difference from this handoff, RFC-078, or the frozen plan;
9. executed gates with observed output;
10. unresolved issues and known limitations;
11. requested review focus.

## 10. After this slice

M5-B covers the interaction cases (P04, P05, P06, P08, P12); M5-C the visual and
navigation cases (P03, P07, P11), the owner's manual passes, and evidence
assembly. Gate D is assessed when the matrix is complete — and on current
knowledge it will not pass while F44 is open, which is a fact about the upstream
release, not about this work.
