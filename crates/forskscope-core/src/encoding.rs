//! Text decoding, encoding, and newline policy (RFC-001 §6.3, RFC-012).
//!
//! Decoding keeps metadata: the resolved encoding label, whether replacement
//! characters were produced, and the dominant newline style. Saving encodes
//! back through the same label so a legacy-encoded file round-trips without
//! a silent conversion to UTF-8 (RFC-012, "preserve by default").

use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_8};

/// Resolved text encoding of a loaded document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEncoding {
    /// Canonical label, e.g. `UTF-8`, `Shift_JIS`, `windows-1252`.
    pub label: String,
}

impl TextEncoding {
    pub fn utf8() -> Self {
        Self {
            label: UTF_8.name().to_string(),
        }
    }
}

/// Dominant newline style of a text document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlineStyle {
    Lf,
    CrLf,
    Cr,
    /// More than one style appears in the document.
    Mixed,
    /// The document contains no newline at all.
    None,
}

/// Decode raw bytes into text with metadata.
///
/// Strategy: valid UTF-8 is used directly; otherwise `chardetng` guesses
/// the encoding and `encoding_rs` decodes. `had_decode_errors` is `true`
/// when replacement characters were emitted.
pub fn decode_bytes(bytes: &[u8]) -> (String, TextEncoding, bool) {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return (s.to_string(), TextEncoding::utf8(), false);
    }
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);
    let (text, used, had_errors) = encoding.decode(bytes);
    (
        text.into_owned(),
        TextEncoding {
            label: used.name().to_string(),
        },
        had_errors,
    )
}

/// Encode text for saving using the given encoding label.
///
/// Unknown labels fall back to UTF-8; the boolean reports whether the
/// fallback was taken so the caller can warn instead of failing silently.
pub fn encode_text(content: &str, label: &str) -> (Vec<u8>, bool) {
    match Encoding::for_label(label.as_bytes()) {
        Some(enc) => {
            let (bytes, _, _) = enc.encode(content);
            (bytes.into_owned(), false)
        }
        None => (content.as_bytes().to_vec(), true),
    }
}

/// Detect the dominant newline style of a text document.
pub fn detect_newline_style(text: &str) -> NewlineStyle {
    let bytes = text.as_bytes();
    let (mut crlf, mut lf, mut cr) = (0usize, 0usize, 0usize);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    crlf += 1;
                    i += 2;
                    continue;
                }
                cr += 1;
            }
            b'\n' => lf += 1,
            _ => {}
        }
        i += 1;
    }
    match (crlf > 0, lf > 0, cr > 0) {
        (false, false, false) => NewlineStyle::None,
        (true, false, false) => NewlineStyle::CrLf,
        (false, true, false) => NewlineStyle::Lf,
        (false, false, true) => NewlineStyle::Cr,
        _ => NewlineStyle::Mixed,
    }
}

// ── RFC-012 §6: Newline save policy ──────────────────────────────────────────

/// How newline endings are handled when saving a merged result (RFC-012 §6).
///
/// The default (`Preserve`) keeps whatever style was detected on load.
/// Conversion to a specific style is an explicit user choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewlinePolicy {
    /// Keep the newline style that was detected at load time. Default.
    #[default]
    Preserve,
    /// Always write LF (`\n`), regardless of what was loaded.
    ForceLf,
    /// Always write CRLF (`\r\n`), regardless of what was loaded.
    ForceCrlf,
}

impl NewlinePolicy {
    /// Apply this policy: return the newline string to use when saving.
    ///
    /// `detected` is the style that was found in the loaded file.
    /// Returns `None` when the loaded style is mixed or unknown and
    /// `Preserve` is requested — the caller should keep original line
    /// endings rather than normalizing.
    pub fn resolve(self, detected: NewlineStyle) -> Option<&'static str> {
        match self {
            Self::ForceLf   => Some("\n"),
            Self::ForceCrlf => Some("\r\n"),
            Self::Preserve  => match detected {
                NewlineStyle::Lf   => Some("\n"),
                NewlineStyle::CrLf => Some("\r\n"),
                NewlineStyle::Cr   => Some("\r"),
                NewlineStyle::Mixed | NewlineStyle::None => None,
            },
        }
    }
}
