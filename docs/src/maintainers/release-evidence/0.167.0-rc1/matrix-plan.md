# Platform Runtime Matrix Plan

**Governing RFC:** RFC-078 (Platform Runtime Acceptance and Release
Evidence), P08 as amended by F37 (2026-08-11).
**Status: structurally complete, not yet frozen.** The case-to-row mapping
below is decided. The per-row execution facts — exact OS version, executor,
host-access status — are owner-dependent and marked `TBD` throughout; see
"Questions for the owner" at the end. Per RFC-078's own Precondition, this
plan is **committed before M5 begins**, not before every field is known —
but M5 cannot actually *start* until the `TBD`s are resolved, since a row
with no named executor and no confirmed host-access status cannot be
executed.

**This slice plans and freezes the plan. It does not execute any case.**
Running the matrix is M5's work.

This directory name (`0.167.0-rc1`) is a placeholder — see this slice's
review request for the version/RC-identifier question to the owner.

---

## 1. Rows

Five rows, per RFC-078's "Durable evidence layout" file list
(`linux-wayland.md`, `linux-x11.md`, `windows-11.md`, `windows-10.md`,
`macos-aarch64.md`) — **not** the six rows in RFC-078's own "Required
platform matrix" table, which splits macOS into "oldest claimed" and
"current" as two separate table rows sharing one evidence file. See
Question 3 below: whether `macos-aarch64.md` needs two sub-sections (one per
macOS version) or the "current" sub-target is dropped for v1.

| Row | Target | Exact OS version | Architecture | Executor (owner/role) | Host-access status |
|---|---|---|---|---|---|
| `linux-wayland` | Linux x86_64, Wayland, WebKitGTK 4.1 | **TBD** | x86_64 | **TBD** | **TBD** |
| `linux-x11` | Linux x86_64, X11, WebKitGTK 4.1 | **TBD** | x86_64 | **TBD** | **TBD** |
| `windows-11` | Windows 11 with WebView2 | **TBD** | x86_64 | **TBD** | **TBD** |
| `windows-10` | Windows 10 1903+ | **TBD** | x86_64 | **TBD** | **TBD** |
| `macos-aarch64` | macOS, aarch64 | **TBD** — see Question 3 | aarch64 | **TBD** | **TBD** |

`ROADMAP.md` currently records "RFC-078 host access for Linux, Windows, and
macOS is confirmed available" — that is not the same claim as a named
executor with a specific machine per row, which is what RFC-078's
Precondition actually requires. Not treated as sufficient here; see
Question 2.

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

**P06 (Spot-check on linux-x11, windows-10).** RFC-078's own case text:
*"Deterministic automated tests remain the primary proof; this case
confirms runtime integration."* RFC-075's deterministic test suite already
exhaustively covers the async-identity state machine itself (close-before-
reindex, overlapping reloads, obsolete completions — see
`state/compare/tests.rs`). What P06 adds beyond that is confirmation the
state machine integrates correctly with each platform's own async
runtime/event loop. linux-wayland and windows-11 (the two "Full functional"
rows) each cover one such engine family (WebKitGTK+GTK's glib main loop,
WebView2's own message loop); linux-x11 shares linux-wayland's engine family
and windows-10 shares windows-11's, so a full second confirmation on the
narrower row of the *same* engine family has low marginal value. macos-aarch64
is Required regardless, since WKWebView/Cocoa's run loop is a third distinct
family with no other row to inherit coverage from.

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
dependencies.** `windows-11` and `windows-10`'s P01 must explicitly include a
sub-case on a machine **without** the VC++ redistributable and, for
`windows-10` specifically, without WebView2 preinstalled (not guaranteed the
way it is on Windows 11) — not only a machine that already happens to have
both. This is inspection-level (artifact import-table analysis), not yet
confirmed by execution.

**F46 — the macOS artifact is unsigned and unnotarized.** `macos-aarch64`'s
P01 must use the **real download path** (a DMG actually downloaded through a
browser or `curl`, carrying the quarantine extended attribute) — a
locally-built, unquarantined bundle will not observe Gatekeeper's expected
refusal at all, defeating the point of the case. Also resolve the
macOS-12-vs-`LSMinimumSystemVersion`-13.0 conflict from observed
build/runtime support, per RFC-078's own macOS platform-specific case text.
This is inspection-level (artifact/Mach-O analysis), not yet confirmed by
execution.

All three are **inspection-level findings except F44** (which has direct
execution evidence — the owner ran the artifact and observed the launch
failure). The matrix's job is to convert F45 and F46 from inspection to
verified-by-execution, and to re-verify F44 once the upstream fix ships.

---

## 4. Questions for the owner

Collected here, not guessed, per the handoff's explicit instruction that "a
frozen plan built on guesses is worse than an unfrozen one."

1. **Exact OS versions per row.** RFC-078 §118 requires concrete versions,
   not "current" or "oldest claimed." Specifically:
   - `linux-wayland`/`linux-x11`: which distribution and version is the
     actual supported baseline? (The CI-built artifact is Ubuntu-family;
     is the *claimed* support baseline also Ubuntu/Debian-family, or does
     it include libxdo-4 distros like Arch, in which case F44 blocks P01
     there until the dioxus fix ships and this needs to be reflected as a
     schedule dependency, not just a footnote?)
   - `windows-11`: a specific build/version string, or "any Windows 11"?
   - `windows-10`: RFC-078 names "1903+" as a starting point — is that the
     actual claimed minimum, or does the owner want to narrow it (RFC-078
     explicitly permits "narrow published minimum with owner approval")?
   - `macos-aarch64`: RFC-078's table names both an "oldest claimed" version
     and "current" as separate targets sharing this row/file — what are
     those two versions concretely? (Ties into Question 3.)

2. **Executor owner/role per row.** Who actually runs each row — a name or
   role, and do the same person/role cover multiple rows, or is each row
   independently owned? `ROADMAP.md`'s "host access is confirmed available"
   note doesn't name anyone; RFC-078's Precondition requires a named
   executor, not just confirmed access to *some* host.

3. **Host-access status per row**, and specifically: is there a real
   physical or virtual host for each of the five rows right now, or does
   access need to be arranged before M5 can start for some of them? RFC-078
   permits VMs "if file-system and WebView behavior are representative and
   recorded" — does the actual access plan rely on VMs for any row, and if
   so, which?

4. **macOS: one row or two?** RFC-078's "Required platform matrix" table
   lists macOS twice (oldest-claimed and current) as separate targets with
   different required levels ("Launch, compare, save, package/Gatekeeper
   matrix" vs. "Full functional matrix"), but the "Durable evidence layout"
   lists only one `macos-aarch64.md` file, and this handoff's own row list
   names only one `macos-aarch64` row. Should `macos-aarch64.md` contain two
   dated sub-sections (one per macOS version, both exercising their
   respective required-level case sets), or is the "current macOS" full-matrix
   target dropped for v1 and only the oldest-claimed row's narrower set
   required? This changes the case-to-row table above for macOS if the
   answer is "two sub-sections, different levels each."

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
