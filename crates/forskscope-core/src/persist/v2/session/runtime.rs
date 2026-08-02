//! Runtime resolution for the session file (RFC-076 §"User-facing behavior",
//! implementation sequence step 4: "Add runtime adapter tests before changing
//! `App`"). Mirrors [`super::super::settings::runtime`]; see its module doc
//! for the full rationale.

use std::path::PathBuf;

use super::{PersistedSessionV2, SessionRepository};
use crate::persist::v2::{PersistenceError, PersistenceLoad};

/// What happened when resolving a session file at startup, after any durable
/// migration commit.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionRuntimeOutcome {
    /// No file existed; the resolved value is defaults (no restorable tabs).
    Fresh,
    /// A current v2 file was loaded as-is.
    Current,
    /// A legacy (v0) or older-envelope (v1) file was migrated. `committed`
    /// is `false` only if the durable rewrite lost a race with an external
    /// change to the file between this load and the commit attempt (review
    /// 037 N1) — the migrated value is still used for this run, but nothing
    /// was written, so the file will be re-migrated on the next run.
    Migrated {
        backup_path: Option<PathBuf>,
        committed: bool,
    },
    /// The file's schema is newer than this build understands. The original
    /// bytes are untouched on disk; the resolved value is temporary
    /// defaults (no restorable tabs).
    Incompatible { schema: String, version: u32 },
    /// The file could not be parsed as any recognized shape. The original
    /// bytes are untouched on disk; the resolved value is temporary
    /// defaults (no restorable tabs).
    CorruptPreserved { detail: PersistenceError },
}

/// The resolved session state for this run.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionRuntimeResolution {
    pub value: PersistedSessionV2,
    /// `true` when `value` must not be written back: the on-disk file is a
    /// future/corrupt source that a later save must not silently overwrite
    /// (RFC-076 "persistence_write_disabled").
    pub write_disabled: bool,
    pub outcome: SessionRuntimeOutcome,
}

/// Loads `repo` and, for a legacy/older-version file, durably commits the
/// migration immediately. Never writes for `Missing`, `Current`,
/// `FutureVersion`, or `Corrupt` — those are read-only outcomes by
/// definition.
pub fn resolve_and_commit(repo: &SessionRepository) -> SessionRuntimeResolution {
    let (load, raw) = repo.load_with_raw();
    match load {
        PersistenceLoad::Missing { defaults } => SessionRuntimeResolution {
            value: defaults,
            write_disabled: false,
            outcome: SessionRuntimeOutcome::Fresh,
        },
        PersistenceLoad::Current { value } => SessionRuntimeResolution {
            value,
            write_disabled: false,
            outcome: SessionRuntimeOutcome::Current,
        },
        PersistenceLoad::MigratedLegacy { value, .. } => commit_migrated(repo, value, raw),
        PersistenceLoad::MigratedVersion { value, .. } => commit_migrated(repo, value, raw),
        PersistenceLoad::FutureVersion { schema, version } => SessionRuntimeResolution {
            value: PersistedSessionV2::default(),
            write_disabled: true,
            outcome: SessionRuntimeOutcome::Incompatible { schema, version },
        },
        PersistenceLoad::Corrupt { detail } => SessionRuntimeResolution {
            value: PersistedSessionV2::default(),
            write_disabled: true,
            outcome: SessionRuntimeOutcome::CorruptPreserved { detail },
        },
    }
}

fn commit_migrated(
    repo: &SessionRepository,
    value: PersistedSessionV2,
    raw: Option<Vec<u8>>,
) -> SessionRuntimeResolution {
    let (backup_path, committed) = match raw
        .as_deref()
        .map(|bytes| repo.commit_migration(&value, bytes))
    {
        Some(Ok(outcome)) => (outcome.backup_path, true),
        Some(Err(_)) | None => (None, false),
    };
    SessionRuntimeResolution {
        value,
        write_disabled: false,
        outcome: SessionRuntimeOutcome::Migrated {
            backup_path,
            committed,
        },
    }
}
