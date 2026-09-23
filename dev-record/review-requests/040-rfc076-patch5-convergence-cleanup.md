# Review Request: RFC-076 M2-B Patch 5 — Convergence Cleanup

**Date:** 2026-08-04
**Reviewer stance:** verification review of a removal + rename patch
**Repository baseline:** `62c61f8` (`persist: RFC-076 patch 5 - convergence
cleanup`), with `17ae878` on top separating your F32/F33/F34/F23 doc work
**Governing documents:** `rfcs/handoffs/076-versioned-runtime-persistence/convergence-cleanup-handoff.md`;
RFC-076's 2026-08-03 amendment; F29, F30

A process note precedes this in `dev-record/review-requests/039-git-history-collision-incident.md`
— a concurrent-edit collision split this patch's file-level operations and
content edits across two of your commits under unrelated messages; I
restructured local history to separate them before this review request.
That note has the full account; this request covers only the patch content.

## Implementation Summary

Removed the core schema-v1 migration path and the legacy models it was the
only consumer of, confirmed unreachable by grepping every candidate type
across the entire workspace (not just `forskscope-core`) before deleting
anything, per the handoff's "check each type for consumers... anything
with a live non-test consumer stays, and you report it" instruction.
Renamed `persist::v2` → `persist::schema` and dropped the `V2` suffix from
the five `Persisted*V2` types and the two `load_*_v2` functions.

## Addressed Items

- **RFC-076 2026-08-03 amendment / F29** — `UserSettings` (and its
  `AppearanceSettings`/`DiffSettings`/`FileSettings`/`LocaleSettings`
  cluster), `WorkspaceSession` (and `WorkspaceTab`/`SessionId`/`TabId`/
  `WorkspaceRoot`/`RecentSessionEntry`/`CloseResult`), and
  `VersionedEnvelope`/`ParsedEnvelope`/`SchemaName`/`MigrationPolicy`/
  `EnvelopeError` are gone.
- **F30** — `persist::schema`, `PersistedSettings`, `PersistedSession`,
  `PersistedDiffProfile`, `PersistedComparePair`, `PersistedDirectoryPair`,
  `load_settings`, `load_session`. Kept version-named, per the handoff's
  explicit exception list: `SETTINGS_SCHEMA_VERSION_V2`,
  `SESSION_SCHEMA_VERSION_V2`, `.pre-v2.bak`, the v0 legacy DTOs.
- **`schema_version = 1` routes to `Corrupt`** — new
  `schema_version_1_is_corrupt_not_migrated` tests (settings and session),
  asserting the behavior directly rather than by absence.
- **`persist.rs` module doc rewritten** — no longer claims every persisted
  file wraps a `VersionedEnvelope`; now names `BatchManifest::to_json` as
  the counter-example and points at F31 for that gap.

## Went Further Than the Handoff's Explicit List, and Why

Two removals beyond the handoff's named table, both driven by its own
"delete what becomes unreachable" principle rather than by a fixed list:

1. **The entire `forskscope-core::session` module** (`session.rs` +
   `session/tab.rs`) — not just the seven named types. Grepping
   `forskscope_core::session::` across the whole workspace outside those
   two files returned zero hits; nothing re-exports it at the crate root.
   `FilePairRoot`, `DirectoryPairRoot`, `DiffTabSession`,
   `BinaryTabSession`, `ExcelTabSession`, `ErrorTabSession`, `Timestamp`,
   `SESSION_SCHEMA_VERSION` (the v1 constant), `RecentKind`,
   `ParsedSession`, `SessionParseError` all had no consumer beyond the
   module itself and its own tests. Confirmed this is unrelated to RFC-075's
   runtime identity (`CompareTabId`/`LoadGeneration` live in
   `forskscope-ui-logic`, a completely different type family) before
   deleting.
2. **`is_core_preset_name`** — its only caller was `migrate_from_v1`.
   Removing it closes **F25** as a side effect ("two divergent built-in
   compare-profile sets... yet which `is_core_preset_name` still
   consults" — the consultation is gone because the function is gone).
3. **`PersistenceLoad::MigratedVersion`** and its two `commit_migrated`
   match arms (settings/session `runtime.rs`) — the variant's entire
   documented purpose was "an existing core v1 envelope was migrated";
   with `migrate_from_v1` gone, nothing ever constructs it. It is a
   plain Rust enum variant (never itself serialized — `PersistenceLoad<T>`
   is a return value, not a persisted type), so removing it does not touch
   the wire format.
4. **`settings.rs` survives partially, not wholesale.** `ThemeId`,
   `LocaleId`, `Density`, `FontFamilySetting`, `DiffFontFamilySetting`
   (the `display` submodule) are live, non-test dependencies of
   `forskscope-ui`/`forskscope-ui-logic` and of
   `persist::schema::settings::PersistedSettings` itself — kept, along
   with `pub mod display;`. Only the `UserSettings` cluster below it is
   gone. `display.rs`'s `SETTINGS_SCHEMA_VERSION` (the v1 constant, a
   different item from `SETTINGS_SCHEMA_VERSION_V2`) had no consumer
   outside the now-deleted `UserSettings`/`settings_tests.rs` and is
   removed with it.

## Files Changed

Deleted: `crates/forskscope-core/src/session.rs`,
`crates/forskscope-core/src/session/tab.rs`,
`crates/forskscope-core/src/tests/persist_tests.rs`,
`crates/forskscope-core/src/tests/settings_tests.rs`,
`crates/forskscope-core/src/tests/session_tests.rs`, and the three v1
fixtures (`settings-v1-envelope.json`,
`session-v1-{filepair,dirpair}-envelope.json`).

Renamed (directory + file): `persist/v2{.rs,/}` → `persist/schema{.rs,/}`
(9 files, `git mv`, content otherwise unchanged by the move itself).

Modified: `lib.rs` (drop `pub mod session;`), `persist.rs` (rewritten to a
6-line module doc plus `pub mod schema;`), `settings.rs` (rewritten,
`display` only), `settings/display.rs` (drop `SETTINGS_SCHEMA_VERSION`),
`save.rs` (doc reference), `tests.rs` (drop `persist_tests`/
`settings_tests`/`session_tests` module declarations), the five
`persist_v2_*_tests.rs` files (mechanical rename + v1-test removal + the
two new `schema_version_1_is_corrupt_not_migrated` tests),
`persist/schema/{settings,session}.rs` (drop `migrate_from_v1`,
`is_core_preset_name`, the `1 =>` match arm → falls through to the
existing `_ => Corrupt` arm), `persist/schema.rs` (drop
`MigratedVersion`, rewritten module doc), `persist/schema/{settings,session}/
runtime.rs` (drop the `MigratedVersion` match arm), `persist/schema/
{settings,session}/repository.rs` (mechanical rename only),
`forskscope-ui-logic`'s two `persistence_recovery.rs` files and
`forskscope-ui`'s `state.rs`/`state/session.rs`/`state/session/tests.rs`/
`state/settings.rs`/`ui/view/settings.rs`/`ui/view/settings/tests.rs`
(mechanical rename only, verified by re-reading each diff — no logic
changed in any of them).

## Confirming the "Mechanical Rename Only" Claim for Untouched Test Categories

Per the handoff's requirement to confirm these moved by symbol rename
alone:

- **UI schema-v0 tests** — `legacy_v0_fixture_migrates_every_field_exactly`
  (both settings and session) and `unrecognized_legacy_shape_is_corrupt_not_defaults`:
  unchanged except `PersistedSettingsV2`/`load_settings_v2` → their
  unsuffixed names. Diffed against `15f3227` to confirm no assertion
  changed.
- **Golden-fixture and wire-format tests** — the two
  `current_v2_golden_fixture_parses_to_the_exact_expected_struct` tests and
  all ten `persist_v2_schema_enum_wire_format_tests.rs` assertions:
  same, symbol rename only. The fixture *files* (`settings-v2.json`,
  `session-v2.json`) are untouched — `git diff 15f3227 62c61f8 --
  '*.json'` shows only the three v1 fixture deletions, nothing to v0/v2
  fixtures.
- **Repository safe-write, backup non-overwrite, failure-window,
  runtime-resolution tests** (`persist_v2_repository_tests.rs`,
  `persist_v2_runtime_tests.rs` minus the one v1 test removed) — symbol
  rename only; every assertion is byte-identical modulo the type names.

## Test-Count Delta, Itemized

Baseline `15f3227`: **1065** (709+27+16+2+21+21+255+6+7+1). Current
`62c61f8`: **1007** (652+27+16+2+21+21+255+6+6+1). **Delta: −58.**

| Source | Delta | Reason |
|---|---:|---|
| `persist_tests.rs` | −19 | file deleted, subject (`VersionedEnvelope` etc.) gone |
| `settings_tests.rs` | −15 | file deleted, subject (`UserSettings`) gone |
| `session_tests.rs` | −21 | file deleted, subject (`WorkspaceSession`) gone |
| `persist_v2_settings_tests.rs` | 0 | −1 (`core_v1_envelope_fixture_migrates_every_represented_field`), +1 (`schema_version_1_is_corrupt_not_migrated`) |
| `persist_v2_session_tests.rs` | −1 | −2 (`core_v1_filepair_envelope_migrates_root_as_the_sole_tab`, `core_v1_dirpair_envelope_migrates_root_as_explorer_roots`), +1 (`schema_version_1_is_corrupt_not_migrated`) |
| `persist_v2_runtime_tests.rs` | −1 | −1 (`settings_resolve_migrates_core_v1_envelope_and_commits_durably`) |
| doctests | −1 | `persist.rs`'s module-doc example (`VersionedEnvelope::new`/`parse`) had no subject left to document |
| **Total** | **−58** | |

Every removed test's subject is confirmed gone in this same patch; none
was deleted because it failed or because I judged it redundant.

## Not Addressed Here (per the handoff's own scope)

- `app-json-settings` remains a declared, unused dependency —
  not removed, per the handoff's explicit prohibition.
- F31 (whether batch manifests/reports adopt a schema envelope) — M4's,
  untouched; `persist.rs`'s doc now names the gap instead of denying it.
- Recovery UI and documentation (patch 6, per review 042's renumbering) —
  not started.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1007 (652+27+16+2+21+21+255+6+6+1), exactly 1065 - 58
cargo clippy --workspace -D warnings                             pass
cargo clippy -p forskscope-core --tests -D warnings              8 pre-existing findings (F6) untouched; 0 new
cargo xtask audit-deps                                           pass
cargo xtask i18n                                                 pass — 203 keys
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
git diff 15f3227 62c61f8 -- '*.json'                             only the three v1 fixture deletions
```

CI run `30827087872`: Test & Lint green, on `17ae878`.

## Requested Review Focus

1. Confirm the three "went further than the explicit list" removals
   (`crate::session` module wholesale, `is_core_preset_name`,
   `PersistenceLoad::MigratedVersion`) are the right call under "delete
   what becomes unreachable" rather than something to have flagged and
   left for a separate decision.
2. F25's closure is a side effect of removing `is_core_preset_name`,
   not a deliberate fix — confirm that's an acceptable way for F25 to
   close, or whether it needs its own explicit resolution note beyond
   what F29's removal implies.
3. Given the history-collision incident (039), is there anything about
   how this content review should be sequenced differently, or is
   reviewing `62c61f8` directly (ignoring that it was assembled via
   reset+re-split rather than a single linear `git commit`) sufficient?
