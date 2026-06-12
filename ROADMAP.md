# ForskScope Roadmap

**Last updated:** v0.103.0 (2026-06-12)
**Current phase:** UI stabilisation — core complete, UI features shipping, governance documents current

---

## Current state

The `forskscope-core` and `forskscope-ui-logic` crates are feature-complete
for the v1 diff/merge workflow. 39 of 48 RFCs are implemented. **875 tests**
pass with zero failures (646 core unit + 25 diff corpus + 2 patch apply +
189 ui-logic + 5 CSS coverage + 7 doctest + 1 ui-logic integration).

The UI crate (`forskscope-ui`) is feature-complete and actively maintained:
two-pane diff layout with independent pane labels and shared horizontal scroll;
theme-aware select styling; ESC key closes modals; i18n complete across all
components (English + Japanese); all modal dialogs translated. User
documentation covers all four primary workflows.

The `platform` module provides runtime diagnostics for the About panel
(RFC-026). The acceptance test corpus covers 26 fixture files across
text/newlines/whitespace/encoding/binary categories.

The next phase is connecting remaining UI slices to the view-model layer
and preparing for a public v1 release candidate (RFC-041).

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
| Docs + platform | v0.95–v0.96 | Testing/architecture/local-dev docs updated; 4 user docs rewritten |
| CONTRIBUTING + limits | v0.97–v0.98 | ROADMAP/release/features updated; CONTRIBUTING.md; known-limitations.md |
| RFC-041 + v0.100 | v0.99–v0.100 | RFC-041 checklist updated; PlatformInfo wired to About; patch export UI |

---

## UI implementation slices

The remaining work is a series of UI slices that wire the Dioxus components
to the core types. Each slice delivers a testable, usable increment.

### Slice 1 — Diff view renders and navigates

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

### Slice 2 — Merge actions wire to core

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

### Slice 3 — Save with safety checks

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

### Slice 4 — Explorer wired to directory compare

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

### Slice 5 — Settings dialog

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
- Unknown settings file fields are silently ignored (schema v1 forward-compat)

---

### Slice 6 — Three-way merge workspace

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

### Slice 7 — Command palette

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

### Slice 8 — Editor adapter prototype (gated, RFC-004)

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
