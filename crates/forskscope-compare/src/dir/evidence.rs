//! Equality vocabulary for directory comparison (RFC-080 tiers, RFC-008 §5).
//!
//! [`EqualityEvidence`] is what the explorer's status icons and the compare
//! report are built from: every variant names one measurement and the
//! conclusion that measurement supports. The UI constructs these values
//! directly from its own classification (`ui/view/explorer.rs`,
//! `ui-logic/src/explore/classify_pair.rs`).
//!
//! RFC-037's index machinery (a `DirectoryIndex` built from a background scan,
//! paired per path by `pair_entries`) was removed in 0.181.0 (F157): nothing
//! in the product built or consumed it, and its one "same size and mtime
//! means equal" variant claimed a conclusion RFC-080 deliberately withdrew.

/// Whether a filesystem entry is a file or a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    File,
    Directory,
}

/// The evidence behind an equality determination for one path pair.
///
/// Variants are named for what was measured, not for the verdict they lead
/// to: a value that claims a different measurement than the one taken is a
/// bug in the comparison, not a label to reword.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EqualityEvidence {
    /// Both sides have identical digests computed with the same algorithm.
    DigestEqual,
    /// Sizes match but digest has not been computed yet (still pending).
    MetadataOnly,
    /// The entry exists only on the left side.
    LeftOnly,
    /// The entry exists only on the right side.
    RightOnly,
    /// Both sides have the same path but different entry types
    /// (e.g. file vs directory).
    TypeMismatch { left: EntryType, right: EntryType },
    /// Sizes differ — content is definitely different, no digest needed.
    SizeDifferent { left_size: u64, right_size: u64 },
    /// Both sides have digests that do not match.
    DigestDifferent,
    /// Tier 1 of RFC-080: the two directories' **names and sizes match**, and
    /// their contents were never read. This is a completed measurement with an
    /// incomplete conclusion, so it is **not** [`is_equal`](Self::is_equal):
    /// a one-character edit preserves both names and sizes, so this cannot mean
    /// "identical". It is deliberately not [`MetadataOnly`](Self::MetadataOnly)
    /// either, which is the "digest in flight" placeholder.
    MetadataMatch,
    /// Tier 1 of RFC-080: the two directory trees **differ** — an entry on one
    /// side only, or a common file whose size differs — established from the walk
    /// alone, with no contents read. Certain, unlike [`MetadataMatch`](Self::MetadataMatch).
    TreeDifferent,
    /// Tier 2 of RFC-080: every file in the two directory trees was read and
    /// matched — a user-triggered `recursive_diff` found no difference.
    /// Deliberately not [`DigestEqual`](Self::DigestEqual): that variant
    /// asserts *one* file's digest was computed, and a directory's tier-2
    /// conclusion is not that (review 129 §2 made the same call for
    /// [`TreeDifferent`](Self::TreeDifferent) against
    /// [`DigestDifferent`](Self::DigestDifferent), for the same reason — an
    /// enum whose whole job is to say what was measured must not reuse a
    /// value that claims a different measurement). The only tier-2 state
    /// entitled to claim identity; tier 1 alone must never reach it.
    TreeIdentical,
    /// One or both sides had an error; comparison result is unreliable.
    Error { message: String },
    /// Comparison has not been attempted yet.
    Unknown,
}

impl EqualityEvidence {
    /// `true` when the evidence conclusively shows equality. Only the two
    /// states that measured contents, one file or a whole tree, qualify.
    pub fn is_equal(&self) -> bool {
        matches!(self, Self::DigestEqual | Self::TreeIdentical)
    }
}
