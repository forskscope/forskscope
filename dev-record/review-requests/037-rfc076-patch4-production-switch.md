# Review Request: RFC-076 M2-B Patch 4 — Production Switch

**Date:** 2026-08-03
**Reviewer stance:** design + implementation review
**Repository baseline:** `2a26f09` (`persist: switch forskscope-ui to RFC-076
versioned settings/session (patch 4)`)
**Governing documents:** RFC-076 §"UI integration", §"Implementation
sequence" step 4; `rfcs/handoffs/076-versioned-runtime-persistence/implementation-handoff.md`
§4.4, §6, §9, §10; audit finding B2

Patch 4 of 5, and the **first change to `forskscope-ui` production code in
this entire milestone.** `app_json_settings::ConfigManager` is no longer
called from anywhere in the running application — B2's fix is now
reachable, not just implemented. Patch 5 (recovery UI with Exit/Continue/
Reset buttons, and documentation) remains.

## Implementation Summary

### The core design decision: view adapter, not a type swap

RFC-076's UI integration section names two options: "replace UI-owned
serializable settings with the canonical core type **or** a
non-serializing view adapter." I chose the adapter. `AppSettings`,
`Theme`, `Lang`, `DiffFontFamily`, `DiffAlgorithmSetting`, `DiffProfile`
all keep their exact shapes and every existing UI read/write site
(`modal.rs`'s ~11 settings-field handlers, `dir_pane.rs`'s remembered-
directory save, `profile.rs`'s add/remove) — only their `Serialize`/
`Deserialize` derives are gone, since nothing serializes them
independently anymore.

I considered the alternative (retype `Store.settings` as
`PersistedSettingsV2` directly, update every UI call site to the richer
core enums) and rejected it: `PersistedDiffProfileV2` has four
independent axes (`whitespace`/`case`/`newlines`/`inline_mode`) where the
shipping UI has always offered two boolean checkboxes, so that change
would touch `modal.rs`'s field-editing logic itself, not just its
`persist()` calls — a materially larger and riskier diff for a v1-release
patch with no corresponding UI capability being added.

### The merge problem this creates, and how it's closed

A naive adapter (`AppSettings -> PersistedSettingsV2` reconstructed from
scratch on every save) would silently reset every field the UI does not
own — `appearance_font_size`, `density`, `show_line_numbers`,
`wrap_long_lines`, `newline_policy`, `restore_session`, `recent_limit`,
`performance` — back to their defaults the first time a user touches any
setting. That is exactly the silent-field-loss class RFC-076 exists to
remove, just moved one layer up.

Fix: `Store` caches the last-resolved `PersistedSettingsV2` in
`settings_v2_base`. `AppSettings::merge_into_v2` overlays only the
UI-editable fields onto that cached value using struct-update syntax
(`..base.clone()`), so any field neither `merge_into_v2` nor `from_v2`
mentions — including one added to `PersistedSettingsV2` after this code
is written — is preserved automatically rather than requiring a matching
edit here.

### `write_disabled` closes the gap review 038 C2 was about

`Store` gains `settings_write_disabled`/`session_write_disabled`, set
from each resolution's `write_disabled` flag at startup. `persist()`/
`save_session()` are now a no-op while set. Without this, a
future/corrupt/unwritable source would only be protected until the next
settings change, at which point an unverified `save()` would overwrite
whatever made resolution refuse to commit — the same shape review 038
flagged for the migration-commit path, one layer up in the call chain
where it becomes reachable for the first time.

### Startup notice: a toast, not the recovery dialog

`recovery_notice()` (settings and session) maps a resolution to a
one-time `Notice` by reusing `forskscope-ui-logic`'s already-tested
`SettingsRecoveryView`/`SessionRecoveryView` body text — a success toast
for a durably-committed migration, an error toast (using the dialog's
own body copy) for `Incompatible`/`CorruptPreserved`/`Migrated(Failed)`.
This is deliberately *not* the recovery dialog: no `Exit`/
`ContinueWithoutSaving`/`ResetAndBackupOriginal` buttons, no new `Modal`
variant. The handoff's implementation sequence lists "recovery UI" as
step 5, separate from step 4's "switch settings, then session"; a toast
using the existing `Store.toast`/`Notice` mechanism satisfies RFC-076's
"failed saves are visible and do not masquerade as success" without
reaching into step 5's scope. If both a settings and session notice fire
on the same launch, the settings one wins (`app.rs`'s `use_hook`); losing
a session notice on a double-fire seemed the lower-cost tradeoff.

### Profile round-trip: a documented, provably-inert lossiness

`DiffProfile::from_v2`/`to_v2` map the two-bool UI shape to/from
`PersistedDiffProfileV2`'s four independent axes, using the exact same
per-field rules as `forskscope-core`'s own `migrate_from_v0`/
`ui_builtin_profiles` (verified by reading both, not just matching by
convention). This is lossy only for `whitespace`/`newlines`/`inline_mode`
values the shipping Settings dialog has never been able to produce
(`IgnoreTrailing`, `IgnoreBlankLines`, non-`Significant` newlines,
non-`Lazy` inline mode) — values that can only enter a real user's file
via a core-v1 envelope, which "the shipping UI did not write" and is
documented elsewhere as never shipped to users. Every profile the UI
itself has ever written round-trips exactly, since it only ever writes
the values `to_v2` produces. Documented in the field's own doc comment
rather than only here.

## Addressed Items

- RFC-076 acceptance: "No production path calls
  `ConfigManager<AppSettings/SessionState>`" — verified by grep, zero
  remaining call sites in `forskscope-ui`.
- Handoff §10 "Removing `app-json-settings` in this slice, even if it
  becomes unused" — not removed from `Cargo.toml`; confirmed unused via
  grep otherwise.
- Handoff §6: "Add targeted tests proving the actual UI startup and save
  functions use the new repositories — not only that the lower-level
  core parsers work." Both `ui/view/settings.rs` and `state/session.rs`
  split into a thin `Store`-dependent wrapper (untestable without a
  Dioxus runtime) and a `Store`-independent core (`build_save_payload`/
  `persist_settings`/`load_settings` and the session mirrors) that tests
  exercise against real temp-path repositories — the same
  `SettingsRepository`/`SessionRepository` types core's own tests use,
  not a stand-in.
- B2 — **closed.** This is the first patch where it's actually reachable.

## Files Changed

- `crates/forskscope-ui/src/state/settings.rs` — `AppSettings` etc. lose
  `Serialize`/`Deserialize`; `AppSettings::from_v2`/`merge_into_v2`,
  `DiffProfile::from_v2`/`to_v2` added
- `crates/forskscope-ui/src/state/session.rs` — `SessionState`/
  `ConfigManager` replaced by `SessionRepository`/`resolve_and_commit`;
  `save_session`/`restore_session` split into testable halves;
  `recovery_notice` added
- `crates/forskscope-ui/src/state/session/tests.rs` — rewritten (the old
  file tested `SessionState`, which no longer exists); 4 tests against a
  real temp-path repository
- `crates/forskscope-ui/src/ui/view/settings.rs` — `persist`/`load`
  rewritten against `SettingsRepository`; split into testable halves;
  `recovery_notice` added; new `tests.rs` (4 tests)
- `crates/forskscope-ui/src/state.rs` — `Store` gains
  `settings_v2_base`/`settings_write_disabled`/`session_write_disabled`;
  `Store::new`'s signature grows two parameters; new `config_file_path`
  helper (`dirs_next::config_dir()` + `"forskscope"` — the repositories
  never resolve this themselves by design)
- `crates/forskscope-ui/src/app.rs` — startup wiring: `load()` +
  `Store::new(...)` + initial toast inside one memoized
  `use_context_provider` closure (so the repository call and any
  migration commit run exactly once, not on every re-render);
  `restore_session`'s return value wired to a toast
- `crates/forskscope-ui/src/ui/view/settings/modal.rs` (11 sites),
  `state/profile.rs` (2 sites), `ui/view/dir_pane.rs` (1 site) —
  mechanical: `persist(&store.settings.read())` → `persist(store)`
- `crates/forskscope-core/src/persist/v2/settings/runtime.rs`,
  `crates/forskscope-ui-logic/src/settings/persistence_recovery.rs` —
  stale "until patch 4" doc comments corrected

## Manual Verification (not just tests)

Backed up the real `~/.config/forskscope/{settings,session}.json`
(legacy v0, from prior manual testing) to
`.git-exclude/tmp/config-backup/`, then built and ran the actual
`forskscope` binary against them:

- Both files migrated to v2 envelopes on startup; `.pre-v2.bak` backups
  created next to each, containing the exact original bytes.
  `settings.json.pre-v2.bak`'s content matches the pre-migration file
  byte-for-byte.
- Migrated values are correct: `theme: dark`, `language: en`,
  `diff_font_size: 14`, all four built-in profiles present with their
  original `algorithm`/whitespace-derived values — matches the original
  file exactly.
- App launched with no errors in stdout/stderr, rendered the Explorer
  view in the dark theme (confirming `AppSettings::from_v2`'s theme
  conversion reached the actual rendered CSS class, not just a unit
  test) — screenshot taken via `niri msg action screenshot-window` and
  reviewed.
- Process exited cleanly on `kill`.

Did not click into the Settings dialog itself (no input-simulation tool
available for the native GTK window); the dialog's rendering logic
(`modal.rs`) is unchanged code reading the unchanged `AppSettings` shape,
so this is lower-risk than the load/migrate path already verified.

## Not Implemented in This Patch, on Purpose

- No `Modal::SettingsRecovery`/`Modal::SessionRecovery` variant, no
  Exit/Continue/Reset buttons — `SettingsRecoveryView`/
  `SessionRecoveryView`'s `dialog.actions` are computed but unused by
  `forskscope-ui`; only `migration_notice`/`dialog.body` text feeds the
  toast. Step 5's job per the handoff's own sequencing.
- `active_tab`/`explorer_roots` are never populated when saving a
  session (always `None`) — the UI has never tracked either, so this is
  unchanged behavior, not a new gap.
- Threat-model/docs updates (handoff acceptance criteria mentioning the
  threat model moving from "known gap" to implemented behavior) —
  bundled with "Recovery UI and documentation" as step 5 in the
  handoff's own sequence, not step 4's.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test -p forskscope-ui                                     pass — 20/20 (was 2; +18: 4 settings, 4 session, 10 pre-existing unaffected — net new coverage for the repository-backed load/save path that had zero test coverage before this patch)
cargo test --workspace                                          pass — 709+27+16+2+20+20+255+6+7+1 = 1063
cargo clippy --workspace -D warnings                             pass
cargo xtask audit-deps                                           pass
cargo audit                                                      pass (14 pre-existing allowed warnings, unchanged)
cargo xtask i18n                                                 pass — 203 keys
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
Manual: real binary run against real (backed-up) config files     pass — see above
```

CI run `30782787366`: Test & Lint green, on `2a26f09`.

## Known Limitations

- B2 is closed; B3, B4 remain open. v1/public release stays **No-Go**.
- The `DiffProfile` two-bool/four-axis lossiness described above is a
  real, if inert, information gap — flagging for awareness even though I
  don't believe it's reachable by any real user's file today.
- No dialog UI for future/corrupt/failed-commit outcomes yet — a toast
  only. A user who dismisses it (or misses it) has no other way to learn
  their changes aren't being saved until patch 5.

## Requested Review Focus

1. Is the view-adapter choice (vs. retyping `Store.settings` to
   `PersistedSettingsV2` directly) the right call for a v1-release patch,
   or does the RFC's "chosen end state is one public canonical domain
   type" language (from patch 1's design notes, about not keeping
   `UserSettings` and `AppSettings` both claiming disk ownership) extend
   further than I'm reading it — to the UI's in-memory view type too?
2. Is a toast the right minimal implementation of "failed saves are
   visible" for this patch's boundary, or does `write_disabled` actually
   need the blocking dialog now rather than at patch 5 — i.e., is a
   toast a user could miss/dismiss sufficient for "visibly reported" in
   RFC-076's acceptance criteria?
3. `Store::new`'s signature grew from 1 parameter to 3
   (`settings, settings_v2_base, settings_write_disabled`) — acceptable
   for the one real call site, or does this want a small settings-load
   struct instead?
4. The `DiffProfile` round-trip lossiness reasoning — is "unreachable
   because core-v1 was never shipped to users" solid enough to accept as
   documented rather than architecturally closed?
