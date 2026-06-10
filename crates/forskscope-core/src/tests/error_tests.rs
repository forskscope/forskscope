//! Tests for `ErrorSeverity`, `RecoveryHint`, and `CoreError` query methods
//! (RFC-017 §"Error Severity", §"Recovery Actions").
//!
//! Every `CoreError` variant must map to a severity and a recovery hint.
//! The tests validate the design contract: conflicts are recoverable (not
//! blocking), write failures are blocking, decode issues are warnings, and
//! internal faults are blocking with a bug-report hint.

use crate::error::{CoreError, ErrorSeverity, IoOperation, RecoveryHint};

fn io(op: IoOperation) -> CoreError {
    CoreError::Io {
        path: Some(std::path::PathBuf::from("/test/path")),
        operation: op,
        message: "simulated error".into(),
    }
}

// ── Severity mapping ──────────────────────────────────────────────────────────

#[test]
fn conflict_is_recoverable_severity() {
    let e = CoreError::Conflict { message: "changed on disk".into() };
    assert_eq!(e.severity(), ErrorSeverity::Recoverable);
}

#[test]
fn read_io_is_recoverable_severity() {
    assert_eq!(io(IoOperation::Read).severity(), ErrorSeverity::Recoverable);
}

#[test]
fn write_io_is_blocking_severity() {
    assert_eq!(io(IoOperation::Write).severity(), ErrorSeverity::Blocking);
}

#[test]
fn rename_io_is_blocking_severity() {
    assert_eq!(io(IoOperation::Rename).severity(), ErrorSeverity::Blocking);
}

#[test]
fn copy_io_is_blocking_severity() {
    assert_eq!(io(IoOperation::Copy).severity(), ErrorSeverity::Blocking);
}

#[test]
fn list_dir_io_is_recoverable_severity() {
    assert_eq!(io(IoOperation::ListDir).severity(), ErrorSeverity::Recoverable);
}

#[test]
fn decode_error_is_warning_severity() {
    let e = CoreError::Decode { path: None, message: "invalid byte".into() };
    assert_eq!(e.severity(), ErrorSeverity::Warning);
}

#[test]
fn unsupported_is_warning_severity() {
    let e = CoreError::Unsupported { message: "not supported".into() };
    assert_eq!(e.severity(), ErrorSeverity::Warning);
}

#[test]
fn internal_invariant_is_blocking_severity() {
    let e = CoreError::InternalInvariant { message: "bug".into() };
    assert_eq!(e.severity(), ErrorSeverity::Blocking);
}

// ── Recovery hints ────────────────────────────────────────────────────────────

#[test]
fn conflict_hints_reload() {
    let e = CoreError::Conflict { message: "stale".into() };
    assert_eq!(e.recovery_hint(), RecoveryHint::Reload);
}

#[test]
fn read_io_hints_choose_another_file() {
    assert_eq!(io(IoOperation::Read).recovery_hint(), RecoveryHint::ChooseAnotherFile);
}

#[test]
fn write_io_hints_save_as() {
    assert_eq!(io(IoOperation::Write).recovery_hint(), RecoveryHint::SaveAs);
}

#[test]
fn rename_io_hints_save_as() {
    assert_eq!(io(IoOperation::Rename).recovery_hint(), RecoveryHint::SaveAs);
}

#[test]
fn copy_io_hints_check_permissions() {
    assert_eq!(io(IoOperation::Copy).recovery_hint(), RecoveryHint::CheckPermissions);
}

#[test]
fn invalid_path_hints_choose_another_file() {
    let e = CoreError::InvalidPath { path: "/bad".into(), reason: "not found".into() };
    assert_eq!(e.recovery_hint(), RecoveryHint::ChooseAnotherFile);
}

#[test]
fn internal_invariant_hints_report_bug() {
    let e = CoreError::InternalInvariant { message: "assertion failed".into() };
    assert_eq!(e.recovery_hint(), RecoveryHint::ReportBug);
}

// ── is_user_recoverable ───────────────────────────────────────────────────────

#[test]
fn conflict_is_user_recoverable() {
    let e = CoreError::Conflict { message: "changed".into() };
    assert!(e.is_user_recoverable());
}

#[test]
fn write_failure_is_not_user_recoverable() {
    assert!(!io(IoOperation::Write).is_user_recoverable());
}

#[test]
fn internal_invariant_is_not_user_recoverable() {
    let e = CoreError::InternalInvariant { message: "bug".into() };
    assert!(!e.is_user_recoverable());
}

// ── Severity ordering ─────────────────────────────────────────────────────────

#[test]
fn severity_ordering_is_correct() {
    assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
    assert!(ErrorSeverity::Warning < ErrorSeverity::Recoverable);
    assert!(ErrorSeverity::Recoverable < ErrorSeverity::Blocking);
}
