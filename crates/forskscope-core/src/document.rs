//! File identity and document loading (RFC-001 §6.1–§6.3).
//!
//! `LoadedDocument` is the canonical loaded representation of one
//! comparison side. The fingerprint captured at load time backs external
//! modification detection at save time (RFC-007).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::encoding::{NewlineStyle, TextEncoding, decode_bytes, detect_newline_style};
use crate::error::{CoreError, IoOperation, Result};
use crate::file_kind::{FileKind, classify};
use crate::path::{canonicalize_lenient, display};
use crate::{fnv1a64, xlsx};

/// Stable identity of a file participating in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileId {
    pub canonical_path: PathBuf,
    pub display_path: String,
}

impl FileId {
    pub fn new(path: &Path) -> Self {
        let canonical = canonicalize_lenient(path);
        let display_path = display(&canonical);
        Self {
            canonical_path: canonical,
            display_path,
        }
    }
}

/// Fingerprint used for external-modification detection (RFC-007).
///
/// The digest is a cheap non-cryptographic content hash; combined with
/// length and mtime it is sufficient to detect on-disk changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFingerprint {
    pub len: u64,
    pub modified_unix_nanos: Option<i128>,
    pub digest: Option<u64>,
}

impl FileFingerprint {
    /// Capture the current fingerprint of `path`, hashing `bytes` if given.
    pub fn capture(path: &Path, bytes: Option<&[u8]>) -> Result<Self> {
        let meta = fs::metadata(path).map_err(|e| CoreError::io(path, IoOperation::Metadata, &e))?;
        let modified_unix_nanos = meta.modified().ok().and_then(|t| {
            t.duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| i128::from(d.as_nanos() as u64))
        });
        Ok(Self {
            len: meta.len(),
            modified_unix_nanos,
            digest: bytes.map(fnv1a64),
        })
    }
}

/// Decoded text plus its metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDocument {
    pub content: String,
    pub encoding: TextEncoding,
    pub newline_style: NewlineStyle,
    pub had_decode_errors: bool,
}

/// A non-fatal observation made while loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWarning {
    DecodeReplacementsEmitted,
    BinaryRenderedAsHexPreview,
    ExcelRenderedAsDerivedText,
}

/// Loading options.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoadOptions {
    /// When `true`, missing paths load as an empty text document instead of
    /// failing, supporting "one side empty" comparisons (RFC-001 §9).
    pub allow_missing: bool,
}

/// Canonical loaded representation of one comparison side.
#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub file_id: Option<FileId>,
    pub fingerprint_at_load: Option<FileFingerprint>,
    pub kind: FileKind,
    pub bytes_len: u64,
    /// Present for `Text` (decoded), `Binary` (hex preview as comparable
    /// text), and `ExcelXlsx` (adapter-derived text).
    pub text: Option<TextDocument>,
    pub warnings: Vec<LoadWarning>,
}

impl LoadedDocument {
    /// Empty pseudo-document used when one side is intentionally absent.
    pub fn empty() -> Self {
        Self {
            file_id: None,
            fingerprint_at_load: None,
            kind: FileKind::Missing,
            bytes_len: 0,
            text: Some(TextDocument {
                content: String::new(),
                encoding: TextEncoding::utf8(),
                newline_style: NewlineStyle::None,
                had_decode_errors: false,
            }),
            warnings: Vec::new(),
        }
    }

    /// Text content for diffing, or `""` when absent.
    pub fn diff_text(&self) -> &str {
        self.text.as_ref().map(|t| t.content.as_str()).unwrap_or("")
    }
}

/// Load a path into a `LoadedDocument` according to its classification.
pub fn load_path(path: &Path, options: LoadOptions) -> Result<LoadedDocument> {
    let kind = classify(path)?;
    match kind {
        FileKind::Missing => {
            if options.allow_missing {
                Ok(LoadedDocument::empty())
            } else {
                Err(CoreError::InvalidPath {
                    path: display(path),
                    reason: "file not found".into(),
                })
            }
        }
        FileKind::Unsupported { reason } => Err(CoreError::Unsupported {
            message: format!("`{}`: {reason}", display(path)),
        }),
        FileKind::Text => {
            let bytes = read_all(path)?;
            let fingerprint = FileFingerprint::capture(path, Some(&bytes))?;
            let (content, encoding, had_errors) = decode_bytes(&bytes);
            let newline_style = detect_newline_style(&content);
            let mut warnings = Vec::new();
            if had_errors {
                warnings.push(LoadWarning::DecodeReplacementsEmitted);
            }
            Ok(LoadedDocument {
                file_id: Some(FileId::new(path)),
                fingerprint_at_load: Some(fingerprint),
                kind: FileKind::Text,
                bytes_len: bytes.len() as u64,
                text: Some(TextDocument {
                    content,
                    encoding,
                    newline_style,
                    had_decode_errors: had_errors,
                }),
                warnings,
            })
        }
        FileKind::Binary => {
            let bytes = read_all(path)?;
            let fingerprint = FileFingerprint::capture(path, Some(&bytes))?;
            Ok(LoadedDocument {
                file_id: Some(FileId::new(path)),
                fingerprint_at_load: Some(fingerprint),
                kind: FileKind::Binary,
                bytes_len: bytes.len() as u64,
                text: Some(TextDocument {
                    content: hex_preview(&bytes),
                    encoding: TextEncoding {
                        label: "(binary)".into(),
                    },
                    newline_style: NewlineStyle::Lf,
                    had_decode_errors: false,
                }),
                warnings: vec![LoadWarning::BinaryRenderedAsHexPreview],
            })
        }
        FileKind::ExcelXlsx => xlsx::load_placeholder(path),
    }
}

fn read_all(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| CoreError::io(path, IoOperation::Read, &e))
}

/// Single normalized hex preview: offset, 16 hex bytes, ASCII column.
/// This replaces the two inconsistent binary renderings of v0.22.x.
pub fn hex_preview(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 + 64);
    for (row, chunk) in bytes.chunks(16).enumerate() {
        out.push_str(&format!("{:08x}  ", row * 16));
        for i in 0..16 {
            match chunk.get(i) {
                Some(b) => out.push_str(&format!("{b:02x} ")),
                None => out.push_str("   "),
            }
            if i == 7 {
                out.push(' ');
            }
        }
        out.push(' ');
        for b in chunk {
            out.push(if b.is_ascii_graphic() || *b == b' ' {
                *b as char
            } else {
                '.'
            });
        }
        out.push('\n');
    }
    out
}
