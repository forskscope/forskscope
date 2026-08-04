//! Explicit-path repository for the session file (RFC-076 "Repository API").

use std::path::PathBuf;

use super::super::repository::{
    PersistenceCommitError, PersistenceIoError, PersistenceSaveOutcome, atomic_write_envelope,
    build_envelope_json, ensure_pre_v2_backup, ensure_reset_backup, read_to_string_or_missing,
    verify_unchanged,
};
use super::{PersistedSession, SESSION_SCHEMA_NAME, SESSION_SCHEMA_VERSION_V2, load_session};
use crate::persist::schema::{PersistenceError, PersistenceLoad};

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

    pub fn load(&self) -> PersistenceLoad<PersistedSession> {
        self.load_with_raw().0
    }

    /// Like [`Self::load`], but also returns the exact raw bytes that were
    /// read (when a file was present). A caller that intends to call
    /// [`Self::commit_migration`] needs this pairing: passing bytes from a
    /// separate, later read would defeat [`verify_unchanged`]'s guarantee.
    pub fn load_with_raw(&self) -> (PersistenceLoad<PersistedSession>, Option<Vec<u8>>) {
        match read_to_string_or_missing(&self.path) {
            Ok(Some(raw)) => {
                let bytes = raw.clone().into_bytes();
                (load_session(&raw), Some(bytes))
            }
            Ok(None) => (
                PersistenceLoad::Missing {
                    defaults: PersistedSession::default(),
                },
                None,
            ),
            Err(e) => (
                PersistenceLoad::Corrupt {
                    detail: PersistenceError::Io(e.0),
                },
                None,
            ),
        }
    }

    /// Ordinary atomic write of the current state. No backup — that policy
    /// belongs to [`Self::commit_migration`], the one-time durable rewrite.
    pub fn save(&self, value: &PersistedSession) -> Result<(), PersistenceIoError> {
        let json = build_envelope_json(
            SESSION_SCHEMA_NAME,
            SESSION_SCHEMA_VERSION_V2,
            &self.path,
            value,
        );
        atomic_write_envelope(&self.path, &json)
    }

    /// Durably commits a migrated value (RFC-076 "On first durable
    /// rewrite"): confirms the file still holds exactly `original_bytes`
    /// (review 037 N1 — refuses to guess if it changed since [`Self::load`]),
    /// preserves it as a non-overwriting `<name>.pre-v2.bak`, then atomically
    /// writes the v2 envelope. `original_bytes` must come from the same
    /// [`Self::load_with_raw`] call that produced `value`.
    pub fn commit_migration(
        &self,
        value: &PersistedSession,
        original_bytes: &[u8],
    ) -> Result<PersistenceSaveOutcome, PersistenceCommitError> {
        verify_unchanged(&self.path, original_bytes)?;
        let backup_path = ensure_pre_v2_backup(&self.path, original_bytes)?;
        self.save(value)?;
        Ok(PersistenceSaveOutcome {
            backup_path: Some(backup_path),
        })
    }

    /// Explicit, user-confirmed reset of a `Corrupt` file (RFC-076: "any
    /// reset is an explicit confirmed action that creates a backup"):
    /// confirms the file still holds exactly `original_bytes`, preserves it
    /// as a non-overwriting `<name>.reset.bak`, then atomically writes
    /// `value`. `original_bytes` must come from the same
    /// [`Self::load_with_raw`] call (or
    /// [`crate::persist::schema::session::runtime::SessionRuntimeResolution::raw_bytes`])
    /// that produced the resolution the user is resetting.
    pub fn reset_with_backup(
        &self,
        value: &PersistedSession,
        original_bytes: &[u8],
    ) -> Result<PersistenceSaveOutcome, PersistenceCommitError> {
        verify_unchanged(&self.path, original_bytes)?;
        let backup_path = ensure_reset_backup(&self.path, original_bytes)?;
        self.save(value)?;
        Ok(PersistenceSaveOutcome {
            backup_path: Some(backup_path),
        })
    }
}
