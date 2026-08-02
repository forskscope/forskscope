//! Canonical settings/session schema v2 (RFC-076, audit finding B2).
//!
//! The running application currently serializes its own plain-JSON structs
//! directly, bypassing the core's tested [`super::VersionedEnvelope`]
//! entirely — a corrupt or future-schema file is then indistinguishable from
//! a missing one, and both silently collapse to defaults. This module makes
//! `forskscope-core` the canonical owner of the settings and session disk
//! schema: it defines the v2 payload shapes, migrates the shipping UI's
//! plain-JSON v0 files and the existing (never-shipped-to-users) core v1
//! envelopes into v2, and reports what happened via [`PersistenceLoad`]
//! instead of collapsing every failure mode into the same silent default.
//!
//! ## Why this does not reuse `VersionedEnvelope::parse`
//!
//! [`super::VersionedEnvelope`]'s parser is a minimal hand-written field
//! extractor — adequate for the fixed shape it was designed for, but not a
//! general JSON parser. RFC-076 requires unfamiliar fields, escaped strings,
//! and nested payloads to be handled correctly, and v0 detection needs to
//! distinguish "no envelope at all" (legacy UI JSON) from "envelope present
//! but malformed" — a distinction the existing parser's `Result` collapses.
//! This module reads the raw file with `serde_json` instead.
//!
//! ## What this module does not do (patch 1 boundary)
//!
//! No file I/O. [`settings::load_settings_v2`] and [`session::load_session_v2`]
//! are pure functions from a JSON string to a [`PersistenceLoad`]; the
//! explicit-path repositories that read and write files are patch 2.
//! Production `forskscope-ui` call sites are unchanged until patch 4.
//!
//! ## Why several fields have no `#[serde(default)]`
//!
//! `PersistedSettingsV2`'s `theme`, `language`, `diff_font_size`,
//! `context_lines`, `profiles`, and `active_profile`, and
//! `PersistedSessionV2`'s `tabs`, are required: a v2 payload missing one of
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
    /// An existing core v1 envelope was migrated (never reinterpreted as v2).
    MigratedVersion { value: T, from: u32 },
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
