# RFC-076 Developer Handoff: Runtime Persistence Convergence

## 1. Summary

Move production settings/session persistence to core-owned schema v2,
including migration of current UI plain JSON and existing core-v1 envelopes,
visible rejection of future/corrupt files, and safe migration backup. RFC-076
owns the design.

## 2. Scope followed

In scope:

- canonical settings/session v2 payloads and serde-backed envelope handling;
- exact UI-v0 and core-v1 DTOs with sanitized fixtures;
- explicit-path repositories and safe migration writes;
- UI runtime adapters, recovery states, and write-disable protection;
- removal of duplicate production serialization ownership;
- tests of functions invoked by application startup/effects.

Out of scope:

- database storage, merge-buffer persistence, settings redesign, or remote
  synchronization;
- silent reset of future/corrupt files;
- broad save durability changes beyond migration needs.

## 3. Files changed

Expected areas:

- `crates/forskscope-core/Cargo.toml`
- `crates/forskscope-core/src/persist.rs` and focused submodules if split
- `crates/forskscope-core/src/settings.rs`
- `crates/forskscope-core/src/session.rs`
- persistence/settings/session tests and sanitized fixtures
- `crates/forskscope-ui/src/state/settings.rs`
- `crates/forskscope-ui/src/state/session.rs`
- `crates/forskscope-ui/src/ui/view/settings.rs`
- startup/modal/notice integration needed for recovery
- user and maintainer persistence documentation

Files above 300 ELOC should be split along schema/repository/migration
boundaries when touched; do not grow `persist.rs` into a monolith.

## 4. Design decisions and assumptions

- Core owns disk schema; UI owns platform path selection and presentation.
- Existing UI plain JSON is schema v0 and must round-trip every active field.
- Existing core schema v1 is an immutable migration input; it is never
  reinterpreted using the v2 payload shape.
- The v2 settings payload preserves the union of UI-v0 and core-v1 settings.
- UI-v0 `CourierNew`/`Consolas` diff-font choices remain exact; core appearance
  font and diff font are separate canonical fields.
- The v2 session payload preserves restorable compare/explorer paths but not
  identifiers, dirty summaries, or content that cannot be reconstructed.
- RFC-075 `CompareTabId` values are freshly allocated after restore and are
  never populated from legacy core `TabId` values.
- Unknown future schema is preserved with persistence writes disabled.
- Corruption is visible and preserved until explicit reset.
- Migration is reversible through a non-overwriting `.pre-v2.bak`.
- Tests use temporary explicit paths, never the developer's config directory.

Recommended patch sequence:

1. Serde envelope, v0/v1/v2 DTOs, fixtures, routing, and migration tests.
2. Repository and safe-write tests.
3. Runtime adapters/tests while old UI path remains active.
4. Switch settings, then session; remove duplicate serialization ownership.
5. Recovery UI and documentation.

Pause for design review after patch 1: verify the migration field matrix,
deliberate legacy metadata discards, and downgrade behavior before production
migration.

## 5. Tests and gates run

No implementation commands have been run for this design handoff. Required
observed evidence:

```sh
cargo fmt --check
cargo test -p forskscope-core persist
cargo test -p forskscope-core settings
cargo test -p forskscope-core session
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Add targeted tests proving the actual UI startup/save functions use the new
repositories. Run `cargo xtask audit-deps` after dependency changes.

## 6. Generated artifacts

- Sanitized legacy settings/session fixtures are expected and committed.
- No real user configuration, paths, or secrets may enter fixtures or logs.
- No release archive is expected until integrated Milestone M4.

## 7. Known limitations

- A schema-v2 envelope may not be readable by v0.164; the preserved legacy
  backup and downgrade documentation are mandatory.
- Cross-platform atomic replacement is accepted later under RFC-078.
- UI recovery flows may require GTK compilation but must retain pure repository
  tests for core correctness.

## 8. Recommended next step

Inventory the exact UI-v0 and core-v1 fields and produce the v0/v1/v2 fixture
and migration-matrix patch for owner review before modifying production
load/save calls.
