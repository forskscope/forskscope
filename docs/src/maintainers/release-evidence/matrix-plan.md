# Platform Runtime Matrix Plan

**Governing RFC:** RFC-078 (Platform Runtime Acceptance and Release
Evidence), P08 as amended by F37 (2026-08-11), execution model and macOS row
count as amended by F49/M4-C2 (2026-08-11), evidence layout restructured and
plan frozen by F56/M4-C4 (2026-08-13).

**Status: FROZEN as of 2026-08-13.** Hosts, cases, and executors below are
fixed. Changing any of them after M5 begins invalidates evidence already
gathered under this plan — a row re-planned mid-matrix is not the row that
produced the evidence collected so far. This is a **standing document**: it
lives at `release-evidence/matrix-plan.md`, not inside a per-cut directory,
because the plan itself (hosts, cases, executors) does not change per
release the way results do — see F56 and RFC-078's "Durable evidence
layout" for the reasoning. Per-cut results (`artifacts.md`, platform
records) live under `release-evidence/<tag>/`, created at the actual cut.

**This plan does not execute any case.** Running the matrix — including the
CI-automated rows, once the release workflow actually produces a tagged
candidate — is M5's work.

---

## 1. Rows

Five rows, per RFC-078's "Durable evidence layout" file list
(`linux-wayland.md`, `linux-x11.md`, `windows-11.md`, `windows-10.md`,
`macos-aarch64.md`) — matching RFC-078's own "Required platform matrix"
table, **now also five rows** after the F49/M4-C2 amendment resolved the
internal inconsistency review 056 flagged (the table previously split macOS
into "oldest claimed" and "current" as two rows sharing one evidence file).
Resolution: there is no manual macOS host in the owner's stated execution
model — `macos-latest` CI is the *only* macOS access this project has — so a
second, CI-inaccessible "current macOS, full matrix" row was never
executable under current resourcing. One row, matching the one real host.

| Row | Target | Exact OS version | Architecture | Verification method | Executor (owner/role) | Host-access status |
|---|---|---|---|---|---|---|
| `linux-wayland` | Linux x86_64, Wayland, WebKitGTK 4.1 | Unqualified — no per-distribution floor (owner, 2026-08-13; §1a below) | x86_64 | **Manual** — CI's Xvfb is X11-family, not real Wayland | Owner | Available — owner-executed |
| `linux-x11` | Linux x86_64, X11, WebKitGTK 4.1 | Unqualified — no per-distribution floor (owner, 2026-08-13; §1a below) | x86_64 | **CI** — `ubuntu-latest` + `xvfb-run`/`dbus-run-session` (F34's mechanism) stands in for this row reasonably | GitHub Actions (CI) | CI — available now |
| `windows-11` | Windows 11 with WebView2 | Windows 11, any build GitHub's `windows-latest` currently backs | x86_64 | **CI** for most cases, **Manual** for F45's prerequisite sub-case (§3) | CI: GitHub Actions; Manual (F45 sub-case): Owner | CI available now; Manual: available — owner-executed |
| `windows-10` | Windows 10, version 1809+ | **1809** — settled (F49b, owner 2026-08-13: `AppxManifest.xml`'s `MinVersion` stays unchanged) | x86_64 | **CI** — `windows-latest` is a Server-based image, not a literal retail Win10/11 install, but stands in reasonably for save/filesystem/WebView2 behavior | GitHub Actions (CI) | CI — available now |
| `macos-aarch64` | macOS 13.0+ | **13.0** — settled (owner, 2026-08-13: already enforced by `Info.plist` and `MACOSX_DEPLOYMENT_TARGET`) | aarch64 | **CI only** — `macos-latest`; **F46 (Gatekeeper) cannot be verified under this model at all** (§3) | GitHub Actions (CI) | CI — available now |

**§1a — supported platforms versus test hosts (owner, 2026-08-13).**
ForskScope supports **Windows, macOS, and Linux, unqualified** — there is
no per-distribution Linux floor, and this plan does not imply one by naming
a test distribution. Support breadth and test hosts are different things:
the *test* hosts remain `ubuntu-latest` (CI), `windows-latest` (CI),
`macos-latest` (CI), plus the two manual passes (Linux Wayland, Windows 11)
above. A CI pass on `ubuntu-latest` is evidence about ForskScope on Linux
generally to the extent `ubuntu-latest` is representative — it is not a
claim that Ubuntu specifically is the supported baseline, nor a claim that
every distribution is separately verified. See §3 for the direct
consequence of this for F44.

**Rolling-label caveat (review 057 §4.3).** `macos-latest`, `windows-latest`,
and `ubuntu-latest` are rolling labels — GitHub advances what they resolve
to on its own schedule, so a row's actual runtime can change without any
commit in this repository. Evidence recorded under "macos-latest" alone is
not reproducible from this plan, which is precisely what RFC-078 §118's
"concrete versions, not 'current'" requirement exists to prevent. Matters
most for `macos-aarch64`: it is the only row with no manual pass behind it,
so the CI run *is* the entire evidence for that platform, with no
independent check to catch a silent runner-image drift. **Each evidence
file must record the resolved image version the run actually used at
execution time** (e.g. the `ImageVersion`/`ImageOS` values GitHub Actions
exposes in the runner environment, or the platform version a diagnostic
step prints), not just the rolling label — so a row states which macOS,
Windows, or Ubuntu build actually ran, even though this plan can only name
the moving label in advance.

`ROADMAP.md` previously recorded "RFC-078 host access for Linux, Windows,
and macOS is confirmed available" — now superseded by the more precise
statement above: CI access is confirmed and already usable *today* for
`linux-x11`, `windows-10`/`windows-11` (except F45's prerequisite sub-case),
and `macos-aarch64` (except F46 entirely); the *manual* passes
(`linux-wayland`, and Windows 11's prerequisite sub-case) are executed by
the owner — see §4 Q2.

---

## 2. Case-to-row mapping

Three levels, used instead of a bare yes/no so the *depth* of coverage per
row is explicit:

- **Required** — the case's full procedure runs on this row, and a Fail
  blocks the row.
- **Spot-check** — a narrower pass suffices, justified below per case
  (usually: this row shares a rendering/persistence engine with a row that
  already gets the full case, or the row's own RFC-078-stated "Required
  level" doesn't name this case).
- **N/A** — not applicable, justified below. No case is marked N/A without a
  reason, per the handoff's explicit instruction that an unjustified N/A is
  how coverage silently shrinks.

| Case | linux-wayland | linux-x11 | windows-11 | windows-10 | macos-aarch64 |
|---|---|---|---|---|---|
| P01 — Install and cold launch | Required | Required | Required | Required | Required |
| P02 — CLI file compare | Required | Required | Required | Required | Required |
| P03 — Compare layout and scrolling | Required (full — "mandatory on WebKitGTK" per RFC-078) | Spot-check | Spot-check (basic, per RFC-078) | Spot-check | Spot-check (basic, per RFC-078) |
| P04 — Merge, undo/redo, safe save | Required | Required | Required | Required | Required |
| P05 — External modification | Required | Required | Required | Required | Required |
| P06 — Async identity regressions | Required | Spot-check | Required | Spot-check | Required |
| P07 — Explorer and directory report | Required | Required | Required | Required | Required |
| P08 — Persistence migration (F37: Exit/Continue/Reset) | Required | Required | Required | Required | Required |
| P09 — Mergetool | Required | Required | Required | Required | Required |
| P10 — Binary/XLSX fail-closed policy | Required | Spot-check | Required | Spot-check | Spot-check |
| P11 — Keyboard and modal safety | Required | Required | Required | Required | Required |
| P12 — Session/settings restart | Required | Spot-check | Required | Spot-check | Required |

### Justifications

**P03 (Spot-check on linux-x11, windows-11/10, macos-aarch64).** RFC-078's
own case text: *"mandatory on WebKitGTK; repeat a basic layout observation
on Windows WebView2 and macOS WebKit."* linux-x11 uses the same WebKitGTK
engine as linux-wayland — the display protocol (X11 vs Wayland) differs at
the GDK windowing/compositing layer, not inside WebKitGTK's own layout
engine, and F32 (the defect this program's whole rendering-check effort
responds to) was a WebKitGTK table-layout bug, not a Wayland/X11-specific
one. A full duplicate P03 pass on X11 would mostly re-verify the same
rendering engine; a spot-check (open the compare view, confirm no
misalignment by eye) catches an X11-specific windowing regression without
duplicating the Wayland row's full pass.

**P06 (Spot-check on linux-x11, windows-10) — with an open axis question
(review 056 N1).** RFC-078's own case text: *"Deterministic automated tests
remain the primary proof; this case confirms runtime integration."*
RFC-075's deterministic test suite already exhaustively covers the
async-identity state machine itself (close-before-reindex, overlapping
reloads, obsolete completions — see `state/compare/tests.rs`). What P06 adds
beyond that is confirmation the state machine integrates correctly with each
platform's own async runtime/event loop. linux-wayland and windows-11 (the
two "Full functional" rows) each cover one such engine family
(WebKitGTK+GTK's glib main loop, WebView2's own message loop); linux-x11
shares linux-wayland's engine family and windows-10 shares windows-11's, so
a full second confirmation on the narrower row of the *same* engine family
has low marginal value. macos-aarch64 is Required regardless, since
WKWebView/Cocoa's run loop is a third distinct family with no other row to
inherit coverage from.

**The engine-family axis may not be the right one for this specific case.**
Async identity is fundamentally about *timing* — a background compare
completing against live tab state — and the completion path runs through
the windowing event loop, which for Linux is `tao`'s Wayland-versus-X11
backend, not WebKitGTK itself. Wayland and X11 share WebKitGTK's rendering
engine but not their own event-loop integration, so `linux-x11` is not as
obviously covered by `linux-wayland` for *this* case as it is for P03 (a
genuinely rendering-engine-scoped case). This is a judgment call, not
settled fact — recorded here rather than silently assumed, per review 056.
**Backstop rule, adopted regardless of how the axis question resolves: if
any P06 defect appears on any row, every P06 spot-check on that platform is
upgraded to Required before the matrix closes.** A cheap assumption that
fails loudly on first contrary evidence is safe to make; one that stays
cheap after evidence contradicts it is not.

**P08 (Required everywhere — F37).** Explicit per the amendment: narrower row
scope does not exempt P08. Exit's process-termination path is the specific
concern; it is exactly the kind of behavior a narrower row would otherwise
never exercise.

**P09 (Required everywhere).** RFC-077 (`rfcs/done/077-mergetool-save-target-model.md`
§"Core commit semantics") states its own dependency explicitly: *"RFC-078
exercises this on every primary platform"* — referring to
`persist_noclobber`'s atomic no-clobber commit, whose only implementation is
`tempfile::NamedTempFile::persist_noclobber`, a platform-dependent guarantee
by RFC-077's own admission ("If that API cannot provide the required
semantic on a supported platform, normal save fails rather than falling back
to replacement"). This is not optional confirmation; it is where RFC-077's
own open question about Windows replacement semantics gets answered.

**P10 (Spot-check on linux-x11, windows-10, macos-aarch64).** Low
platform-variance risk — this case verifies a translated UI message
displays and that binary/XLSX content is not parsed, not a filesystem or
process-lifecycle behavior that could plausibly differ per platform. Full
depth is reserved for `linux-wayland` and `windows-11` (the two "Full
functional" rows); the other three get a lighter confirmation (message
renders once) rather than the full procedure, because RFC-078's acceptance
criteria does not name P10 as a cross-platform-sensitive case the way
P04/P09 (save semantics) or P08 (process termination) are. `macos-aarch64`
is treated the same as the other narrower rows here rather than promoted to
Required, since nothing about binary/XLSX handling is WKWebView-specific.

**P12 (Spot-check on linux-x11, windows-10).** Significant overlap with P08,
which is Required on every row per F37: both exercise the same persistence
layer, just from a different trigger (fresh launch against a fixture vs. a
live restart after changing settings). The narrower rows inherit meaningful
coverage of the underlying persistence code from their own Required P08
pass; P12 specifically adds "does a live in-process restart round-trip
theme/language/font," which is lower-risk of platform-specific failure than
P08's fixture-based migration paths.

### Platform-specific addenda (not separate case IDs)

RFC-078's "Platform-specific cases" section adds bullets under Windows/
macOS/Linux headings rather than new case IDs. Folded into the row that
already covers the relevant numbered case, not tracked separately:

- **Windows** (`windows-11`, `windows-10`): overwrite-on-existing-destination
  and `.bak`/temp-replacement-on-NTFS verification folds into P04/P09 (both
  Required); long-path behavior and the no-WebView2 prerequisite check fold
  into P01 (Required, see F45 below); zip root/layout inspection folds into
  P01.
- **macOS** (`macos-aarch64`): the macOS-12-vs-`LSMinimumSystemVersion`-13.0
  documentation conflict, DMG/bundle/Gatekeeper verification, and
  signing/notarization disposition all fold into P01 (Required, see F46
  below).
- **Linux** (`linux-wayland`, `linux-x11`): the maintained GTK checklist
  (`docs/src/maintainers/gtk-smoke-test.md`) and WebKitGTK/GTK version
  recording fold into P01/P03; testing the packaged binary outside the
  source tree folds into P01 (Required, see F44 below).

---

## 3. Known facts folded in (not rediscovered at M5)

**F44 — the published Linux artifact does not start on libxdo-4
distributions, and this is now a Go/No-Go input, not a footnote (F56/M4-C4,
2026-08-13).** Fixed upstream (`DioxusLabs/dioxus#5749`, merged 2026-08-10),
not yet released as of this freeze. Because §1a establishes Linux support as
**unqualified** — no per-distribution floor — a libxdo-4 distribution
(Arch/CachyOS-family) is a supported platform like any other, not an
out-of-scope one. "We only tested Ubuntu" does not satisfy a claim of Linux
support, and P01 cannot pass on a libxdo-4 host while F44 is open.

**How this plan represents it (the one judgment call this freeze makes,
per the handoff's explicit instruction to decide rather than default):**

- **If the `dioxus-desktop` release carrying the fix has landed by the time
  M5 runs P01 on `linux-wayland`/`linux-x11`:** the constraint is gone. P01
  is Required in full, tested against the fixed artifact, no caveat needed.
- **If it has not landed:** P01 on a libxdo-4 host **must still be run and
  recorded as Fail** — not silently skipped, not waived, not tested only
  against a Debian/Ubuntu-family host to avoid the failure. RFC-078's own
  Waiver policy already forbids waiving "inability to launch on a claimed
  supported platform," and F44's failure is exactly that. The evidence
  record states plainly: known cause, upstream fix merged, release not yet
  cut, tracked as F44 — an **expected** failure in the sense that it is not
  a new discovery, but not a **waived** one in the sense that matters for
  release decisions.
- **What this plan does not do:** decide, in advance, that this failure
  blocks the release. That is Gate D's call, weighing this alongside
  everything else — this plan's job is to make sure the failure is
  *visible and correctly attributed* when Gate D happens, not to
  pre-resolve it into a No-Go before the evidence exists. Hiding it (by
  testing only a compatible host) would be the opposite failure — a green
  matrix that doesn't mean what it appears to.

**Tie to F55/review 060's residual:** the `dioxus-desktop` bump that fixes
F44 is, by definition, a `Cargo.lock` change — the natural first real
exercise of `audit.yml`'s path-filtered `push`/`pull_request` trigger
(F55 N1), which has so far only been exercised via `workflow_dispatch`
(review 060). That slice's review request should record whether the audit
workflow fired, closing that residual as a side effect of unrelated,
already-scheduled work rather than needing a dedicated demonstration.

**F45 — the Windows artifact's undeclared `VCRUNTIME140.dll`/WebView2
dependencies — structurally invisible to CI (RFC-078 "Execution model"
amendment).** `windows-latest` runners ship with the VC++ redistributable
and WebView2 already preinstalled, so a CI pass on `windows-11`/`windows-10`
proves the binary runs on a machine that already has both — it cannot
observe the actual failure mode. **P01's prerequisite sub-case is marked
manual-only** on both Windows rows; it depends entirely on the owner's
stated occasional Windows 11 manual pass (there is no manual Windows 10 pass
in the stated model, so `windows-10`'s prerequisite sub-case has the same
gap as `macos-aarch64`'s F46 below — recorded, not yet resolved by this
plan). This is inspection-level (artifact import-table analysis) until that
manual pass runs.

**F46 — the macOS artifact is unsigned and unnotarized — unverifiable under
current resourcing (RFC-078 "Execution model" amendment).** Gatekeeper only
refuses a file carrying the quarantine extended attribute, which a real
browser/`curl` download applies and `actions/checkout` does not — a
`macos-latest` CI job launching a locally-built, unquarantined bundle cannot
distinguish "properly signed" from "unsigned but Gatekeeper never saw the
quarantine bit." **With no manual macOS host anywhere in the owner's stated
execution model, F46 cannot be verified at all**, not just "not yet." This
is not the same status as F45 (which *can* be resolved by the stated
Windows 11 manual pass) — F46 has no path to resolution under current
resourcing, full stop. Record this as an **explicit open Gate D input**
with release impact in `macos-aarch64.md`'s evidence — not a passing row,
and not something Gate D can treat as silently resolved by a green CI
run — because it is Gate D, not this plan, that weighs an unverifiable
Gatekeeper posture against everything else at release-decision time. Per
RFC-078's Waiver policy, this is adjacent to "inability to launch on a claimed
supported platform," which is never waivable. The macOS-12-vs-
`LSMinimumSystemVersion`-13.0 conflict itself is resolved by F49 (§1) —
what remains open here is Gatekeeper specifically, not the version conflict.

**All three now have a clear disposition.** F44 has direct execution
evidence (the owner ran the artifact and observed the launch failure) and,
as of this freeze, a defined resolution path with a real Go/No-Go
consequence if the upstream fix hasn't landed by M5 — see above. F45 has a
resolution path (the manual Windows 11 pass) that this plan records but has
not yet executed. F46 has no resolution path under current resourcing at
all — the matrix's job for F46 is to make that gap an explicit Gate D
input, not to close it.

---

## 4. Owner questions — all resolved (2026-08-13)

Every question this plan raised before freezing, with the answer applied
above. Kept as a record rather than deleted, so a future reader can see
what was asked and why, not just the resulting numbers.

1. ~~Exact OS versions per row.~~ **Resolved.**
   `macos-aarch64`: **13.0**, matching `Info.plist`'s
   `LSMinimumSystemVersion`, already enforced on every DMG-installed copy —
   confirmed, no widening. `windows-10`/`windows-11`: **1809** stays,
   matching `AppxManifest.xml`'s current `MinVersion`; `MaxVersionTested`
   stays unchanged too, pending real M5 evidence rather than a
   speculative bump (F49b — the architect's own earlier recommendation to
   raise it was reviewed and withdrawn, since the field records what was
   *validated*, not what merely installs). `linux-wayland`/`linux-x11`:
   **unqualified — no per-distribution floor** (§1a) — this was the
   question with the largest consequence, since it's what makes F44 a
   Go/No-Go input rather than a footnote (§3).

2. ~~Executor owner/role for the manual passes.~~ **Resolved.** CI rows are
   executed by GitHub Actions (no named human). The two manual
   rows/sub-cases — `linux-wayland` in full, `windows-11`'s F45
   prerequisite sub-case — are executed by the owner.

3. ~~Host-access status for the manual passes.~~ **Resolved as a
   consequence of Q2**, not asked separately: the owner is the named
   executor for both manual passes, so access is the owner's own by
   taking on the role.

4. ~~macOS: one row or two?~~ **Resolved earlier (M4-C2).** One row — there
   is no manual macOS host in the stated execution model, so a second row
   requiring one was never executable. See RFC-078's "Execution model"
   amendment and §1 above.

5. ~~Version/RC identifier for the evidence directory.~~ **Resolved by
   F56/M4-C4 — the question no longer applies.** There is no RC
   identifier: this plan is a standing document at
   `release-evidence/matrix-plan.md`, and results live under
   `release-evidence/<tag>/`, named for the tag actually cut, created at
   the cut. See RFC-078's amended "Durable evidence layout."

---

## 5. What this plan is not

This plan does not gather any platform evidence, does not run any case,
and does not create `release-evidence/<tag>/`'s contents
(`README.md`, `artifacts.md`, `linux-wayland.md`, `linux-x11.md`,
`windows-11.md`, `windows-10.md`, `macos-aarch64.md`) — those are M5's
outputs, created at the actual cut, each following the evidence record
schema in RFC-078 §"Evidence record schema," populated only once a real
release candidate exists to test.
