# ForskScope Roadmap

**Last updated:** 0.165.0 released; 0.166.0 in development (2026-08-01)
**Current phase:** v1 release stabilization — release-baseline reconciliation,
then correctness workstreams, then runtime/platform acceptance and a new
architecture go/no-go review.
**Planning basis:** ordered tasks, dependencies, and exit gates. Milestones
carry no calendar windows or effort estimates; a milestone completes when its
gate evidence exists.

---

## Current state

The `forskscope-core` and `forskscope-ui-logic` crates are feature-complete for
the v1 two-way diff/merge workflow. The current observed headless gate passes
**943 tests** with zero failures: 643 core unit tests, 45 core integration
tests, 241 ui-logic unit tests, 6 CSS integration tests, and 8 doctests.

The UI crate (`forskscope-ui`) has the v1 two-way workflow implemented:
two-pane diff with independent pane labels and shared horizontal scroll;
English/Japanese translation-key coverage enforced by `cargo xtask i18n`
(203 `t(...)` keys); per-file and batch copy in the directory report view;
F3/Shift+F3 search navigation; compare profiles; session restore; patch export;
and release-gate CSS freshness checks.

Release-readiness hardening completed after v0.140.0:

- XLSX parsing was security-disabled: `.xlsx` files are recognized but
  comparison fails closed until the `sheets-diff -> calamine -> quick-xml`
  dependency path is remediated.
- Dioxus desktop network-capable transitive dependencies were reviewed. Default
  Dioxus features/devtools are disabled, and the accepted loopback WebSocket IPC
  path is enforced by `cargo xtask audit-deps`.
- Source archives now have a no-parent-directory contract, verified locally and
  in the release workflow by `cargo xtask archive-layout`.
- CI and release preflight now run the documented gates: format, CSS, audit,
  dependency-path audit, version sync, i18n coverage, tests, and clippy. Release
  tags are checked against the workspace version before artifacts are created.

The 2026-07-15 architecture audit approved continued development but issued a
v1/public-release No-Go. Three correctness workstreams now precede runtime QA:
stable async tab/load identity, versioned production settings/session
persistence, and a distinct Git-mergetool save-target model. GTK/WebKitGTK and
cross-platform package verification follow only after those workstreams pass.
Three-way merge conflict workspace UI, command palette, and editor adapter work
remain post-v1.

---

## v1 release stabilization program

[RFC-074](rfcs/proposed/074-v1-release-stabilization-program.md) is the
authoritative program design. RFC-075 through RFC-078 define the detailed
workstreams. Milestones are ordered by dependency and completed by gate
evidence. No milestone carries a target date or effort estimate.

| # | Milestone | Scope | Depends on | Exit gate | Release |
|---|---|---|---|---|---|
| — | M0 — Design approval | RFC-074–078 designs and compatibility decisions | — | Owner and architect accept detailed designs | — |
| — | M1 — Async identity | Stable tab IDs, load generations, deterministic race tests | M0 | RFC-075 acceptance complete | — |
| — | R0 — Stabilization baseline | Version/CHANGELOG reconciliation for the unreleased delta; release-trigger reconciliation; documentation-truth fixes; version-sync tag check | M1 | Gate B plus an observed release-workflow run and owner release approval | 0.165.0 |
| 1 | M2 — Release mechanics and persistence convergence | **M2-A:** release-notes composition, release policy documentation, threat-model currency (F19–F22). **M2-B:** canonical schema v2 plus UI-v0/core-v1 migrations | R0 | M2-A content review plus verification at the next real release cut; RFC-076 acceptance complete | level decided at release time |
| 2 | M3 — Mergetool target safety | Separate remote input/output identity and explicit match/absence preconditions | M1 (hard); sequenced after M2 | RFC-077 acceptance complete | 0.167.0 |
| 3 | M4 — Integrated stabilization | Full gates, docs/RFC reconciliation, advisory dispositions, frozen `matrix-plan.md` | M2, M3 | Gate C — release-core candidate approved for QA | 0.168.0 candidate |
| 4 | M5 — Platform acceptance | Linux Wayland/X11, Windows, macOS runtime matrix | M4 | Gate D — RFC-078 evidence complete | candidate re-cuts as needed |
| 5 | M6 — Handoff and go/no-go | Refresh handoff and independent architecture review | M5 | Gate E — explicit v1 Go or continued No-Go | — |

M3's only hard dependency is M1; it is sequenced after M2 because a single
developer owns the overlapping UI state files. If ownership ever separates,
M2 and M3 may run concurrently without changing any gate.

**Progress (2026-08-01):** M0, M1, and R0 are complete. RFC-075 guards
asynchronous compare completion with stable tab IDs and per-load generations,
resolving audit finding B1.

R0 released `0.165.0`. It reconciled the version and CHANGELOG with the
26-commit delta that had accumulated past the published `0.164.0` tag, repaired
the release-workflow trigger, added a published-tag check to
`cargo xtask version-sync`, and corrected five documentation-truth defects. The
first real release-workflow run surfaced F17, a Windows build failure in the
`app-json-settings` dependency that no amount of configuration review would
have found; it was reported upstream, fixed in 2.4.1, and re-verified before
the tag. All six release jobs are green and the four artifacts are digest-
recorded. Reviewed and conditionally approved with six documentation-currency
follow-ups carried into `0.166.0`.

`0.165.0` is published (2026-08-02). The working tree is at `0.165.1` — the
post-release patch default, not a claim that the next release is a patch. M2's
content will decide its level; RFC-076's persistence schema change is expected
to promote it to `0.166.0` at release time. The N1–N6 documentation-currency
follow-ups from review 032 (registered as F19–F21) ride with M2 rather than
shipping separately.

RFC-078 host access for Linux, Windows, and macOS is confirmed available, so M5
is schedulable once M4 completes. M2–M6 remain outstanding and R0 closed no
audit blocker, so v1/public release remains **No-Go**.

### R0 rationale

`0.164.0` is a published, immutable tag. The working tree has advanced well
beyond it — including an MSRV change, the fail-closed XLSX decision, the
Dioxus dependency-path constraint, release-archive and CI gate alignment, and
the RFC-075 correctness fix — while `Cargo.toml` still declares `0.164.0`.

A source build therefore reports a version whose published artifact behaves
differently, and `PlatformInfo` propagates that version into `--diagnostics`
and the About panel, so defect reports would be misattributed. `cargo xtask
version-sync` does not detect this because it compares the workspace version
against PKGBUILD and the MSIX manifest, not against published tags.

R0 closes that gap before any further production change lands, and pulls the
version/CHANGELOG reconciliation and the documentation-truth subset forward
out of M4 so M4 is not a single large batch.

R0 also repairs the release trigger. `.github/workflows/release.yml` fires on
`v[0-9]+.[0-9]+.[0-9]+`, but this project tags without a `v` prefix and none of
its published tags match that pattern. The workflow's gates and artifact jobs
were aligned after the last tag was pushed, so the trigger has never been
exercised. Until it is corrected, tagging a release runs no gates, builds no
artifacts, and publishes nothing — so R0's own release evidence would not
exist. The correction follows the documented tag rule rather than changing the
convention.

### Workstream dependencies

```text
RFC-074 program
└── RFC-075 async identity ......... done (M1)
      └── R0 release baseline ...... 0.165.0
            ├── RFC-076 runtime persistence ──┐
            └── RFC-077 mergetool target ─────┤
                      requires RFC-075        │
                                              └── integrated Gate C
                                                    └── RFC-078 platform matrix
                                                          └── refreshed handoff
                                                                └── architect Gate E
```

RFC-075 is complete, so the remaining single-developer sequence is
R0 → RFC-076 → RFC-077 → integrated gates → RFC-078 → refreshed handoff.

### Release cycle

The program releases one unit per resolved workstream rather than batching to
a single pre-v1 cut. This matches the project's logical-breaking-point rule and
keeps the CHANGELOG, version metadata, and packaging inputs continuously true.

| Element | Policy |
|---|---|
| Release unit | One release per resolved workstream — an RFC disposition or a completed hardening theme |
| Numbering | `docs/src/maintainers/release.md` is authoritative and content-driven: `PATCH` for bug fixes and documentation updates within a stable feature set, `MINOR` for new user-visible features or significant internal changes |
| Post-release default | The commit after a release bumps to the next **patch** level. That satisfies the version invariant while claiming nothing about content |
| Promotion | At release time, the accumulated content decides the level. Promoting patch → minor is a rename across the six enforced locations plus the CHANGELOG heading, confirmed by the owner with the content visible |
| Trigger | Workstream gate passes → set the release version → CHANGELOG entry → source tarball delivered to the owner |
| Version invariant | The workspace version must never equal an existing tag on any commit other than that tag's own |
| Approval | Every release and its version level are confirmed by the project owner; no release is automatic |
| v1.0.0 | Reserved. Gate E yields a Go/No-Go verdict only. Whether and when a Go becomes 1.0.0 is the project owner's decision alone |

The version invariant is enforced by `cargo xtask version-sync`'s published-tag
check, added in R0.

The post-release default and promotion rules exist because the level cannot be
known before the content is. An earlier revision of this table specified
`0.MINOR.0` unconditionally, which contradicted `release.md` and caused R0's
post-release bump to pre-commit the next release to a minor level before its
scope existed. Defaulting to patch keeps the number mechanical and the level a
decision made from evidence.

### Release-blocking outcomes

- Obsolete async completions cannot mutate another/newer load.
- Production settings/session files use core-owned versioned schemas and
  migrate current plain JSON and existing core-v1 envelopes without silent
  loss or schema reinterpretation.
- Git mergetool fingerprints and guards the actual merged output, including
  no-clobber creation when the target was initially absent.
- Exact release artifacts pass the retained runtime/platform matrix.
- A refreshed handoff receives an independent architecture Go verdict.

Until all five outcomes are evidenced, v1 remains No-Go.

### Fix and improvement register

Tracked non-blocking work, each assigned to the milestone that owns it. Items
sourced from the 2026-07-15 architecture audit keep their audit finding ID.

| ID | Item | Source | Milestone |
|----|------|--------|-----------|
| F1 | `docs/src/maintainers/testing.md` test counts stale (930/228 versus observed 943/241) | 2026-08-01 review | R0 |
| F2 | `docs/src/maintainers/architecture.md` documents a shim re-export layer that no longer exists | 2026-08-01 review / audit N3 | R0 |
| F3 | `docs/src/maintainers/threat-model.md` settings-persistence section claims no residual concerns, contradicting audit B2 | 2026-08-01 review | R0 |
| F4 | Workspace version and CHANGELOG behind the published `0.164.0` tag | 2026-08-01 review | R0 |
| F5 | `cargo xtask version-sync` cannot detect workspace version equal to a published tag | 2026-08-01 review | R0 |
| F6 | `cargo clippy --workspace --all-targets` fails on test-target lints | audit N6 | M4 |
| F7 | Allowed `cargo audit` warnings include unsoundness advisories needing individual disposition | audit N5 | M4 |
| F8 | `FileFingerprint` digest is captured but unused at save time | audit N1 | M4 |
| F9 | Atomic/power-loss durability wording exceeds what the implementation proves | audit N2 | M4/M5 |
| F10 | VCS discovery tests assume the OS temp directory sits outside any repository | audit N7 | M4 |
| F11 | RFC-058 status does not record the fail-closed security suspension | audit N4 | M4 |
| F12 | RFC-062 is fully shipped but still filed under `proposed/` | RFC-074 | M4 |
| F13 | Source files above the 300-ELOC soft threshold; `xtask/src/main.rs` is the largest | audit N6 | opportunistic, when touched |
| F14 | Release workflow triggers on `v`-prefixed tags that this project never creates, so it has never run | 2026-08-01 review of 001 | R0 |
| F15 | `README.md` describes the three-way conflict workspace UI as "in progress" although it is a deferred post-v1 slice and an explicit RFC-074 non-goal | 2026-08-01 review of 001 / review 001 finding B | R0 |
| F16 | Public feature claims are not systematically audited for core-complete versus user-reachable status | review 001 finding B | M4 |
| F18 | `xtask` is outside `cargo fmt --check` (not a workspace member, DEC-005), so `xtask/src/main.rs` has drifted from current rustfmt output; the drift nearly pushed R0's addition over the 500-ELOC hard threshold | review 032 / R0 review question 3 | M4 |
| F19 | `docs/src/maintainers/release.md` has no re-release or immutability policy, although that policy governed R0's tag re-cut; it exists only in the superseded v0.164.0 handoff bundle | review 032 (N2) | M2 |
| F20 | Threat-model audit history omits the RFC-075 integrity fix and retains the superseded v0.148.0 stale-tab-guard claim; section heading still reads v0.164.0 | review 032 (N3, N4) | M2 |
| F21 | `release.md` must record the corrected release-cycle rules: post-release patch default, promotion at release time, and the definition of "published" as a release out of draft state — aligned with the `version-sync` check, which keys on tag existence | 2026-08-02 owner decision | M2 |
| F22 | Release notes are produced by `generate_release_notes: true`, which summarises pull requests; this project commits directly to `main`, so it emits only a compare link and ignores the CHANGELOG. Compose notes in CI from the tag's CHANGELOG section, failing closed when absent, and document the publish step as an explicit owner action | 2026-08-02 owner question | M2 |
| F17 | **Resolved.** Windows release build failed: `app-json-settings` 2.3.0/2.4.0 had an out-of-scope `use std::os::windows::ffi::OsStrExt` in `replace_file`'s local scope that did not cover the sibling `wide_null_terminated` function, which also calls `.encode_wide()`. Discovered by R0's first real release-workflow run — this is why R0 required an observed run rather than a configuration review. Reported upstream to `github.com/nabbisen/app-json-settings-rs`; fixed same-day in 2.4.1. Bumped and re-verified (Windows cross-compile, full gate suite) before the 0.165.0 tag. | 2026-08-01 R0 release run | R0 |

Phase 3 candidates are recorded under "Remaining proposed RFCs" and the
post-v1 slices below. They are deliberately unscheduled: post-v1 planning
resumes as a joint discussion after the Gate E verdict.

---

## Delivered milestones

| Milestone | Version | What landed |
|-----------|---------|-------------|
| Core extraction | v0.23 | `forskscope-core` crate, domain model, error taxonomy |
| Diff engine | v0.23 | `similar` v3, normalised diff/inline model |
| Dioxus shell | v0.23 | App shell, tabs, reactive state runtime |
| Explorer | v0.25 | Two-pane explorer, digest status icons |
| Diff/merge workspace | v0.26 | Hunk nav, merge transactions, undo/redo |
| Save safety | v0.27 | Atomic write, backup, dirty-close guard, fingerprint |
| Document buffer | v0.28 | Loaded document + result buffer model |
| Three-way merge | v0.40 | `ThreeWayMergeSession`, diff3 engine, conflict resolution |
| Explorer tree | v0.36 | Tree view, breadcrumb nav, ignore patterns |
| Patch export | v0.39 | Unified-diff export from file/directory diffs |
| Core data layer | v0.40–v0.72 | All RFC data types, 629 tests, clippy clean |
| View-model layer | v0.74–v0.87 | 14 `ui-logic` modules, 189 tests, all 7 slices covered |
| CSS contract | v0.88 | `fs-line-*`, `fs-inline-*`, `fsk-conflict-*` classes; 4 coverage tests |
| CSS bug fixes | v0.89 | `--danger-bg` defined; path.rs tests (16); `cancel_tests`, `file_kind_tests` |
| Test coverage | v0.90–v0.91 | All core modules tested; 26-file diff corpus; 856 tests total |
| UI four-bug fix | v0.92 | Two-pane split, dark theme select colour, ESC modal close, i18n expanded |
| Platform diag | v0.93 | `platform` module, `PlatformInfo`, corpus extended (encoding/binary/large) |
| Scroll fix + i18n | v0.94 | ISSUE-001 resolved (shared scrollbar); modals i18n complete |
| ELOC compliance | v0.115–v0.116 | command, error, session, report, settings, job modules split; zero files over 500 lines |
| Docs + platform | v0.95–v0.96 | Testing/architecture/local-dev docs updated; 4 user docs rewritten |
| CONTRIBUTING + limits | v0.97–v0.98 | ROADMAP/release/features updated; CONTRIBUTING.md; known-limitations.md |
| RFC-041 + v0.100 | v0.99–v0.100 | RFC-041 checklist updated; PlatformInfo wired to About; patch export UI |
| UI polish + i18n | v0.111–v0.139 | Full i18n (158 keys, 0 gaps); CSS cleanup (583→504 lines); keyboard shortcuts; per-file copy; bug fixes |
| Release readiness hardening | v0.164 | XLSX parser path disabled, dependency/network policy enforced, source archive contract fixed, CI/release gates aligned |

---

## UI implementation slices — status at v0.165.0

The remaining work is a series of UI slices that wire the Dioxus components
to the core types. Each slice delivers a testable, usable increment.

### ✓ Slice 1 — Diff view renders and navigates *(shipped)*

**Goal:** A user can open two files, see the diff rendered with correct
colour + gutter symbols, and navigate prev/next hunk with keyboard.

**Core types consumed:**
- `DiffDecorationSet::from_diff` → CSS classes, gutter symbols, aria labels
- `LineMap::from_diff` → aligned row sequence, `ScrollAnchor`
- `cmd::NEXT_DIFFERENCE`, `cmd::PREV_DIFFERENCE` → `CommandRegistry`
- `FileSizeClass::classify` → large-file prompt before diff

**Acceptance criteria:**
- Line diff renders in two synchronised panes with correct decoration classes
- `F7`/`F8` navigate hunks; both panes scroll together
- Large files (> 4 MiB) show the FileSizeClass prompt before diffing

---

### ✓ Slice 2 — Merge actions wire to core *(shipped)*

**Goal:** A user can apply hunks left-to-right, undo, and see the dirty-state
marker in the tab title.

**Core types consumed:**
- `TextEditOperation::Replace` → applied to result buffer
- `TransactionLog::push` / `undo` / `redo`
- `WorkspaceSession::mark_tab_dirty` / `mark_tab_clean`
- `cmd::COPY_HUNK_LEFT_RIGHT`, `cmd::UNDO`, `cmd::REDO`

**Acceptance criteria:**
- Apply-hunk updates the right-pane rendered content
- Ctrl+Z undoes the last merge; Ctrl+Y/Ctrl+Shift+Z redoes
- Tab title shows `*` when dirty; clears after save

---

### ✓ Slice 3 — Save with safety checks *(shipped)*

**Goal:** A user can save a merge result; external modification is detected
and the reconciliation dialog is shown.

**Core types consumed:**
- `save_text` with `AtomicSaveStrategy` and `BackupPolicy`
- `check_external_state` before write
- `AppError::from_core` → `RecoveryAction` → dialog buttons
- `cmd::SAVE`, `cmd::SAVE_AS`

**Acceptance criteria:**
- Save writes atomically and optionally creates a `.bak` backup
- External modification triggers the reconciliation dialog
  (Compare / Reload / Save As / Cancel)
- Failed save preserves dirty state

---

### ✓ Slice 4 — Explorer and directory compare *(shipped)*

**Goal:** A user can browse two directories and see equal/modified/only-left
/only-right status icons.

**Core types consumed:**
- `DirectoryIndex::from_records` + `pair_entries` → `EqualityEvidence`
- `JobRegistry` → progress bar while scanning
- `ConflictFilter` / `AvailabilityRule::SelectedPathExists` → explorer actions
- `ExternalToolCommand::file_manager_reveal` → "Reveal in Finder" action

**Acceptance criteria:**
- Digest icons show ✓ / ⚠ / left-only / right-only correctly
- Progress bar shown while background digest jobs run
- Double-click same-name file opens diff tab (RFC-054 §2-ii)

---

### ✓ Slice 5 — Settings dialog *(shipped)*

**Goal:** A user can change theme, font size, compare profile, and newline
policy from a settings dialog; changes persist across restarts.

**Core types consumed:**
- `UserSettings::to_json` / `from_json` → config file read/write
- `ThemeId::css_var_names` → CSS variable injection
- `CompareProfile::all_presets` → profile dropdown
- `BomPolicy`, `NewlinePolicy` → file settings section

**Acceptance criteria:**
- Settings persist to `~/.config/forskscope/settings.json`
- Theme change applies immediately without restart
- Current v0.164 plain-JSON settings ignore unknown fields; RFC-076 replaces
  this runtime path with schema v2 and explicit future-version handling

---

### ○ Slice 6 — Three-way merge workspace *(core complete; UI deferred post-v1)*

**Goal:** A user can open a three-way merge session, resolve conflicts with
Use Left / Use Right / Edit, and save the merged result.

**Core types consumed:**
- `ThreeWayMergeSession::from_texts`
- `ConflictNavigator::build` → navigator rail
- `resolve_left` / `resolve_right` / `resolve_manual` / `ignore`
- `can_save()` → save-block predicate
- `cmd::USE_LEFT`, `cmd::USE_RIGHT`, `cmd::NEXT_CONFLICT`

**Acceptance criteria:**
- Navigator rail shows `!`/`L`/`R`/`B`/`~`/`-` status for each conflict
- Keyboard: `Alt+L` / `Alt+R` resolve focused conflict
- Ctrl+S disabled while any conflict is unresolved; enabled when all resolved

---

### ○ Slice 7 — Command palette *(deferred post-v1)*

**Goal:** A user can open the command palette (`Ctrl+Shift+P`), type to
filter, and execute any available command.

**Core types consumed:**
- `CommandRegistry::builtin()` + `search(query)`
- `AvailabilityRule::evaluate(ctx)` → disabled-with-reason
- `CommandContext` snapshot from session state

**Acceptance criteria:**
- Palette filters commands by label and description (case-insensitive)
- Unavailable commands show as dimmed with tooltip reason
- Escape closes palette; Enter executes selected command

---

### ○ Slice 8 — Editor adapter prototype *(gated on RFC-004; post-v1)*

**Goal:** Text editing is model-backed; edits flow through
`TextEditOperation` and diff is recomputed on change.

**Gate:** Requires a stable CodeMirror or equivalent editor integration.
This slice is not on the critical path for a functional v1 (the result
buffer can be write-only in v1), but is required for full manual-edit support.

**Core types consumed:**
- `TextEditOperation`, `RevisionId`, `OperationAck`/`OperationReject`
- `EditTransaction` + `TransactionLog`
- `DiffDecorationSet` → editor decoration push

---

## Remaining proposed RFCs

| RFC | When | What |
|-----|------|------|
| 004 | Slice 8 | Editor adapter and CodeMirror bridge |
| 010 | Post-slice-5 | Packaging, diagnostics, QA |
| 016 | Slice 8 | Editor bridge security and contract |
| 020 | Ongoing | CI and architecture test gates |
| 025 | Slice 8 | Editor adapter prototype and kill-switch |
| 026 | Post-slice-3 | Cross-platform WebView compatibility |
| 030 | Post-slice-5 | User documentation and onboarding |
| 040 | Slice 8 | Editor adapter verification harness |
| 041 | Post-v1 | v1.0 product stabilization |
| 042 | Ongoing | Roadmap (this document) |
| 074 | Pre-v1 stabilization | Umbrella schedule, milestones, gates, and final go/no-go package |
| 076 | Milestone M2 | Versioned runtime settings/session persistence and legacy migration |
| 077 | Milestone M3 | Git mergetool save-target identity and fingerprint safety |
| 078 | Milestone M5 | Platform runtime acceptance and retained release evidence |

---

## Non-goals (unchanged)

ForskScope is not and will not become:
- A full Git GUI
- An IDE
- A cloud diff service
- A file synchronization suite
- A universal document comparator
- An AI auto-merge agent
- A plugin marketplace

See `rfcs/done/001-core-extraction-and-domain-model.md` and
`rfcs/notes/forskscope-non-goals-v0.22.md` for the full non-goals policy.
