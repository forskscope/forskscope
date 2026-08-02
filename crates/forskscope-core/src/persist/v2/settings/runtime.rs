//! Runtime resolution for the settings file (RFC-076 §"User-facing behavior",
//! implementation sequence step 4: "Add runtime adapter tests before changing
//! `App`").
//!
//! [`resolve_and_commit`] decides what value a run should actually use and,
//! for a legacy/older-version file, durably commits the migration
//! immediately — this is the caller [`super::SettingsRepository::commit_migration`]
//! did not yet have when review 037 raised N1. Pure data out: no dialog text,
//! no Dioxus/GTK dependency. Nothing here is wired into `App` yet;
//! `forskscope-ui` still calls `app_json_settings::ConfigManager` directly
//! until patch 4.

use std::path::PathBuf;

use super::{PersistedSettingsV2, SettingsRepository};
use crate::persist::v2::{PersistenceError, PersistenceLoad};

/// What happened when resolving a settings file at startup, after any
/// durable migration commit.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsRuntimeOutcome {
    /// No file existed; the resolved value is defaults.
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
    /// defaults.
    Incompatible { schema: String, version: u32 },
    /// The file could not be parsed as any recognized shape. The original
    /// bytes are untouched on disk; the resolved value is temporary
    /// defaults.
    CorruptPreserved { detail: PersistenceError },
}

/// The resolved settings state for this run.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsRuntimeResolution {
    pub value: PersistedSettingsV2,
    /// `true` when `value` must not be written back: the on-disk file is a
    /// future/corrupt source that a later save must not silently overwrite
    /// (RFC-076 "persistence_write_disabled").
    pub write_disabled: bool,
    pub outcome: SettingsRuntimeOutcome,
}

/// Loads `repo` and, for a legacy/older-version file, durably commits the
/// migration immediately. Never writes for `Missing`, `Current`,
/// `FutureVersion`, or `Corrupt` — those are read-only outcomes by
/// definition.
pub fn resolve_and_commit(repo: &SettingsRepository) -> SettingsRuntimeResolution {
    let (load, raw) = repo.load_with_raw();
    match load {
        PersistenceLoad::Missing { defaults } => SettingsRuntimeResolution {
            value: defaults,
            write_disabled: false,
            outcome: SettingsRuntimeOutcome::Fresh,
        },
        PersistenceLoad::Current { value } => SettingsRuntimeResolution {
            value,
            write_disabled: false,
            outcome: SettingsRuntimeOutcome::Current,
        },
        PersistenceLoad::MigratedLegacy { value, .. } => commit_migrated(repo, value, raw),
        PersistenceLoad::MigratedVersion { value, .. } => commit_migrated(repo, value, raw),
        PersistenceLoad::FutureVersion { schema, version } => SettingsRuntimeResolution {
            value: PersistedSettingsV2::default(),
            write_disabled: true,
            outcome: SettingsRuntimeOutcome::Incompatible { schema, version },
        },
        PersistenceLoad::Corrupt { detail } => SettingsRuntimeResolution {
            value: PersistedSettingsV2::default(),
            write_disabled: true,
            outcome: SettingsRuntimeOutcome::CorruptPreserved { detail },
        },
    }
}

fn commit_migrated(
    repo: &SettingsRepository,
    value: PersistedSettingsV2,
    raw: Option<Vec<u8>>,
) -> SettingsRuntimeResolution {
    let (backup_path, committed) = match raw
        .as_deref()
        .map(|bytes| repo.commit_migration(&value, bytes))
    {
        Some(Ok(outcome)) => (outcome.backup_path, true),
        Some(Err(_)) | None => (None, false),
    };
    SettingsRuntimeResolution {
        value,
        write_disabled: false,
        outcome: SettingsRuntimeOutcome::Migrated {
            backup_path,
            committed,
        },
    }
}
