# Review Request: F26 Closure and Review 039 N1

**Date:** 2026-08-03
**Reviewer stance:** verification of two independent, pre-patch-4 closures
**Repository baseline:** `014debf` (`persist: close F26 (schema-enum
wire-format coverage) and review-039 N1`)
**Governing documents:** review 036 (F26), review 039 (N1),
`ROADMAP.md` fix register

Review 038/039's recommended next actions were: close F26 before patch 4,
and fix N1 before patch 5 ("can ride with any pre-patch-4 work if
convenient"). Both land together here since F26 was already the scheduled
pre-patch-4 work.

## F26 — schema-enum wire-format coverage

`PersistedSettingsV2` reuses ten schema-invariant enums directly. Five
(`WhitespaceMode`, `NewlineCompareMode`, `CaseSensitivity`, `InlineMode`,
`DiffAlgorithm`) appear only on `profiles`, a list field — the golden
fixture's four profile entries already exercise every variant of those
five. The other five (`ThemeId`, `Density`, `FontFamilySetting`,
`DiffFontFamilySetting`, `NewlinePolicy`) appear only on scalar fields,
where a payload holds exactly one value per field, so the fixture could pin
only whichever variant happened to be on disk — twelve variants across
those five enums were provably unpinned (review 036's mutation-testing
finding: renaming `ThemeId::Dark` passed all 969 tests).

New `crates/forskscope-core/src/tests/persist_v2_schema_enum_wire_format_tests.rs`:
one test per enum, asserting every variant's literal kebab-case JSON string
via `serde_json::to_value`/`from_value`, e.g. `ThemeId::Dark` ↔ `"dark"`,
`DiffFontFamilySetting::CourierNew` ↔ `"courier-new"`. This is the fix
recommended in review 036/registered as F26 — "add per-variant
serialization assertions", not more fixtures — and is fixture-independent:
it fails on any variant rename regardless of what a fixture happens to
hold. Covers all ten enums (32 variants total), not just the five that were
unpinned, so the file is a complete reference rather than a partial patch.

No production code changed — this is test-only, as the finding itself
specified.

## Review 039 N1 — `Failed` dialog no longer claims settings were lost

`RecoveryDialogAction` gains `ContinueWithoutSaving`, distinct from
`ContinueWithTemporaryDefaults`. The `Migrated(Failed)` dialog now offers
`Exit` + `ContinueWithoutSaving` instead of reusing
`ContinueWithTemporaryDefaults` — the migrated value is correct and in use
for `Failed`, unlike `Incompatible`/`CorruptPreserved` where the resolved
value genuinely is defaults. `Incompatible`/`CorruptPreserved` are
unchanged. Both settings and session view-models updated identically;
tests updated to assert the new action and explicitly assert the absence
of the old (misleading) one.

## Files Changed

New:

- `crates/forskscope-core/src/tests/persist_v2_schema_enum_wire_format_tests.rs`
  — 10 tests, one per schema enum

Modified:

- `crates/forskscope-core/src/tests.rs` — declares the new module
- `crates/forskscope-ui-logic/src/settings/persistence_recovery.rs`,
  `.../session/persistence_recovery.rs` — `ContinueWithoutSaving` action,
  `Failed` dialog uses it, tests updated
- `ROADMAP.md` — F26 marked resolved (same convention as F17), milestone
  column updated from "before RFC-076 patch 4" to "RFC-076 pre-patch-4"
  since it's now done rather than scheduled

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test -p forskscope-core -p forskscope-ui-logic            pass — 1023 (was 1013; +10 F26 tests, ui-logic count unchanged — the N1 fix edited existing test assertions, added none)
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
cargo clippy -p forskscope-core --tests -D warnings              8 pre-existing findings (F6) untouched; 0 new
cargo clippy -p forskscope-ui-logic --tests -D warnings          1 pre-existing finding untouched; 0 new
cargo test --workspace                                           pass
cargo clippy --workspace -D warnings                              pass
cargo xtask version-sync                                          pass — v0.165.1
git diff --check                                                  pass
```

CI run `30775978671`: Test & Lint green, on `014debf`.

## Requested Review Focus

1. F26's fix is test-only, per the finding's own recommendation — confirm
   nothing about the enum reuse pattern itself (schema-invariant core enums
   serialized directly) needs to change, only the test gap.
2. Is `ContinueWithoutSaving` the right name/granularity, or would you
   prefer the action carry more context (e.g. distinguishing "changes lost"
   from "changes preserved but unsaved") given `RecoveryDialogAction` still
   has no implementation behind it and this is the cheapest point to adjust
   it?
