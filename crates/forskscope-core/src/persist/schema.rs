//! Canonical settings/session schema (RFC-076, audit finding B2).
//!
//! The running application used to serialize its own plain-JSON structs
//! directly — a corrupt or future-schema file was then indistinguishable
//! from a missing one, and both silently collapsed to defaults. This module
//! makes `forskscope-core` the canonical owner of the settings and session
//! disk schema: it defines the canonical payload shapes, migrates the
//! shipping UI's plain-JSON v0 files, and reports what happened via
//! [`PersistenceLoad`] instead of collapsing every failure mode into the
//! same silent default.
//!
//! Core schema v1 (RFC-031's `UserSettings`/`WorkspaceSession`, wrapped in
//! the now-removed `VersionedEnvelope`) was never shipped to users and its
//! migration path was removed by RFC-076's 2026-08-03 amendment (patch 5). A
//! v1 envelope is preserved and reported as `Corrupt`, like any other
//! unrecognized version — see [`settings`] and [`session`]'s module docs.
//!
//! This module reads raw JSON with `serde_json` rather than a hand-written
//! field extractor, since v0 detection needs to distinguish "no envelope at
//! all" (legacy UI JSON) from "envelope present but malformed", and
//! unfamiliar fields, escaped strings, and nested payloads must be handled
//! correctly.
//!
//! ## What this module does not do (patch 1 boundary)
//!
//! No file I/O. [`settings::load_settings`] and [`session::load_session`]
//! are pure functions from a JSON string to a [`PersistenceLoad`]; the
//! explicit-path repositories that read and write files are patch 2.
//! Production `forskscope-ui` call sites are unchanged until patch 4.
//!
//! ## Why several fields have no `#[serde(default)]`
//!
//! `PersistedSettings`'s `theme`, `language`, `diff_font_size`,
//! `context_lines`, `profiles`, and `active_profile`, and
//! `PersistedSession`'s `tabs`, are required: a v2 payload missing one of
//! them is [`PersistenceError::MalformedPayload`], not silently defaulted.
//! This is deliberate, not an oversight — B2 exists because the running
//! application collapsed corrupt/unfamiliar files into defaults, so
//! `Corrupt` here is the fix, not a rough edge to smooth over. If a bug
//! report about a rejected file leads to adding `#[serde(default)]` to one
//! of these fields, that change reintroduces the exact silent-reset
//! behaviour this module exists to remove — reconsider before doing that.

mod repository;
pub mod session;
pub mod settings;

/// Re-exported because it appears in [`settings::SettingsRepository::commit_migration`]
/// and [`session::SessionRepository::commit_migration`]'s public signatures.
pub use repository::PersistenceCommitError;

/// The outcome of loading one persisted file, distinguishing every case RFC-076
/// requires the caller to handle differently. Never collapses a distinguishable
/// failure into a default.
#[derive(Debug, Clone, PartialEq)]
pub enum PersistenceLoad<T> {
    /// No file existed yet. Caller supplies the defaults; not itself an error.
    Missing { defaults: T },
    /// A current (v2) envelope, loaded and validated.
    Current { value: T },
    /// The shipping UI's plain-JSON v0 file was migrated. `source_backup_required`
    /// is `true` because migration has not yet written a durable backup — that
    /// happens at the repository layer (patch 2), not here.
    MigratedLegacy {
        value: T,
        source_backup_required: bool,
    },
    /// The schema name matched but the version is newer than this build
    /// understands. The original bytes must be preserved, never overwritten.
    FutureVersion { schema: String, version: u32 },
    /// The input could not be recognized as any known shape: malformed JSON,
    /// a schema-name mismatch, or a top-level object that has no `schema_name`
    /// but does not match the exact legacy DTO either. The original bytes
    /// must be preserved; this is never treated as `Missing`.
    Corrupt { detail: PersistenceError },
}

/// Why a persisted file could not be loaded as any recognized shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    /// The bytes are not valid JSON, or not a JSON object at the top level.
    MalformedJson,
    /// A `schema_name` field is present but does not match the schema this
    /// loader is for (e.g. a session file passed to the settings loader).
    SchemaNameMismatch { found: String },
    /// No `schema_name` field is present, so this is a v0 candidate, but it
    /// does not deserialize as the exact legacy DTO. Per RFC-076: a failure to
    /// deserialize a recognized legacy candidate is `Corrupt`, not defaults —
    /// never infer a legacy format from partially matching arbitrary JSON.
    UnrecognizedLegacyShape,
    /// An envelope's `schema_version` field is missing or not a valid version
    /// number, or `schema_version` is `0` (there is no v0 envelope — v0 is
    /// defined as the absence of an envelope).
    MalformedVersion,
    /// The `payload` field is missing, or does not deserialize as the DTO its
    /// declared schema version implies.
    MalformedPayload,
    /// The file exists but could not be read (e.g. permission denied). Not
    /// `Missing` — the file's existence is known, only its content is not;
    /// treating this as `Missing` would mean writing over content that may
    /// still be there and readable once the underlying problem is fixed.
    Io(String),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedJson => write!(f, "not valid JSON"),
            Self::SchemaNameMismatch { found } => write!(f, "unexpected schema name: {found}"),
            Self::UnrecognizedLegacyShape => {
                write!(
                    f,
                    "no schema_name, and does not match the legacy JSON shape"
                )
            }
            Self::MalformedVersion => write!(f, "missing or invalid schema_version"),
            Self::MalformedPayload => write!(f, "payload does not match its declared schema"),
            Self::Io(message) => write!(f, "could not read file: {message}"),
        }
    }
}

impl std::error::Error for PersistenceError {}
