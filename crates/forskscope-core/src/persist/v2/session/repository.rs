//! Explicit-path repository for the session file (RFC-076 "Repository API").

use std::path::PathBuf;

use super::super::repository::{
    PersistenceIoError, PersistenceSaveOutcome, atomic_write_envelope, build_envelope_json,
    ensure_pre_v2_backup, read_to_string_or_missing,
};
use super::{PersistedSessionV2, SESSION_SCHEMA_NAME, SESSION_SCHEMA_VERSION_V2, load_session_v2};
use crate::persist::v2::{PersistenceError, PersistenceLoad};

/// Reads and writes the session file at an explicit path. Never resolves a
/// platform config directory itself — see [`super::super::settings::SettingsRepository`]
/// for the same rationale.
pub struct SessionRepository {
    path: PathBuf,
}

impl SessionRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> PersistenceLoad<PersistedSessionV2> {
        match read_to_string_or_missing(&self.path) {
            Ok(Some(raw)) => load_session_v2(&raw),
            Ok(None) => PersistenceLoad::Missing {
                defaults: PersistedSessionV2::default(),
            },
            Err(e) => PersistenceLoad::Corrupt {
                detail: PersistenceError::Io(e.0),
            },
        }
    }

    /// Ordinary atomic write of the current state. No backup — that policy
    /// belongs to [`Self::commit_migration`], the one-time durable rewrite.
    pub fn save(&self, value: &PersistedSessionV2) -> Result<(), PersistenceIoError> {
        let json = build_envelope_json(
            SESSION_SCHEMA_NAME,
            SESSION_SCHEMA_VERSION_V2,
            &self.path,
            value,
        );
        atomic_write_envelope(&self.path, &json)
    }

    /// Durably commits a migrated value (RFC-076 "On first durable
    /// rewrite"): preserves `original_bytes` as a non-overwriting
    /// `<name>.pre-v2.bak`, then atomically writes the v2 envelope.
    /// `original_bytes` should be exactly what was read to produce `value`
    /// via [`Self::load`] — the caller owns that pairing, since this method
    /// has no way to verify it.
    pub fn commit_migration(
        &self,
        value: &PersistedSessionV2,
        original_bytes: &[u8],
    ) -> Result<PersistenceSaveOutcome, PersistenceIoError> {
        let backup_path = ensure_pre_v2_backup(&self.path, original_bytes)?;
        self.save(value)?;
        Ok(PersistenceSaveOutcome {
            backup_path: Some(backup_path),
        })
    }
}
