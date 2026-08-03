# RFC-076 Developer Handoff: Runtime Persistence Convergence

**Governing RFC.** [RFC-076](../../proposed/076-versioned-runtime-persistence.md)
**Program.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M2-B, following the completed M2-A release-mechanics slice
**Audit finding.** B2

This handoff directs execution. It does not redefine RFC-076. If implementation
evidence contradicts a decision below, amend the RFC first, then update this
handoff to match.

*Refreshed 2026-08-02.* The original was written before R0 and M2-A. Its design
substance was sound and is preserved; its surrounding assumptions about
releases, dependencies, and gates were stale and have been corrected.

## 1. Summary

Move production settings/session persistence to core-owned schema v2, including
migration of the current UI plain JSON and existing core-v1 envelopes, visible
rejection of future and corrupt files, and safe migration backup.

The defect is concrete and small in surface. The running application never
touches the core's tested versioned models. It serializes its own structs
through `app_json_settings::ConfigManager` at three call sites, and
`crates/forskscope-ui/src/ui/view/settings.rs:28` reads them as:

```rust
m.load_or_default().unwrap_or_default()
```

A corrupt file and a file written by a future version are both indistinguishable
from a missing one: each silently becomes defaults, discarding the user's
configuration without a word. That is audit finding B2, and it is the gap
between what `docs/src/maintainers/threat-model.md` now admits and what the
product should do.

This closes B2. B3 and B4 remain open; v1/public release stays **No-Go**.

## 2. Scope followed

In scope:

- canonical settings/session v2 payloads and serde-backed envelope handling;
- exact UI-v0 and core-v1 DTOs with sanitized fixtures;
- explicit-path repositories and safe migration writes;
- UI runtime adapters, recovery states, and write-disable protection;
- removal of duplicate production serialization ownership;
- tests of the functions actually invoked by application startup and effects.

Out of scope:

- database storage, merge-buffer persistence, settings redesign, or remote
  synchronization;
- silent reset of future or corrupt files;
- broad save-durability changes beyond what migration needs;
- RFC-077 mergetool work, which follows this milestone;
- F18, F23, and F24, which are registered against M4 — with the one timing
  caveat in §5.

## 3. Files changed

Expected areas:

| Path | Current ELOC | Note |
|---|---:|---|
| `crates/forskscope-core/Cargo.toml` | — | serde/serde_json addition, see §4.1 |
| `crates/forskscope-core/src/persist.rs` | 234 | hand-written envelope parser is replaced |
| `crates/forskscope-core/src/settings.rs` | 277 | v1 `UserSettings` becomes a migration input |
| `crates/forskscope-core/src/session.rs` | 322 | **already over the 300 soft threshold** |
| `crates/forskscope-ui/src/state/settings.rs` | 175 | duplicate `AppSettings` |
| `crates/forskscope-ui/src/state/session.rs` | 47 | `ConfigManager` call site |
| `crates/forskscope-ui/src/ui/view/settings.rs` | 64 | `ConfigManager` call sites |

Plus persistence/settings/session tests, sanitized fixtures, startup and
modal/notice integration for recovery, and user/maintainer persistence
documentation.

`session.rs` is already above the 300-ELOC soft threshold before this work adds
to it. Split it along schema, repository, and migration boundaries as you touch
it rather than at the end. Do not grow `persist.rs` into a monolith.

## 4. Design decisions and assumptions

RFC-076 carries the design — payload shapes, the load-result taxonomy, the
migration routing table, and field precedence. Do not re-derive them here. The
decisions below are the ones this handoff pins.

### 4.1 `serde` is a genuine new dependency for core

`forskscope-core` currently depends on `encoding_rs`, `chardetng`, `chrono`, and
`similar`, with `tempfile` as a dev-dependency. It has **no serde**. RFC-076
requires adding `serde` and `serde_json` as normal dependencies.

That is a dependency-policy change, not an implementation detail:

- run `cargo xtask audit-deps` and `cargo audit` after the change, and record
  both;
- add the two crates to the dependency table in
  `docs/src/maintainers/threat-model.md` with their role and risk note;
- they are local serialization only and introduce no network data flow — state
  that explicitly rather than leaving it inferred.

### 4.2 The `ConfigManager` removal target is exact

Production use of `app_json_settings` is precisely three `ConfigManager`
constructions across two files, plus their imports and one helper signature:

| Location | What |
|---|---|
| `state/session.rs:5` · `ui/view/settings.rs:9` | `use app_json_settings::ConfigManager` |
| `state/session.rs:17` | `fn session_manager() -> ConfigManager<SessionState>` |
| `state/session.rs:18` | construction — `session.json` |
| `ui/view/settings.rs:23` | construction — `persist()` |
| `ui/view/settings.rs:28` | construction — `load()`, the silent-reset site |

When they are gone, `app-json-settings` may have no remaining consumer. **Do not
remove the dependency as part of this slice.** Report whether it has become
unused and propose removal separately. Dropping a dependency changes the
policy that `audit-deps` enforces and the threat model documents, and it is not
the implementer's call.

Worth knowing why this matters beyond tidiness: `app-json-settings` is the crate
whose Windows build defect blocked the `0.165.0` release (F17). If convergence
makes it removable, that is a real supply-chain reduction and the owner will
want to decide on it deliberately.

### 4.3 Preserved design decisions

- Core owns the disk schema; the UI owns platform path selection and
  presentation.
- Existing UI plain JSON is schema v0 and must round-trip every active field.
- Existing core schema v1 is an immutable migration input, never reinterpreted
  using the v2 payload shape.
- The v2 settings payload preserves the union of UI-v0 and core-v1 settings.
- UI-v0 `CourierNew` and `Consolas` diff-font choices stay exact; core
  appearance font and diff font remain separate canonical fields.
- The v2 session payload preserves restorable compare/explorer paths, but not
  identifiers, dirty summaries, or content that cannot be safely reconstructed.
- RFC-075 `CompareTabId` values are freshly allocated after restore and are
  never populated from legacy core `TabId` values. This one is load-bearing:
  installing a persisted identifier as a runtime concurrency token would let a
  restored value validate a task created in another process lifetime.
- Unknown future schema is preserved, with persistence writes disabled.
- Corruption is visible and preserved until an explicit, confirmed reset.
- Migration is reversible through a non-overwriting `.pre-v2.bak`.
- Tests use temporary explicit paths, never the developer's config directory.

### 4.4 Patch sequence, and the mandatory pause

1. Serde envelope, v0/v1/v2 DTOs, fixtures, routing, and migration tests.
2. Repository and safe-write tests.
3. Runtime adapters and their tests, while the old UI path remains active.
4. Switch settings, then session; remove duplicate serialization ownership.
5. **Convergence cleanup** — see
   `convergence-cleanup-handoff.md`. Added 2026-08-03; it precedes the recovery
   UI so neither the UI nor the documentation is written against type names and
   models that are about to be removed or renamed.
6. Recovery UI and documentation.

**Stop after patch 1 and request design review.** Verify the migration field
matrix, every deliberate legacy-metadata discard, and downgrade behaviour before
any production load or save call is modified. This pause is the main control on
a change that rewrites how user configuration is read; do not carry patch 1 and
patch 4 into one review.

## 5. Release context

M2-B's completion triggers a release. This corrects the original handoff, which
predated the current release cycle and said no archive was expected until M4.

- **Version level is decided at release time, from content.** The tree sits at
  `0.165.1`, the post-release patch default; it asserts nothing. A persistence
  schema change with migration is a significant internal change, so promotion to
  `0.166.0` is expected — but the owner confirms it with the content visible.
  See `docs/src/maintainers/release.md`.
- **Write the CHANGELOG entry as work lands**, not at the end. Release notes are
  now composed in CI from the `## [X.Y.Z]` section, and the job fails closed on
  an empty one. A section reconstructed at tag time is how the 26-commit backlog
  happened.
- **F23 must settle before this milestone's cut.** No workflow file is parsed or
  linted today, and `release.yml` was edited in M2-A without any parser
  validating it. The first thing that exercises it is this cut, where a syntax
  error means no release at all. F24 is optional and cheap.
- **Release-bearing rules from R0 apply.** A red platform job is a
  stop-and-report condition, not an occasion to complete the release another
  way. CI builds the artifacts and creates the draft; CI composes the notes; a
  human publishes. Never create or publish a release by hand.

## 6. Tests and gates run

No implementation commands have been run for this design handoff. Required
observed evidence:

```sh
cargo fmt --check
cargo test -p forskscope-core persist
cargo test -p forskscope-core settings
cargo test -p forskscope-core session
cargo test -p forskscope-core -p forskscope-ui-logic
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo xtask audit-deps
cargo audit
cargo xtask version-sync
git diff --check
```

The current headless baseline is **943 tests** across 8 suites. Record the new
count rather than asserting it grew.

Add targeted tests proving the actual UI startup and save functions use the new
repositories — not only that the lower-level core parsers work. That distinction
is the whole of B2: the core models were always tested; they were never the ones
running.

Expect one pre-existing `git diff --check` hit on
`packaging/windows/AppxManifest.xml` if that file is touched; it uses CRLF and
git flags the `\r` inside a diff hunk. Not a defect.

## 7. Generated artifacts

- Sanitized legacy settings and session fixtures are expected and committed.
- Fixtures must contain only temporary test paths. No real user configuration,
  home paths, host names, or secrets in fixtures, logs, or evidence.
- The release archive is produced by CI at this milestone's cut, not by hand.

## 8. Known limitations

- A schema-v2 envelope may not be readable by `0.165.0`, the last published
  version. The preserved `.pre-v2.bak` and downgrade documentation are
  mandatory, not optional.
- Cross-platform atomic replacement behaviour is accepted later, under RFC-078.
- UI recovery flows require GTK to compile, but core correctness must remain
  provable through pure repository tests that need no display server.
- This milestone closes B2 only. B3 and B4 remain open.

## 9. Acceptance criteria

- The running application reads and writes versioned envelopes; no production
  path calls `ConfigManager<AppSettings>` or `ConfigManager<SessionState>`.
- Every currently persisted UI setting survives a legacy-migration test.
- Existing core-v1 settings and session envelopes migrate without schema
  reinterpretation, preserving every restorable field.
- Future and corrupt schemas are preserved, visibly reported, and never
  overwritten; temporary-default mode carries the write-disable flag.
- Runtime-path tests exercise the same functions invoked at startup and in
  effects.
- Core and UI no longer contain competing persisted settings/session models.
- Migration backup and atomicity are tested on Linux; other platforms are
  accepted under RFC-078.
- RFC-011 and core documentation distinguish legacy persisted IDs from RFC-075's
  runtime-only identity.
- The threat model's settings-persistence section is updated from "known gap" to
  the implemented behaviour, and the dependency table lists serde.
- All gates in §6 pass with recorded output.

## 10. Prohibited shortcuts

- Treating any parse failure as defaults. That is the defect.
- Reinterpreting a v1 envelope with the v2 payload shape.
- Installing legacy persisted `TabId` values as runtime `CompareTabId` values.
- Removing `app-json-settings` in this slice, even if it becomes unused.
- Carrying patch 1 and patch 4 into a single review, or skipping the pause.
- Writing a migration path that overwrites the original before the replacement
  is durable.
- Putting real user paths into fixtures.
- Reconstructing the CHANGELOG entry at tag time.
- Reporting gate results that were not observed in this workstream.

## 11. Compatibility and security constraints

Compatibility:

- `0.165.0` is published and immutable.
- Existing `0.164.0`/`0.165.0` plain settings and session files must import as
  UI schema v0 without user intervention.
- Existing core-v1 envelope fixtures remain supported migration inputs.
- New files use schema v2; version 1 is never reinterpreted.
- MSRV 1.91 and edition 2024 are unchanged.

Security:

- serde and serde_json are local serialization only; no network surface is
  introduced, and `audit-deps` must confirm it.
- The schemas store local paths and preferences — never file content, tokens, or
  credentials.
- Migration must not print full paths into general logs.
- Corruption and future-version handling must not become a data-loss path: the
  original bytes survive every failure mode.

## 12. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (RFC-076 acceptance criteria, audit finding B2);
3. changed files;
4. important implementation decisions, especially the migration field matrix and
   every deliberate legacy discard;
5. any difference from this handoff or from RFC-076;
6. executed gates with observed output, including the new test count and the
   post-dependency-change `audit-deps` and `cargo audit` runs;
7. whether `app-json-settings` has become unused, with a recommendation but not
   a removal;
8. unresolved issues and known limitations;
9. requested review focus.

Patch 1 submits under the same format and stops for the design review in §4.4.
