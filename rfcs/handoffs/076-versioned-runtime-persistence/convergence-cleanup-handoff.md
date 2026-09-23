# RFC-076 Patch 5 Developer Handoff: Convergence Cleanup

**Governing RFC.** [RFC-076](../../proposed/076-versioned-runtime-persistence.md),
including its 2026-08-03 amendment
**Program.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md) — milestone M2-B
**Position.** Patch 5 of 6, after the production switch, **before** the recovery
UI and documentation
**Register items.** F29, F30 (F31 is M4's)

This handoff directs execution. It does not redefine RFC-076. If implementation
evidence contradicts a decision below, amend the RFC first, then update this
handoff.

## 1. Summary

Patches 1–4 built the canonical schema and switched production onto it. They
deliberately left the old models in place so each patch stayed reviewable. This
patch removes them.

Without it, the milestone ends with **three** settings shapes and **two** session
shapes where RFC-076's acceptance criterion asks for one canonical model per
document. Adding a single settings field would touch the canonical type, the UI
adapter, `from_v2`, `merge_into_v2`, the golden fixture, and the expected struct
— plus a judgement call about a legacy model nobody runs. That ratio is the
maintenance cost this patch exists to remove.

It runs before the recovery UI so that neither the UI nor the documentation is
written against names and models that are about to change.

No user-visible behaviour changes. No on-disk format changes.

## 2. Scope followed

In scope:

- remove the core-v1 migration path per the RFC's 2026-08-03 amendment;
- remove the legacy models it was the only consumer of;
- drop the version suffix from the canonical module and types;
- reconcile the module documentation left untrue by the removals.

Out of scope:

- **UI schema v0 support.** Mandatory, untouched, and its tests must not be
  weakened. This is the only legacy format users actually have.
- any change to the v2 on-disk format, field names, or enum wire strings;
- the recovery UI and documentation — patch 6;
- F31 (whether batch manifests and reports adopt a schema envelope) — M4;
- removing `app-json-settings` from `Cargo.toml` — still a dependency-policy
  decision, still not the implementer's;
- F25, F27, F28 unless they fall out naturally.

## 3. Design decisions and assumptions

### 3.1 Why v1 goes — and why v0 stays

RFC-076's amendment records this in full. In short: v1 was specified in RFC-031,
implemented in core, and never wired to the application. `git log -S` over the
entire history of `crates/forskscope-ui/` finds no reference to `UserSettings`
or `WorkspaceSession` at any point. No released version produced a v1 file, so
none can exist.

v0 is the opposite: it is what every real user has on disk right now. Its
migration path, DTOs, fixtures, and tests are untouchable.

If you find any evidence contradicting the v1 finding — a call site in history, a
tool that wrote one, anything — **stop and report before deleting**. The whole
decision rests on that one fact.

### 3.2 Removal set

Driven by need, not by list — delete what becomes unreachable and let the
compiler confirm:

| Item | Why removable |
|---|---|
| `migrate_from_v1` (settings, session) | only caller of the v1 readers |
| `UserSettings`, `AppearanceSettings`, `DiffSettings`, `FileSettings`, `LocaleSettings` | v1 model; no runtime consumer |
| `WorkspaceSession`, `WorkspaceTab`, `SessionId`, `TabId`, `WorkspaceRoot`, `RecentSessionEntry`, `CloseResult` | v1 model; no runtime consumer |
| `VersionedEnvelope`, `ParsedEnvelope`, `SchemaName`, `MigrationPolicy`, `EnvelopeError` | reachable in production only through the v1 read path |

Removing `session::TabId` is worth doing for its own sake: RFC-075's handoff and
RFC-076's both had to carry explicit prohibitions against confusing it with
`CompareTabId`. Those prohibitions exist because the type does.

Check each type for consumers before deleting. Anything with a live non-test
consumer stays, and you report it rather than working around it.

### 3.3 Routing after removal

`schema_version = 1` becomes `Corrupt`, not a migration candidate — a preserved,
reported file, never silently defaulted. Keep a test asserting exactly that, so
the removal is a deliberate documented behaviour rather than an absence.

### 3.4 The rename

Drop the version suffix from the canonical module and types, so a future schema
version renames nothing:

| From | To |
|---|---|
| `persist::v2` | `persist::schema` |
| `PersistedSettingsV2` | `PersistedSettings` |
| `PersistedSessionV2` | `PersistedSession` |
| `PersistedDiffProfileV2` | `PersistedDiffProfile` |
| `PersistedComparePairV2` | `PersistedComparePair` |
| `PersistedDirectoryPairV2` | `PersistedDirectoryPair` |
| `load_settings_v2` / `load_session_v2` | `load_settings` / `load_session` |

**Keep version-named**, because these are genuinely tied to a specific version
or a one-time event:

- `SETTINGS_SCHEMA_VERSION_V2` / `SESSION_SCHEMA_VERSION_V2` — or rename to
  `SCHEMA_VERSION`, but the number stays a single constant either way;
- the v0 legacy DTOs and their module;
- **`.pre-v2.bak`** — this filename is on users' disks and names a specific
  historical event, "the backup of what existed before the v2 migration."
  Do not generalise it. Renaming it would orphan every backup already written.

**The rename cannot touch the wire format.** serde serialises field names, not
type names; type names appear in zero fixtures. Verified before this handoff was
written. If any golden-fixture test changes behaviour during the rename,
something is wrong — stop and report.

### 3.5 What the module documentation must stop claiming

`persist.rs`'s module doc currently states that every file ForskScope writes
wraps its payload in a `VersionedEnvelope`. That was never true —
`dir::batch::BatchManifest::to_json` hand-rolls its own JSON — and after this
patch the type is gone entirely. Rewrite the docs to describe what exists.

Do **not** extend the envelope to batch manifests or reports here. Whether those
should be schema-versioned is an RFC-level question, registered as F31 against
M4.

## 4. Tests and gates run

No implementation commands have been run for this design handoff. Required
observed evidence:

```sh
cargo fmt --check
cargo test -p forskscope-core -p forskscope-ui-logic
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo clippy -p forskscope-core --tests -- -D warnings
cargo xtask audit-deps
cargo xtask version-sync
git diff --check
```

Test count will **drop**, and that is the expected outcome — v1 migration tests,
`settings_tests`, `session_tests`, and `persist_tests` cover models that no
longer exist. Report the delta with a per-suite breakdown and account for every
removed test: deleted-because-the-subject-is-gone is correct; deleted-because-it-
failed is not.

The tests that must **not** move:

- every UI schema-v0 migration test;
- the golden-fixture tests for both documents;
- the per-variant wire-format tests from F26;
- the repository safe-write, backup non-overwrite, and failure-window tests;
- the runtime resolution tests.

If any of those needs editing beyond a mechanical symbol rename, stop and report
— it means the cleanup changed behaviour, which it must not.

## 5. Generated artifacts

None. This patch cuts no release and produces no fixtures. Existing fixtures for
v0 and v2 are unchanged; the v1 envelope fixtures are removed with the path they
served.

## 6. Known limitations

- A hand-built v1 file becomes `Corrupt` rather than migrating. Bounded and
  intended: preserved, reported, never overwritten.
- `forskscope-core` carries no `publish = false`, so removing public types would
  be a breaking API change if it were ever published. It is not published today;
  the manifest gap is registered for M4.
- F31 remains open — after this patch nothing versions batch manifests or
  reports, and the aspiration to do so loses its only implementation.
- B2 closes at patch 4, not here. B3 and B4 remain open; v1/public release stays
  **No-Go**.

## 7. Acceptance criteria

- Exactly one canonical persisted model per document exists in
  `forskscope-core`, with no version suffix in its name.
- No production code path references `UserSettings`, `WorkspaceSession`,
  `VersionedEnvelope`, `SchemaName`, or `MigrationPolicy` — verified by grep, not
  by assertion.
- `schema_version = 1` routes to `Corrupt`, with a test.
- UI schema-v0 migration behaviour is bit-for-bit unchanged, evidenced by its
  tests passing unmodified.
- The golden-fixture and wire-format tests pass with only mechanical symbol
  renames.
- `persist` module documentation describes what exists.
- `.pre-v2.bak` is unchanged.
- All gates in §4 pass with recorded output, and the test-count delta is
  itemised.

## 8. Prohibited shortcuts

- Weakening or deleting any UI schema-v0 test.
- Changing any on-disk field name, enum wire string, or the backup filename.
- Extending the envelope to batch manifests or reports (F31, M4).
- Removing `app-json-settings` from `Cargo.toml`.
- Deleting a type that still has a live non-test consumer instead of reporting it.
- Editing a golden-fixture or wire-format test beyond a symbol rename.
- Bundling the recovery UI or documentation — that is patch 6.

## 9. Compatibility and security constraints

- Users' existing v0 files must load exactly as they do after patch 4.
- No change to the v2 format means no downgrade implication beyond what patch 4
  already established.
- MSRV 1.91, edition 2024 unchanged. No dependency added or removed.
- Removing code cannot introduce a network or filesystem surface, but
  `audit-deps` and `version-sync` still run so the claim is evidenced.

## 10. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (RFC-076 amendment, F29, F30);
3. changed and deleted files;
4. the removal set actually applied, and anything you declined to remove because
   it still had a consumer;
5. any difference from this handoff or from RFC-076;
6. executed gates with observed output, including the itemised test-count delta
   and the grep evidence for the removal;
7. confirmation that no v0 test and no golden-fixture or wire-format test changed
   beyond a symbol rename;
8. unresolved issues and known limitations;
9. requested review focus.
