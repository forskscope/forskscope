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

Existing production plain-JSON files will be imported as legacy UI schema v0.
The already-implemented core settings/session envelopes are schema v1 and will
be migrated rather than reinterpreted. The converged runtime schemas are v2.
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
- Migrate existing core schema-v1 envelopes without treating them as v2.
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

### Settings schema v2

Schema v2 is a superset of settings represented by either the shipping UI
model or the existing core `UserSettings` v1 model. This prevents convergence
from silently discarding core fields that are not yet wired to the UI.

```rust
pub struct PersistedSettingsV2 {
    pub theme: ThemeId,
    pub density: Density,
    pub language: LocaleId,
    pub diff_font_size: u32,
    pub ui_font_family: FontFamilySetting,
    pub diff_font_family: DiffFontFamilySetting,
    pub context_lines: usize,
    pub show_line_numbers: bool,
    pub wrap_long_lines: bool,
    pub last_left_dir: Option<PathBuf>,
    pub last_right_dir: Option<PathBuf>,
    pub profiles: Vec<PersistedDiffProfileV2>,
    pub active_profile: usize,
    pub ignore_extensions: String,
    pub ignore_dirs: String,
    pub explorer_compact: bool,
    pub enable_binary_comparison: bool,
    pub remember_explorer_dirs: bool,
    pub newline_policy: NewlinePolicy,
    pub performance: PerformanceLimits,
    pub restore_session: bool,
    pub recent_limit: usize,
}

pub enum DiffFontFamilySetting {
    SystemMono,
    SystemSans,
    SystemSerif,
    CourierNew,
    Consolas,
}

pub struct PersistedDiffProfileV2 {
    pub name: String,
    pub whitespace: WhitespaceMode,
    pub newlines: NewlineCompareMode,
    pub case: CaseSensitivity,
    pub inline_mode: InlineMode,
    pub algorithm: DiffAlgorithm,
    pub built_in: bool,
}
```

Before implementation, reconcile this payload with `UserSettings`. The chosen
end state is one public canonical domain type plus private version-specific
payload DTOs. Do not keep both a core `UserSettings` and a UI `AppSettings`
that each claim disk ownership. UI-only enums become projections of core types
or are removed after the runtime switch. The core domain must represent all
five currently selectable diff-font families; `CourierNew` and `Consolas`
cannot be normalized to `SystemMono` during migration. The existing core
appearance font and shipping UI diff font remain separate fields because they
have different documented scopes.

Validation normalizes invalid indexes/ranges without dropping otherwise valid
fields:

- font size: clamp to the supported UI range;
- context lines: clamp to the supported range;
- empty profile list: restore built-in defaults;
- active profile: clamp to a valid index;
- built-in profiles: recreate canonical built-ins, then append valid custom
  profiles without duplicate IDs/names according to the existing policy.

### Session schema v2

The canonical payload stores restorable path state, not live task identity or
unsaved content:

```rust
pub struct PersistedSessionV2 {
    pub tabs: Vec<PersistedComparePairV2>,
    pub active_tab: Option<usize>,
    pub explorer_roots: Option<PersistedDirectoryPairV2>,
}

pub struct PersistedComparePairV2 {
    pub left: PathBuf,
    pub right: PathBuf,
}

pub struct PersistedDirectoryPairV2 {
    pub left: PathBuf,
    pub right: PathBuf,
}
```

`active_tab` is a validated ordering hint, not a durable identity. RFC-075's
`CompareTabId` and `LoadGeneration` are runtime concurrency tokens and are
always freshly allocated on restore. Existing core-v1 `SessionId`/`TabId`
values are accepted by the v1 migration DTO but are not copied into v2. Dirty
flags, merge buffers, and mergetool output targets are not restored because the
session file has no content needed to reconstruct them safely. CLI startup
arguments continue to take precedence over session restore.

This deliberately narrows the runtime session domain to restorable paths. The
RFC-011 status note and core documentation must be amended to record that its
v1 identity-rich model is a legacy migration input, while v2 is the canonical
runtime persistence contract.

## Envelope contract

Each file uses:

```json
{
  "schema_name": "settings",
  "schema_version": 2,
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

## Migration inputs and routing

Parse the top-level object once with `serde_json`, then route without guessing:

| Input | Detection | Action |
|---|---|---|
| UI settings/session v0 | no `schema_name`; exact legacy DTO matches | migrate to v2 |
| Core settings envelope v1 | `schema_name = settings`, version 1 | parse `CoreSettingsEnvelopeV1`, migrate union fields to v2 |
| Core session envelope v1 | `schema_name = session`, version 1 | parse `CoreSessionEnvelopeV1`, preserve restorable roots/path pairs, migrate to v2 |
| Current envelope v2 | matching schema name and version 2 | validate and load |
| Future envelope | matching schema name and version greater than 2 | preserve and return `FutureVersion` |
| Wrong schema name or malformed recognized input | any | preserve and return `Corrupt`/schema mismatch |

Unversioned legacy detection occurs only when the top-level object lacks
`schema_name`. Deserialize exact legacy DTOs corresponding to the current UI
structs:

- `LegacyAppSettingsV0`;
- `LegacySessionStateV0` with `tabs: Vec<(String, String)>`.

Do not infer a legacy format from partially matching arbitrary JSON. A failure
to deserialize a recognized legacy candidate is `Corrupt`, not defaults.

Core-v1 migration rules are explicit:

- settings preserve appearance, diff, file, performance, and locale fields;
- the single v1 core compare profile becomes the selected v2 profile and
  canonical built-ins are added without replacing its values;
- session file/directory roots become their corresponding v2 path state;
- any parseable v1 diff/binary/XLSX tab paths are preserved as compare pairs;
- v1 IDs, timestamps, error-tab messages, and dirty summaries are not carried
  forward because they are not sufficient to restore unsaved content safely;
- every deliberate discard is asserted in migration tests and documented in
  the RFC-011 compatibility note.

Field precedence is source-specific rather than “last value wins”:

| v2 field group | UI-v0 source | Core-v1 source |
|---|---|---|
| theme, locale, diff font size | matching active UI fields | appearance/locale fields |
| UI/diff font families | exact five-way UI diff-font value; UI font defaults | core appearance font; diff font defaults to the corresponding system family |
| profiles | every UI profile plus selected index | one exact selected core profile plus canonical built-ins |
| display/diff behavior | context and explorer fields; other fields default | density, line-number, wrapping, newline, and performance fields |
| file/session policy | remembered directories, ignores, binary, remember flag | restore-session and recent-limit fields |

A field absent from one source receives the canonical default; it is never
allowed to overwrite a value actually represented by that source.

On first durable rewrite:

1. read and validate the legacy source;
2. write `<name>.migration-tmp` in the same directory;
3. flush according to the selected save contract;
4. copy original to `<name>.pre-v2.bak` without overwriting an existing backup;
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
- existing core-v1 settings fixtures migrate every represented field;
- existing core-v1 session fixtures preserve all restorable path state;
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

- Existing v0.164 plain settings/session files are supported as UI schema v0.
- Existing core settings/session envelope fixtures remain supported as schema
  v1 migration inputs even though the shipping UI did not write them.
- New files use schema v2; schema version 1 is never reinterpreted with a new
  payload shape.
- Downgrading to v0.164 after migration may not understand the envelope; the
  `.pre-v2.bak` file provides recovery. Document this before release.
- Unknown future versions are never silently downgraded.

## Security and privacy

The schemas store local paths and preferences, not file content or secrets.
Migration must not print full paths in general logs. Diagnostics may show the
config file location only under the existing privacy policy. No network or
authentication surface is introduced.

## Implementation sequence

1. Add serde-backed envelope plus v0/v1/v2 payload DTOs and fixtures in core.
2. Implement routing, validation, both migration paths, and load taxonomy.
3. Implement explicit-path repositories and safe migration writes.
4. Add runtime adapter tests before changing `App`.
5. Migrate UI settings load/save and remove duplicate serialization ownership.
6. Migrate UI session load/save.
7. Add recovery UI and write-disable protection.
8. Update docs, requirements mapping, and threat model.

## Acceptance criteria

- The running app reads and writes versioned settings/session envelopes.
- Every current UI setting survives a legacy migration test.
- Existing core-v1 settings/session envelopes migrate without schema
  reinterpretation and preserve every restorable setting/path field.
- Future and corrupt schemas are preserved and visibly reported.
- No production path calls plain `ConfigManager<AppSettings/SessionState>`.
- Runtime-path tests exercise the same functions invoked at startup/effects.
- Migration backup/atomicity behavior is tested on Linux and exercised during
  RFC-078 on Windows and macOS.
- Core and UI no longer contain competing persisted settings/session models.
- RFC-011 and core documentation distinguish legacy persisted IDs from
  RFC-075's fresh runtime-only compare/load identity.

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
