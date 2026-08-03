//! Runtime resolution for the settings file (RFC-076 §"User-facing behavior",
//! implementation sequence step 4: "Add runtime adapter tests before changing
//! `App`").
//!
//! [`resolve_and_commit`] decides what value a run should actually use and,
//! for a legacy/older-version file, durably commits the migration
//! immediately — this is the caller [`super::SettingsRepository::commit_migration`]
//! did not yet have when review 037 raised N1. Pure data out: no dialog text,
//! no Dioxus/GTK dependency. As of patch 4, `forskscope_ui::ui::view::settings::load`
//! calls this directly at startup; `app_json_settings::ConfigManager` is no
//! longer called from production code.

use std::path::PathBuf;

use super::{PersistedSettingsV2, SettingsRepository};
use crate::persist::v2::{PersistenceCommitError, PersistenceError, PersistenceLoad};

/// What happened when resolving a settings file at startup, after any
/// durable migration commit.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsRuntimeOutcome {
    /// No file existed; the resolved value is defaults.
    Fresh,
    /// A current v2 file was loaded as-is.
    Current,
    /// A legacy (v0) or older-envelope (v1) file was migrated. The migrated
    /// value is used for this run regardless of [`MigrationCommitOutcome`].
    Migrated(MigrationCommitOutcome),
    /// The file's schema is newer than this build understands. The original
    /// bytes are untouched on disk; the resolved value is temporary
    /// defaults.
    Incompatible { schema: String, version: u32 },
    /// The file could not be parsed as any recognized shape. The original
    /// bytes are untouched on disk; the resolved value is temporary
    /// defaults.
    CorruptPreserved { detail: PersistenceError },
}

/// What happened when a migrated value's durable rewrite was attempted.
/// Review 038 C1: `Conflict` and a persistent I/O failure are not alike and
/// must not be collapsed into one untyped "didn't commit" — only the former
/// is safe to stay silent about.
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationCommitOutcome {
    /// Durably written.
    Committed { backup_path: Option<PathBuf> },
    /// Refused because the file changed underneath the migration (review
    /// 037 N1's guard). Benign and self-healing: the next run re-reads and
    /// re-migrates from scratch, so nothing was lost — safe to stay silent
    /// about.
    DeferredByConflict,
    /// The commit failed for a reason unrelated to a race — permission
    /// denied, a read-only config directory, a full disk. Unlike
    /// `DeferredByConflict`, this recurs on every launch until the
    /// underlying cause is fixed, so it must be surfaced rather than
    /// silently retried forever (RFC-076: "failed saves are visible and do
    /// not masquerade as success").
    Failed { detail: String },
}

/// The resolved settings state for this run.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsRuntimeResolution {
    pub value: PersistedSettingsV2,
    /// `true` whenever this run must not write the file back: a
    /// future/corrupt source (RFC-076 "persistence_write_disabled"), or a
    /// migration commit that was refused or failed (review 038 C2) — in
    /// both cases nothing has established that overwriting the file is
    /// safe.
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
    let commit = match raw
        .as_deref()
        .map(|bytes| repo.commit_migration(&value, bytes))
    {
        Some(Ok(outcome)) => MigrationCommitOutcome::Committed {
            backup_path: outcome.backup_path,
        },
        Some(Err(PersistenceCommitError::Conflict)) => MigrationCommitOutcome::DeferredByConflict,
        Some(Err(PersistenceCommitError::Io(detail))) => MigrationCommitOutcome::Failed { detail },
        // Defensive: a `Migrated*` load always pairs with `Some` raw bytes
        // (see `load_with_raw`). Treat the unexpected case as a failure to
        // surface rather than silently committing or staying quiet.
        None => MigrationCommitOutcome::Failed {
            detail: "no bytes were read to migrate".into(),
        },
    };
    let write_disabled = !matches!(commit, MigrationCommitOutcome::Committed { .. });
    SettingsRuntimeResolution {
        value,
        write_disabled,
        outcome: SettingsRuntimeOutcome::Migrated(commit),
    }
}
