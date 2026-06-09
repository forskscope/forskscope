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
