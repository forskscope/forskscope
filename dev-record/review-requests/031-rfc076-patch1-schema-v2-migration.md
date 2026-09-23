# Review Request: RFC-076 M2-B Patch 1 — Settings/Session Schema v2, Routing, Migration

**Date:** 2026-08-02
**Reviewer stance:** design review — this is the mandatory pause after patch 1
**Repository baseline:** `5054000` (`persist: add settings/session schema v2
routing and migration (RFC-076 patch 1)`)
**Governing documents:** RFC-076; `rfcs/handoffs/076-versioned-runtime-persistence/implementation-handoff.md`
§4.4 patch sequence, audit finding B2

This is **patch 1 of 5** in the handoff's sequence: "Serde envelope, v0/v1/v2
DTOs, fixtures, routing, and migration tests." Per §4.4, implementation stops
here for design review before any production load/save call is touched.
**No production code path in `forskscope-ui` is changed by this patch.**
`app_json_settings::ConfigManager` remains the only thing the running
application actually calls; nothing here is reachable yet.

## Implementation Summary

Added a new, pure (no file I/O) settings/session schema-v2 module to
`forskscope-core`: `crates/forskscope-core/src/persist/v2/` with `settings.rs`
(+ `settings/legacy.rs`) and `session.rs`. Each exposes one routing function —
`load_settings_v2(raw: &str) -> PersistenceLoad<PersistedSettingsV2>` and
`load_session_v2(raw: &str) -> PersistenceLoad<PersistedSessionV2>` — that
inspects a raw JSON string with `serde_json` and returns one of six outcomes
(`Missing` is unused by these pure functions; it's a repository-layer concern
for patch 2): `Current`, `MigratedLegacy`, `MigratedVersion`, `FutureVersion`,
or `Corrupt` with a specific `PersistenceError`.

`serde`/`serde_json` were added to `forskscope-core` (previously absent) and
several existing core enums gained `Serialize`/`Deserialize` so the v2
canonical payload can reuse them directly rather than duplicating them.

## Addressed Items

- RFC-076 §"Canonical schemas", §"Envelope contract", §"Load result
  taxonomy", §"Migration inputs and routing" — implemented for settings and
  session.
- Audit finding B2 — **not yet closed**. Patch 1 makes the migration logic
  exist and pass tests; it does not make the application use it. B2 closes
  only when patch 4 switches the production call sites.
- Handoff §4.1 (serde as a genuine new core dependency) — done, with
  `audit-deps`/`cargo audit` re-run and the threat-model dependency table
  updated.
- Handoff §4.3 preserved design decisions — see the field matrix below for how
  each was honored.

## Files Changed

New:

- `crates/forskscope-core/src/persist/v2.rs` — `PersistenceLoad<T>`,
  `PersistenceError`, module docs (39 ELOC)
- `crates/forskscope-core/src/persist/v2/settings.rs` — `PersistedSettingsV2`,
  `PersistedDiffProfileV2`, routing, migration, normalization (327 ELOC)
- `crates/forskscope-core/src/persist/v2/settings/legacy.rs` —
  `LegacyAppSettingsV0` and its nested types, an exact frozen mirror of the
  shipping UI's `AppSettings` (101 ELOC)
- `crates/forskscope-core/src/persist/v2/session.rs` — `PersistedSessionV2`,
  `PersistedComparePairV2`, `PersistedDirectoryPairV2`, `LegacySessionStateV0`,
  routing, migration (155 ELOC)
- `crates/forskscope-core/src/tests/persist_v2_settings_tests.rs` (12 tests)
- `crates/forskscope-core/src/tests/persist_v2_session_tests.rs` (11 tests)
- `crates/forskscope-core/src/tests/fixtures/persistence/*.json` (5 sanitized
  fixtures — see §"Fixtures" below)

Modified:

- `crates/forskscope-core/Cargo.toml` — added `serde`, `serde_json`
- `crates/forskscope-core/src/persist.rs` — added `pub mod v2;` and a doc note
  on why it doesn't reuse `VersionedEnvelope::parse`; the existing RFC-031
  envelope/schema/migration-policy types are otherwise untouched
- `crates/forskscope-core/src/settings.rs` — `UserSettings::from_payload_json`
  changed from private to `pub(crate)`, logic unchanged, to let v2 migration
  reuse the frozen v1 parser instead of re-deriving the v1 shape
- `crates/forskscope-core/src/session.rs` — same change to
  `WorkspaceSession::from_payload_json`
- `crates/forskscope-core/src/settings/display.rs` — added
  `Serialize`/`Deserialize` to `ThemeId`, `Density`, `FontFamilySetting`,
  `LocaleId`; added a new `DiffFontFamilySetting` (5-variant, core-owned)
- `crates/forskscope-core/src/diff/options.rs` — added
  `Serialize`/`Deserialize` to `DiffAlgorithm`, `InlineMode`, `WhitespaceMode`,
  `NewlineCompareMode`, `CaseSensitivity`
- `crates/forskscope-core/src/encoding.rs` — added `Serialize`/`Deserialize`
  to `NewlinePolicy`
- `crates/forskscope-core/src/job/limits.rs` — added
  `Serialize`/`Deserialize` to `PerformanceLimits`
- `crates/forskscope-core/src/tests.rs` — declared the two new test modules
- `docs/src/maintainers/threat-model.md` — dependency table: added
  serde/serde_json; corrected `app_json_settings` from the stale `2.0.3` to
  the actual locked `2.4.1` (noticed in passing; not otherwise in this
  patch's scope, but wrong immediately next to what I was editing)

Not staged: `xtask/Cargo.lock` (pre-existing untracked convention, unrelated).

## Important Implementation Decisions

### Reused existing core types as the v2 canonical form, instead of duplicating them

`PersistedSettingsV2` uses `ThemeId`, `Density`, `FontFamilySetting`,
`LocaleId`, `NewlinePolicy`, `WhitespaceMode`, `NewlineCompareMode`,
`CaseSensitivity`, `InlineMode`, `DiffAlgorithm`, and `PerformanceLimits`
directly — the existing core types, now `Serialize`/`Deserialize`-derived —
rather than inventing parallel `*V2` duplicates with manual conversions. This
directly matches RFC-076's "one public canonical domain type plus private
version-specific payload DTOs": the canonical type *is* the existing core
type; only the v0 legacy shape needs its own frozen DTO, because it has a
genuinely different field layout the production UI already committed to.

The one genuinely new type is `DiffFontFamilySetting` (`SystemMono`,
`SystemSans`, `SystemSerif`, `CourierNew`, `Consolas`), added to
`settings/display.rs` alongside the existing 3-variant `FontFamilySetting`,
because core had no 5-variant equivalent to the shipping UI's diff-pane font
setting. Naming follows RFC-076's own illustrative example.

### Reused the frozen v1 hand-written parsers instead of re-deriving the v1 shape with serde

`UserSettings::from_payload_json` and `WorkspaceSession::from_payload_json`
were exposed as `pub(crate)` and called directly for v1 migration input,
rather than writing fresh `serde`-derived v1 DTOs. The v1 JSON shape has
transforms a straightforward derive would not reproduce — most notably,
`compare_profile` is serialized as **only its name**, and reconstructed by
looking the name up in `CompareProfile::all_presets()` on read. Re-deriving
that with serde risked silently diverging from what the (untested-in-
production, but still an immutable migration input per RFC-076) v1 format
actually is. This also means v1's own logic is genuinely untouched — only
its visibility changed.

### Built-in compare profiles: UI's four, not core's four

The shipping UI's `default_profiles()` (`Exact (default)`, `Ignore
whitespace`, `Ignore case`, `Histogram`) and core's
`CompareProfile::all_presets()` (`Default`, `Code Review`, `Loose Text`,
`Large File Safe`) are two **different** sets of four "built-in" profiles —
a pre-existing inconsistency in the codebase, not something this patch
introduces. I chose the UI's set as v2's canonical built-ins, because those
are the profiles a real user has actually seen; core's set has never been
exposed through any UI control. Concretely:

- v0 → v2: the UI's profile list (already seeded with its 4 built-ins by
  `default_profiles()`, plus any user-added custom ones) carries over with
  its `built_in` flags and `active_profile` index preserved exactly, mapped
  into the richer v2 field shape (`ignore_whitespace: bool` →
  `WhitespaceMode::IgnoreAll | Significant`, `ignore_case: bool` →
  `CaseSensitivity`, etc.).
- v1 → v2: the one selected `CompareProfile` becomes `profiles[0]`
  (`active_profile = 0`), tagged `built_in` if its name matches one of
  core's four preset names; the four UI built-ins are then appended, skipping
  any whose name already matches — "recreate canonical built-ins, then append
  valid custom profiles without duplicate names," per RFC-076.

### v1 session migration is bounded by what v1 itself already restores

`WorkspaceSession::from_payload_json` has always returned `tabs: vec![]` —
its own existing doc comment says "tab list restoration is deferred to v2."
So v1 migration can only recover `root`: a `FilePair` root becomes the sole
v2 compare tab, a `DirectoryPair` root becomes `explorer_roots`, `Empty`
recovers nothing. `active_tab` is `Some(0)` when a tab was recovered, else
`None` — there is no way to reconstruct a meaningful index from a list that
was never populated. This is a deliberate, bounded discard consistent with
RFC-076 ("v1 IDs, timestamps... are not carried forward because they are not
sufficient to restore unsaved content safely"), not a regression — v1 files
were never written by the shipping UI in the first place.

### `explorer_roots` (session) is distinct from `last_left_dir`/`last_right_dir` (settings)

These are two different concepts that happen to sound similar.
`last_left_dir`/`last_right_dir` (in `AppSettings`, gated by
`remember_explorer_dirs`) is "the last individually browsed directory in
each Explorer pane" — browsing-position memory. `explorer_roots` (in
`PersistedSessionV2`) is core-v1's `WorkspaceRoot::DirectoryPair` — a
directory-pair **comparison root**, conceptually a persisted directory-diff
tab. RFC-076's illustrative schema put `explorer_roots` under session, which
is where I kept it; `last_left_dir`/`last_right_dir` stayed under settings,
matching where they actually live today. Neither is invented — I traced
both to their real current owners before deciding.

### Migration field-precedence table (as actually implemented)

| v2 field group | UI-v0 source | Core-v1 source |
|---|---|---|
| theme | `theme` (kebab-case, 1:1) | `appearance.theme` |
| language | `language` (`en`/`ja`) | `locale.locale` |
| diff font size/family (UI-owned) | `diff_font_size`, `diff_font_family` (5-way, exact) | not represented; UI default (14, `SystemMono`) |
| appearance font size/family, density (core-owned) | not represented; core default (14, `SystemMono`, `Comfortable`) | `appearance.font_size`, `appearance.font_family`, `appearance.density` |
| context_lines | `context_lines` | not represented; UI default (3) |
| last_left_dir / last_right_dir | exact | not represented; `None` |
| profiles / active_profile | every UI profile + index, mapped to richer v2 shape | one selected core profile + canonical UI built-ins appended |
| ignore_extensions / ignore_dirs / explorer_compact / enable_binary_comparison / remember_explorer_dirs | exact | not represented; UI defaults |
| show_line_numbers / wrap_long_lines / newline_policy | not represented; core defaults (`true`/`false`/`Preserve`) | `diff.show_line_numbers`, `diff.wrap_long_lines`, `files.newline_policy` |
| restore_session / recent_limit / performance | not represented; core defaults | `files.restore_session`, `files.recent_limit`, `files.performance` |

Every "not represented" cell is a deliberate default fill, not a discard of
real data — the source format genuinely has no such field.

## Fixtures

Five files under `crates/forskscope-core/src/tests/fixtures/persistence/`,
`include_str!`-embedded (compile-time, immune to the stale-test-binary issue
that affected the diff/merge corpora):

- `settings-v0.json`, `session-v0.json` — hand-authored, matching the real
  production serde shapes exactly (verified by the tests deserializing and
  asserting on every field).
- `settings-v1-envelope.json`, `session-v1-filepair-envelope.json`,
  `session-v1-dirpair-envelope.json` — **not** hand-authored. Generated by a
  temporary `#[test]` that called the real `UserSettings::to_json()` /
  `WorkspaceSession::to_json()` against constructed non-default values,
  printed the output, and was then deleted (`git diff` on
  `persist_tests.rs` is empty — confirmed clean before proceeding). This
  guarantees byte-for-byte fidelity to what the real v1 writer actually
  produces, rather than risking a hand-typed fixture drifting from it.

All paths inside fixtures are synthetic (`/tmp/fixtures/...`); no real user
data.

## Differences From the Handoff

None identified in scope or design. Two implementer choices where the
handoff specified the requirement but not the mechanism (both explained
above): which built-in profile set is canonical for v2, and the exact
field-precedence mapping for fields with no cross-source equivalent.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test -p forskscope-core persist                          pass — 42/42 (incl. new persist_v2_* tests)
cargo test -p forskscope-core settings                         pass — 28/28
cargo test -p forskscope-core session                           pass — 46/46
cargo test -p forskscope-core -p forskscope-ui-logic            pass — 966/966 (was 943; +23 new)
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
cargo test --workspace                                          pass — adds forskscope-ui 14+14 + 1 doctest ignored, unchanged
cargo clippy --workspace -D warnings                             pass
cargo xtask audit-deps                                          pass (unchanged: sheets-diff/calamine absent, dioxus-devtools
                                                                       inactive, reqwest/hyper/ureq absent, quick-xml limited
                                                                       to wayland-scanner, network paths reviewed)
cargo audit                                                      pass — 14 allowed warnings (same set as before the
                                                                       serde/serde_json addition; no new advisories)
cargo xtask version-sync                                        pass — v0.165.1
git diff --check                                                pass (no CRLF files touched this patch)
```

23 new tests: 12 in `persist_v2_settings_tests.rs`, 11 in
`persist_v2_session_tests.rs`. All passed on first run against the
implementation (no test was adjusted to match implementation behavior after
the fact).

## Known Limitations / Deliberately Out of Scope for Patch 1

- **No file I/O, no repository types.** `SettingsRepository`/
  `SessionRepository` and their explicit-path, safe-write behavior are
  patch 2.
- **`app_json_settings` usage is unchanged.** Per the handoff's required
  review-request content: it has **not** become unused — all three
  production `ConfigManager` call sites are untouched. No removal
  recommendation applies yet; that question is live only after patch 4.
- **`crates/forskscope-core/src/session.rs` remains at 322 ELOC**, already
  over the 300 soft threshold before this patch (per the handoff's own
  files-changed table). This patch's only edit there is the
  `from_payload_json` visibility change plus a doc comment — I did not split
  it, since splitting a file I'm not otherwise substantively modifying would
  be a separate, larger change of its own, and the handoff's instruction to
  split "as you touch it" reads to me as applying to substantive edits, not
  a one-line visibility change. Flagging for the reviewer's judgment rather
  than deciding unilaterally.
- **`crates/forskscope-core/src/persist/v2/settings.rs` is 327 ELOC**, over
  the 300 soft threshold (legacy DTOs were already split out to keep it
  under 500; further splitting routing from migration was considered and
  rejected as adding indirection for modest gain at this size — flagging for
  review rather than deciding it's fine unilaterally).
- **Threat model's settings-persistence "known gap" section is intentionally
  unchanged.** Updating it to describe "implemented behaviour" is correct
  only once production actually uses this path (patch 4); doing so now would
  overclaim. I did correct one adjacent, unrelated staleness (`app_json_settings`
  version in the dependency table) noticed while editing that section.
- B2 remains open. B3, B4 remain open. v1/public release stays **No-Go**.

## Requested Review Focus

1. Is reusing existing core types (rather than defining parallel `*V2`
   types) the right call for the canonical v2 shape, or does that create a
   coupling risk (a future unrelated change to, say, `WhitespaceMode` now
   also changes the settings disk schema)?
2. Is the UI's built-in-profile set the right canonical choice for v2,
   given core's `CompareProfile::all_presets()` is a different, richer, but
   currently UI-unreached set? This is the single most consequential
   modeling decision in this patch.
3. Does the v1 session migration's bounded recovery (root only, never the
   tabs list, since v1's own parser never populated it) match RFC-076's
   intent, or should this patch synthesize something richer from `root`
   alone?
4. Is exposing the v1 parsers as `pub(crate)` (rather than re-deriving them
   with serde) the right trust boundary, given it means v2 migration
   depends on v1 code that RFC-076 calls "immutable" but that is still,
   mechanically, living code in the same crate?
5. Any concern with the two ELOC-soft-threshold files flagged above?

Per the handoff, **I am stopping here** and not proceeding to patch 2
(repository, safe-write tests) until this review lands.
