//! Error model (RFC-001 §6.5, RFC-017 §"Error Severity and Recovery").
//!
//! No core operation panics for normal user-facing failures. Every error
//! carries enough context (operation, path) for the UI to render a
//! human-readable message without string parsing.
//!
//! `CoreError` exposes two query methods — [`severity`](CoreError::severity)
//! and [`recovery_hint`](CoreError::recovery_hint) — so the UI can decide
//! whether to show a toast, an inline warning, or a blocking modal, and
//! which recovery actions to offer, without pattern-matching on message
//! strings.

pub mod app;
pub use app::{AppError, AppErrorKind, ErrorId, RecoveryAction, TechnicalDetail, UserMessage};

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

// ── RFC-017 §"Error Severity" ─────────────────────────────────────────────────

/// How severe an error is, used by the UI to choose the appropriate surface
/// (RFC-017 §"Error Severity").
///
/// Ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// The operation completed but with a note the user should see.
    /// Surface: status bar or toast.
    Info,
    /// The operation can continue but the user should be aware.
    /// Surface: toast or inline warning banner.
    Warning,
    /// A user action can resolve this. The operation cannot proceed.
    /// Surface: dialog with labelled action buttons.
    Recoverable,
    /// The operation cannot proceed and no simple recovery is available.
    /// Surface: blocking modal or error tab.
    Blocking,
}

// ── RFC-017 §"Recovery Actions" ───────────────────────────────────────────────

/// A predefined recovery action the UI can offer when this error occurs
/// (RFC-017 §"Recovery Actions"). The UI decides which actions to show
/// based on context; not all hints are applicable in every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryHint {
    /// Ask the user to choose a different file or directory.
    ChooseAnotherFile,
    /// Offer to reload the file from disk (e.g. after external change).
    Reload,
    /// Offer a Save As dialog rather than overwriting.
    SaveAs,
    /// Offer to overwrite despite the conflict (after explicit confirmation).
    OverwriteAnyway,
    /// Suggest the user check filesystem permissions.
    CheckPermissions,
    /// No specific action — inform and close.
    Dismiss,
    /// Report a bug; the error should not have occurred.
    ReportBug,
}

// ── Canonical core error taxonomy ────────────────────────────────────────────

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

    /// The severity level of this error (RFC-017 §"Error Severity").
    /// The UI uses this to choose a toast, inline warning, or blocking modal.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Conflict { .. } => ErrorSeverity::Recoverable,
            Self::Io { operation, .. } => match operation {
                IoOperation::Read | IoOperation::ListDir | IoOperation::Metadata => {
                    ErrorSeverity::Recoverable
                }
                IoOperation::Write
                | IoOperation::Rename
                | IoOperation::Copy
                | IoOperation::CreateBackup => ErrorSeverity::Blocking,
            },
            Self::InvalidPath { .. } => ErrorSeverity::Recoverable,
            Self::Decode { .. } => ErrorSeverity::Warning,
            Self::Unsupported { .. } => ErrorSeverity::Warning,
            Self::InternalInvariant { .. } => ErrorSeverity::Blocking,
        }
    }

    /// A suggested recovery action for this error (RFC-017 §"Recovery Actions").
    /// The UI may offer additional context-specific actions alongside this hint.
    pub fn recovery_hint(&self) -> RecoveryHint {
        match self {
            Self::Conflict { .. } => RecoveryHint::Reload,
            Self::Io { operation, .. } => match operation {
                IoOperation::Read | IoOperation::Metadata => RecoveryHint::ChooseAnotherFile,
                IoOperation::ListDir => RecoveryHint::ChooseAnotherFile,
                IoOperation::Write | IoOperation::Rename => RecoveryHint::SaveAs,
                IoOperation::Copy => RecoveryHint::CheckPermissions,
                IoOperation::CreateBackup => RecoveryHint::CheckPermissions,
            },
            Self::InvalidPath { .. } => RecoveryHint::ChooseAnotherFile,
            Self::Decode { .. } => RecoveryHint::Dismiss,
            Self::Unsupported { .. } => RecoveryHint::ChooseAnotherFile,
            Self::InternalInvariant { .. } => RecoveryHint::ReportBug,
        }
    }

    /// `true` when this error indicates a user-recoverable situation rather
    /// than an internal fault. Convenience for branch-free UI decisions.
    pub fn is_user_recoverable(&self) -> bool {
        self.severity() <= ErrorSeverity::Recoverable
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


