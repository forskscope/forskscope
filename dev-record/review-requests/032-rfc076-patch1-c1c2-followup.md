# Review Request: RFC-076 Patch 1 — C1/C2 Follow-up

**Date:** 2026-08-02
**Reviewer stance:** targeted correction review
**Repository baseline:** `f899193` (`persist: clamp diff_font_size and pin
the v2 wire format (review 035 C1/C2)`)
**Responds to:** `dev-record/reviews/035-rfc076-patch1-schema-v2-review.md`,
mandatory corrections C1 and C2

## Summary

Applies both mandatory corrections from review 035, plus N1, N2, and the
two documentation asks (§4.1 schema-invariant comments, §4.4 frozen-parser
note). No product/production behaviour changes — still patch-1 scope, no
`forskscope-ui` call site touched.

## Addressed Items

- **C1** — `diff_font_size` now clamped to 6..=50 in `normalize()`, matching
  `appearance_font_size` and `forskscope-ui-logic::validate_font_size`. New
  test `out_of_range_diff_font_size_clamps` (both directions: too large,
  too small).
- **C2** — `settings-v2.json` / `session-v2.json` golden fixtures added:
  literal JSON, not derived from the current struct definitions, covering
  every enum variant reachable from each payload. New tests
  `current_v2_golden_fixture_parses_to_the_exact_expected_struct` (both
  files) assert the exact expected struct, so a variant rename anywhere in
  `diff/options.rs`, `encoding.rs`, `settings/display.rs`, or
  `job/limits.rs` now fails a test instead of silently corrupting every
  existing user's v2 file.
- **N1** — `default_diff_font_size()` split from
  `default_appearance_font_size()`.
- **N2** — `persist/v2`'s module doc now states why `theme`, `language`,
  `diff_font_size`, `context_lines`, `profiles`, `active_profile`
  (settings) and `tabs` (session) intentionally have no `#[serde(default)]`.
- **§4.1** — every core enum given `Serialize`/`Deserialize` in patch 1
  (`ThemeId`, `Density`, `FontFamilySetting`, `DiffFontFamilySetting`,
  `LocaleId`, `NewlinePolicy`, `WhitespaceMode`, `NewlineCompareMode`,
  `CaseSensitivity`, `InlineMode`, `DiffAlgorithm`, `PerformanceLimits`) now
  documents that its variant/field names are part of the settings v2
  on-disk schema.
- **§4.4** — `UserSettings::from_payload_json` and
  `WorkspaceSession::from_payload_json` now state they are frozen migration
  inputs, not parsers to evolve.
- **F25** — registered against M4 by the architect; not this commit's
  concern, not touched.

## Files Changed

- `crates/forskscope-core/src/persist/v2/settings.rs` — clamp,
  `default_diff_font_size`
- `crates/forskscope-core/src/persist/v2.rs` — module-doc strictness note
- `crates/forskscope-core/src/settings/display.rs`,
  `crates/forskscope-core/src/diff/options.rs`,
  `crates/forskscope-core/src/encoding.rs`,
  `crates/forskscope-core/src/job/limits.rs` — schema-invariant doc comments
  only, no behaviour change
- `crates/forskscope-core/src/settings.rs`,
  `crates/forskscope-core/src/session.rs` — frozen-input doc comments only
- `crates/forskscope-core/src/tests/persist_v2_settings_tests.rs`,
  `crates/forskscope-core/src/tests/persist_v2_session_tests.rs` — new tests
- `crates/forskscope-core/src/tests/fixtures/persistence/settings-v2.json`,
  `.../session-v2.json` (new)
- `ROADMAP.md` — F25 registration, pre-staged by the architect, carried in
  this commit

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test -p forskscope-core persist_v2                       pass — 26/26 (was 23; +1 C1 test, +2 C2 golden tests)
cargo test -p forskscope-core -p forskscope-ui-logic            pass — 969/969 (was 966; +3)
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
cargo test --workspace                                          pass — unchanged forskscope-ui counts
cargo clippy --workspace -D warnings                             pass
cargo xtask version-sync                                        pass — v0.165.1
git diff --check                                                pass
```

CI run `30749636074`: Test & Lint green, all 17 steps pass.

The golden-fixture tests were verified to actually exercise every variant
by construction (each fixture's literal strings were written independently
of the struct definitions, using the documented `#[serde(rename_all =
"kebab-case")]` convention, then confirmed correct by running the test — I
did not derive the fixture from a serialized struct).

## Known Limitations

Unchanged from the parent request
(`031-rfc076-patch1-schema-v2-migration.md`): no file I/O, no repository
types, `app_json_settings` usage unchanged, B2 remains open until patch 4.
`persist/v2/settings.rs` is now 333 ELOC (was 327; still under the 500 hard
threshold, already accepted at this size by review 035 §4.5).

## Requested Review Focus

Whether C1 and C2 are sufficiently closed to begin patch 2 (repositories
and safe-write tests), per review 035's recommended next action.
