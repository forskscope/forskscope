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
use std::time::{SystemTime, UNIX_EPOCH};

use crate::persist::{MigrationPolicy, SchemaName, VersionedEnvelope};

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
        let secs = unix_now();
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
    pub fn now() -> Self { Self(unix_now()) }
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

// ── Session ───────────────────────────────────────────────────────────────────

/// The canonical workspace session (RFC-011 §6).
///
/// Lives outside Dioxus component local state; Dioxus signals are a
/// projection. `SessionId` and `TabId` are stable across redraws.
#[derive(Debug, Clone)]
pub struct WorkspaceSession {
    pub session_id:    SessionId,
    pub created_at:    Timestamp,
    pub updated_at:    Timestamp,
    pub root:          WorkspaceRoot,
    pub tabs:          Vec<WorkspaceTab>,
    pub active_tab_id: Option<TabId>,
}

impl WorkspaceSession {
    // ── Constructors ──────────────────────────────────────────────────────

    /// Empty session — used on empty startup (RFC-011 §5.1).
    pub fn empty() -> Self {
        let now = Timestamp::now();
        Self {
            session_id:    SessionId::new(),
            created_at:    now,
            updated_at:    now,
            root:          WorkspaceRoot::Empty,
            tabs:          vec![],
            active_tab_id: None,
        }
    }

    /// Session initialised from two file paths (RFC-011 §5.2).
    pub fn from_file_pair(left: PathBuf, right: PathBuf) -> Self {
        let tab = WorkspaceTab::Diff(DiffTabSession {
            tab_id:     TabId::new(),
            left_path:  left.clone(),
            right_path: right.clone(),
            is_dirty:   false,
        });
        let tab_id = tab.tab_id().clone();
        let now = Timestamp::now();
        Self {
            session_id:    SessionId::new(),
            created_at:    now,
            updated_at:    now,
            root:          WorkspaceRoot::FilePair(FilePairRoot { left, right }),
            tabs:          vec![tab],
            active_tab_id: Some(tab_id),
        }
    }

    /// Session initialised from two directory paths (RFC-011 §5.3).
    pub fn from_directory_pair(left: PathBuf, right: PathBuf) -> Self {
        let now = Timestamp::now();
        Self {
            session_id:    SessionId::new(),
            created_at:    now,
            updated_at:    now,
            root:          WorkspaceRoot::DirectoryPair(DirectoryPairRoot { left, right }),
            tabs:          vec![],
            active_tab_id: None,
        }
    }

    // ── Tab management ────────────────────────────────────────────────────

    /// Add a tab and make it active.
    pub fn open_tab(&mut self, tab: WorkspaceTab) {
        let id = tab.tab_id().clone();
        self.tabs.push(tab);
        self.active_tab_id = Some(id);
        self.touch();
    }

    /// Close a tab by id. Returns `CloseResult` so the caller can decide
    /// whether to show the unsaved-changes dialog (RFC-011 §5.4).
    pub fn close_tab(&mut self, id: &TabId) -> CloseResult {
        if let Some(pos) = self.tabs.iter().position(|t| t.tab_id() == id) {
            if self.tabs[pos].is_dirty() {
                return CloseResult::BlockedDirty;
            }
            self.tabs.remove(pos);
            // Advance active tab to the previous one, or None.
            if self.active_tab_id.as_ref() == Some(id) {
                self.active_tab_id = if pos > 0 {
                    self.tabs.get(pos - 1).map(|t| t.tab_id().clone())
                } else {
                    self.tabs.first().map(|t| t.tab_id().clone())
                };
            }
            self.touch();
            CloseResult::Closed
        } else {
            CloseResult::NotFound
        }
    }

    /// Force-close a tab (user confirmed discard). No dirty check.
    pub fn force_close_tab(&mut self, id: &TabId) {
        if let Some(pos) = self.tabs.iter().position(|t| t.tab_id() == id) {
            self.tabs.remove(pos);
            if self.active_tab_id.as_ref() == Some(id) {
                self.active_tab_id = if pos > 0 {
                    self.tabs.get(pos - 1).map(|t| t.tab_id().clone())
                } else {
                    self.tabs.first().map(|t| t.tab_id().clone())
                };
            }
            self.touch();
        }
    }

    /// Mark a tab dirty (e.g. after the first merge edit).
    pub fn mark_tab_dirty(&mut self, id: &TabId) -> bool {
        if let Some(t) = self.tabs.iter_mut().find(|t| t.tab_id() == id) {
            t.mark_dirty();
            self.touch();
            true
        } else { false }
    }

    /// Mark a tab clean after a successful save.
    pub fn mark_tab_clean(&mut self, id: &TabId) -> bool {
        if let Some(t) = self.tabs.iter_mut().find(|t| t.tab_id() == id) {
            t.mark_clean();
            self.touch();
            true
        } else { false }
    }

    // ── Queries ───────────────────────────────────────────────────────────

    /// `true` when any tab has unsaved changes.
    pub fn any_dirty(&self) -> bool { self.tabs.iter().any(|t| t.is_dirty()) }

    /// All dirty tabs.
    pub fn dirty_tabs(&self) -> Vec<&WorkspaceTab> {
        self.tabs.iter().filter(|t| t.is_dirty()).collect()
    }

    /// The active tab, if any.
    pub fn active_tab(&self) -> Option<&WorkspaceTab> {
        self.active_tab_id.as_ref().and_then(|id|
            self.tabs.iter().find(|t| t.tab_id() == id))
    }

    // ── Serialisation ─────────────────────────────────────────────────────

    /// Serialise to a `VersionedEnvelope` JSON string (RFC-011 §8,
    /// RFC-031 §"Schema versioning").
    pub fn to_json(&self) -> String {
        let payload = self.to_payload_json();
        VersionedEnvelope::new(SchemaName::Session, SESSION_SCHEMA_VERSION, payload)
            .to_json()
    }

    /// Parse from envelope JSON. Returns `Err` on any structural failure.
    /// Callers should check `ParsedSession::migration` for schema decisions.
    pub fn from_json(json: &str) -> Result<ParsedSession, SessionParseError> {
        let envelope = VersionedEnvelope::parse(json)
            .map_err(|e| SessionParseError::EnvelopeError(e.to_string()))?;
        let migration = envelope.migration_policy(SESSION_SCHEMA_VERSION);
        if let MigrationPolicy::NewerSchema { file_version, .. } = migration {
            return Err(SessionParseError::TooNew { version: file_version });
        }
        let session = Self::from_payload_json(&envelope.payload_json)
            .map_err(SessionParseError::PayloadError)?;
        Ok(ParsedSession { session, migration })
    }

    // ── Internal JSON helpers (hand-written, no serde) ────────────────────

    fn to_payload_json(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(s, "{{");
        let _ = writeln!(s, "  \"session_id\": {:?},",  self.session_id.0);
        let _ = writeln!(s, "  \"created_at\": {},",    self.created_at.0);
        let _ = writeln!(s, "  \"updated_at\": {},",    self.updated_at.0);
        let _ = writeln!(s, "  \"root_kind\": {:?},",   root_kind_str(&self.root));
        match &self.root {
            WorkspaceRoot::Empty => {}
            WorkspaceRoot::FilePair(p) => {
                let _ = writeln!(s, "  \"left_root\": {:?},",  p.left.display().to_string());
                let _ = writeln!(s, "  \"right_root\": {:?},", p.right.display().to_string());
            }
            WorkspaceRoot::DirectoryPair(p) => {
                let _ = writeln!(s, "  \"left_root\": {:?},",  p.left.display().to_string());
                let _ = writeln!(s, "  \"right_root\": {:?},", p.right.display().to_string());
            }
        }
        let _ = writeln!(s, "  \"active_tab_id\": {},",
            self.active_tab_id.as_ref().map_or("null".into(),
                |id| format!("{:?}", id.0)));
        let _ = writeln!(s, "  \"tabs\": [");
        for (i, tab) in self.tabs.iter().enumerate() {
            let comma = if i + 1 < self.tabs.len() { "," } else { "" };
            let _ = writeln!(s, "    {}{}", tab_to_json(tab), comma);
        }
        let _ = writeln!(s, "  ]");
        let _ = write!(s, "}}");
        s
    }

    fn from_payload_json(json: &str) -> Result<Self, String> {
        let session_id = extract_str(json, "session_id")
            .ok_or("missing session_id")?;
        let created_at = extract_u64(json, "created_at")
            .ok_or("missing created_at")?;
        let updated_at = extract_u64(json, "updated_at")
            .ok_or("missing updated_at")?;
        let root_kind  = extract_str(json, "root_kind")
            .ok_or("missing root_kind")?;
        let left_root  = extract_str(json, "left_root");
        let right_root = extract_str(json, "right_root");

        let root = match root_kind.as_str() {
            "empty" => WorkspaceRoot::Empty,
            "file_pair" => WorkspaceRoot::FilePair(FilePairRoot {
                left:  PathBuf::from(left_root.ok_or("missing left_root")?),
                right: PathBuf::from(right_root.ok_or("missing right_root")?),
            }),
            "directory_pair" => WorkspaceRoot::DirectoryPair(DirectoryPairRoot {
                left:  PathBuf::from(left_root.ok_or("missing left_root")?),
                right: PathBuf::from(right_root.ok_or("missing right_root")?),
            }),
            other => return Err(format!("unknown root_kind: {other}")),
        };

        let active_tab_id = extract_str(json, "active_tab_id").map(TabId);

        Ok(Self {
            session_id:    SessionId(session_id),
            created_at:    Timestamp(created_at),
            updated_at:    Timestamp(updated_at),
            root,
            tabs:          vec![], // tab list restoration is deferred to v2
            active_tab_id,
        })
    }

    fn touch(&mut self) { self.updated_at = Timestamp::now(); }
}

// ── CloseResult ───────────────────────────────────────────────────────────────

/// Result of a [`WorkspaceSession::close_tab`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseResult {
    /// Tab was closed cleanly.
    Closed,
    /// Tab was not closed because it has unsaved changes.
    /// The UI must show the unsaved-changes dialog (RFC-011 §5.4).
    BlockedDirty,
    /// No tab with that id exists.
    NotFound,
}

// ── ParsedSession ─────────────────────────────────────────────────────────────

/// Result of [`WorkspaceSession::from_json`].
pub struct ParsedSession {
    pub session:   WorkspaceSession,
    pub migration: MigrationPolicy,
}

// ── SessionParseError ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionParseError {
    EnvelopeError(String),
    PayloadError(String),
    /// The file was written by a newer ForskScope; the app must not overwrite it.
    TooNew { version: u32 },
}

impl std::fmt::Display for SessionParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnvelopeError(e) => write!(f, "session envelope error: {e}"),
            Self::PayloadError(e)  => write!(f, "session payload error: {e}"),
            Self::TooNew { version } => write!(f,
                "session was written by a newer ForskScope (schema v{version}); upgrade the app"),
        }
    }
}

impl std::error::Error for SessionParseError {}

// ── Recent session entry ──────────────────────────────────────────────────────

/// An entry in the recent-sessions list (RFC-011 §9).
///
/// Stores only metadata and paths — never file contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentSessionEntry {
    pub session_id:     SessionId,
    pub title:          String,
    pub left_path:      PathBuf,
    pub right_path:     PathBuf,
    pub kind:           RecentKind,
    pub last_opened_at: Timestamp,
}

/// Whether the recent entry was a file-pair or directory-pair session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecentKind { FilePair, DirectoryPair }

impl RecentSessionEntry {
    /// `true` when both paths still exist on disk.
    pub fn paths_available(&self) -> bool {
        self.left_path.exists() && self.right_path.exists()
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn root_kind_str(root: &WorkspaceRoot) -> &'static str {
    match root {
        WorkspaceRoot::Empty          => "empty",
        WorkspaceRoot::FilePair(_)    => "file_pair",
        WorkspaceRoot::DirectoryPair(_) => "directory_pair",
    }
}

fn tab_to_json(tab: &WorkspaceTab) -> String {
    match tab {
        WorkspaceTab::Diff(t) => format!(
            "{{\"kind\":\"diff\",\"tab_id\":{:?},\"left\":{:?},\"right\":{:?},\"dirty\":{}}}",
            t.tab_id.0,
            t.left_path.display().to_string(),
            t.right_path.display().to_string(),
            t.is_dirty,
        ),
        WorkspaceTab::Binary(t) => format!(
            "{{\"kind\":\"binary\",\"tab_id\":{:?},\"left\":{:?},\"right\":{:?}}}",
            t.tab_id.0,
            t.left_path.display().to_string(),
            t.right_path.display().to_string(),
        ),
        WorkspaceTab::Excel(t) => format!(
            "{{\"kind\":\"excel\",\"tab_id\":{:?},\"left\":{:?},\"right\":{:?}}}",
            t.tab_id.0,
            t.left_path.display().to_string(),
            t.right_path.display().to_string(),
        ),
        WorkspaceTab::Error(t) => format!(
            "{{\"kind\":\"error\",\"tab_id\":{:?},\"message\":{:?}}}",
            t.tab_id.0, t.message,
        ),
    }
}

/// Extract a JSON string field value (simple, no escape handling for tabs).
fn extract_str(json: &str, field: &str) -> Option<String> {
    let key = format!("\"{}\":", field);
    let start = json.find(&key)? + key.len();
    let rest = json[start..].trim_start();
    if rest.starts_with("null") { return None; }
    if !rest.starts_with('"') { return None; }
    let inner = &rest[1..];
    let end = inner.find('"')?;
    Some(inner[..end].into())
}

fn extract_u64(json: &str, field: &str) -> Option<u64> {
    let key = format!("\"{}\":", field);
    let start = json.find(&key)? + key.len();
    let rest = json[start..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}
