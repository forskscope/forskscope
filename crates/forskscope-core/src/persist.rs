//! Persisted-data schema ownership (RFC-076, audit finding B2).
//!
//! [`schema`] defines the canonical settings/session schema, migrates the
//! shipping UI's plain-JSON v0 files, and reports load outcomes via
//! `PersistenceLoad` — see its module doc for the full design.
//!
//! This module does not claim to cover every file ForskScope writes:
//! `dir::batch::BatchManifest::to_json` and `report::{file,dir}`'s
//! `to_json` hand-roll their own JSON with no schema envelope. **Decided
//! (F31, 2026-08-13): both stay explicitly unversioned** — see their module
//! docs for the reasoning (in short: nothing ever reads a batch manifest or
//! an exported report back into this app, so there is no read path for a
//! schema version to protect).
//!
//! RFC-031's original `VersionedEnvelope`/`SchemaName`/`MigrationPolicy`
//! wrapper and the v1 settings/session records it carried (`UserSettings`,
//! `WorkspaceSession`) were removed by RFC-076's 2026-08-03 amendment
//! (patch 5): no released version of ForskScope ever wrote a v1 file, so
//! there was nothing left for that machinery to read.

pub mod schema;
