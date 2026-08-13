# Platform Runtime Matrix Plan

**Governing RFC:** RFC-078 (Platform Runtime Acceptance and Release
Evidence), P08 as amended by F37 (2026-08-11), execution model and macOS row
count as amended by F49/M4-C2 (2026-08-11).
**Status: structurally complete, not yet frozen.** The case-to-row mapping
below is decided, and the CI-vs-manual verification method per row is now
settled (RFC-078's "Execution model" amendment). The per-row execution
facts that remain open — exact OS version confirmation, executor,
host-access status for the manual passes — are owner-dependent and marked
`TBD`/provisional throughout; see "Questions for the owner" at the end. Per
RFC-078's own Precondition, this plan is **committed before M5 begins**, not
before every field is known — but M5 cannot actually *start* until the open
items are resolved, since a manual-pass row with no named executor cannot
be executed.

**This slice plans and freezes the plan. It does not execute any case.**
Running the matrix — including the CI-automated rows, once the release
workflow actually produces a tagged candidate — is M5's work.

This directory name (`0.167.0-rc1`) is a placeholder — see this slice's
review request for the version/RC-identifier question to the owner.

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
| `linux-wayland` | Linux x86_64, Wayland, WebKitGTK 4.1 | **TBD** | x86_64 | **Manual** — CI's Xvfb is X11-family, not real Wayland | **TBD** | **TBD** |
| `linux-x11` | Linux x86_64, X11, WebKitGTK 4.1 | **TBD** | x86_64 | **CI** — `ubuntu-latest` + `xvfb-run`/`dbus-run-session` (F34's mechanism) stands in for this row reasonably | n/a (CI) | CI — available now |
| `windows-11` | Windows 11 with WebView2 | **TBD** | x86_64 | **CI** for most cases, **Manual** for F45's prerequisite sub-case (§3) | CI: n/a; Manual: **TBD** | CI available now; Manual: **TBD** |
| `windows-10` | Windows 10, version 1809+ (F49, provisional) | provisional: **1809** | x86_64 | **CI** — `windows-latest` is a Server-based image, not a literal retail Win10/11 install, but stands in reasonably for save/filesystem/WebView2 behavior | n/a (CI) | CI — available now |
| `macos-aarch64` | macOS 13.0+ (F49, provisional) | provisional: **13.0** | aarch64 | **CI only** — `macos-latest`; **F46 (Gatekeeper) cannot be verified under this model at all** (§3) | n/a (CI) | CI — available now |

`ROADMAP.md` previously recorded "RFC-078 host access for Linux, Windows,
and macOS is confirmed available" — now superseded by the more precise
statement above: CI access is confirmed and already usable *today* for
`linux-x11`, `windows-10`/`windows-11` (except F45's prerequisite sub-case),
and `macos-aarch64` (except F46 entirely); the *manual* passes
(`linux-wayland`, and Windows 11's prerequisite sub-case) still need a named
executor per RFC-078's Precondition — see Question 2.

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

**F44 — the published Linux artifact does not start on libxdo-4 distributions.**
Fixed upstream (`DioxusLabs/dioxus#5749`, merged 2026-08-10), not yet
released. `linux-wayland` and `linux-x11`'s P01 must record which exact
artifact was tested (the CI-built artifact is Debian/Ubuntu-family,
`libxdo.so.3`) and, if run against a host shipping `libxdo.so.4` (e.g.
Arch/CachyOS-family) before the fix ships, must record the launch failure as
an **expected, already-tracked** result — not a new finding, and not silently
waived. Once the `dioxus-desktop` release carrying the fix lands, this
constraint is removed and P01 is re-verified against the fixed artifact.

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
resourcing, full stop. Record this as an explicit open gap with release
impact in `macos-aarch64.md`'s evidence, not as a passing row; per RFC-078's
Waiver policy, this is adjacent to "inability to launch on a claimed
supported platform," which is never waivable. The macOS-12-vs-
`LSMinimumSystemVersion`-13.0 conflict itself is resolved by F49 (§ above) —
what remains open here is Gatekeeper specifically, not the version conflict.

**F44 remains the only one of the three with direct execution evidence** —
the owner ran the artifact and observed the launch failure. F45 has a
resolution path (the manual Windows 11 pass) that this plan records but has
not yet executed. F46 has no resolution path under current resourcing at
all — the matrix's job for F46 is to make that gap explicit and visible in
Gate D's evidence, not to close it.

---

## 4. Questions for the owner

Collected here, not guessed, per the handoff's explicit instruction that "a
frozen plan built on guesses is worse than an unfrozen one."

1. **Exact OS versions per row — two now have a proposed default, three
   remain fully open.** RFC-078 §118 requires concrete versions, not
   "current" or "oldest claimed":
   - `macos-aarch64`: **proposed 13.0**, matching `Info.plist`'s
     `LSMinimumSystemVersion` (already enforced on every DMG-installed copy
     today) — F49 set `MACOSX_DEPLOYMENT_TARGET` to match. Confirm, or state
     the real floor if it should widen to capture more Apple Silicon Macs
     (11.0 is the hardware floor).
   - `windows-10`/`windows-11`: **proposed 1809**, matching
     `AppxManifest.xml`'s current `MinVersion` — deliberately *not* changed
     by this plan, since that manifest may back a live Microsoft Store
     submission (`docs/src/users/installation.md` links one) and editing its
     declared constraints without knowing the submission's real state is not
     a call this slice is positioned to make. Confirm whether the manifest's
     values themselves should change, and separately, whether
     `MaxVersionTested` (currently 10.0.19041.0, Windows 10 2004 — below
     Windows 11) should be raised.
   - `linux-wayland`/`linux-x11`: still fully open. Which distribution and
     version is the actual supported baseline? The CI-built artifact is
     Ubuntu-family; is the *claimed* support baseline also Ubuntu/
     Debian-family, or does it include libxdo-4 distros like Arch, in which
     case F44 blocks P01 there until the dioxus fix ships and this needs to
     be a schedule dependency, not just a footnote?

2. **Executor owner/role for the manual passes.** The CI rows
   (`linux-x11`, `windows-10`, `windows-11` except F45's sub-case,
   `macos-aarch64` except F46) need no named human — CI runs them. The
   manual rows/sub-cases (`linux-wayland` in full; `windows-11`'s F45
   prerequisite sub-case) do: who actually runs them, a name or role?
   `ROADMAP.md`'s former "host access is confirmed available" note named no
   one; RFC-078's Precondition requires a named executor for whatever is not
   automated.

3. **Host-access status for the manual passes specifically.** Is there a
   real Linux-with-real-Wayland host and a real Windows 11 host available
   right now for the two manual passes, or does access need arranging
   before M5 can start? (The five CI rows need no separate host-access
   question — GitHub-hosted runners are the host, already available.)

4. ~~macOS: one row or two?~~ **Resolved by this slice.** One row — there is
   no manual macOS host in the stated execution model, so a second row
   requiring one was never executable. See RFC-078's "Execution model"
   amendment and §1 above.

5. **Version/RC identifier for this evidence directory.** `0.167.0-rc1` was
   chosen only as a placeholder so this plan has somewhere to live before a
   real release candidate exists (see this slice's review request). What
   should the actual directory be named, or should it stay a placeholder
   renamed at real cut time?

---

## 5. What this plan is not

Per the handoff's explicit scope boundary: this plan does not gather any
platform evidence, does not run any case, and does not create
`linux-wayland.md`/`linux-x11.md`/`windows-11.md`/`windows-10.md`/
`macos-aarch64.md`/`artifacts.md`/`README.md` — those are M5's outputs,
each following the evidence record schema in RFC-078 §"Evidence record
schema," populated only once a real release candidate exists to test.
