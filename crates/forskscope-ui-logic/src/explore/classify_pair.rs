//! A file pair's equality classification, with an automatic-digest cap
//! (RFC-080 §5 / F77, handoff 056).
//!
//! F145 (handoff 060 §4): this was `explorer.rs`'s `classify_two_files` — a
//! pure decision, two paths and a bound in, a classification out, no Dioxus
//! type involved — the same shape as this module's tier-1/tier-2 verdict
//! folds, which already live here. Moved with its tests; `explorer.rs` keeps
//! only [`AUTO_DIGEST_CAP_BYTES`] itself (a UI-policy constant: what browsing
//! starts automatically) and calls this with it.

use std::path::PathBuf;

use forskscope_core::dir::EqualityEvidence;

/// What a left-side entry's classification will be, before any async work
/// starts. `Final` is inserted immediately; `NeedsDigest` means the caller
/// inserts a pending placeholder and starts the real digest comparison - a
/// file present on both sides is the one case this function cannot resolve
/// synchronously.
#[derive(Debug, PartialEq)]
pub enum EntryClassification {
    Final(EqualityEvidence),
    NeedsDigest {
        left_abs: PathBuf,
        right_abs: PathBuf,
    },
}

/// The two-files case of `classify_entry` (`explorer.rs`), with the cap as a
/// parameter so tests can exercise the real logic against a small cap instead
/// of writing real multi-megabyte fixtures. `explorer.rs` always calls this
/// with `AUTO_DIGEST_CAP_BYTES`.
///
/// F77 (handoff 056): only a pair that *would* need an uncapped read has to
/// be decided here — either side at or over `cap`. A pair that stays under
/// it is untouched: `NeedsDigest` behaves exactly as it did before this cap
/// existed. A `metadata` failure on either side also falls through
/// unchanged, to the read path's own error handling — not this cap's job to
/// report. Only when both sizes are known and at least one is at or over
/// `cap`: a mismatch is free and certain regardless (`SizeDifferent`, so the
/// cap never *hides* a difference), and a match rests at `MetadataMatch`
/// instead of being read.
pub fn classify_two_files(left_abs: PathBuf, right_abs: PathBuf, cap: u64) -> EntryClassification {
    if let (Ok(lm), Ok(rm)) = (std::fs::metadata(&left_abs), std::fs::metadata(&right_abs)) {
        let (left_size, right_size) = (lm.len(), rm.len());
        if left_size.max(right_size) >= cap {
            return EntryClassification::Final(if left_size != right_size {
                EqualityEvidence::SizeDifferent {
                    left_size,
                    right_size,
                }
            } else {
                EqualityEvidence::MetadataMatch
            });
        }
    }
    EntryClassification::NeedsDigest {
        left_abs,
        right_abs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "fsk-ui-logic-classify-pair-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Falsifies the cap directly: a same-size pair at the cap must not be
    /// read — proven, not asserted, the tier-1 precedent (criterion 1):
    /// one side is `chmod 000`, so a real read would fail with `EACCES`.
    /// `stat(2)` (what `fs::metadata` calls) needs no permission on the file
    /// itself, only on its parent directory, so classification can still see
    /// the size. Reaching `Final` at all — not merely `MetadataMatch` — is
    /// the proof: only `NeedsDigest` ever triggers a read, and this returns
    /// before that branch is reached.
    #[cfg(unix)]
    #[test]
    fn a_same_size_pair_at_the_cap_is_never_read() {
        use std::os::unix::fs::PermissionsExt;
        let d = cap_dir("unread");
        let (a, b) = (d.join("a.bin"), d.join("b.bin"));
        std::fs::write(&a, [7u8; 10]).unwrap();
        std::fs::write(&b, [7u8; 10]).unwrap();
        let _ = std::fs::set_permissions(&a, std::fs::Permissions::from_mode(0o000));
        if std::fs::read(&a).is_ok() {
            crate::test_support::permission_guard_failed();
            let _ = std::fs::set_permissions(&a, std::fs::Permissions::from_mode(0o644));
            let _ = std::fs::remove_dir_all(&d);
            return;
        }

        let result = classify_two_files(a.clone(), b.clone(), 10);

        let _ = std::fs::set_permissions(&a, std::fs::Permissions::from_mode(0o644));
        assert_eq!(
            result,
            EntryClassification::Final(EqualityEvidence::MetadataMatch),
            "a real read would have failed with EACCES; classification must \
             never have attempted one"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A size mismatch at the cap is still `Different` — free, certain, and
    /// the cap must not hide it: `SizeDifferent`, not silently `MetadataMatch`.
    #[test]
    fn a_size_mismatch_at_the_cap_is_still_reported_different() {
        let d = cap_dir("mismatch");
        let (a, b) = (d.join("a.bin"), d.join("b.bin"));
        std::fs::write(&a, [1u8; 10]).unwrap();
        std::fs::write(&b, [1u8; 20]).unwrap();

        assert_eq!(
            classify_two_files(a, b, 10),
            EntryClassification::Final(EqualityEvidence::SizeDifferent {
                left_size: 10,
                right_size: 20,
            })
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Under the cap, nothing changes: the pair still needs a real digest,
    /// exactly as before this cap existed.
    #[test]
    fn under_the_cap_the_pair_still_needs_a_digest() {
        let d = cap_dir("under");
        let (a, b) = (d.join("a.bin"), d.join("b.bin"));
        std::fs::write(&a, [1u8; 5]).unwrap();
        std::fs::write(&b, [1u8; 5]).unwrap();

        assert_eq!(
            classify_two_files(a.clone(), b.clone(), 10),
            EntryClassification::NeedsDigest {
                left_abs: a,
                right_abs: b,
            }
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A pair at the cap but with unequal sizes needs no digest either —
    /// `SizeDifferent` above is resolved without ever reaching `NeedsDigest`,
    /// which this pins directly against the enum variant, not only its payload.
    #[test]
    fn a_pair_at_the_cap_never_reaches_needs_digest_whichever_way_it_resolves() {
        let d = cap_dir("no-digest");
        let (a, b) = (d.join("a.bin"), d.join("b.bin"));
        std::fs::write(&a, [1u8; 10]).unwrap();
        std::fs::write(&b, [1u8; 99]).unwrap();
        assert!(!matches!(
            classify_two_files(a, b, 10),
            EntryClassification::NeedsDigest { .. }
        ));
        let _ = std::fs::remove_dir_all(&d);
    }
}
