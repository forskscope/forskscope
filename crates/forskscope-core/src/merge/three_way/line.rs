//! Line model shared by the three-way merge engine (RFC-033).
//!
//! Lines preserve their original terminator so a merged result round-trips
//! the source line endings, consistent with the two-way diff engine
//! (RFC-002, RFC-010 newline semantics).

use crate::diff::NewlineMarker;

/// One source line: content without terminator, plus its terminator marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeLine {
    pub content: String,
    pub newline: NewlineMarker,
}

impl MergeLine {
    /// Reconstruct the line including its terminator.
    pub fn rendered(&self) -> String {
        let mut s = String::with_capacity(self.content.len() + 2);
        s.push_str(&self.content);
        s.push_str(self.newline.as_str());
        s
    }
}

/// Split text into terminator-preserving lines. Identical splitting rules
/// to the diff engine so three-way and two-way views agree line-for-line.
pub fn split_lines(text: &str) -> Vec<MergeLine> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                out.push(MergeLine {
                    content: text[start..i].to_string(),
                    newline: NewlineMarker::Lf,
                });
                i += 1;
                start = i;
            }
            b'\r' => {
                let (marker, step) = if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    (NewlineMarker::CrLf, 2)
                } else {
                    (NewlineMarker::Cr, 1)
                };
                out.push(MergeLine {
                    content: text[start..i].to_string(),
                    newline: marker,
                });
                i += step;
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < bytes.len() {
        out.push(MergeLine {
            content: text[start..].to_string(),
            newline: NewlineMarker::None,
        });
    }
    out
}

/// Render a slice of lines back to a string.
pub fn render_lines(lines: &[MergeLine]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(&line.rendered());
    }
    out
}

/// Comparison key for one line. Three-way merge compares on full
/// content+terminator (no ignore options): a newline-only change is a real
/// change for merge purposes.
pub(super) fn key(line: &MergeLine) -> String {
    let mut k = String::with_capacity(line.content.len() + 2);
    k.push_str(&line.content);
    k.push_str(line.newline.as_str());
    k
}
