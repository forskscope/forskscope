//! Text decoding, encoding, and newline policy (RFC-001 §6.3, RFC-012).
//!
//! Decoding keeps metadata: the resolved encoding label, whether replacement
//! characters were produced, and the dominant newline style. Saving encodes
//! back through the same label so a legacy-encoded file round-trips without
//! a silent conversion to UTF-8 (RFC-012, "preserve by default").

use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_8};
use serde::{Deserialize, Serialize};

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

/// Decode already-BOM-stripped bytes, given what BOM (if any) preceded
/// them (RFC-083 §2). A BOM is an explicit, authoritative encoding
/// declaration — present, it selects that encoding directly, the same way
/// it did before this RFC when a BOM'd file happened to reach
/// [`decode_bytes`] with the BOM still attached (`encoding_rs::Encoding::decode`
/// sniffs a leading BOM itself and overrides the detected encoding to
/// match). Stripping the BOM first and then falling through to
/// [`decode_bytes`]'s `chardetng` guess on the *remainder alone* would
/// throw that signal away: a bare invalid byte with no BOM can legitimately
/// decode losslessly under some single-byte legacy encoding, but the same
/// byte immediately after a UTF-8 BOM must not silently reinterpret as one
/// — the BOM already said what this is. Absent a BOM, `decode_bytes` runs
/// exactly as before, untouched by this function.
pub fn decode_body(bom: BomPresence, rest: &[u8]) -> (String, TextEncoding, bool) {
    match bom {
        BomPresence::Utf8 => {
            let (text, had_errors) = encoding_rs::UTF_8.decode_without_bom_handling(rest);
            (text.into_owned(), TextEncoding::utf8(), had_errors)
        }
        BomPresence::Utf16Le => {
            let (text, had_errors) = encoding_rs::UTF_16LE.decode_without_bom_handling(rest);
            (
                text.into_owned(),
                TextEncoding {
                    label: encoding_rs::UTF_16LE.name().to_string(),
                },
                had_errors,
            )
        }
        BomPresence::Utf16Be => {
            let (text, had_errors) = encoding_rs::UTF_16BE.decode_without_bom_handling(rest);
            (
                text.into_owned(),
                TextEncoding {
                    label: encoding_rs::UTF_16BE.name().to_string(),
                },
                had_errors,
            )
        }
        BomPresence::Absent => decode_bytes(rest),
    }
}

/// Encoding labels offered by the toolbar's override control (RFC-083 §3) —
/// a curated subset of what `encoding_rs`/`Encoding::for_label` actually
/// accepts, not the full ~40-encoding WHATWG list. Each is a canonical
/// `encoding_rs` name, so it round-trips through [`decode_with_label`] and
/// [`encode_text`] unchanged. Ordered as a user would scan it: UTF, then
/// the legacy families the audit's own misdetection example (Shift_JIS) and
/// this project's stated legacy-encoding-preservation goal are about.
pub const COMMON_ENCODING_LABELS: &[&str] = &[
    "UTF-8",
    "UTF-16LE",
    "UTF-16BE",
    "Shift_JIS",
    "EUC-JP",
    "GBK",
    "GB18030",
    "Big5",
    "EUC-KR",
    "windows-1252",
    "windows-1251",
    "KOI8-R",
    "macintosh",
];

/// Re-decode bytes already held in memory with an explicit, user-chosen
/// encoding label — no `chardetng` guess (RFC-083 §3: the user has already
/// told us what it is). `bytes` should already be BOM-stripped, same as
/// [`decode_body`]'s `rest` — the override changes which charset
/// interprets the body, not whether a BOM is present, which stays tracked
/// separately (`TextDocument::bom`) and round-trips regardless of the
/// chosen label.
///
/// An unrecognized `label` falls back to UTF-8, matching [`encode_text`]'s
/// same-shaped fallback on the write side — defensive only: the toolbar
/// control offers a fixed, always-valid list, so this path is not expected
/// to be reached in practice.
pub fn decode_with_label(bytes: &[u8], label: &str) -> (String, TextEncoding, bool) {
    let enc = Encoding::for_label(label.as_bytes()).unwrap_or(UTF_8);
    let (text, had_errors) = enc.decode_without_bom_handling(bytes);
    (
        text.into_owned(),
        TextEncoding {
            label: enc.name().to_string(),
        },
        had_errors,
    )
}

/// Result of encoding text for saving.
#[derive(Debug, Clone)]
pub struct EncodeOutcome {
    pub bytes: Vec<u8>,
    /// `true` when `label` was not a recognized encoding name and UTF-8 was
    /// used in its place. A different condition from `lossy`: this is about
    /// the *label* being unrecognized, not the *content* being
    /// unrepresentable in a label that was understood.
    pub unknown_label_fallback: bool,
    /// `true` when one or more characters in `content` could not be
    /// represented in the target encoding and `encoding_rs` substituted
    /// numeric character references (e.g. `&#128512;`) in their place
    /// (RFC-082 §D4). The save path must treat this as a refusal, not a
    /// warning — see `save::save_text`.
    pub lossy: bool,
}

/// Encode text for saving using the given encoding label.
///
/// Unknown labels fall back to UTF-8. This is the fast path: exactly one
/// `encoding_rs` encode pass, no per-character scanning — that only happens
/// in [`unmappable_characters`], called by the caller *only* when
/// `lossy` comes back `true` (F87/RFC-082 §D4 §4a).
pub fn encode_text(content: &str, label: &str) -> EncodeOutcome {
    match Encoding::for_label(label.as_bytes()) {
        Some(enc) => {
            let (bytes, _, had_errors) = enc.encode(content);
            EncodeOutcome {
                bytes: bytes.into_owned(),
                unknown_label_fallback: false,
                lossy: had_errors,
            }
        }
        None => EncodeOutcome {
            bytes: content.as_bytes().to_vec(),
            unknown_label_fallback: true,
            lossy: false,
        },
    }
}

/// Default cap on how many distinct unmappable characters
/// [`unmappable_characters`] reports by name before summarizing the rest as
/// a count — a file with hundreds of them must not produce a dialog with
/// hundreds of character glyphs in it.
pub const MAX_REPORTED_UNMAPPABLE_CHARS: usize = 5;

#[cfg(test)]
thread_local! {
    /// Test-only call counter for [`unmappable_characters`] (F87 §4a's "the
    /// fast path does not scan" requirement) — a `thread_local`, not a
    /// process-global `AtomicUsize`, so concurrently running tests on other
    /// threads can never make a reset-then-check test flaky.
    pub(crate) static UNMAPPABLE_SCAN_CALLS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// Distinct characters in `content` that `label`'s encoding cannot
/// represent, in order of first appearance, capped at `cap` — plus how many
/// *additional* distinct unmappable characters exist beyond the cap.
/// Returns `(Vec::new(), 0)` for an unrecognized label (nothing to report;
/// [`encode_text`] already handles that case as a fallback, not a loss).
///
/// Walks `content` character-by-character re-encoding each one — real cost,
/// deliberately paid only here, never on [`encode_text`]'s success path
/// (F87/RFC-082 §D4 §4a): call this only after `encode_text` has already
/// reported `lossy: true`.
pub fn unmappable_characters(content: &str, label: &str, cap: usize) -> (Vec<char>, usize) {
    #[cfg(test)]
    UNMAPPABLE_SCAN_CALLS.with(|c| c.set(c.get() + 1));

    let Some(enc) = Encoding::for_label(label.as_bytes()) else {
        return (Vec::new(), 0);
    };

    let mut seen = std::collections::HashSet::new();
    let mut sample = Vec::new();
    let mut buf = [0u8; 4];
    for ch in content.chars() {
        let (_, _, had_errors) = enc.encode(ch.encode_utf8(&mut buf));
        if had_errors && seen.insert(ch) && sample.len() < cap {
            sample.push(ch);
        }
    }
    let additional_count = seen.len().saturating_sub(sample.len());
    (sample, additional_count)
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
///
/// Part of the settings v2 on-disk schema (RFC-076); a variant rename is a
/// schema change, not just a Rust-level rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
            Self::ForceLf => Some("\n"),
            Self::ForceCrlf => Some("\r\n"),
            Self::Preserve => match detected {
                NewlineStyle::Lf => Some("\n"),
                NewlineStyle::CrLf => Some("\r\n"),
                NewlineStyle::Cr => Some("\r"),
                NewlineStyle::Mixed | NewlineStyle::None => None,
            },
        }
    }
}

// ── RFC-012 §7.2 bullet 5: BOM preservation policy ───────────────────────────

/// Whether a Byte Order Mark was present at the start of a loaded file.
///
/// The BOM (U+FEFF) is commonly used in UTF-8 and UTF-16 files from Windows
/// tools. ForskScope detects and records its presence so the save path can
/// preserve or strip it deliberately (RFC-012 §7.2 "Preserve BOM policy
/// unless the user changes it").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BomPresence {
    /// No BOM was found at the start of the file.
    #[default]
    Absent,
    /// A UTF-8 BOM (`EF BB BF`) was present and stripped during decode.
    Utf8,
    /// A UTF-16 LE BOM (`FF FE`) was present.
    Utf16Le,
    /// A UTF-16 BE BOM (`FE FF`) was present.
    Utf16Be,
}

impl BomPresence {
    /// `true` when any BOM was detected.
    pub fn is_present(self) -> bool {
        !matches!(self, Self::Absent)
    }

    /// The raw BOM bytes for this presence kind, if any.
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Self::Absent => &[],
            Self::Utf8 => &[0xEF, 0xBB, 0xBF],
            Self::Utf16Le => &[0xFF, 0xFE],
            Self::Utf16Be => &[0xFE, 0xFF],
        }
    }
}

/// Policy for BOM handling when saving a file (RFC-012 §7.2 bullet 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BomPolicy {
    /// Preserve whatever BOM was present (or absent) in the loaded file.
    /// This is the safe default: a file that came in with a UTF-8 BOM
    /// will be saved with one; a file that had none will continue to have none.
    #[default]
    Preserve,
    /// Always strip the BOM on save, regardless of the loaded file.
    Strip,
    /// Always write a UTF-8 BOM (`EF BB BF`) before the content.
    AddUtf8,
}

impl BomPolicy {
    /// Resolve the BOM bytes to prepend when saving.
    ///
    /// `original` is what was detected in the loaded file.
    /// Returns the bytes (possibly empty) to prepend before the content.
    pub fn resolve_bytes(self, original: BomPresence) -> &'static [u8] {
        match self {
            Self::Preserve => original.bytes(),
            Self::Strip => &[],
            Self::AddUtf8 => BomPresence::Utf8.bytes(),
        }
    }
}

/// Detect a BOM at the start of a byte slice and return the presence kind
/// plus the remaining bytes (after the BOM has been stripped).
pub fn detect_bom(bytes: &[u8]) -> (BomPresence, &[u8]) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return (BomPresence::Utf8, &bytes[3..]);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return (BomPresence::Utf16Le, &bytes[2..]);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return (BomPresence::Utf16Be, &bytes[2..]);
    }
    (BomPresence::Absent, bytes)
}
