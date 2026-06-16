//! App-layer error types: `AppErrorKind`, `RecoveryAction`, `UserMessage` (RFC-017).

use super::{CoreError, ErrorSeverity, IoOperation};

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

// ── AppError — structured error envelope (RFC-017 §5) ────────────────────────

/// Technical detail for diagnostics (shown in the copy-diagnostics panel,
/// never in the main dialog body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnicalDetail {
    /// Short machine-readable code, e.g. `"io::permission_denied"`.
    pub code: String,
    /// Full detail text including paths and OS messages.
    pub detail: String,
}

impl TechnicalDetail {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { code: code.into(), detail: detail.into() }
    }
}

/// Stable unique identifier for one error instance (for log correlation).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorId(pub String);

impl ErrorId {
    pub fn new() -> Self {
        // Millisecond timestamp + pid for reasonable uniqueness without uuid.
        use std::time::{SystemTime, UNIX_EPOCH};
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        Self(format!("err-{ms}-{}", std::process::id()))
    }
}

impl Default for ErrorId {
    fn default() -> Self { Self::new() }
}

/// The complete structured error envelope presented to the UI (RFC-017 §5).
///
/// Constructed via [`AppError::from_core`] or [`AppError::new`].
/// The UI reads `severity` to choose the surface (toast vs dialog vs banner),
/// `recovery` to render action buttons, and `message` for the dialog copy.
/// `technical` is only shown in the copy-diagnostics panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    pub error_id:  ErrorId,
    pub kind:      AppErrorKind,
    pub severity:  ErrorSeverity,
    pub message:   UserMessage,
    pub technical: TechnicalDetail,
    pub recovery:  Vec<RecoveryAction>,
}

impl AppError {
    /// Build an `AppError` from a `CoreError` using the standard mappings.
    pub fn from_core(err: &CoreError) -> Self {
        let kind      = AppErrorKind::from_core(err);
        let severity  = kind.default_severity();
        let message   = UserMessage::for_kind(kind);
        let recovery  = kind.default_recovery_actions().to_vec();
        let technical = TechnicalDetail::new(
            format!("{kind:?}").to_lowercase().replace(' ', "_"),
            err.to_string(),
        );
        Self {
            error_id: ErrorId::new(),
            kind,
            severity,
            message,
            technical,
            recovery,
        }
    }

    /// Build an `AppError` from explicit components when the kind is known
    /// directly (e.g. from application-layer code that doesn't go through
    /// `CoreError`).
    pub fn new(kind: AppErrorKind, technical_detail: impl Into<String>) -> Self {
        let severity  = kind.default_severity();
        let message   = UserMessage::for_kind(kind);
        let recovery  = kind.default_recovery_actions().to_vec();
        Self {
            error_id:  ErrorId::new(),
            kind,
            severity,
            message,
            technical: TechnicalDetail::new(
                format!("{kind:?}").to_lowercase(),
                technical_detail.into(),
            ),
            recovery,
        }
    }

    /// `true` when this error should block the user from proceeding.
    pub fn is_blocking(&self) -> bool {
        self.severity >= ErrorSeverity::Blocking
    }

    /// `true` when the user can take an action to recover.
    pub fn is_recoverable(&self) -> bool {
        !self.recovery.is_empty()
            && self.recovery.iter().any(|r| *r != RecoveryAction::Dismiss)
    }
}
