//! Editability classification and newline save-policy tests (RFC-012 §6, §8).

use crate::encoding::{NewlinePolicy, NewlineStyle};
use crate::file_kind::{EditabilityClass, FileKind};

// ── EditabilityClass derivation ───────────────────────────────────────────────

#[test]
fn utf8_text_no_errors_is_read_write() {
    assert_eq!(
        EditabilityClass::from_kind(&FileKind::Text, false, "UTF-8"),
        EditabilityClass::ReadWrite
    );
}

#[test]
fn text_with_decode_errors_requires_guard() {
    assert_eq!(
        EditabilityClass::from_kind(&FileKind::Text, true, "UTF-8"),
        EditabilityClass::ReadWriteWithGuard
    );
}

#[test]
fn non_utf8_encoding_requires_guard() {
    assert_eq!(
        EditabilityClass::from_kind(&FileKind::Text, false, "Shift_JIS"),
        EditabilityClass::ReadWriteWithGuard
    );
}

#[test]
fn binary_is_read_only() {
    assert_eq!(
        EditabilityClass::from_kind(&FileKind::Binary, false, ""),
        EditabilityClass::ReadOnly
    );
}

#[test]
fn excel_is_read_only() {
    assert_eq!(
        EditabilityClass::from_kind(&FileKind::ExcelXlsx, false, ""),
        EditabilityClass::ReadOnly
    );
}

#[test]
fn missing_is_read_only() {
    assert_eq!(
        EditabilityClass::from_kind(&FileKind::Missing, false, ""),
        EditabilityClass::ReadOnly
    );
}

#[test]
fn unsupported_kind_is_unsupported_class() {
    let kind = FileKind::Unsupported { reason: "not a file".into() };
    assert_eq!(
        EditabilityClass::from_kind(&kind, false, ""),
        EditabilityClass::Unsupported
    );
}

// ── EditabilityClass predicates ───────────────────────────────────────────────

#[test]
fn read_write_is_editable_and_saveable() {
    let c = EditabilityClass::ReadWrite;
    assert!(c.is_editable());
    assert!(c.is_saveable());
    assert!(!c.requires_save_guard());
}

#[test]
fn read_write_with_guard_is_editable_saveable_and_guarded() {
    let c = EditabilityClass::ReadWriteWithGuard;
    assert!(c.is_editable());
    assert!(c.is_saveable());
    assert!(c.requires_save_guard());
}

#[test]
fn read_only_is_not_editable_or_saveable() {
    let c = EditabilityClass::ReadOnly;
    assert!(!c.is_editable());
    assert!(!c.is_saveable());
}

#[test]
fn unsupported_is_not_editable_or_saveable() {
    let c = EditabilityClass::Unsupported;
    assert!(!c.is_editable());
    assert!(!c.is_saveable());
}

// ── EditabilityClass ordering (RFC-012 §8 table is ordered by permissiveness) ─

#[test]
fn editability_ordering_read_write_greater_than_read_only() {
    assert!(EditabilityClass::ReadWrite > EditabilityClass::ReadOnly);
    assert!(EditabilityClass::ReadWriteWithGuard > EditabilityClass::ReadOnly);
    assert!(EditabilityClass::ReadOnly > EditabilityClass::Unsupported);
}

// ── FileKind::editability convenience method ──────────────────────────────────

#[test]
fn file_kind_editability_delegates_correctly() {
    assert_eq!(FileKind::Text.editability(false, "UTF-8"), EditabilityClass::ReadWrite);
    assert_eq!(FileKind::Binary.editability(false, ""), EditabilityClass::ReadOnly);
}

// ── NewlinePolicy ─────────────────────────────────────────────────────────────

#[test]
fn force_lf_always_returns_lf() {
    for style in [NewlineStyle::Lf, NewlineStyle::CrLf, NewlineStyle::Cr,
                  NewlineStyle::Mixed, NewlineStyle::None] {
        assert_eq!(NewlinePolicy::ForceLf.resolve(style), Some("\n"),
            "ForceLf must return \\n for any detected style");
    }
}

#[test]
fn force_crlf_always_returns_crlf() {
    for style in [NewlineStyle::Lf, NewlineStyle::CrLf, NewlineStyle::Cr,
                  NewlineStyle::Mixed, NewlineStyle::None] {
        assert_eq!(NewlinePolicy::ForceCrlf.resolve(style), Some("\r\n"),
            "ForceCrlf must return \\r\\n for any detected style");
    }
}

#[test]
fn preserve_returns_detected_lf() {
    assert_eq!(NewlinePolicy::Preserve.resolve(NewlineStyle::Lf), Some("\n"));
}

#[test]
fn preserve_returns_detected_crlf() {
    assert_eq!(NewlinePolicy::Preserve.resolve(NewlineStyle::CrLf), Some("\r\n"));
}

#[test]
fn preserve_returns_detected_cr() {
    assert_eq!(NewlinePolicy::Preserve.resolve(NewlineStyle::Cr), Some("\r"));
}

#[test]
fn preserve_returns_none_for_mixed() {
    assert_eq!(
        NewlinePolicy::Preserve.resolve(NewlineStyle::Mixed),
        None,
        "Preserve on mixed style must return None (caller keeps original endings)"
    );
}

#[test]
fn preserve_returns_none_for_no_newlines() {
    assert_eq!(NewlinePolicy::Preserve.resolve(NewlineStyle::None), None);
}

#[test]
fn preserve_is_the_default_policy() {
    assert_eq!(NewlinePolicy::default(), NewlinePolicy::Preserve);
}
