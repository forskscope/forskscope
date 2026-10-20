use crate::encoding::{
    MAX_REPORTED_UNMAPPABLE_CHARS, NewlineStyle, decode_bytes, detect_newline_style, encode_text,
    unmappable_characters,
};

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
    let outcome = encode_text("data", "UTF-8");
    assert_eq!(outcome.bytes, b"data");
    assert!(!outcome.unknown_label_fallback);
    assert!(!outcome.lossy);
}

#[test]
fn unknown_encoding_label_falls_back_to_utf8() {
    let outcome = encode_text("data", "not-a-real-encoding");
    assert_eq!(outcome.bytes, b"data");
    assert!(outcome.unknown_label_fallback);
    assert!(!outcome.lossy);
}

// ── F87/RFC-082 §D4: encode_text reports lossy encodes, and the
// unmappable characters can be named on the failure path ──────────────────

#[test]
fn encoding_an_emoji_into_shift_jis_is_reported_lossy() {
    let outcome = encode_text("hi 😀\n", "shift_jis");
    assert!(
        outcome.lossy,
        "Shift_JIS cannot represent U+1F600 — encode_text must report this, \
         not silently write the numeric character reference"
    );
    assert!(
        !outcome.unknown_label_fallback,
        "shift_jis is a recognized label — this is not the unknown-label case"
    );
}

#[test]
fn a_representable_string_is_never_reported_lossy() {
    let outcome = encode_text("hello\n", "shift_jis");
    assert!(!outcome.lossy);
}

#[test]
fn unmappable_characters_names_the_emoji_shift_jis_cannot_represent() {
    let (sample, additional) = unmappable_characters("hi 😀\n", "shift_jis", 5);
    assert_eq!(
        sample,
        vec!['😀'],
        "the actual offending character must be named, not merely \"lossy\""
    );
    assert_eq!(additional, 0);
}

#[test]
fn unmappable_characters_caps_the_list_and_counts_the_rest() {
    // 6 distinct emoji, cap of 5 — the first 5 (in order of first
    // appearance) are named, and the 6th is folded into the count.
    let content = "😀😁😂😃😄😅";
    let (sample, additional) = unmappable_characters(content, "shift_jis", 5);
    assert_eq!(sample, vec!['😀', '😁', '😂', '😃', '😄']);
    assert_eq!(additional, 1);
}

#[test]
fn unmappable_characters_deduplicates_repeated_occurrences() {
    let content = "😀😀😀 and 😁";
    let (sample, additional) = unmappable_characters(content, "shift_jis", 5);
    assert_eq!(
        sample,
        vec!['😀', '😁'],
        "each distinct character must be named once, not once per occurrence"
    );
    assert_eq!(additional, 0);
}

#[test]
fn unmappable_characters_unknown_label_reports_nothing() {
    // encode_text already treats an unknown label as a fallback, not a
    // loss — unmappable_characters must agree, not invent a false report.
    let (sample, additional) = unmappable_characters("😀", "not-a-real-encoding", 5);
    assert!(sample.is_empty());
    assert_eq!(additional, 0);
}

/// F87 §4a, handoff 017 §7 test 3: the fast path must not run the
/// per-character identification scan. Proven, not argued — a thread-local
/// call counter (test-only) around `unmappable_characters`, reset then
/// checked immediately around a clean `encode_text` call.
#[test]
fn encode_text_success_path_never_calls_the_unmappable_scan() {
    use crate::encoding::UNMAPPABLE_SCAN_CALLS;

    UNMAPPABLE_SCAN_CALLS.with(|c| c.set(0));
    let outcome = encode_text("hello, world\n", "UTF-8");
    assert!(
        !outcome.lossy,
        "test setup: this content must encode cleanly"
    );
    UNMAPPABLE_SCAN_CALLS.with(|c| {
        assert_eq!(
            c.get(),
            0,
            "encode_text's success path must never call unmappable_characters"
        )
    });
}

#[test]
fn default_unmappable_char_cap_is_five() {
    assert_eq!(MAX_REPORTED_UNMAPPABLE_CHARS, 5);
}

// ── F88a/RFC-082 §D3: why the load-time decode-substitution guard exists —
// F87's save-time check alone does not catch this ──────────────────────────

/// §2a's exact fixture and the reasoning behind it: a UTF-8 BOM forces the
/// UTF-8 interpretation, so decoding cannot fall back to a lossless
/// single-byte encoding the way it would for a bare invalid byte with no
/// BOM — the replacement character is the only option, and it is valid
/// UTF-8, so F87's own lossy check (`encode_text`) stays silent on the
/// re-encode. This documents *why* F88a's guard exists, not only that it
/// fires: the bytes a save would actually write are provably not the
/// file's original bytes.
#[test]
fn a_decode_substituted_reencode_would_differ_from_the_original_bytes() {
    let original: &[u8] = &[0xEF, 0xBB, 0xBF, 0xFF, b'a', b'\n'];
    let (text, encoding, had_errors) = decode_bytes(original);
    assert!(
        had_errors,
        "test setup: this fixture must decode with replacement characters"
    );

    let encoded = encode_text(&text, &encoding.label);
    assert!(
        !encoded.lossy,
        "test setup: F87's own guard must stay silent here — the decoded \
         text is valid UTF-8 and re-encodes losslessly, which is exactly \
         the gap F88a's guard exists to close"
    );

    assert_ne!(
        encoded.bytes, original,
        "the bytes a save would actually write differ from the file's \
         original bytes — this is why the save must be blocked before it \
         happens, not merely warned about afterward"
    );
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
    assert!(BomPresence::Utf8.is_present());
    assert!(BomPresence::Utf16Le.is_present());
    assert!(BomPresence::Utf16Be.is_present());
}

#[test]
fn bom_presence_bytes_match_known_bom_sequences() {
    assert_eq!(BomPresence::Absent.bytes(), &[] as &[u8]);
    assert_eq!(BomPresence::Utf8.bytes(), &[0xEF, 0xBB, 0xBF]);
    assert_eq!(BomPresence::Utf16Le.bytes(), &[0xFF, 0xFE]);
    assert_eq!(BomPresence::Utf16Be.bytes(), &[0xFE, 0xFF]);
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
    for presence in [BomPresence::Absent, BomPresence::Utf16Le] {
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
