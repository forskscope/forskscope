# Proposed RFC Backlog — Consolidated Write-Up (as of v0.40.0)

> **Note — v0.114.0 (2026-06-12).** This document was written at v0.40.0 to
> audit the proposed RFC backlog against shipped reality at that point. Many
> items labelled "Partly shipped" or "Open" here have since been fully
> shipped. For current RFC state see `rfcs/README.md` and the individual RFC
> files. This document is preserved as a historical planning record.

**Purpose.** A single view of every RFC still in `rfcs/proposed/`, written
up against what has actually shipped through v0.40.0. For each RFC: its
intent, its **status vs. shipped reality** (much of the early backlog was
written during migration planning and has since been partly delivered under
other RFC numbers), key dependencies, and what remains.

**How to read the status tags:**
- **Open** — not started; design stands.
- **Partly shipped** — some of the RFC's surface already landed under a
  `done/` RFC or in code; the RFC should be narrowed to the remainder.
- **Superseded-in-practice** — the working implementation diverged from or
  replaced this RFC's approach; needs reconciliation (rewrite, or move to
  archive with a pointer).
- **Stale refs** — content references pre-migration assumptions
  (Tauri/Svelte, "the current app") that no longer hold.

This is a planning note, not an RFC. It changes no RFC's lifecycle state.
Authoritative state remains the folder + each file's Status field
(RFC-000).

---

## Cross-cutting observation

The backlog was authored in three waves: the migration packages (RFC-004
through RFC-042, themed around moving off Tauri/Svelte) and the post-v0.34
feature line (RFC-054+). Because the migration itself completed around
v0.27–v0.36, **a large fraction of the migration-wave RFCs describe work
that is now done, in progress, or reframed.** The single most useful action
this write-up enables is a triage pass: narrow the partly-shipped RFCs to
their unfinished remainder and archive the ones fully overtaken by shipped
code, so the proposed list reflects real open work rather than migration
history.

---

## Foundation / core (mostly delivered elsewhere)

### RFC-004 — Editor Adapter and CodeMirror Bridge — **Open (high-risk)**
The editable-surface adapter (CodeMirror behind a narrow Rust-owned
boundary). Not shipped: the Compare view today renders read-only diff rows;
there is no embedded editor. This is the gateway to in-pane manual editing
and underpins RFC-016/024/025/035/040/032. Remains the highest-risk UI bet;
RFC-025 (prototype + kill switch) gates it. **Action:** keep Open; treat as
a deliberate, separately-gated track, not a casual feature.

### RFC-008 — Directory Comparison and Background Job Model — **Partly shipped**
Background digesting, progress, cancellation, incremental row updates.
Shipped in spirit: `deep_compare.rs` runs a two-phase async scan with
per-file `spawn_blocking` digests and live row refresh (RFC-037 territory).
Not shipped: a first-class cancellation token and a unified job model.
**Action:** narrow to the job-model/cancellation remainder; fold the rest
into RFC-037.

### RFC-011 — Workspace Session Persistence — **Partly shipped**
Represent/persist/restore/close sessions. Shipped: `save_session` /
`restore_session` with `SessionState`, tab restore on launch (RFC-035 of the
roadmap / RFC-007 line). Not fully addressed: crash recovery, schema
versioning of the persisted blob, three-way sessions. **Action:** narrow to
versioning + recovery + three-way.

### RFC-012 — Text Encoding, Newline, and Binary Policy — **Partly shipped**
Read/compare/edit/save policy for text/binary/Excel. Shipped: `encoding.rs`
(UTF-8 + chardetng/encoding_rs, newline preservation), binary hex fallback,
`FileKind`, read-only Excel. Remaining: explicit "save encoding transform"
UX and the editable-binary boundary (tied to RFC-004). **Action:** narrow to
save-time encoding decisions; cross-ref RFC-058 for the spreadsheet kind.

### RFC-013 — Large File, Performance, and Virtualization — **Partly shipped**
Shipped: large-file policy + deadline + inline-skip in the diff engine
(`DiffWarning::LargeFilePolicyApplied`/`DeadlineExpired`/`InlineSkipped…`)
and equal-hunk collapsing in the UI. Remaining: row **virtualization** for
very long files (the diff currently renders all rows), and the directory
cell-count caps RFC-058 references. **Action:** narrow to virtualization +
directory bounds.

### RFC-015 — Undo/Redo Transaction Log and Merge Operation History — **Partly shipped**
Shipped: the two-way `MergeSession` transaction log (apply/undo/redo, dirty
state) and the three-way session's resolution undo/redo (RFC-033). Remaining:
a *unified* operation history once text editing (RFC-032) and directory
batch ops (RFC-022) exist. **Action:** keep Open as the umbrella; note the
two merge logs already conform.

### RFC-017 — Error Taxonomy and Diagnostics UX — **Partly shipped**
Shipped: `CoreError` taxonomy, toast surface, structured save/conflict
errors, read-only notices. Remaining: the full diagnostics panel / "copy
diagnostics", error-details modal depth. **Action:** narrow to the
diagnostics-UX surface.

### RFC-021 — Document and Result Buffer Model — **SHIPPED (v0.28.0)**
Listed here only because dependents reference it. It is in `done/`.

---

## Diff / merge surfaces

### RFC-006 family already shipped; these refine it:

### RFC-024 — Diff Visual Semantics and Decoration Contract — **Partly shipped**
Shipped: colour-independent glyph + `sr-only` markers, focus outline,
applied/✓ states, warning banners. Remaining (and now also owns audit
findings via RFC-059): edge-of-list navigation feedback (M3), "applied N/M"
confirmation (L3), and a formal decoration contract for the future editor
surface. **Action:** absorb RFC-059 M3/L3; keep decoration-contract Open
pending RFC-004.

### RFC-014 — Search, Filter, and Navigation — **Partly shipped**
Shipped: in-diff substring search with match count + highlight; hunk
prev/next (F7/F8); ignore-pattern filtering in Explorer (RFC-056).
Remaining: **next/prev match traversal + scroll-to-match** (RFC-059 M4),
explorer file filtering UI, whole-word/regex decision. **Action:** narrow to
match traversal + filter UI; absorb RFC-059 M4.

### RFC-032 — Text Editing Operation Model and Editor Truth Boundary — **Open**
Typed edit operations crossing the adapter into core; editor never
authoritative. Blocked on RFC-004. Critical for manual conflict edits
(RFC-034 currently uses whole-text `resolve_manual` as the interim).
**Action:** keep Open; it is the model half of the editor track.

### RFC-034 — Conflict Resolution Workspace — **Open (newly rewritten)**
Rewritten in this cycle to target the shipped `ThreeWayMergeSession`. UI-only;
core is ready. The marquee next UI feature. **Action:** implement when UI
testing bandwidth exists (owner will test).

### RFC-035 — Scroll Sync, Line Mapping, Diff Decoration Engine — **Partly shipped / reframed**
Shipped (structurally): the unified single-grid diff row makes two-way
scroll-sync inherent — there are no two independently-scrolling panes to
sync. Remaining: line mapping + decoration for an *editor* surface
(RFC-004) and for the **three-way** four-region layout (RFC-034), where
independent panes do exist. **Action:** reframe around editor + three-way;
note two-way is already solved by layout.

---

## Directory / batch

### RFC-022 — Directory Merge and Batch Operations — **Open**
Previewable, recoverable copy/delete batches from the Directory Report.
Not shipped (Deep Compare is read-only today). High data-loss sensitivity;
depends on RFC-023. **Action:** keep Open; gate on RFC-023.

### RFC-023 — Atomic File Operations, Backup, and Restore — **Partly shipped**
Shipped: file save uses atomic write + `.bak` backup + fingerprint conflict
check (`save_text`, `BackupPolicy::SiblingBak`). Remaining: directory-level
batch atomicity/rollback (for RFC-022), and the **digest-cache policy**
RFC-059 M1/M2 lean on. **Action:** narrow to directory-batch safety +
digest-cache lifetime.

### RFC-037 — Scalable Directory Compare Index and Incremental Refresh — **Partly shipped**
Shipped: two-phase recursive listing + incremental digest fill in
`deep_compare.rs`. Remaining: a persistent index, true incremental refresh
on change, cancellation, and large-tree bounds. **Action:** narrow to
index + refresh + cancellation; this is the natural home for RFC-008's
remainder and a strong, core-testable next slice.

---

## Editor track (all gated on RFC-004; coherent sub-package)

### RFC-016 — Editor Bridge Security and Contract — **Open**
Isolation + API contract for the JS editor bridge. Blocked on RFC-004.

### RFC-025 — Editor Adapter Prototype and Kill Switch — **Open (gate)**
Mandatory prototype gate + fallback if the bridge is unstable. Should be the
*first* editor-track work item; it explicitly de-risks RFC-004.

### RFC-040 — Editor Adapter Verification Harness and Golden Corpus — **Open**
Test harness proving editor ops/decorations/scroll behave. Depends on
RFC-004/016/025. **Action:** sequence after the prototype gate.

---

## Settings / platform / release

### RFC-009 — Settings, Theme, Localization, Accessibility — **Partly shipped**
Shipped: theme (dark/light/night), fonts, diff font size, EN/JA i18n,
settings dialog (RFC-057), colour-independent diff. Remaining: full
localization coverage (RFC-059 L1 found English leaks), accessibility audit
completion. **Action:** narrow to i18n coverage + a11y pass.

### RFC-010 — Packaging, Diagnostics, QA, Release Gates — **Partly shipped**
Shipped: per-version release archives (the `forskscope-vX.Y.Z/` tar
convention), Linux/AUR + Windows/macOS packaging scripts under `packaging/`.
Remaining: a formal QA matrix, smoke tests, signed builds decision.
**Action:** narrow to QA matrix + smoke tests.

### RFC-018 — Migration Compatibility and Parity Plan — **Superseded-in-practice**
A plan to *prove* migration parity vs. the old Tauri/Svelte app. The
migration is effectively complete; parity has been demonstrated through the
shipped feature set. **Action:** strong candidate to **archive** with a note
("migration complete; parity tracked by shipped RFCs 005–057"), per RFC-000
anti-silent-withdrawal guidance.

### RFC-019 — Command Registry, Shortcuts, Palette, Accessibility — **Open / now load-bearing**
No command registry exists yet; shortcuts are ad-hoc `onkeydown` matches.
RFC-059 (H2/H3/L2) makes this load-bearing: Explorer keyboard completeness
needs it. **Action:** promote priority; absorb RFC-059 keyboard findings.

### RFC-020 — Developer Architecture, CI, and Test Gates — **Partly shipped / Open**
ELOC limits, test organization, English-only docs are observed in practice.
Not shipped: actual CI gates, the CSS-duplicate lint RFC-059 H1 wants, the
`unsafe`-review gate (L5). **Action:** narrow to concrete CI checks.

### RFC-026 — Cross-Platform WebView and Linux Desktop Compatibility — **Open**
WebView/GTK compatibility + fallback across Linux/Win/macOS. Still relevant
(the build needs WebKitGTK/GTK3); diagnostics + documented constraints
remain. **Action:** keep Open; pair with RFC-010.

### RFC-027 — Report Export and Session Evidence — **Open**
Exportable comparison reports. Patch *export* shipped (RFC-039) covers the
diff-as-patch case; human-readable/HTML reports remain. RFC-058 and RFC-039
both reference it. **Action:** keep Open; scope to human-readable reports.

### RFC-028 — Preferences, Profiles, and Compare Options — **Partly shipped**
Shipped: diff profiles (`DiffProfile`, add/remove), ignore-WS/case,
algorithm select, ignore patterns. Remaining: named reusable *compare*
profiles surfaced as first-class, per-session overrides. **Action:** narrow
to profile management UX.

### RFC-029 — Integration with External Tools and Open With — **Open**
Open-with external editor/terminal/file-manager. Partial: file-manager open
existed in v0.22 baseline; not confirmed in current UI. **Action:** verify
current state, then scope.

### RFC-030 — User Documentation, Onboarding, Help System — **Partly shipped**
Shipped: `docs/` mdbook structure, keyboard reference modal, growing user
pages (incl. new patch-export page). Remaining: onboarding/first-run, FAQ
depth, in-app contextual help. **Action:** narrow to onboarding + in-app
help.

### RFC-031 — Release Channel, Migration, and Data Compatibility — **Open**
Channels + settings/session schema migration + preview→stable. Ties to
RFC-011 versioning. **Action:** keep Open; sequence with RFC-011.

### RFC-038 — VCS Context Integration Boundary — **Open**
Optional Git/JJ context (read-only) without becoming a VCS client. Partial:
git-mergetool startup mode exists (`STARTUP_MERGED`). Remaining: the broader
read-only context surface, kept strictly within NG-001. **Action:** keep
Open; honour non-goals.

---

## Governance / planning (meta)

### RFC-041 — v1.0 Product Stabilization and RFC Governance — **Open (meta)**
Defines the v1.0 freeze + RFC governance going forward. **Action:** keep
Open; activate as v1.0 approaches.

### RFC-042 — Roadmap and RFC Execution Plan — **Living / partly stale**
The execution roadmap. Now partly historical (sequencing assumed migration
not yet done). **Action:** refresh against shipped state, or supersede with
a v0.40+ roadmap note.

---

## New feature-line RFCs (post-v0.34)

### RFC-058 — Spreadsheet (`.xlsx`) Structural Diff — **Open (authored this cycle)**
Structured cell-level model behind the `sheets-diff` adapter; first-class
aligned view gated behind a test corpus. Deferred by priority. **Action:**
implement the core structured-adapter slice when scheduled.

### RFC-059 — Explorer and Compare UI/UX Audit Remediation — **Open (authored this cycle)**
Consolidates the v0.40 audit. Owns the new findings (CSS de-dup, typed
digest keys, `explorer.rs` split + alignment tests, unsafe removal) and
cross-references RFC-019/023/024/014/009 for the rest. **Action:** the
core-testable parts (alignment extraction + tests, typed keys) are a good
near-term slice; the keyboard/search parts need UI testing.

---

## Suggested triage outcomes

1. **Archive:** RFC-018 (migration parity — complete).
2. **Refresh:** RFC-042 (roadmap) against shipped state.
3. **Narrow to remainder** (and note the shipped portion in each Status):
   RFC-008, 011, 012, 013, 014, 017, 023, 024, 028, 030, 037.
4. **Promote priority:** RFC-019 (now blocks Explorer keyboard completeness),
   RFC-037 (core-testable, natural next), RFC-034 (UI marquee).
5. **Keep as a gated sub-package:** RFC-004 → 025 → 016 → 040 → 032/035
   (editor track), sequenced behind the RFC-025 prototype gate.
6. **Keep Open as designed:** RFC-022, 026, 027, 029, 031, 038, 041, 058,
   059.
