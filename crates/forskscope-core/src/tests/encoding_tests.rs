use crate::encoding::{NewlineStyle, decode_bytes, detect_newline_style, encode_text};

#[test]
fn utf8_decodes_as_utf8_without_errors() {
    let (text, enc, had_errors) = decode_bytes("héllo".as_bytes());
    assert_eq!(text, "héllo");
    assert_eq!(enc.label, "UTF-8");
    assert!(!had_errors);
}

#[test]
fn legacy_bytes_are_decoded_via_detection() {
    // Shift_JIS bytes for "ã‚ã„ã†" should not be valid UTF-8 and should
    // decode through detection without panicking.
    let sjis = [0x82u8, 0xA0, 0x82, 0xA2, 0x82, 0xA4];
    let (text, enc, _) = decode_bytes(&sjis);
    assert!(!text.is_empty());
    assert_ne!(enc.label, "UTF-8");
}

#[test]
fn encode_round_trips_utf8() {
    let (bytes, fallback) = encode_text("data", "UTF-8");
    assert_eq!(bytes, b"data");
    assert!(!fallback);
}

#[test]
fn unknown_encoding_label_falls_back_to_utf8() {
    let (bytes, fallback) = encode_text("data", "not-a-real-encoding");
    assert_eq!(bytes, b"data");
    assert!(fallback);
}

#[test]
fn newline_style_detection_covers_all_cases() {
    assert_eq!(detect_newline_style("a\nb\n"), NewlineStyle::Lf);
    assert_eq!(detect_newline_style("a\r\nb\r\n"), NewlineStyle::CrLf);
    assert_eq!(detect_newline_style("a\rb\r"), NewlineStyle::Cr);
    assert_eq!(detect_newline_style("a\r\nb\n"), NewlineStyle::Mixed);
    assert_eq!(detect_newline_style("no newline"), NewlineStyle::None);
}

// ── BomPresence (RFC-012 §7.2 bullet 5) ──────────────────────────────────────

use crate::encoding::{BomPolicy, BomPresence, detect_bom};

#[test]
fn detect_bom_absent_returns_absent_and_full_slice() {
    let bytes = b"hello world";
    let (presence, rest) = detect_bom(bytes);
    assert_eq!(presence, BomPresence::Absent);
    assert_eq!(rest, bytes);
}

#[test]
fn detect_bom_utf8_strips_three_bytes() {
    let bom: &[u8] = &[0xEF, 0xBB, 0xBF];
    let content = b"hello";
    let bytes = [bom, content].concat();
    let (presence, rest) = detect_bom(&bytes);
    assert_eq!(presence, BomPresence::Utf8);
    assert_eq!(rest, content.as_ref());
}

#[test]
fn detect_bom_utf16le_strips_two_bytes() {
    let bytes: &[u8] = &[0xFF, 0xFE, 0x41, 0x00];
    let (presence, rest) = detect_bom(bytes);
    assert_eq!(presence, BomPresence::Utf16Le);
    assert_eq!(rest, &[0x41, 0x00]);
}

#[test]
fn detect_bom_utf16be_strips_two_bytes() {
    let bytes: &[u8] = &[0xFE, 0xFF, 0x00, 0x41];
    let (presence, rest) = detect_bom(bytes);
    assert_eq!(presence, BomPresence::Utf16Be);
    assert_eq!(rest, &[0x00, 0x41]);
}

#[test]
fn bom_presence_is_present_only_for_non_absent() {
    assert!(!BomPresence::Absent.is_present());
    assert!( BomPresence::Utf8.is_present());
    assert!( BomPresence::Utf16Le.is_present());
    assert!( BomPresence::Utf16Be.is_present());
}

#[test]
fn bom_presence_bytes_match_known_bom_sequences() {
    assert_eq!(BomPresence::Absent.bytes(),   &[] as &[u8]);
    assert_eq!(BomPresence::Utf8.bytes(),     &[0xEF, 0xBB, 0xBF]);
    assert_eq!(BomPresence::Utf16Le.bytes(),  &[0xFF, 0xFE]);
    assert_eq!(BomPresence::Utf16Be.bytes(),  &[0xFE, 0xFF]);
}

// ── BomPolicy (RFC-012 §7.2 bullet 5) ────────────────────────────────────────

#[test]
fn bom_policy_preserve_keeps_original_bom() {
    assert_eq!(
        BomPolicy::Preserve.resolve_bytes(BomPresence::Utf8),
        BomPresence::Utf8.bytes(),
    );
}

#[test]
fn bom_policy_preserve_keeps_absent_when_absent() {
    assert_eq!(
        BomPolicy::Preserve.resolve_bytes(BomPresence::Absent),
        &[] as &[u8],
        "Preserve with absent original must produce no BOM bytes",
    );
}

#[test]
fn bom_policy_strip_always_produces_empty() {
    for presence in [
        BomPresence::Absent,
        BomPresence::Utf8,
        BomPresence::Utf16Le,
        BomPresence::Utf16Be,
    ] {
        assert_eq!(
            BomPolicy::Strip.resolve_bytes(presence),
            &[] as &[u8],
            "Strip must produce no BOM bytes regardless of original",
        );
    }
}

#[test]
fn bom_policy_add_utf8_always_produces_utf8_bom() {
    for presence in [
        BomPresence::Absent,
        BomPresence::Utf16Le,
    ] {
        assert_eq!(
            BomPolicy::AddUtf8.resolve_bytes(presence),
            &[0xEF, 0xBB, 0xBF],
            "AddUtf8 must produce UTF-8 BOM bytes regardless of original",
        );
    }
}

#[test]
fn default_bom_policy_is_preserve() {
    assert_eq!(BomPolicy::default(), BomPolicy::Preserve);
}

#[test]
fn default_bom_presence_is_absent() {
    assert_eq!(BomPresence::default(), BomPresence::Absent);
}
