//! Versioned schema envelope for all persisted ForskScope data (RFC-031).
//!
//! Every file written by ForskScope — settings, sessions, compare profiles,
//! operation manifests — wraps its payload in a [`VersionedEnvelope`] so the
//! application can make safe forward/backward migration decisions at load time.
//!
//! ## Design
//!
//! `VersionedEnvelope` owns the metadata (schema name, schema version, app
//! version, timestamps) and accepts a pre-serialized JSON string for the
//! payload. Serialization of the payload is the caller's responsibility.
//! This keeps the envelope independent of `serde` and consistent with the
//! project's existing hand-written JSON pattern.
//!
//! ## Usage
//!
//! ```rust,no_run
//! # use forskscope_core::persist::{MigrationPolicy, VersionedEnvelope, SchemaName};
//! // Writing
//! let env = VersionedEnvelope::new(SchemaName::Settings, 1, "{\"theme\":\"dark\"}");
//! let json = env.to_json();
//!
//! // Reading
//! let parsed = VersionedEnvelope::parse(&json).unwrap();
//! match parsed.migration_policy(1) {
//!     MigrationPolicy::CompatibleRead => { /* use payload directly */ }
//!     MigrationPolicy::ForwardMigration { from_version } => { /* migrate */ }
//!     _ => { /* handle incompatibility */ }
//! }
//! ```

use std::fmt::Write as _;

// ── Schema names ──────────────────────────────────────────────────────────────

/// All schema names used by ForskScope persisted data (RFC-031 §"Versioned app data").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaName {
    /// Application-wide settings (`settings.json`).
    Settings,
    /// Named compare profiles (`profiles.json`).
    Profiles,
    /// Single workspace session (`sessions/<id>.json`).
    Session,
    /// Batch operation restore manifest (`manifests/<op>.json`).
    BatchManifest,
    /// File or directory comparison report.
    Report,
    /// Unknown schema — encountered when reading data from a future version.
    Unknown(String),
}

impl SchemaName {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Settings      => "settings",
            Self::Profiles      => "profiles",
            Self::Session       => "session",
            Self::BatchManifest => "batch_manifest",
            Self::Report        => "report",
            Self::Unknown(s)    => s.as_str(),
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "settings"       => Self::Settings,
            "profiles"       => Self::Profiles,
            "session"        => Self::Session,
            "batch_manifest" => Self::BatchManifest,
            "report"         => Self::Report,
            other            => Self::Unknown(other.into()),
        }
    }

    /// Public entry point for tests and migration code that need to
    /// construct a `SchemaName` from a string without going through parse.
    pub fn from_str_pub(s: &str) -> Self {
        Self::from_str(s)
    }
}

// ── Migration policy ──────────────────────────────────────────────────────────

/// The migration decision for a given schema file (RFC-031 §"Migration policy").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationPolicy {
    /// Schema version matches the application's current version.
    /// The file can be read and used directly.
    CompatibleRead,
    /// The file was written by an older version. The application can
    /// migrate automatically after optional user confirmation.
    ForwardMigration { from_version: u32 },
    /// The file was written by a *newer* version of the application.
    /// The application must not overwrite it; user must upgrade the app.
    NewerSchema { file_version: u32, app_version: u32 },
    /// The schema name is not recognized. Preserve the file untouched
    /// and show a clear error.
    UnknownSchema { schema_name: String },
}

impl MigrationPolicy {
    /// `true` when the payload can be used without migration.
    pub fn is_compatible(&self) -> bool {
        matches!(self, Self::CompatibleRead)
    }

    /// `true` when the application may safely overwrite this file.
    pub fn can_write(&self) -> bool {
        matches!(self, Self::CompatibleRead | Self::ForwardMigration { .. })
    }
}

// ── Envelope ──────────────────────────────────────────────────────────────────

/// A parsed, validated envelope read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEnvelope {
    pub schema_name:    SchemaName,
    pub schema_version: u32,
    pub app_version:    String,
    /// The raw JSON string of the inner payload, as written by the producer.
    pub payload_json:   String,
}

impl ParsedEnvelope {
    /// Determine the migration policy given the *current* schema version
    /// this application understands for this schema.
    pub fn migration_policy(&self, current_schema_version: u32) -> MigrationPolicy {
        if let SchemaName::Unknown(name) = &self.schema_name {
            return MigrationPolicy::UnknownSchema { schema_name: name.clone() };
        }
        match self.schema_version.cmp(&current_schema_version) {
            std::cmp::Ordering::Equal => MigrationPolicy::CompatibleRead,
            std::cmp::Ordering::Less  => MigrationPolicy::ForwardMigration {
                from_version: self.schema_version,
            },
            std::cmp::Ordering::Greater => MigrationPolicy::NewerSchema {
                file_version: self.schema_version,
                app_version:  current_schema_version,
            },
        }
    }
}

/// A versioned data envelope — the outer JSON wrapper for all persisted files.
#[derive(Debug, Clone)]
pub struct VersionedEnvelope {
    pub schema_name:    SchemaName,
    pub schema_version: u32,
    pub app_version:    String,
    pub created_unix:   u64,
    pub updated_unix:   u64,
    /// Pre-serialized JSON for the inner payload. Must be valid JSON.
    payload_json: String,
}

impl VersionedEnvelope {
    /// Create a new envelope with the current app version and timestamp.
    pub fn new(
        schema_name:    SchemaName,
        schema_version: u32,
        payload_json:   impl Into<String>,
    ) -> Self {
        let now = unix_now();
        Self {
            schema_name,
            schema_version,
            app_version: env!("CARGO_PKG_VERSION").into(),
            created_unix: now,
            updated_unix: now,
            payload_json: payload_json.into(),
        }
    }

    /// Create with explicit timestamps (for testing and migration).
    pub fn with_timestamps(
        schema_name:    SchemaName,
        schema_version: u32,
        app_version:    impl Into<String>,
        created_unix:   u64,
        updated_unix:   u64,
        payload_json:   impl Into<String>,
    ) -> Self {
        Self {
            schema_name,
            schema_version,
            app_version: app_version.into(),
            created_unix,
            updated_unix,
            payload_json: payload_json.into(),
        }
    }

    /// Serialize to a deterministic JSON string.
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{{");
        let _ = writeln!(s, "  \"schema_name\": {:?},",    self.schema_name.as_str());
        let _ = writeln!(s, "  \"schema_version\": {},",   self.schema_version);
        let _ = writeln!(s, "  \"app_version\": {:?},",    self.app_version);
        let _ = writeln!(s, "  \"created_unix\": {},",     self.created_unix);
        let _ = writeln!(s, "  \"updated_unix\": {},",     self.updated_unix);
        let _ = writeln!(s, "  \"payload\": {}",           self.payload_json);
        let _ = write!(s,   "}}");
        s
    }

    // ── Parse (minimal hand-written parser for the envelope fields) ───────────

    /// Parse the envelope metadata from a JSON string.
    ///
    /// Only the envelope fields are parsed; `payload_json` is extracted as a
    /// raw substring. This is a minimal parser for the fixed envelope shape —
    /// not a general-purpose JSON parser.
    pub fn parse(json: &str) -> Result<ParsedEnvelope, EnvelopeError> {
        let schema_name = extract_str_field(json, "schema_name")
            .ok_or(EnvelopeError::MissingField("schema_name"))?;
        let schema_version = extract_u32_field(json, "schema_version")
            .ok_or(EnvelopeError::MissingField("schema_version"))?;
        let app_version = extract_str_field(json, "app_version")
            .ok_or(EnvelopeError::MissingField("app_version"))?;
        let payload_json = extract_payload(json)
            .ok_or(EnvelopeError::MissingField("payload"))?;

        Ok(ParsedEnvelope {
            schema_name:    SchemaName::from_str(&schema_name),
            schema_version,
            app_version,
            payload_json,
        })
    }
}

// ── Parse error ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    MissingField(&'static str),
    InvalidJson,
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(name) => write!(f, "envelope missing field: {name}"),
            Self::InvalidJson        => write!(f, "envelope JSON is not valid"),
        }
    }
}

impl std::error::Error for EnvelopeError {}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Extract a JSON string field value from a flat JSON object.
/// Only handles `"field": "value"` with double-quoted strings.
fn extract_str_field(json: &str, field: &str) -> Option<String> {
    let key = format!("\"{}\":", field);
    let start = json.find(&key)? + key.len();
    let rest = json[start..].trim_start();
    if !rest.starts_with('"') { return None; }
    let inner = &rest[1..];
    let end = inner.find('"')?;
    Some(inner[..end].into())
}

/// Extract a JSON unsigned-integer field value.
fn extract_u32_field(json: &str, field: &str) -> Option<u32> {
    let key = format!("\"{}\":", field);
    let start = json.find(&key)? + key.len();
    let rest = json[start..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Extract the raw payload JSON value (the value of the `"payload"` key).
/// Handles objects `{...}` and arrays `[...]`.
fn extract_payload(json: &str) -> Option<String> {
    let key = "\"payload\":";
    let start = json.find(key)? + key.len();
    let rest = json[start..].trim_start();
    let open = rest.chars().next()?;
    let close = match open { '{' => '}', '[' => ']', _ => return None };
    // Balance-count to find the closing delimiter.
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;
    let mut end = 0usize;
    for (i, ch) in rest.char_indices() {
        if escape_next { escape_next = false; continue; }
        if in_string {
            if ch == '\\' { escape_next = true; }
            else if ch == '"' { in_string = false; }
            continue;
        }
        if ch == '"' { in_string = true; continue; }
        if ch == open  { depth += 1; }
        if ch == close { depth -= 1; if depth == 0 { end = i + ch.len_utf8(); break; } }
    }
    if depth != 0 { return None; }
    Some(rest[..end].into())
}
