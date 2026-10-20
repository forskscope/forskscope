//! SHA-256 checksums for comparison reports (RFC-092 §5).
//!
//! A report is worth something only if anyone running `sha256sum` on the
//! files it describes gets the hashes it shows. So this hashes exactly the
//! bytes the caller already read — nothing here reads a file itself; that
//! stays the caller's job, off the UI thread, with its own cancellation
//! (RFC-092 §5, handoff 084 §0). Hashes are computed only for the export,
//! never kept or written anywhere else.

use sha2::{Digest, Sha256};

/// Lower-case hex SHA-256 of `bytes`, matching `sha256sum`'s own output
/// format exactly, so the digit string in a report can be pasted straight
/// into a `sha256sum -c` check.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_matches_sha256sums_well_known_value() {
        // `printf '' | sha256sum` — the standard empty-string SHA-256,
        // reproduced here rather than trusted blindly.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn known_short_input_matches_sha256sums_well_known_value() {
        // `printf 'abc' | sha256sum`.
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn output_is_lowercase_hex_of_exactly_64_characters() {
        let hex = sha256_hex(b"forskscope");
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn different_bytes_produce_different_hashes() {
        assert_ne!(sha256_hex(b"left"), sha256_hex(b"right"));
    }
}
