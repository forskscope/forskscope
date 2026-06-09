# RFC 031 — Release Channel, Migration, and Data Compatibility

**Status.** Proposed

## Status

Proposed.

## Summary

Define release channels, migration policy, session/settings compatibility, and preview-to-stable upgrade behavior for the Dioxus-based ForskScope v3 line.

## Goals

- Avoid breaking users' local settings and sessions unexpectedly.
- Make preview releases clearly distinct from stable releases.
- Provide versioned session/settings files.
- Define migration and rollback behavior.
- Ensure v3 can coexist with previous versions during evaluation.

## Non-goals

- Auto-update system in the first release.
- Cloud account migration.
- Sync across devices.

## Release channels

```text
nightly    developer/CI artifacts; may break compatibility
preview    user-testable migration builds; migration warnings required
stable     recommended public release channel
lts        future only, not required for v3 initial release
```

## Versioned app data

Recommended local data layout:

```text
ForskScope/
  v3/
    settings.json
    profiles.json
    sessions/
    logs/
    backups/
```

This prevents accidental corruption of previous application data.

## Schema versioning

```rust
pub struct VersionedEnvelope<T> {
    pub schema_name: String,
    pub schema_version: u32,
    pub app_version: String,
    pub payload: T,
}
```

All persisted files should use a schema envelope.

## Migration policy

```text
Compatible read:
  App can read and use the file directly.

Forward migration:
  App can read old schema and write new schema after user confirmation or safe automatic migration.

Unsupported old schema:
  App shows clear error and offers to preserve file untouched.

Newer schema:
  App refuses to write and tells user the file was created by a newer ForskScope version.
```

## Session compatibility

Sessions are convenience state, not source files. Corrupt or incompatible sessions must not block the app from starting.

Startup behavior:

```text
1. Load settings.
2. If settings fail, start with defaults and show warning.
3. Load recent sessions list.
4. If a session fails, skip it and show recoverable warning.
5. Never mutate failed files unless user explicitly chooses repair/migrate.
```

## Preview release warning

First launch of preview channel:

```text
This is a ForskScope v3 preview build.

It is intended for testing the Dioxus migration.
Your source files are not modified unless you explicitly save or execute a batch operation.
Preview settings and sessions are stored separately from stable data.

[Continue] [Open Data Folder]
```

## Stable release gate

A stable release requires:

- all Tier 1 platform smoke tests,
- no known data-loss bugs,
- save/backup/restore tests,
- session migration tests,
- editor fallback mode,
- user documentation,
- public known limitations.

## Rollback policy

Users should be able to delete v3 app data without affecting source files.

The app should provide:

```text
Settings → Advanced → Open App Data Folder
Settings → Advanced → Reset Window State
Settings → Advanced → Export Settings
```

Destructive reset should be manual, documented, and not performed automatically.

## Acceptance criteria

- Settings and session files are schema-versioned.
- Preview channel uses separate app data path.
- Newer schema files are not overwritten by older app versions.
- Corrupt session does not prevent startup.
- Migration tests exist for all persisted schemas.
- Stable release gate is documented.

## Test strategy

- Schema migration unit tests.
- Corrupt JSON tests.
- Newer schema refusal tests.
- Preview/stable data-path tests.
- Manual downgrade/rollback simulation.

## Dependencies

- RFC 011 Workspace/session persistence.
- RFC 020 Developer architecture/CI gates.
- RFC 027 Report/export.
- RFC 030 Documentation/onboarding.
