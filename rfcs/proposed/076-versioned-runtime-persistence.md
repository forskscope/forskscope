# RFC 076: Versioned Runtime Settings and Session Persistence

**Status.** Proposed
**Tracks.** Release-stabilization audit finding B2.
**Touches.** Core persistence/settings/session models, UI settings/session
adapters, config-file migration, user-visible recovery, and runtime-path tests.

## Summary

ForskScope will have one canonical persisted settings contract and one
canonical persisted session contract, both owned by `forskscope-core` and
wrapped in `VersionedEnvelope`. The UI may keep transient view state, but it
will not define an independent disk schema.

Existing production plain-JSON files will be imported as legacy schema v0.
Unknown future envelope versions will be rejected visibly and preserved
unchanged. Migration writes will use backup plus atomic replacement.

## Problem

The core contains tested versioned models, while the running UI serializes
duplicate `AppSettings` and `SessionState` structs directly through
`app_json_settings`. Consequently:

- core schema/migration tests do not exercise production persistence;
- future schemas and corrupt files can silently reset to defaults;
- UI fields and core fields can drift;
- the documented compatibility contract is not true for actual users.

## Goals

- Make core the canonical owner of settings/session disk schemas.
- Preserve every currently persisted UI setting during migration.
- Import the current plain JSON format without user intervention.
- Reject unknown future schemas without overwriting them.
- Distinguish first run, legacy migration, corruption, and future-version cases.
- Test the exact repository/load/save path used by the application.
- Keep platform config-directory selection in the UI/infrastructure layer.

## Non-goals

- Persist open document contents, merge buffers, undo history, or credentials.
- Introduce a database or background synchronization.
- Store platform config paths inside core.
- Redesign the Settings UI except for required recovery messages.
- Recover arbitrary malformed JSON. Corruption is reported and preserved.

## Ownership boundary

`forskscope-core` owns:

- schema names and versions;
- canonical serializable payloads;
- validation and default values;
- v0-to-current migration;
- future-schema rejection;
- load/save result taxonomy.

The UI/infrastructure layer owns:

- locating `settings.json` and `session.json`;
- presenting recovery choices;
- mapping canonical settings to Dioxus display choices when necessary;
- triggering persistence after deliberate changes.

The file repository accepts explicit paths, which keeps core tests isolated
from the host's real config directory.

## Canonical schemas

Use serde-backed payloads and envelope parsing. Add `serde` and `serde_json` to
`forskscope-core`; they are local serialization dependencies and introduce no
network data flow.

### Settings schema v1

The canonical payload must cover every setting currently persisted by the UI:

```rust
pub struct PersistedSettingsV1 {
    pub theme: ThemeId,
    pub language: LocaleId,
    pub diff_font_size: u32,
    pub diff_font_family: FontFamilySetting,
    pub context_lines: usize,
    pub last_left_dir: Option<PathBuf>,
    pub last_right_dir: Option<PathBuf>,
    pub profiles: Vec<PersistedDiffProfileV1>,
    pub active_profile: usize,
    pub ignore_extensions: String,
    pub ignore_dirs: String,
    pub explorer_compact: bool,
    pub enable_binary_comparison: bool,
    pub remember_explorer_dirs: bool,
}
```

Before implementation, reconcile this payload with `UserSettings`. The chosen
end state is one public canonical domain type plus explicit versioned payload
DTOs. Do not keep both a core `UserSettings` and a UI `AppSettings` that each
claim disk ownership.

Validation normalizes invalid indexes/ranges without dropping otherwise valid
fields:

- font size: clamp to the supported UI range;
- context lines: clamp to the supported range;
- empty profile list: restore built-in defaults;
- active profile: clamp to a valid index;
- built-in profiles: recreate canonical built-ins, then append valid custom
  profiles without duplicate IDs/names according to the existing policy.

### Session schema v1

The canonical payload stores path pairs only:

```rust
pub struct PersistedSessionV1 {
    pub tabs: Vec<PersistedComparePairV1>,
}

pub struct PersistedComparePairV1 {
    pub left: PathBuf,
    pub right: PathBuf,
}
```

Tab identity, load generation, dirty buffers, and mergetool output targets are
not restored. CLI startup arguments continue to take precedence over session
restore.

## Envelope contract

Each file uses:

```json
{
  "schema_name": "settings",
  "schema_version": 1,
  "app_version": "0.x.y",
  "created_unix": 0,
  "updated_unix": 0,
  "payload": {}
}
```

Replace or harden the current hand-written JSON field extraction so escaped
strings, nested payloads, and unfamiliar fields are handled by a real JSON
parser. Unknown fields in a known schema are ignored. An unknown schema name or
newer version is not treated as defaults.

## Load result taxonomy

```rust
pub enum PersistenceLoad<T> {
    Missing { defaults: T },
    Current { value: T },
    MigratedLegacy { value: T, source_backup_required: bool },
    MigratedVersion { value: T, from: u32 },
    FutureVersion { schema: String, version: u32 },
    Corrupt { detail: PersistenceError },
}
```

User-facing behavior:

- `Missing`: start with defaults; no warning.
- `Current`: load normally.
- `MigratedLegacy`/`MigratedVersion`: load the migrated value; show a single
  informational notice after the migration is durably written.
- `FutureVersion`: do not overwrite; show an incompatibility dialog offering
  Exit, Continue with temporary defaults, or choose a different config file
  location if that capability is later approved. Continuing must not save over
  the future file.
- `Corrupt`: preserve the file; offer temporary defaults and reveal/copy the
  path. Any reset is an explicit confirmed action that creates a backup.

## Legacy import

Legacy detection occurs only when the top-level object lacks `schema_name`.
Deserialize exact legacy DTOs corresponding to the current UI structs:

- `LegacyAppSettingsV0`;
- `LegacySessionStateV0` with `tabs: Vec<(String, String)>`.

Do not infer a legacy format from partially matching arbitrary JSON. A failure
to deserialize a recognized legacy candidate is `Corrupt`, not defaults.

On first durable rewrite:

1. read and validate the legacy source;
2. write `<name>.migration-tmp` in the same directory;
3. flush according to the selected save contract;
4. copy original to `<name>.pre-v1.bak` without overwriting an existing backup;
5. atomically replace the original where the platform supports it;
6. retain the backup path in the success result for diagnostics.

The implementation must reuse or generalize core safe-file primitives rather
than creating an unrelated unsafe writer.

## Repository API

Introduce explicit-path repositories, for example:

```rust
pub struct SettingsRepository { path: PathBuf }
pub struct SessionRepository { path: PathBuf }

impl SettingsRepository {
    pub fn load(&self) -> Result<PersistenceLoad<UserSettings>, PersistenceError>;
    pub fn save(&self, value: &UserSettings) -> Result<PersistenceSave, PersistenceError>;
}
```

The UI creates repositories from the platform config directory. Production
code no longer calls `ConfigManager<AppSettings>` or
`ConfigManager<SessionState>` directly.

## UI integration

- Replace UI-owned serializable settings with the canonical core type or a
  non-serializing view adapter.
- Route every deliberate settings change through one persistence service.
- Session save/restore uses `SessionRepository`.
- Failed saves are visible and do not masquerade as success.
- Temporary-default mode carries a `persistence_write_disabled` flag so later
  effects cannot overwrite a future/corrupt source accidentally.
- Restore continues to skip a pair only when both paths are absent, matching
  current behavior.

## Test design

### Core schema tests

- settings/session current envelope round-trip;
- legacy v0 fixtures import every field exactly;
- older envelope migrates forward;
- future version returns `FutureVersion` and leaves bytes unchanged;
- unknown schema name is rejected;
- malformed JSON returns `Corrupt` and leaves bytes unchanged;
- unknown fields in current payload are tolerated;
- invalid ranges/indexes normalize predictably;
- migration backup is created and original bytes are preserved there;
- failed migration write leaves original intact.

Commit sanitized fixture files representing the exact current plain JSON shape.
They must contain only temporary test paths, never real user paths.

### Runtime-path tests

- application settings loader consumes a legacy fixture and returns the same
  user-visible settings;
- UI persistence writes an envelope, not plain `AppSettings` JSON;
- session restore reads the envelope and opens only eligible path pairs;
- future schema selects temporary defaults with writes disabled;
- explicit reset creates a backup before replacement.

These tests must call the functions used by `App`, not only lower-level core
parsers.

## Compatibility

- Existing v0.164 plain settings/session files are supported as schema v0.
- New files use schema v1.
- Downgrading to v0.164 after migration may not understand the envelope; the
  `.pre-v1.bak` file provides recovery. Document this before release.
- Unknown future versions are never silently downgraded.

## Security and privacy

The schemas store local paths and preferences, not file content or secrets.
Migration must not print full paths in general logs. Diagnostics may show the
config file location only under the existing privacy policy. No network or
authentication surface is introduced.

## Implementation sequence

1. Add serde-backed envelope/payload DTOs and test fixtures in core.
2. Implement validation, legacy import, and load result taxonomy.
3. Implement explicit-path repositories and safe migration writes.
4. Add runtime adapter tests before changing `App`.
5. Migrate UI settings load/save and remove duplicate serialization ownership.
6. Migrate UI session load/save.
7. Add recovery UI and write-disable protection.
8. Update docs, requirements mapping, and threat model.

## Acceptance criteria

- The running app reads and writes versioned settings/session envelopes.
- Every current UI setting survives a legacy migration test.
- Future and corrupt schemas are preserved and visibly reported.
- No production path calls plain `ConfigManager<AppSettings/SessionState>`.
- Runtime-path tests exercise the same functions invoked at startup/effects.
- Migration backup/atomicity behavior is tested on Linux and exercised during
  RFC-078 on Windows and macOS.
- Core and UI no longer contain competing persisted settings/session models.

## Alternatives considered

### Wrap existing UI JSON without reconciling models

Rejected: it would preserve duplicate ownership and leave core tests detached
from production.

### Treat all parse failures as defaults

Rejected: it can silently erase future-version or corrupt user configuration.

### Delete and recreate on migration

Rejected: it violates reversibility and downgrade recovery.

### Database persistence

Rejected as unnecessary complexity for two small local documents.

## Dependencies

- Parent: RFC-074.
- May proceed independently of RFC-075 with coordinated ownership of UI state
  files.
- Platform migration behavior is accepted under RFC-078.

