# RFC-076 Patch 6 Developer Handoff: Recovery UI and Documentation

**Governing RFC.** [RFC-076](../../proposed/076-versioned-runtime-persistence.md),
including its 2026-08-03 amendment
**Program.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md) — milestone M2-B, final patch
**Register items.** F28, F28b, review 043 N1
**Completes.** M2-B, and with it M2

This handoff directs execution. It does not redefine RFC-076. If implementation
evidence contradicts a decision below, amend the RFC first, then update this
handoff.

## 1. Summary

Patch 4 wired production onto the versioned persistence path and reported
recovery states as **toasts**, explicitly deferring the dialogs to this patch.
That was accepted on one condition, recorded in review 041 §4.2: patches 4, 5,
and 6 ship in a single release, so no user ever experiences the toast-only
state. **This patch is what makes that condition true, which is why its dialog
is release-blocking for M2's cut.**

RFC-076 does not say "notify" for a future-version file. It says *show an
incompatibility dialog offering Exit, Continue with temporary defaults*. A toast
a user can miss does not satisfy that.

This is also the documentation half of M2-B: the threat model still describes
the settings-persistence gap as open, and it has been closed since patch 4.

## 2. Scope

In scope:

- recovery dialogs for `Incompatible`, `CorruptPreserved`, and
  `Migrated(Failed)`, replacing the toast for those three outcomes;
- behaviour behind every action the dialogs offer;
- F28 — dialog copy stating that changes will not be saved;
- F28b — communicating that *both* documents can be write-disabled;
- review 043 N1 — two stale doc lines referencing the deleted `UserSettings`;
- documentation: threat model, RFC-011 status, user-facing persistence and
  downgrade guidance;
- moving RFC-076 to `rfcs/done/`.

Out of scope — do not include:

- **`README.md` and `docs/src/users/installation.md`.** F33 is the architect's
  and is being held until this patch lands, specifically to avoid a second
  concurrent-edit collision under `docs/`. See §7 for the file boundary.
- F23 (`actionlint`) — separate, also before M2's cut;
- F24, F25/F25b, F31, F34, F35 — M4's;
- the migration notice for a *successful* migration, which stays a toast and is
  already correct.

## 3. Design decisions

### 3.1 Dialogs replace toasts for three outcomes only

| Outcome | Presentation |
|---|---|
| `Fresh`, `Current` | nothing |
| `Migrated(Committed)` | toast — unchanged, already correct |
| `Migrated(DeferredByConflict)` | silent — unchanged, benign and self-healing |
| `Migrated(Failed)` | **dialog** |
| `Incompatible` | **dialog** |
| `CorruptPreserved` | **dialog** |

The view-models already compute exactly this. `SettingsRecoveryView` and
`SessionRecoveryView` return `dialog: Option<RecoveryDialogView>` with title,
body, and ordered `actions` — all currently discarded by `forskscope-ui`, which
uses only `dialog.body` as toast text. Render what is already there rather than
recomputing it in the component.

### 3.2 Every offered action must do something

`RecoveryDialogAction` has four variants and **none has behaviour behind it**
today. A dialog button that does nothing is a defect, not a placeholder — the
same standard applied when `ChooseAnotherLocation` was correctly declined in
patch 3.

- `Exit` — terminate cleanly. Nothing is written.
- `ContinueWithTemporaryDefaults` — proceed with defaults; writes stay disabled.
- `ContinueWithoutSaving` — proceed with the *migrated* value (`Failed` only);
  writes stay disabled.
- `ResetAndBackupOriginal` — **the only one that writes.** Back up the original
  before replacing it, and only on explicit confirmation. RFC-076: "Any reset is
  an explicit confirmed action that creates a backup." Reuse the existing
  safe-write primitives; do not hand-roll a writer.

If any action cannot be implemented safely in this patch, remove it from the
view-model's action list rather than rendering a button that fails silently, and
report that in the review request.

### 3.3 F28 — the dialogs must state the consequence

`write_disabled` is true for all three dialog outcomes, but only
`Migrated(Failed)`'s body currently says changes will not be saved.
`Incompatible` says "The file has not been modified"; `CorruptPreserved` says
the file "is preserved but could not be parsed". Neither tells the user that
nothing they change will persist.

A user who picks *Continue* on a future-version file gets a working application
whose settings silently do not save. Extend both bodies to state it.

### 3.4 F28b — two documents, two possible failures

Settings and session resolve independently and both can be write-disabled on the
same launch — a corrupt `settings.json` beside a future-version `session.json`
is entirely possible. The startup notice currently drops the session message when
a settings message exists, which was an accepted trade-off for toasts.

It must not survive into the dialogs. A user told their settings are read-only
and told nothing about their session is being misinformed by omission. Whether
that is one dialog covering both or two shown in sequence is your call; the
property is that **neither can be silently dropped**.

### 3.5 i18n is a gate, not an afterthought

Every new user-visible string goes through `t(...)` and needs a Japanese
translation. `cargo xtask i18n` is a release gate and will fail otherwise. Dialog
titles, bodies, and all four action labels are user-visible.

### 3.6 Documentation

- **Threat model** — §4 "Settings persistence" still describes the B2 gap as
  open, with the residual-concerns paragraph added in M2-A. B2 closed at patch 4.
  Rewrite it to describe the implemented behaviour: core-owned versioned schema,
  explicit `Missing`/`Current`/`Migrated`/`FutureVersion`/`Corrupt` handling,
  write-disable, and migration backup. Add an audit-history row for the release
  this ships in.
- **RFC-011** — its identity-rich session model was removed entirely by patch 5.
  Amend its status to record that, so the RFC folder does not imply a model that
  no longer exists. Do not rewrite its history; add the note.
- **User docs** — `docs/src/users/settings.md` and `faq.md` should cover what a
  user sees when a config file cannot be read, and the downgrade caveat: a
  schema-v2 file is not readable by `0.165.0` or earlier, and `.pre-v2.bak` is
  the recovery path.
- **RFC-076 lifecycle** — move `rfcs/proposed/076-*.md` to `rfcs/done/`, update
  its Status field, and update `rfcs/README.md`'s counts and links in the same
  commit (RFC-000). The release version is not known until the cut, so use a
  form like `Implemented (M2-B)` and let the cut record the version, as RFC-075
  did.

## 4. Tests and gates

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo xtask i18n
cargo xtask css --check
cargo xtask audit-deps
cargo xtask version-sync
git diff --check
```

Baseline is **1007**. Report the delta and what each new test covers.

Required test coverage:

- each dialog outcome produces the expected view-model, including the F28 copy;
- both-documents-write-disabled produces both messages (F28b);
- `ResetAndBackupOriginal` creates a backup before replacing, and does not
  proceed without explicit confirmation;
- `ContinueWithTemporaryDefaults` and `ContinueWithoutSaving` leave writes
  disabled — assert the file is byte-identical after the session, in the manner
  of `future_version_session_stays_byte_identical_through_a_disabled_save`.

**Runtime check.** This patch adds UI that no headless test renders. F32 was a
visual defect that every gate passed for two releases. Run the real binary
against a future-version fixture under an isolated `HOME`, confirm the dialog
appears and its buttons behave, and include the evidence. The technique is in
the F32 handoff §4.

## 5. Known limitations to expect

- `Exit` from a Dioxus desktop app needs a clean shutdown path; if one does not
  exist, say so rather than approximating it.
- The dialogs are modal at startup, before the main workspace is usable. Confirm
  the existing `ModalLayer` handles that ordering.
- F35 (blank counterpart rows announcing `Changed:`) is untouched and remains
  M4's.

## 6. Acceptance criteria

- `Incompatible`, `CorruptPreserved`, and `Migrated(Failed)` each render a
  dialog, not a toast.
- Every rendered action has behaviour; none is inert.
- Both dialog bodies state that changes will not be saved (F28).
- Neither document's failure can be silently dropped (F28b).
- `ResetAndBackupOriginal` backs up before replacing, only on confirmation.
- `cargo xtask i18n` passes with the new strings covered.
- The threat model describes implemented behaviour; RFC-011 records the removal;
  user docs cover recovery and the downgrade caveat.
- RFC-076 is in `rfcs/done/` with `rfcs/README.md` updated in the same commit.
- Review 043 N1's two stale `UserSettings` doc lines are corrected.
- Runtime evidence shows a dialog actually appearing.
- All gates in §4 pass.

## 7. File boundary with F33

F33 (README, installation section, screenshots) is held until this patch lands,
to avoid a repeat of the 2026-08-04 collision. To keep the boundary clean, this
patch owns:

```text
docs/src/maintainers/threat-model.md
docs/src/users/settings.md
docs/src/users/faq.md
rfcs/proposed/076-*.md → rfcs/done/
rfcs/README.md
```

F33 will own `README.md` and a new `docs/src/users/installation.md`, and will not
touch the files above. If this patch needs to edit `README.md`, say so in the
review request rather than editing it — I will fold it into F33.

## 8. Required review-request content

Standard format, plus:

1. which actions were implemented and which, if any, were removed from the
   view-model for lack of safe behaviour;
2. the runtime evidence — how the dialog was exercised and what was observed;
3. confirmation that no file listed in §7 as F33's was touched;
4. the RFC-076 lifecycle move, with `rfcs/README.md` counts reconciled.
