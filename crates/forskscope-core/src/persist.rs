//! Persisted-data schema ownership (RFC-076, audit finding B2).
//!
//! [`schema`] defines the canonical settings/session schema, migrates the
//! shipping UI's plain-JSON v0 files, and reports load outcomes via
//! `PersistenceLoad` — see its module doc for the full design.
//!
//! This module does not claim to cover every file ForskScope writes:
//! `dir::batch::BatchManifest::to_json` hand-rolls its own JSON with no
//! schema envelope, and nothing here versions it (tracked as F31, M4).
//!
//! RFC-031's original `VersionedEnvelope`/`SchemaName`/`MigrationPolicy`
//! wrapper and the v1 settings/session records it carried (`UserSettings`,
//! `WorkspaceSession`) were removed by RFC-076's 2026-08-03 amendment
//! (patch 5): no released version of ForskScope ever wrote a v1 file, so
//! there was nothing left for that machinery to read.

pub mod schema;
