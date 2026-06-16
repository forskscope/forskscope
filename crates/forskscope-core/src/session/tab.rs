//! Per-tab session types: `WorkspaceTab`, `DiffTabSession`, etc. (RFC-011).

//! Workspace session model (RFC-011).
//!
//! Defines the types that represent the user's current workspace — which
//! files are open, which tab is active, and what state can be safely
//! persisted across restarts. Session truth lives here, in core; Dioxus
//! signals are a projection of this model, not its source.
//!
//! ## Restorable vs transient state
//!
//! Only restorable state (root paths, tab path pairs, active tab, dirty
//! summary) is stored in JSON via [`VersionedEnvelope`]. Transient state
//! (modal-open, toast messages, raw editor DOM, background job handles) is
//! never persisted (RFC-011 §7.2).
//!
//! ## Serialisation
//!
//! [`WorkspaceSession::to_json`] wraps the session in a
//! [`VersionedEnvelope`] with `SchemaName::Session`. Callers write the
//! result to the config directory. [`WorkspaceSession::from_json`] parses
//! it back, applying the migration policy check.

use std::path::PathBuf;


// ── Current session schema version ───────────────────────────────────────────

/// The schema version understood by this build of ForskScope.
/// Increment when a breaking change is made to the JSON layout.
pub const SESSION_SCHEMA_VERSION: u32 = 1;

// ── Identity types ────────────────────────────────────────────────────────────

/// Stable identifier for a session. Generated once at creation; survives
/// redraws and restores.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        let secs = super::unix_now();
        let pid  = std::process::id();
        Self(format!("sess-{secs}-{pid}"))
    }
}

impl Default for SessionId {
    fn default() -> Self { Self::new() }
}

/// Stable identifier for one diff or compare tab. Stable across redraws
/// (RFC-011 §12).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TabId(pub String);

impl TabId {
    pub fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicU64
            = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(format!("tab-{}", n))
    }
}

impl Default for TabId {
    fn default() -> Self { Self::new() }
}

/// A Unix-epoch timestamp in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn now() -> Self { Self(super::unix_now()) }
}

// ── Workspace root ────────────────────────────────────────────────────────────

/// A single compare path entry for a file-pair root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePairRoot {
    pub left:  PathBuf,
    pub right: PathBuf,
}

/// Directory roots for an explorer-based session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryPairRoot {
    pub left:  PathBuf,
    pub right: PathBuf,
}

/// The top-level context for the workspace (RFC-011 §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceRoot {
    /// No paths chosen yet (empty startup).
    Empty,
    /// Opened from two file arguments or the file-pair picker.
    FilePair(FilePairRoot),
    /// Opened from two directory arguments or the directory picker.
    DirectoryPair(DirectoryPairRoot),
}

impl WorkspaceRoot {
    /// `true` when this root was opened from two files.
    pub fn is_file_pair(&self) -> bool { matches!(self, Self::FilePair(_)) }
    /// `true` when this root was opened from two directories.
    pub fn is_directory_pair(&self) -> bool { matches!(self, Self::DirectoryPair(_)) }
}

// ── Workspace tabs ────────────────────────────────────────────────────────────

/// A text/line diff compare tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffTabSession {
    pub tab_id:     TabId,
    pub left_path:  PathBuf,
    pub right_path: PathBuf,
    pub is_dirty:   bool,
}

/// A binary-only compare tab (no merge actions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryTabSession {
    pub tab_id:     TabId,
    pub left_path:  PathBuf,
    pub right_path: PathBuf,
}

/// An Excel (.xlsx) compare tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcelTabSession {
    pub tab_id:     TabId,
    pub left_path:  PathBuf,
    pub right_path: PathBuf,
}

/// A tab showing a load or comparison error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorTabSession {
    pub tab_id:  TabId,
    pub message: String,
}

/// One tab in the workspace (RFC-011 §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceTab {
    Diff(DiffTabSession),
    Binary(BinaryTabSession),
    Excel(ExcelTabSession),
    Error(ErrorTabSession),
}

impl WorkspaceTab {
    pub fn tab_id(&self) -> &TabId {
        match self {
            Self::Diff(t)   => &t.tab_id,
            Self::Binary(t) => &t.tab_id,
            Self::Excel(t)  => &t.tab_id,
            Self::Error(t)  => &t.tab_id,
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self { Self::Diff(t) => t.is_dirty, _ => false }
    }

    /// Mark a `Diff` tab as dirty. No-op for other tab kinds.
    pub fn mark_dirty(&mut self) {
        if let Self::Diff(t) = self { t.is_dirty = true; }
    }

    /// Clear the dirty flag on a `Diff` tab after a successful save.
    pub fn mark_clean(&mut self) {
        if let Self::Diff(t) = self { t.is_dirty = false; }
    }
}

