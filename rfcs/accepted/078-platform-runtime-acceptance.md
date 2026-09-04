# RFC 078: Platform Runtime Acceptance and Release Evidence

**Status.** Accepted — review complete; implementer may start. Moves to `done/` when the work ships (RFC-000, 5-folder variant, adopted 2026-09-02).
**Scheduling.** Milestone M5 — **defines Gate D**, the current v1 blocker. Executed from `proposed/` by design. See `ROADMAP.md` § "Remaining proposed RFCs", which must list every file in `proposed/` and `accepted/` and nothing else (F83).
**Tracks.** Release-stabilization audit finding B4.
**Touches.** Release candidate artifacts, GTK/WebKitGTK smoke tests, Windows
and macOS runtime tests, save semantics, evidence records, and v1 go/no-go.

## Summary

ForskScope will distinguish artifact construction from runtime acceptance.
Before a v1 release, the exact release candidate must be executed on supported
platforms using a committed matrix. Results are retained as reviewable Markdown
records with artifact hashes, host/runtime versions, case results, failures,
and time-bounded waivers.

This RFC starts only after RFC-075–077 and the integrated release-core gate
pass. A missing required platform result is a release blocker, not an assumed
pass.

## Goals

- Verify the shipping artifact rather than a developer build where possible.
- Exercise core Compare/Explorer/save/mergetool workflows on real runtimes.
- Confirm WebKitGTK layout and keyboard behavior on a display server.
- Verify platform-specific packaging and clean-install prerequisites.
- Retain concise evidence sufficient for owner and architect audit.
- Resolve platform minimum-version/documentation conflicts from evidence.

## Non-goals

- Automate every visual interaction in this RFC.
- Claim support for platforms outside the published matrix.
- Require signing/notarization if the owner explicitly defers it with accurate
  user guidance; functional launch evidence is still required.
- Store screenshots, user documents, secrets, certificates, or private host
  identifiers in the repository.
- Treat a waiver as a pass.

## Preconditions

- RFC-075–077 acceptance criteria are complete.
- `cargo xtask version-sync` matches the release-candidate version/tag.
- The integrated Gate C in RFC-074 passes.
- Release artifacts are built by the release workflow or a documented
  equivalent from the same commit.
- Every artifact has a SHA-256 digest before testing begins.
- A committed `matrix-plan.md` freezes the exact OS/distribution versions,
  architecture, executor owner/role, host-access status, and applicable case
  IDs for every row before M5 begins.

If a correctness fix lands during the matrix, rebuild all affected artifacts,
record new hashes, and rerun affected cases. Evidence from an older hash cannot
approve a newer artifact.

## Durable evidence layout

**Amended by F56/M4-C4 (2026-08-13).** The original layout below hard-coded
a `vX.Y.Z-rcN/` directory per candidate. Three problems with that, found
when the first real plan tried to use it: this project's tags are
unprefixed (`0.166.0`, not `v0.166.0` — `release.md`'s trigger only
matches the unprefixed form); nothing in Gate D actually requires an RC
number — its real requirements are artifacts built by the release workflow
from a known commit, with every result naming the artifact digest it
tested, and the project's own draft-release mechanism (build, create a
**draft**, tag re-cuttable while in draft, publish is a separate owner
action) already makes the draft *the* candidate without a second numbering
scheme; and naming a directory after a version before that version's
content exists pre-commits a release level the same way `release.md`'s own
post-F21 rule forbids elsewhere.

**Current layout** splits what doesn't change per release from what does:

```text
docs/src/maintainers/release-evidence/
  matrix-plan.md      # standing: hosts, cases, executors — frozen once, read every cut
  advisories.md       # standing: dispositions, policy, upgrade triggers
  <tag>/               # per-cut: created at the cut, named for the tag actually cut
    README.md
    artifacts.md
    linux-wayland.md
    linux-x11.md
    windows-11.md
    windows-10.md
    macos-aarch64.md
```

`README.md` summarizes verdict and links to every required record. Evidence
files contain commands/results and sanitized environment facts, not raw logs
with home paths. Large raw logs and screenshots may live in local review
storage, but the committed record includes their checksum/reference only when
the owner has a durable approved location.

## Evidence record schema

Every platform record contains:

```text
Artifact filename:
SHA-256:
Source commit:
Test date (UTC):
Tester role or review handle:
Host OS and version:
Architecture:
Display server / WebView runtime:
Install source and prerequisites:
Cases: ID, result (Pass/Fail/Waived/Blocked), evidence note
Failures and issue/RFC links:
Waivers: owner, reason, expiry, release impact
```

Do not include operating-system usernames, real home paths, signing identities,
device serials, or customer data. A project role or public review handle is
sufficient for accountability.

## Required platform matrix

| Target | Environment | Required level |
|---|---|---|
| Linux x86_64 | Current supported distribution, Wayland, WebKitGTK 4.1 | Full functional + visual matrix |
| Linux x86_64 | X11 session, WebKitGTK 4.1 | Launch, compare, explorer, save, keyboard |
| Windows x86_64 | Windows 11 with WebView2 | Full functional + packaging/save matrix |
| Windows x86_64 | Windows 10, version 1809+ (F49/F49b: matches `AppxManifest.xml`'s current `MinVersion`, confirmed by the owner — no change) | Launch/prerequisite/save matrix, or narrow published minimum with owner approval |
| macOS aarch64 | macOS 13.0+ (F49: one row, not two, confirmed by the owner — see "Execution model" below) | Launch, compare, save, package/Gatekeeper matrix, full functional where CI can observe it |

One host may satisfy multiple rows only when it genuinely provides the named
runtime/session. Virtual machines are acceptable if file-system and WebView
behavior are representative and recorded.

The descriptive names above are policy targets, not permission to choose hosts
after seeing results. `matrix-plan.md` replaces “current” and “oldest claimed”
with exact versions and records the owner-approved support minimum before the
first artifact is exercised. Missing ownership or unavailable access blocks M5
start and triggers schedule rebaselining; it does not narrow support silently.

P08's three recovery-dialog actions (F37 amendment above) are required on
every row regardless of its stated "Required level" here — a row whose level
is narrower than "Full functional matrix" is not exempt from P08 specifically,
because Exit's platform-specific process-termination path is exactly the kind
of thing a narrower row would otherwise never exercise.

## Execution model (M4-C2 amendment, 2026-08-11)

This RFC originally assumed named humans executing all twelve cases across
six rows. That was never the actual resourcing, and freezing a matrix plan
on top of an assumption the project cannot staff would have produced a
schedule nobody could meet. The owner has stated the real model:

> **Users:** Windows, macOS, Linux.
> **Verification:** GitHub Actions CI on `windows-latest`, `macos-latest`,
> `ubuntu-latest`, plus occasional manual tests on Linux Wayland and
> Windows 11.

This is a better fit than the original text, and F34 (the AT-SPI geometry
rendering check added in `release.yml`'s `linux` job) already proves headless
GUI verification genuinely works under this model — it is not a downgrade in
rigor, it is describing what has already been demonstrated capable.

**What CI covers, concretely:**

- `ubuntu-latest` + `xvfb-run` + `dbus-run-session` (F34's mechanism):
  functional cases that don't require a specific display protocol — this is
  an X11-family virtual framebuffer, so it stands in for the `linux-x11` row
  reasonably, but it is **not** a real Wayland compositor. The `linux-wayland`
  row's Wayland-specific coverage still depends on the stated manual pass.
- `windows-latest`: a Windows Server-based image, not literally a retail
  Windows 10 or 11 install — close enough for save/filesystem/WebView2
  behavior in the common case, but not a substitute for the manual Windows 11
  pass where the two might actually differ (prerequisite state is the sharp
  example, below).
- `macos-latest`: whatever macOS version GitHub currently backs this label
  with — a real Apple Silicon host, not virtualized. This is the **only**
  macOS host in the stated model; there is no manual macOS pass. That is why
  macOS collapses to one row above instead of two: a "current macOS" row with
  no host to run it on is not a row, it is an aspiration, and RFC-000's own
  discipline is to record what is true rather than what was hoped for.

**Two cases are structurally invisible to CI, and a green matrix must not be
read as covering them:**

- **F45 — Windows prerequisites.** `windows-latest` runners ship with the
  VC++ redistributable and WebView2 already preinstalled. A CI launch proves
  the binary runs *on a machine that already has both* — the actual failure
  mode this case exists to catch is a clean machine, and no CI runner is
  one. **P01's prerequisite sub-case on Windows is marked manual-only**; the
  owner's occasional Windows 11 manual pass is what covers it, not CI. A
  Windows CI job passing P01 says nothing about F45 either way.
- **F46 — macOS Gatekeeper.** Gatekeeper only refuses a file carrying the
  quarantine extended attribute, which a real browser/`curl` download applies
  and a CI `actions/checkout` does not. A `macos-latest` job launching a
  locally-built, unquarantined app bundle will succeed regardless of whether
  a real user's download would be refused — the case cannot distinguish
  "signed and notarized" from "unsigned, Gatekeeper just never saw the
  quarantine bit." With no macOS manual host anywhere in the stated model,
  **F46 cannot be verified at all under current resourcing.** This is
  recorded as an explicit open gap with release impact — not waived, not
  marked Pass, not silently absorbed into a green `macos-aarch64` row. Per
  RFC-078's own Waiver policy, "inability to launch on a claimed supported
  platform" is never waivable, and an unverifiable Gatekeeper posture is
  adjacent to that: a real user's first-launch experience on macOS is
  unknown, not merely untested-but-probably-fine. Gate D's evidence for the
  `macos-aarch64` row must state this gap explicitly rather than let a
  passing P01 on `macos-latest` imply it.

## Test corpus

Use repository fixtures copied to an isolated temporary workspace. Never modify
tracked fixtures in place.

- text change: `tests/fixtures/text/left_function.txt` and
  `right_function.txt`;
- identical text;
- CRLF/no-final-newline fixtures;
- binary fixture with comparison disabled;
- large-file fixture triggering the prompt;
- two small temporary directory trees for equal/changed/one-sided states;
- mergetool local/remote/merged temporary files.

Record expected file hashes before destructive cases so backup and output can
be verified objectively.

## Functional cases

### P01 — Install and cold launch

- unpack/install the release artifact into a clean location;
- confirm documented prerequisites are sufficient;
- launch without a source checkout or developer environment;
- Explorer renders without blank window or crash;
- diagnostics reports the expected app version and redacts home data.

### P02 — CLI file compare

- launch the artifact with two fixture paths;
- Loading transitions to Ready;
- changed lines, labels, gutters, and non-color indicators render;
- file contents do not leave the host.

### P03 — Compare layout and scrolling

- short-row backgrounds span the full widest-line area;
- action rows align with left/right rows across multiple hunks;
- vertical rows remain aligned;
- horizontal scrolling mirrors between panes without feedback/jitter;
- word wrap and narrow window modes remain usable.

This is mandatory on WebKitGTK; repeat a basic layout observation on Windows
WebView2 and macOS WebKit.

### P04 — Merge, undo/redo, and safe save

- copy fixtures before opening;
- apply focused hunk with keyboard and mouse;
- verify dirty marker, undo, redo;
- save and verify output hash;
- verify `.bak` bytes equal the pre-save target;
- verify no temp sidecar remains after success.

### P05 — External modification

- open, then change target externally;
- normal save is blocked and external bytes remain;
- cancel preserves dirty session;
- confirmed overwrite backs up the externally changed version;
- Save As does not mutate the original target.

### P06 — Async identity regressions

- open at least two deliberately slow comparisons;
- close a lower-index loading tab;
- verify remaining tabs receive their own contents;
- trigger reload twice and verify the latest request wins.

Deterministic automated tests remain the primary proof; this case confirms
runtime integration.

### P07 — Explorer and directory report

- navigation/history/focused pane keyboard behavior;
- equal/different/one-sided statuses;
- deep comparison progress and filters;
- per-file and batch copy confirmation, backup, manifest, and result summary.

### P08 — Persistence migration

- start from sanitized legacy settings/session fixtures placed in the platform
  config directory;
- verify settings and eligible tabs migrate without loss;
- verify backup and versioned envelope;
- future-schema fixture produces visible incompatibility and is not overwritten;
- corrupt fixture is preserved until explicit reset;
- **F37 amendment (2026-08-11):** this case predates RFC-076's blocking
  recovery dialog (`RecoveryDialogAction`: `Exit`,
  `ContinueWithTemporaryDefaults`, `ContinueWithoutSaving`,
  `ResetAndBackupOriginal`), so the dialog's actions have been verified on
  Linux/WebKitGTK only. Each of the three user-facing choices — **Exit**,
  **Continue** (either variant — both dismiss the dialog and proceed without
  blocking further), and **Reset** — must be exercised on every platform row
  in the matrix, not only where a fixture happens to trigger the dialog for
  an unrelated reason. **Exit is the one that matters most**: it terminates
  the process from inside a modal during startup, invoked while a WebView-hosted
  GUI event loop is running — exactly the kind of path that can hang, leave an
  orphaned process, or behave inconsistently across window toolkits (WebKitGTK,
  WebView2, WKWebView), and Linux-only evidence says nothing about whether it
  does. A platform row is not P08-complete until all three choices have been
  observed to actually resolve the dialog and leave the process in the
  expected state (running normally for Continue, fully exited with no
  orphaned process for Exit, dialog dismissed with the file reset for Reset).

### P09 — Mergetool

- existing merged target saves without false conflict and creates backup;
- missing merged target is created;
- externally changed merged target blocks normal save;
- UI continues to show remote input separately from result destination.

### P10 — Binary permission / XLSX read-only policy

- binary comparison disabled shows localized guidance;
- enabling binary permits read-only preview but not merge/save;
- XLSX structurally compares two workbooks (sheet and cell changes shown in
  the ordinary diff view, RFC-085) and never permits merge/save — comparison
  is restored, read-only is not lifted with it.

### P11 — Keyboard and modal safety

- execute the maintained keyboard checklist;
- modal focus starts on safe/cancel action for destructive operations;
- global shortcuts do not affect the background view while a modal is open;
- Escape behavior is consistent.

### P12 — Session/settings restart

- change theme/language/font and restart;
- open tabs restore only without explicit CLI paths;
- Japanese labels remain covered in practical workflows.

## Platform-specific cases

### Windows

- **F49 amendment (2026-08-11):** four sources disagreed on the minimum
  Windows version — `packaging/windows/AppxManifest.xml`'s `MinVersion`
  (10.0.17763.0, Windows 10 1809), its own `MaxVersionTested` (10.0.19041.0,
  Windows 10 2004 — predating Windows 11 entirely, in tension with this
  RFC's own Windows 11 row below), this table's prior "1903+", and
  `docs/src/users/installation.md`'s formerly-vague "Windows 10 or later."
  `installation.md` now states **Windows 10, version 1809** explicitly,
  matching `AppxManifest.xml`'s `MinVersion` as written — chosen because
  `AppxManifest.xml` may back a live Microsoft Store submission
  (`docs/src/users/installation.md` links one), and changing its declared
  version constraints without knowing that submission's actual state carries
  real risk this slice was not positioned to take. **Resolved by the owner
  (F49b, 2026-08-13): both `MinVersion` and `MaxVersionTested` stay
  unchanged.** Raising the floor excludes users for no demonstrated benefit;
  `MaxVersionTested` records what was *validated*, not what merely
  installs, and no Windows 11 validation evidence exists yet — an earlier
  recommendation to bump it preemptively was reviewed and withdrawn for
  exactly that reason. Both fields are revisited at M5 as an **output** of
  the Windows evidence, to whatever build was actually tested, not chosen
  in advance;
- test overwrite semantics on an existing destination;
- verify backup and temp replacement behavior on NTFS;
- verify long-path behavior according to documented support;
- test Windows 10 without WebView2 or confirm installer/prerequisite messaging;
- inspect zip root/layout and launch executable from the extracted package.

If the current `rename` strategy cannot replace an existing destination on
Windows, that is a release-blocking correctness failure requiring a save RFC
amendment and implementation fix.

### macOS

- **F49 amendment (2026-08-11):** the documentation conflict this RFC
  originally named ("macOS 12" here vs. the package's `LSMinimumSystemVersion`
  13.0) had drifted — "macOS 12" appeared nowhere else in the repository, and
  neither number came from an explicit build-time deployment target
  (`MACOSX_DEPLOYMENT_TARGET` was unset, so the Mach-O's `minos` was whatever
  the build runner's SDK happened to yield — observed as 11.0 on the
  `0.166.0` artifact, a third, undocumented number). `MACOSX_DEPLOYMENT_TARGET`
  is now set explicitly to **13.0** in `release.yml`'s macOS job, matching
  `Info.plist`'s existing `LSMinimumSystemVersion` — the value already
  enforced by macOS Launch Services on every DMG-installed copy today, so
  this reconciliation does not change what already runs. **Confirmed by the
  owner (2026-08-13): 13.0 is settled, no widening.** A future decision to
  widen support to an earlier macOS version requires updating both
  `Info.plist` and this build-time target together, not just one;
- verify DMG opens, app bundle layout, executable launch, and Gatekeeper
  guidance;
- record signing/notarization as Pass, Deferred-with-warning, or Blocked.

### Linux

- **F56/M4-C4 amendment (2026-08-13):** Linux support is **unqualified —
  Windows, macOS, Linux, no per-distribution floor** (owner-confirmed).
  This is a support-breadth statement, distinct from which host tests it
  (`ubuntu-latest` in CI, plus a manual Wayland pass) — see
  `matrix-plan.md` §1a. The direct consequence is F44: the published Linux
  artifact fails to start on libxdo-4 distributions (Arch/CachyOS-family),
  which are supported platforms under this statement, not out-of-scope
  ones. `matrix-plan.md` §3 records how this is represented at M5 without
  either hiding the expected failure or pre-declaring a release No-Go;
- run the maintained GTK checklist on real Wayland and X11 sessions;
- record WebKitGTK/GTK versions;
- verify no blank region, row drift, or missing scrollbars;
- test the packaged binary outside the source tree.

## Security advisory disposition

The release evidence includes all current `cargo audit` warnings. For each
unsoundness advisory, record:

- dependency path;
- whether affected APIs/runtime conditions are reachable;
- upstream issue/version status;
- owner and review date;
- upgrade or removal trigger;
- release decision.

Unmaintained warnings may be grouped by dependency family when they share an
upstream constraint, but unsoundness advisories require individual analysis.

## Waiver policy

A waiver is permitted only when:

- the failed/missing case is not a correctness or user-data safety guarantee;
- user-visible limitations are documented;
- the owner names a responsible follow-up and expiry version/date;
- the architect sees the waiver in the final package.

No waiver may turn these into a release pass:

- wrong-file/stale-load behavior;
- silent settings/session loss;
- unguarded overwrite of the actual save target;
- inability to launch on a claimed supported platform;
- missing runtime evidence for every claimed primary platform.

## Automation

Automate stable checks when practical:

- artifact hash generation and record template creation;
- archive structure and executable presence;
- headless file-output/backup hash verification;
- future-schema preservation;
- mergetool preparation/save tests.

Visual layout and accessibility observations may remain manual for v1, but the
case IDs and results are mandatory.

## Acceptance criteria

- Every required matrix row has an evidence file tied to an exact artifact.
- P01–P12 pass where applicable, or a permitted waiver is recorded.
- Windows save/backup semantics are observed, not inferred from Linux tests.
- WebKitGTK layout/scroll behavior passes on a display server.
- macOS minimum version and signing posture are consistent across package and
  docs.
- Windows WebView2 prerequisite behavior is verified and documented.
- Advisory dispositions are complete.
- The final `README.md` evidence summary states Go/No-Go without interpreting
  missing evidence as success.

## Implementation and execution sequence

1. Approve the matrix and evidence template during RFC review.
2. Complete RFC-075–077 and integrated Gate C.
3. Build/tag a release candidate; record source commit and hashes.
4. Execute Linux Wayland/X11, then Windows and macOS in parallel where hosts
   permit.
5. Fix failures; rebuild and invalidate affected evidence.
6. Complete advisory dispositions and documentation.
7. Produce refreshed handoff and request independent architecture review.

## Dependencies

- Parent: RFC-074.
- Requires RFC-075, RFC-076, RFC-077, and integrated Gate C.
- Extends RFC-010 packaging/QA and RFC-041 release readiness.
