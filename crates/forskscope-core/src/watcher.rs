//! File change monitor trait boundary (RFC-036 §"Watcher Boundary").
//!
//! Defines the stable interface between ForskScope's session layer and any
//! platform file-watcher backend. The trait is intentionally thin:
//!
//! - `watch(path)` registers a path and returns an opaque `WatchToken`.
//! - `poll_events()` returns any pending change events since the last poll.
//! - `unwatch(token)` deregisters a path.
//!
//! ## Design (RFC-036 §"Watcher Boundary")
//!
//! The watcher is an **optimization layer only**. Save safety never relies
//! solely on watcher events; `check_external_state` in `document.rs`
//! always re-reads the filesystem before a save. The watcher's job is to
//! trigger a timely `check_external_state` call so the user is warned
//! promptly rather than only at save time.
//!
//! ## Implementations
//!
//! - [`MockFileChangeMonitor`] — test-only, driven by explicit `inject_event`
//!   calls. No real filesystem interaction.
//! - A real `notify`-backed implementation lives in a platform crate (not
//!   yet implemented; blocked on platform CI). It implements this trait.
//!
//! ## Platform notes (RFC-036 §"Platform Considerations")
//!
//! - Linux: inotify/fanotify; network FS and container mounts may not
//!   deliver events reliably.
//! - Windows: may report rename/replace sequences differently from
//!   Linux; `ReplacedOnDisk` detection needs platform CI.
//! - macOS: `kqueue` / FSEvents may coalesce events.
//!
//! Consumers must never assume events are exhaustive; always verify with
//! `check_external_state` before any destructive action.

use std::path::{Path, PathBuf};

// ── Watch token ───────────────────────────────────────────────────────────────

/// An opaque handle returned by [`FileChangeMonitor::watch`].
///
/// Pass back to [`FileChangeMonitor::unwatch`] to stop watching a path.
/// The `u64` value is implementation-defined; do not rely on its meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WatchToken(pub u64);

// ── Watch error ───────────────────────────────────────────────────────────────

/// Errors that can occur when registering a path for watching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchError {
    /// The path does not exist.
    PathNotFound(PathBuf),
    /// The watcher backend is unavailable (e.g. inotify limit reached).
    BackendUnavailable(String),
    /// The path is already being watched by this monitor.
    AlreadyWatched(PathBuf),
    /// An unclassified error.
    Other(String),
}

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathNotFound(p)        => write!(f, "path not found: {}", p.display()),
            Self::BackendUnavailable(m)  => write!(f, "watcher backend unavailable: {m}"),
            Self::AlreadyWatched(p)      => write!(f, "already watching: {}", p.display()),
            Self::Other(m)               => write!(f, "watch error: {m}"),
        }
    }
}

impl std::error::Error for WatchError {}

// ── File change event ─────────────────────────────────────────────────────────

/// The kind of filesystem change observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    /// File content or metadata was modified.
    Modified,
    /// File was deleted.
    Deleted,
    /// A file appeared at a path that was previously deleted (or is new).
    Created,
    /// File was renamed or moved (path is the new path when known).
    Renamed,
    /// The event kind is not known or not representable.
    Unknown,
}

/// One file change event emitted by a [`FileChangeMonitor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChangeEvent {
    pub token: WatchToken,
    pub path:  PathBuf,
    pub kind:  FileChangeKind,
}

impl FileChangeEvent {
    pub fn new(token: WatchToken, path: PathBuf, kind: FileChangeKind) -> Self {
        Self { token, path, kind }
    }
}

// ── FileChangeMonitor trait ───────────────────────────────────────────────────

/// Trait for platform file-watcher backends (RFC-036 §"Watcher Boundary").
///
/// Consumers call `watch` to register paths, then poll `poll_events` on a
/// timer or after user interaction to drain the event queue.
///
/// The watcher must never be the *sole* source of truth for save safety.
/// Always validate with `check_external_state` before acting on events.
pub trait FileChangeMonitor: Send {
    /// Register `path` for change monitoring. Returns a `WatchToken` that
    /// identifies this registration. The same path can be registered
    /// multiple times (each call returns a distinct token).
    fn watch(&mut self, path: &Path) -> Result<WatchToken, WatchError>;

    /// Drain all pending events since the last call. Returns an empty `Vec`
    /// when no changes have occurred. Never blocks.
    fn poll_events(&mut self) -> Vec<FileChangeEvent>;

    /// Stop monitoring the path identified by `token`. No-op when the
    /// token is unknown (already unwatched or was never registered).
    fn unwatch(&mut self, token: WatchToken);

    /// `true` when the monitor is active and capable of delivering events.
    /// A monitor that failed to initialise or lost its backend returns `false`.
    fn is_active(&self) -> bool;
}

// ── MockFileChangeMonitor ─────────────────────────────────────────────────────

/// Test-only file change monitor driven by explicit `inject_event` calls.
///
/// No real filesystem interaction. Use in unit tests to verify that session
/// and UI code reacts correctly to watcher events without needing a real
/// platform watcher.
///
/// ```
/// use std::path::PathBuf;
/// use forskscope_core::watcher::{
///     FileChangeKind, FileChangeMonitor, MockFileChangeMonitor, WatchToken,
/// };
///
/// let mut monitor = MockFileChangeMonitor::new();
/// let token = monitor.watch(&PathBuf::from("/tmp/test.rs")).unwrap();
/// monitor.inject_event(token, PathBuf::from("/tmp/test.rs"), FileChangeKind::Modified);
/// let events = monitor.poll_events();
/// assert_eq!(events.len(), 1);
/// assert_eq!(events[0].kind, FileChangeKind::Modified);
/// ```
#[derive(Debug, Default)]
pub struct MockFileChangeMonitor {
    next_token: u64,
    watched:    Vec<(WatchToken, PathBuf)>,
    pending:    Vec<FileChangeEvent>,
    active:     bool,
}

impl MockFileChangeMonitor {
    pub fn new() -> Self {
        Self { active: true, ..Default::default() }
    }

    /// Inject a synthetic event into the pending queue.
    ///
    /// The `token` should be one returned by a previous `watch` call on
    /// this monitor, but the mock does not enforce this — any token value
    /// is accepted, which allows testing error paths.
    pub fn inject_event(&mut self, token: WatchToken, path: PathBuf, kind: FileChangeKind) {
        self.pending.push(FileChangeEvent::new(token, path, kind));
    }

    /// Set the monitor's active state (default: `true`).
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// The list of currently watched paths.
    pub fn watched_paths(&self) -> Vec<&PathBuf> {
        self.watched.iter().map(|(_, p)| p).collect()
    }
}

impl FileChangeMonitor for MockFileChangeMonitor {
    fn watch(&mut self, path: &Path) -> Result<WatchToken, WatchError> {
        if !self.active {
            return Err(WatchError::BackendUnavailable("mock is inactive".into()));
        }
        let token = WatchToken(self.next_token);
        self.next_token += 1;
        self.watched.push((token, path.to_path_buf()));
        Ok(token)
    }

    fn poll_events(&mut self) -> Vec<FileChangeEvent> {
        std::mem::take(&mut self.pending)
    }

    fn unwatch(&mut self, token: WatchToken) {
        self.watched.retain(|(t, _)| *t != token);
    }

    fn is_active(&self) -> bool {
        self.active
    }
}
