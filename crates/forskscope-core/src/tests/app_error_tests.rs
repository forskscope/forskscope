//! AppErrorKind, RecoveryAction, and UserMessage tests (RFC-017 §5, §"Recovery Actions", §"UserMessage").

use crate::error::{
    AppErrorKind, CoreError, ErrorSeverity, IoOperation, RecoveryAction, UserMessage,
};

// ── AppErrorKind::default_severity ───────────────────────────────────────────

#[test]
fn path_not_found_is_recoverable() {
    assert_eq!(AppErrorKind::PathNotFound.default_severity(), ErrorSeverity::Recoverable);
}

#[test]
fn file_write_failed_is_blocking() {
    assert_eq!(AppErrorKind::FileWriteFailed.default_severity(), ErrorSeverity::Blocking);
}

#[test]
fn decode_lossy_is_warning() {
    assert_eq!(AppErrorKind::DecodeLossy.default_severity(), ErrorSeverity::Warning);
}

#[test]
fn internal_fault_is_blocking() {
    assert_eq!(AppErrorKind::InternalFault.default_severity(), ErrorSeverity::Blocking);
}

#[test]
fn background_job_cancelled_is_warning() {
    assert_eq!(AppErrorKind::BackgroundJobCancelled.default_severity(), ErrorSeverity::Warning);
}

// ── AppErrorKind::default_recovery_actions ───────────────────────────────────

#[test]
fn path_not_found_offers_choose_another_file() {
    let actions = AppErrorKind::PathNotFound.default_recovery_actions();
    assert!(actions.contains(&RecoveryAction::ChooseAnotherFile),
        "PathNotFound must offer ChooseAnotherFile");
}

#[test]
fn save_conflict_offers_reload_and_save_as() {
    let actions = AppErrorKind::SaveConflict.default_recovery_actions();
    assert!(actions.contains(&RecoveryAction::Reload));
    assert!(actions.contains(&RecoveryAction::SaveAs));
}

#[test]
fn external_modification_offers_overwrite_anyway() {
    let actions = AppErrorKind::ExternalModificationDetected.default_recovery_actions();
    assert!(actions.contains(&RecoveryAction::OverwriteAnyway));
}

#[test]
fn file_too_large_offers_open_limited_diff_and_cancel() {
    let actions = AppErrorKind::FileTooLarge.default_recovery_actions();
    assert!(actions.contains(&RecoveryAction::OpenLimitedDiff));
    assert!(actions.contains(&RecoveryAction::Cancel));
}

#[test]
fn internal_fault_offers_report_bug() {
    let actions = AppErrorKind::InternalFault.default_recovery_actions();
    assert!(actions.contains(&RecoveryAction::ReportBug));
}

#[test]
fn session_too_new_offers_start_fresh() {
    let actions = AppErrorKind::SessionTooNew.default_recovery_actions();
    assert!(actions.contains(&RecoveryAction::StartFresh));
}

// ── AppErrorKind::from_core ───────────────────────────────────────────────────

#[test]
fn io_read_maps_to_file_read_failed() {
    let err = CoreError::Io { path: None, operation: IoOperation::Read, message: "err".into() };
    assert_eq!(AppErrorKind::from_core(&err), AppErrorKind::FileReadFailed);
}

#[test]
fn io_write_maps_to_file_write_failed() {
    let err = CoreError::Io { path: None, operation: IoOperation::Write, message: "err".into() };
    assert_eq!(AppErrorKind::from_core(&err), AppErrorKind::FileWriteFailed);
}

#[test]
fn io_backup_maps_to_backup_failed() {
    let err = CoreError::Io { path: None, operation: IoOperation::CreateBackup, message: "err".into() };
    assert_eq!(AppErrorKind::from_core(&err), AppErrorKind::BackupFailed);
}

#[test]
fn conflict_maps_to_external_modification_detected() {
    let err = CoreError::Conflict { message: "changed".into() };
    assert_eq!(AppErrorKind::from_core(&err), AppErrorKind::ExternalModificationDetected);
}

#[test]
fn internal_invariant_maps_to_internal_fault() {
    let err = CoreError::InternalInvariant { message: "bug".into() };
    assert_eq!(AppErrorKind::from_core(&err), AppErrorKind::InternalFault);
}

// ── RecoveryAction ────────────────────────────────────────────────────────────

#[test]
fn all_recovery_action_tokens_are_non_empty_and_unique() {
    use std::collections::HashSet;
    let actions = [
        RecoveryAction::Dismiss, RecoveryAction::ChooseAnotherFile,
        RecoveryAction::Reload, RecoveryAction::SaveAs,
        RecoveryAction::OverwriteAnyway, RecoveryAction::OpenLimitedDiff,
        RecoveryAction::OpenAsBinary, RecoveryAction::Retry,
        RecoveryAction::RetryWithoutInline, RecoveryAction::Cancel,
        RecoveryAction::StartFresh, RecoveryAction::ReportBug,
    ];
    let tokens: Vec<&str> = actions.iter().map(|a| a.token()).collect();
    assert!(tokens.iter().all(|t| !t.is_empty()), "all tokens must be non-empty");
    let unique: HashSet<&&str> = tokens.iter().collect();
    assert_eq!(unique.len(), tokens.len(), "all tokens must be unique");
}

#[test]
fn destructive_actions_are_correctly_flagged() {
    assert!(RecoveryAction::OverwriteAnyway.is_destructive());
    assert!(RecoveryAction::StartFresh.is_destructive());
    assert!(!RecoveryAction::Dismiss.is_destructive());
    assert!(!RecoveryAction::Reload.is_destructive());
    assert!(!RecoveryAction::SaveAs.is_destructive());
}

// ── UserMessage ───────────────────────────────────────────────────────────────

#[test]
fn user_message_new_stores_both_fields() {
    let m = UserMessage::new("short", "detail");
    assert_eq!(m.short, "short");
    assert_eq!(m.detail, "detail");
}

#[test]
fn user_message_short_only_has_empty_detail() {
    let m = UserMessage::short_only("summary");
    assert_eq!(m.short, "summary");
    assert!(m.detail.is_empty());
}

#[test]
fn for_kind_produces_non_empty_short_for_all_variants() {
    let all = [
        AppErrorKind::PathNotFound, AppErrorKind::PathNotFile,
        AppErrorKind::PathNotDirectory, AppErrorKind::PermissionDenied,
        AppErrorKind::SymlinkUnsupported, AppErrorKind::FileReadFailed,
        AppErrorKind::FileWriteFailed, AppErrorKind::EncodingDetectionFailed,
        AppErrorKind::DecodeLossy, AppErrorKind::BinaryNotComparable,
        AppErrorKind::FileTooLarge, AppErrorKind::DiffFailed,
        AppErrorKind::InlineDiffTooLarge, AppErrorKind::SaveConflict,
        AppErrorKind::ExternalModificationDetected, AppErrorKind::BackupFailed,
        AppErrorKind::BackgroundJobFailed, AppErrorKind::BackgroundJobCancelled,
        AppErrorKind::SessionTooNew, AppErrorKind::SessionCorrupted,
        AppErrorKind::VcsUnavailable, AppErrorKind::VcsCommandFailed,
        AppErrorKind::SpreadsheetReadFailed, AppErrorKind::EncryptedWorkbook,
        AppErrorKind::InternalFault,
    ];
    for kind in all {
        let msg = UserMessage::for_kind(kind);
        assert!(!msg.short.is_empty(), "{kind:?} must have a non-empty short message");
    }
}

// ── AppError envelope (RFC-017 §5) ────────────────────────────────────────────

#[test]
fn app_error_from_core_io_read_has_expected_kind_and_recovery() {
    let core_err = CoreError::Io {
        path: Some(std::path::PathBuf::from("/some/file.rs")),
        operation: IoOperation::Read,
        message: "permission denied".into(),
    };
    let app_err = crate::error::AppError::from_core(&core_err);
    assert_eq!(app_err.kind, AppErrorKind::FileReadFailed);
    assert!(!app_err.message.short.is_empty());
    assert!(!app_err.recovery.is_empty());
    assert!(!app_err.technical.detail.is_empty());
}

#[test]
fn app_error_from_core_conflict_has_external_modification_kind() {
    let core_err = CoreError::Conflict { message: "file changed".into() };
    let app_err  = crate::error::AppError::from_core(&core_err);
    assert_eq!(app_err.kind, AppErrorKind::ExternalModificationDetected);
    assert!(app_err.recovery.contains(&crate::error::RecoveryAction::Reload));
}

#[test]
fn app_error_new_builds_from_kind_directly() {
    let app_err = crate::error::AppError::new(AppErrorKind::FileTooLarge, "84 MB");
    assert_eq!(app_err.kind, AppErrorKind::FileTooLarge);
    assert!(app_err.technical.detail.contains("84 MB"));
    assert!(app_err.recovery.contains(&crate::error::RecoveryAction::OpenLimitedDiff));
}

#[test]
fn app_error_is_blocking_for_blocking_severity() {
    let app_err = crate::error::AppError::new(AppErrorKind::InternalFault, "oops");
    assert!(app_err.is_blocking());
}

#[test]
fn app_error_is_not_blocking_for_recoverable_severity() {
    let app_err = crate::error::AppError::new(AppErrorKind::PathNotFound, "missing");
    assert!(!app_err.is_blocking());
}

#[test]
fn app_error_is_recoverable_when_non_dismiss_actions_exist() {
    let app_err = crate::error::AppError::new(AppErrorKind::SaveConflict, "changed");
    assert!(app_err.is_recoverable(),
        "SaveConflict must offer non-dismiss recovery actions");
}

#[test]
fn error_id_is_non_empty() {
    let id = crate::error::ErrorId::new();
    assert!(!id.0.is_empty());
    assert!(id.0.starts_with("err-"));
}

#[test]
fn technical_detail_stores_code_and_detail() {
    let td = crate::error::TechnicalDetail::new("io::read", "permission denied at /path");
    assert_eq!(td.code, "io::read");
    assert!(td.detail.contains("permission denied"));
}
