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


// ── RFC-017: AppErrorKind, RecoveryAction, UserMessage ────────────────────────

/// The full taxonomy of user-facing error situations (RFC-017 §5).
///
/// Maps `CoreError` into UI-presentable categories that drive error dialog
/// copy, recovery button labels, and help links without string parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppErrorKind {
    // ── Path / filesystem ─────────────────────────────────────────────────
    PathNotFound,
    PathNotFile,
    PathNotDirectory,
    PermissionDenied,
    SymlinkUnsupported,
    FileReadFailed,
    FileWriteFailed,
    // ── Encoding ─────────────────────────────────────────────────────────
    EncodingDetectionFailed,
    DecodeLossy,
    // ── Comparison ────────────────────────────────────────────────────────
    BinaryNotComparable,
    FileTooLarge,
    DiffFailed,
    InlineDiffTooLarge,
    // ── Merge / save ──────────────────────────────────────────────────────
    SaveConflict,
    ExternalModificationDetected,
    BackupFailed,
    // ── Background jobs ───────────────────────────────────────────────────
    BackgroundJobFailed,
    BackgroundJobCancelled,
    // ── Session ───────────────────────────────────────────────────────────
    SessionTooNew,
    SessionCorrupted,
    // ── VCS ───────────────────────────────────────────────────────────────
    VcsUnavailable,
    VcsCommandFailed,
    // ── Spreadsheet ───────────────────────────────────────────────────────
    SpreadsheetReadFailed,
    EncryptedWorkbook,
    // ── Internal ─────────────────────────────────────────────────────────
    InternalFault,
}

impl AppErrorKind {
    /// The default severity for this kind.
    pub fn default_severity(self) -> ErrorSeverity {
        match self {
            Self::PathNotFound
            | Self::PathNotFile
            | Self::PathNotDirectory
            | Self::PermissionDenied
            | Self::SymlinkUnsupported
            | Self::FileReadFailed
            | Self::BinaryNotComparable
            | Self::FileTooLarge
            | Self::DiffFailed
            | Self::SessionTooNew
            | Self::SessionCorrupted
            | Self::SpreadsheetReadFailed
            | Self::EncryptedWorkbook
            | Self::VcsUnavailable
            | Self::VcsCommandFailed => ErrorSeverity::Recoverable,

            Self::DecodeLossy
            | Self::EncodingDetectionFailed
            | Self::InlineDiffTooLarge
            | Self::BackgroundJobCancelled => ErrorSeverity::Warning,

            Self::FileWriteFailed
            | Self::SaveConflict
            | Self::BackupFailed
            | Self::ExternalModificationDetected
            | Self::BackgroundJobFailed => ErrorSeverity::Blocking,

            Self::InternalFault => ErrorSeverity::Blocking,
        }
    }

    /// The default recovery actions the UI should offer for this kind.
    pub fn default_recovery_actions(self) -> &'static [RecoveryAction] {
        match self {
            Self::PathNotFound
            | Self::PathNotFile
            | Self::PathNotDirectory
            | Self::PermissionDenied
            | Self::SymlinkUnsupported
            | Self::FileReadFailed
            | Self::BinaryNotComparable
            | Self::SpreadsheetReadFailed
            | Self::EncryptedWorkbook => &[RecoveryAction::ChooseAnotherFile, RecoveryAction::Dismiss],

            Self::EncodingDetectionFailed
            | Self::DecodeLossy => &[RecoveryAction::OpenAsBinary, RecoveryAction::Dismiss],

            Self::FileTooLarge => &[RecoveryAction::OpenLimitedDiff, RecoveryAction::OpenAsBinary, RecoveryAction::Cancel],

            Self::DiffFailed
            | Self::InlineDiffTooLarge => &[RecoveryAction::RetryWithoutInline, RecoveryAction::Dismiss],

            Self::SaveConflict
            | Self::ExternalModificationDetected => &[RecoveryAction::Reload, RecoveryAction::SaveAs, RecoveryAction::OverwriteAnyway],

            Self::FileWriteFailed
            | Self::BackupFailed => &[RecoveryAction::SaveAs, RecoveryAction::Dismiss],

            Self::BackgroundJobCancelled => &[RecoveryAction::Retry, RecoveryAction::Dismiss],
            Self::BackgroundJobFailed    => &[RecoveryAction::Retry, RecoveryAction::Dismiss],

            Self::SessionTooNew
            | Self::SessionCorrupted => &[RecoveryAction::StartFresh, RecoveryAction::Dismiss],

            Self::VcsUnavailable
            | Self::VcsCommandFailed => &[RecoveryAction::Dismiss],

            Self::InternalFault => &[RecoveryAction::ReportBug, RecoveryAction::Dismiss],
        }
    }

    /// Derive an `AppErrorKind` from a `CoreError` (best-effort mapping).
    pub fn from_core(err: &CoreError) -> Self {
        match err {
            CoreError::Io { operation, .. } => match operation {
                IoOperation::Read | IoOperation::Metadata => Self::FileReadFailed,
                IoOperation::Write | IoOperation::Rename  => Self::FileWriteFailed,
                IoOperation::Copy                         => Self::FileWriteFailed,
                IoOperation::CreateBackup                 => Self::BackupFailed,
                IoOperation::ListDir                      => Self::PathNotDirectory,
            },
            CoreError::InvalidPath { .. }      => Self::PathNotFound,
            CoreError::Decode { .. }           => Self::DecodeLossy,
            CoreError::Unsupported { .. }      => Self::BinaryNotComparable,
            CoreError::Conflict { .. }         => Self::ExternalModificationDetected,
            CoreError::InternalInvariant { .. } => Self::InternalFault,
        }
    }
}

/// A typed recovery action for an error dialog button (RFC-017 §"Recovery Actions").
///
/// More specific than [`RecoveryHint`] (which is a broad directive).
/// The UI maps each variant to a localised button label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Dismiss the error without taking action.
    Dismiss,
    /// Let the user pick a different file.
    ChooseAnotherFile,
    /// Reload the file from disk (e.g. after external change).
    Reload,
    /// Open a Save As dialog rather than overwriting.
    SaveAs,
    /// Overwrite despite the conflict, after explicit user confirmation.
    OverwriteAnyway,
    /// Open a limited diff (e.g. disable inline diff for a large file).
    OpenLimitedDiff,
    /// Open as binary / hex view.
    OpenAsBinary,
    /// Retry the operation.
    Retry,
    /// Retry without inline character diff.
    RetryWithoutInline,
    /// Cancel the current operation or dialog.
    Cancel,
    /// Open a new empty session (discard the corrupted session).
    StartFresh,
    /// File a bug report.
    ReportBug,
}

impl RecoveryAction {
    /// A stable string token for the action, suitable for keybinding or i18n.
    pub fn token(self) -> &'static str {
        match self {
            Self::Dismiss          => "dismiss",
            Self::ChooseAnotherFile => "choose_another_file",
            Self::Reload           => "reload",
            Self::SaveAs           => "save_as",
            Self::OverwriteAnyway  => "overwrite_anyway",
            Self::OpenLimitedDiff  => "open_limited_diff",
            Self::OpenAsBinary     => "open_as_binary",
            Self::Retry            => "retry",
            Self::RetryWithoutInline => "retry_without_inline",
            Self::Cancel           => "cancel",
            Self::StartFresh       => "start_fresh",
            Self::ReportBug        => "report_bug",
        }
    }

    /// Whether this action is destructive (requires extra confirmation).
    pub fn is_destructive(self) -> bool {
        matches!(self, Self::OverwriteAnyway | Self::StartFresh)
    }
}

/// A structured user-facing error message (RFC-017 §"UserMessage").
///
/// `short` fits a toast; `detail` fits a dialog body. Neither field should
/// contain internal paths by default — use [`UserMessage::with_path`] when
/// the path adds actionable context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMessage {
    /// One-line summary for a toast or dialog title.
    pub short: String,
    /// Longer explanation for the dialog body. May be empty.
    pub detail: String,
}

impl UserMessage {
    pub fn new(short: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { short: short.into(), detail: detail.into() }
    }

    pub fn short_only(short: impl Into<String>) -> Self {
        Self { short: short.into(), detail: String::new() }
    }

    /// Build a standard message for an `AppErrorKind`.
    pub fn for_kind(kind: AppErrorKind) -> Self {
        let (short, detail) = match kind {
            AppErrorKind::PathNotFound    => ("File not found", "The file may have been moved or deleted."),
            AppErrorKind::PathNotFile     => ("Not a file", "The path exists but is not a regular file."),
            AppErrorKind::PathNotDirectory => ("Not a directory", "The path exists but is not a directory."),
            AppErrorKind::PermissionDenied => ("Permission denied", "ForskScope cannot read this file. Check the file permissions."),
            AppErrorKind::SymlinkUnsupported => ("Symbolic links are not supported", "Follow the link manually and open the target file."),
            AppErrorKind::FileReadFailed  => ("Could not read file", "An I/O error occurred while reading the file."),
            AppErrorKind::FileWriteFailed => ("Could not write file", "An I/O error occurred while saving."),
            AppErrorKind::EncodingDetectionFailed => ("Encoding unknown", "The file encoding could not be detected."),
            AppErrorKind::DecodeLossy     => ("Decoding may be lossy", "Some characters could not be decoded in the detected encoding."),
            AppErrorKind::BinaryNotComparable => ("Binary file", "This file cannot be compared as text."),
            AppErrorKind::FileTooLarge    => ("File is large", "This file may take a long time to compare."),
            AppErrorKind::DiffFailed      => ("Comparison failed", "An error occurred while computing the diff."),
            AppErrorKind::InlineDiffTooLarge => ("Inline diff skipped", "This hunk is too large for character-level diff."),
            AppErrorKind::SaveConflict    => ("Save conflict", "The file was modified by another process since it was loaded."),
            AppErrorKind::ExternalModificationDetected => ("File changed on disk", "The file was modified externally. Saving now would overwrite those changes."),
            AppErrorKind::BackupFailed    => ("Backup failed", "Could not create a backup before overwriting."),
            AppErrorKind::BackgroundJobFailed => ("Background task failed", "The background comparison task encountered an error."),
            AppErrorKind::BackgroundJobCancelled => ("Task cancelled", ""),
            AppErrorKind::SessionTooNew   => ("Session from newer version", "This session was saved by a newer version of ForskScope. Upgrade the app to open it."),
            AppErrorKind::SessionCorrupted => ("Session file corrupted", "The session file could not be read. It may be from an incompatible version."),
            AppErrorKind::VcsUnavailable  => ("VCS not available", "No supported version control system was found at this path."),
            AppErrorKind::VcsCommandFailed => ("VCS error", "A version control operation failed."),
            AppErrorKind::SpreadsheetReadFailed => ("Could not read spreadsheet", "The .xlsx file could not be opened."),
            AppErrorKind::EncryptedWorkbook => ("Encrypted workbook", "This spreadsheet is password-protected and cannot be compared."),
            AppErrorKind::InternalFault   => ("Internal error", "An unexpected error occurred. Please report this."),
        };
        Self::new(short, detail)
    }
}
