# Review Request: RFC-076 M2-B Patch 6 — Recovery UI and Documentation

**Date:** 2026-08-04
**Reviewer stance:** verification review of the final M2-B patch
**Repository baseline:** `95b184e`, on top of three same-day commits:
`db17ea7` (`persist: RFC-076 patch 6 - recovery UI and documentation
(closes B2)`, the substantive patch), `4af3678` (`action_label` test
coverage noticed while writing this request), `95b184e` (ROADMAP.md test-count
reconciliation for the above)
**Governing documents:** `rfcs/handoffs/076-versioned-runtime-persistence/recovery-ui-and-docs-handoff.md`;
RFC-076 including its 2026-08-03 amendment; F28, F28b, review 043 N1

## Implementation Summary

Replaced the interim startup toast for `Incompatible`, `CorruptPreserved`,
and `Migrated(Failed)` with blocking `SettingsRecoveryModal`/
`SessionRecoveryModal` dialogs (`crates/forskscope-ui/src/ui/overlay/modals/recovery.rs`,
new file), wired to real behavior for all four `RecoveryDialogAction`
variants, added F28/F28b wording, and updated the threat model, RFC-011, and
user docs. This completes M2-B and, per the handoff, M2.

## 1. Actions implemented (all four — none removed from the view-model)

Per §3.2, every offered action needed real behavior or had to be dropped from
the view-model with a note. All four were implementable safely; none was
dropped:

- **`Exit`** — `dioxus_desktop::window().close()`. With this app's single
  window and the default `exit_on_last_window_close: true`, this cleanly
  exits the event loop (confirmed in patch 6's earlier investigation of the
  vendored `dioxus-desktop` source, and now confirmed live — see runtime
  evidence, action 2).
- **`ContinueWithTemporaryDefaults`** / **`ContinueWithoutSaving`** —
  `advance_recovery_queue(&mut store)`. Neither writes anything; the run
  already holds `resolution.value` and `write_disabled = true` from startup,
  so "continue" only needs to stop blocking the UI.
- **`ResetAndBackupOriginal`** — calls a new core repository method,
  `SettingsRepository::reset_with_backup` / `SessionRepository::reset_with_backup`
  (`crates/forskscope-core/src/persist/schema/{settings,session}/repository.rs`),
  which reuses the existing `verify_unchanged` + non-overwriting-backup +
  atomic-write primitives `commit_migration` already used — no hand-rolled
  writer. The backup name is `<name>.reset.bak`, deliberately distinct from
  `.pre-v2.bak` per patch 5's handoff ("do not generalise it" — a `Corrupt`
  reset is not the v2-migration event `.pre-v2.bak` names). On success,
  `SettingsRecoveryModal` updates `store.settings`/`store.settings_v2_base`/
  `store.settings_write_disabled`; `SessionRecoveryModal` updates
  `store.session_write_disabled`. On failure (e.g. the file changed
  underneath the dialog), `store.notify(e.to_string())` reports it and the
  queue still advances — the dialog does not get stuck open.

## 2. Runtime evidence

Built the debug binary and ran it under an isolated `XDG_CONFIG_HOME`
(F32-handoff technique), driving it via AT-SPI (`Atspi.Action.do_action`) since
no pointer/keyboard-injection tool is available in this Wayland session —
`ydotool`/`wtype` are absent and `xdotool` doesn't reach a native Wayland
window. `niri msg action screenshot-window --path <absolute>` captured each
state.

1. **`Incompatible` (settings)** — future-version `settings.json`
   (`schema_version: 99`). Dialog shows: title "Settings file is from a newer
   version", body ending "Changes you make this session will not be saved.",
   buttons **Exit** / **Continue with defaults** (no Reset — correct, a future
   file may be valid to a newer build). AT-SPI confirms the accessible tree:
   `dialog > heading, paragraph, button "Exit", button "Continue with
   defaults"`.
2. **Clicked "Continue with defaults"** via `Atspi.Action.do_action` — dialog
   dismissed cleanly, Explorer became interactive, no panic in stdout.
3. **`CorruptPreserved` (settings) + `Incompatible` (session) simultaneously**
   — `settings.json` = `{not valid json`, `session.json` = future-version.
   Settings dialog shows "Settings file could not be read", buttons
   **Continue with defaults** / **Reset and back up** (no Exit — correct).
   Clicked **Reset and back up** via AT-SPI: `settings.json.reset.bak`
   appeared on disk holding the original 15 corrupt bytes, `settings.json`
   was rewritten with valid defaults (1985 bytes) — confirms the backup lands
   before the new value, per §3.2's "only on explicit confirmation."
4. **F28b confirmed** — immediately after the settings dialog was resolved,
   the *queued* session dialog ("Session file is from a newer version")
   appeared automatically, with its own Exit/Continue buttons. Neither
   document's failure was dropped; both were shown in sequence, exactly as
   §3.4 requires. This is `Store.pending_recovery` populated from both
   `app.rs` call sites (settings at `use_context_provider`, session at
   `use_hook`) and drained one at a time by `advance_recovery_queue`.

Screenshots are session-scratch artifacts, not committed (transient
verification evidence, not a repo deliverable) — happy to re-run and attach
if you want them archived somewhere durable.

## 3. F33 boundary — confirmed untouched

`README.md` and `docs/src/users/installation.md` were not opened or edited.
`git show --stat db17ea7` touches only the files §7 lists for this patch
(`docs/src/maintainers/threat-model.md`, `docs/src/users/settings.md`,
`docs/src/users/faq.md`, the `rfcs/proposed/076-*.md` → `rfcs/done/` move,
`rfcs/README.md`), plus two files not in §7's list that I judged were the
same "as RFC-075 did" lifecycle-move convention §3.6 points at:
`ROADMAP.md` (test-count line, a closed progress note, dropping RFC-076's now-done
row from the workstream-dependency table) and `rfcs/proposed/074-v1-release-stabilization-program.md`
(a Progress-record entry for M2-B/B2, mirroring the existing B1 entry —
verified against `f04f5cad` before writing it). Flagging this explicitly per
the handoff's "if this patch needs to edit README.md, say so" instruction,
even though what I touched is ROADMAP/RFC-074, not README — wanted the
boundary check visible rather than assumed.

## 4. RFC-076 lifecycle move

- `git mv rfcs/proposed/076-versioned-runtime-persistence.md rfcs/done/076-versioned-runtime-persistence.md`.
- Status line: `Proposed` → `Implemented (Milestone M2-B)` (version left for
  the release cut, per the handoff — RFC-075's precedent).
- Added an `## Implementation outcome` section citing all six patch commits
  (`5054000`, `b710e7f`, `9abafab`, `2a26f09`, `62c61f8`, and this patch) and
  confirming every acceptance criterion in the RFC's own list is met.
- `rfcs/README.md`: Implemented 49 → 50 (row added under `done/`), Proposed
  18 → 17 (row removed from `proposed/`), prose sentence updated to name
  RFC-076 alongside RFC-075 as implemented-but-still-No-Go.

## Files Changed

New: `crates/forskscope-ui/src/ui/overlay/modals/recovery.rs`.

Core: `persist/schema/repository.rs` (+`ensure_reset_backup`/`reset_backup_path`),
`persist/schema/{settings,session}/repository.rs` (+`reset_with_backup`),
`persist/schema/{settings,session}/runtime.rs` (+`raw_bytes` field, threaded
through every `resolve_and_commit` arm and `commit_migrated`).

ui-logic: `settings/persistence_recovery.rs`, `session/persistence_recovery.rs`
(+`action_label`, F28 body-text extensions), `lib.rs` (+`action_label`
re-exports), `settings/settings_view.rs` (review 043 N1 doc fix, no code
change).

UI: `state.rs` (+`Modal::{Settings,Session}Recovery`, +`Store.pending_recovery`,
+`advance_recovery_queue`), `state/session.rs` (+`recovery_modal`,
+`reset_session`/`reset_session_with_backup`, `recovery_notice` no longer
toasts a dialog outcome), `ui/view/settings.rs` (mirror +`recovery_modal`,
+`reset_settings`/`reset_settings_with_backup`, `ModalLayer` match arms),
`ui/overlay/modals.rs` (+`recovery` submodule), `app.rs` (queue population at
both startup call sites, Escape exemption for the two recovery variants),
`i18n.rs` (+34 new keys: 6 titles, 9 body fragments — some shared between
settings/session — and 4 action labels, doubled for their translations).

Tests: `persist_v2_repository_tests.rs` (+6 — core-level `raw_bytes`/
`reset_with_backup` coverage), `persist_v2_runtime_tests.rs` (2 tests
extended, not new), `state/session/tests.rs` (+4:
`reset_session_writes_backup_and_new_value_through_the_real_repository`,
`reset_session_rejects_stale_bytes_after_external_change`,
`recovery_modal_is_none_for_outcomes_without_a_dialog`,
`recovery_modal_is_some_for_outcomes_with_a_dialog`), `ui/view/settings/tests.rs`
(mirror +4 — see the itemized table below).

Docs: `docs/src/maintainers/threat-model.md` (§4 rewritten + audit-history
row), `rfcs/done/011-workspace-session-persistence.md` (+amendment note),
`docs/src/users/settings.md` (+"Where your settings and session are stored"
section), `docs/src/users/faq.md` (+one entry, cross-linked).

## Test-Count Delta, Itemized

Baseline (handoff §4): **1007** (652 core unit + 27+16+2 core integration +
21 `forskscope-ui` unit ×2 lib/bin targets + 255 ui-logic unit + 6 CSS + 6
core doctests + 1 ui-logic doctest). Current: **1031**
(658+27+16+2+29+29+257+6+6+1). **Delta: +24.**

| Source | Delta | Reason |
|---|---:|---|
| `persist_v2_repository_tests.rs` (core) | +6 | `reset_with_backup`/`ensure_reset_backup` coverage: writes-through, non-overwrite, stale-bytes-rejected, for both settings and session |
| core unit total | +6 (652→658) | same six, counted at the crate level |
| `ui/view/settings/tests.rs` | +4, run twice (lib+bin) = +8 | see table below |
| `state/session/tests.rs` | +4, run twice (lib+bin) = +8 | session mirror of the same four |
| `settings/persistence_recovery.rs` (ui-logic) | +1 | `all_recovery_dialog_actions_have_non_empty_labels` |
| `session/persistence_recovery.rs` (ui-logic) | +1 | mirror |
| **Total** | **+24** | |

The four new `forskscope-ui` tests (run once per `lib`/`bin` target, hence
counted twice above):

| Test | File | Covers |
|---|---|---|
| `reset_settings_writes_backup_and_new_value_through_the_real_repository` | `ui/view/settings/tests.rs` | `reset_settings` round-trips through the real `SettingsRepository`, not just core's own `reset_with_backup` |
| `reset_settings_rejects_stale_bytes_after_external_change` | same | the UI wrapper still refuses a stale-bytes reset |
| `recovery_modal_is_none_for_outcomes_without_a_dialog` | same | `Fresh`/`Current`/`Migrated(Committed\|DeferredByConflict)` never queue a modal |
| `recovery_modal_is_some_for_outcomes_with_a_dialog` | same | `Incompatible` (and by the shared `SettingsRecoveryView` logic, the other two dialog outcomes) does |
| (session mirror ×4) | `state/session/tests.rs` | same four, for `SessionRepository`/`SessionRuntimeResolution` |

Ran: `cargo test --workspace` — 1031 passed, 0 failed, 1 ignored
(pre-existing doctest, unrelated).

## Known Gap: F28b's Queue Mechanism Has No Headless Unit Test

`advance_recovery_queue` and the `pending_recovery` population in `app.rs`
touch `Store`, which needs a live Dioxus scope (`Signal::new_in_scope`
panics otherwise — the same constraint patch 4 hit and documented for
`persist`/`Store::new`). No test anywhere in this codebase constructs a real
`Store`, so I did not add one either rather than introduce a new,
unprecedented test-harness pattern in a patch that's already touching a lot
of surface. What *is* unit-tested is the pure half each queue entry is built
from (`recovery_modal` returning the right `Modal` variant per outcome, both
sides). What proves the queue itself works is runtime evidence item 4 above —
I'd rather flag this honestly than claim coverage that isn't there. If you
want a `Store`-level test added, I can look at what a minimal `VirtualDom`-backed
harness would take, but wanted to raise it as a question rather than assume
the answer.

Similarly, `ContinueWithTemporaryDefaults`/`ContinueWithoutSaving` leaving the
file byte-identical is not re-tested at the dialog-action level — structurally
true (neither arm calls anything that writes; `advance_recovery_queue` only
mutates `store.modal`/`store.pending_recovery`), and generically covered by
patch 4's `future_version_session_stays_byte_identical_through_a_disabled_save`
at the write-disable-gate level, but there's no test that simulates clicking
the button specifically. Same reasoning as above for why I didn't add one.

## Not Addressed Here (per the handoff's own scope)

- F23 (`actionlint`), F24, F25/F25b, F31, F34, F35 — explicitly out of scope,
  per §2.
- The migration-notice-for-a-successful-migration toast — unchanged, already
  correct per §3.1.
- `README.md` / `docs/src/users/installation.md` — F33's, untouched (§3 above).

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1031 (658+27+16+2 core, 257 ui-logic, 58 forskscope-ui, 6 css, 7 doctests)
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 220 keys
cargo xtask css --check                                          pass
cargo xtask audit-deps                                           pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
mdbook build (docs/)                                             pass, no broken-reference warnings
```

CI runs, all green: `30871069834` (`db17ea7`), `30871567218` (`4af3678`),
`30872691671` (`95b184e`, current `main`).

## Requested Review Focus

1. Is `Exit` → `dioxus_desktop::window().close()` the right call for every
   platform this app ships on, or does it need a platform-specific fallback
   the handoff's §5 "known limitations" anticipated? I only verified it on
   Linux/WebKitGTK (runtime evidence above); Windows/macOS are RFC-078's.
2. The F28b known-gap above — is the runtime-evidence-only coverage
   acceptable for a release-blocking dialog, or does this patch need a
   `Store`-level test harness before M2 can close? I'd rather raise this now
   than have it surface at RFC-078's platform-acceptance stage.
3. `.reset.bak` vs `.pre-v2.bak` — confirm the distinct naming (and the
   accompanying doc language in `settings.md`/`faq.md`) reads clearly to a
   non-technical user who doesn't know what "v2" or "migration" means.
4. The ROADMAP.md/RFC-074 touches beyond §7's explicit list (item 3 above) —
   confirm these were the right scope to include under "as RFC-075 did"
   rather than something that should have been left for a separate note.
