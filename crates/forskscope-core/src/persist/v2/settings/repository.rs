//! Explicit-path repository for the settings file (RFC-076 "Repository API").

use std::path::PathBuf;

use super::super::repository::{
    PersistenceIoError, PersistenceSaveOutcome, atomic_write_envelope, build_envelope_json,
    ensure_pre_v2_backup, read_to_string_or_missing,
};
use super::{
    PersistedSettingsV2, SETTINGS_SCHEMA_NAME, SETTINGS_SCHEMA_VERSION_V2, load_settings_v2,
};
use crate::persist::v2::{PersistenceError, PersistenceLoad};

/// Reads and writes the settings file at an explicit path. Never resolves a
/// platform config directory itself — RFC-076 keeps that in the UI/
/// infrastructure layer, which is what lets this type's tests use a temp
/// path instead of the developer's real config directory.
pub struct SettingsRepository {
    path: PathBuf,
}

impl SettingsRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> PersistenceLoad<PersistedSettingsV2> {
        match read_to_string_or_missing(&self.path) {
            Ok(Some(raw)) => load_settings_v2(&raw),
            Ok(None) => PersistenceLoad::Missing {
                defaults: PersistedSettingsV2::default(),
            },
            Err(e) => PersistenceLoad::Corrupt {
                detail: PersistenceError::Io(e.0),
            },
        }
    }

    /// Ordinary atomic write of the current state. No backup — that policy
    /// belongs to [`Self::commit_migration`], the one-time durable rewrite.
    pub fn save(&self, value: &PersistedSettingsV2) -> Result<(), PersistenceIoError> {
        let json = build_envelope_json(
            SETTINGS_SCHEMA_NAME,
            SETTINGS_SCHEMA_VERSION_V2,
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
        value: &PersistedSettingsV2,
        original_bytes: &[u8],
    ) -> Result<PersistenceSaveOutcome, PersistenceIoError> {
        let backup_path = ensure_pre_v2_backup(&self.path, original_bytes)?;
        self.save(value)?;
        Ok(PersistenceSaveOutcome {
            backup_path: Some(backup_path),
        })
    }
}
