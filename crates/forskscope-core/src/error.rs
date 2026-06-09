//! Error model (RFC-001 §6.5).
//!
//! No core operation panics for normal user-facing failures. Every error
//! carries enough context (operation, path) for the UI to render a
//! human-readable message without string parsing.

use std::fmt;
use std::path::PathBuf;

/// Result alias used across the core crate.
pub type Result<T> = std::result::Result<T, CoreError>;

/// The filesystem operation during which an I/O error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOperation {
    Read,
    Write,
    Rename,
    Copy,
    Metadata,
    ListDir,
    CreateBackup,
}

impl fmt::Display for IoOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Rename => "rename",
            Self::Copy => "copy",
            Self::Metadata => "metadata",
            Self::ListDir => "list directory",
            Self::CreateBackup => "create backup",
        };
        f.write_str(s)
    }
}

/// Canonical core error taxonomy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// The supplied path is malformed or violates path policy.
    InvalidPath { path: String, reason: String },
    /// A filesystem operation failed.
    Io {
        path: Option<PathBuf>,
        operation: IoOperation,
        message: String,
    },
    /// Text decoding failed or produced unusable content.
    Decode {
        path: Option<PathBuf>,
        message: String,
    },
    /// The requested operation is not supported for this input.
    Unsupported { message: String },
    /// A safety conflict, e.g. the target file changed on disk after load.
    Conflict { message: String },
    /// An internal invariant was violated; indicates a bug, not user error.
    InternalInvariant { message: String },
}

impl CoreError {
    pub(crate) fn io(path: impl Into<PathBuf>, operation: IoOperation, err: &std::io::Error) -> Self {
        Self::Io {
            path: Some(path.into()),
            operation,
            message: err.to_string(),
        }
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath { path, reason } => write!(f, "invalid path `{path}`: {reason}"),
            Self::Io {
                path,
                operation,
                message,
            } => match path {
                Some(p) => write!(f, "{operation} failed for `{}`: {message}", p.display()),
                None => write!(f, "{operation} failed: {message}"),
            },
            Self::Decode { path, message } => match path {
                Some(p) => write!(f, "decode failed for `{}`: {message}", p.display()),
                None => write!(f, "decode failed: {message}"),
            },
            Self::Unsupported { message } => write!(f, "unsupported: {message}"),
            Self::Conflict { message } => write!(f, "conflict: {message}"),
            Self::InternalInvariant { message } => write!(f, "internal invariant: {message}"),
        }
    }
}

impl std::error::Error for CoreError {}
