//! File size classification and performance limits (RFC-013 §5).

// ── RFC-013 §5: File size classification and performance limits ───────────────

/// Classification of a file by size, used to select the diff strategy
/// (RFC-013 §5 "Threshold Policy").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileSizeClass {
    /// Small: full diff + inline diff eager.
    Small,
    /// Medium: full line diff, inline diff lazy / on demand.
    Medium,
    /// Large: prompt user before full diff; inline diff disabled.
    Large,
    /// Very large: metadata / binary summary only, unless forced.
    VeryLarge,
}

impl FileSizeClass {
    /// Classify a file by its byte count using `limits`.
    pub fn classify(bytes: u64, limits: &PerformanceLimits) -> Self {
        if bytes <= limits.max_eager_text_bytes {
            Self::Small
        } else if bytes <= limits.medium_text_threshold_bytes {
            Self::Medium
        } else if bytes <= limits.large_text_threshold_bytes {
            Self::Large
        } else {
            Self::VeryLarge
        }
    }

    /// `true` when inline character diff should run eagerly.
    pub fn inline_diff_eager(self) -> bool {
        self == Self::Small
    }

    /// `true` when the user should be prompted before starting the diff.
    pub fn requires_user_prompt(self) -> bool {
        matches!(self, Self::Large | Self::VeryLarge)
    }

    /// `true` when the file is too large for any text diff.
    pub fn too_large_for_diff(self) -> bool {
        self == Self::VeryLarge
    }
}

/// Configurable thresholds governing large-file and large-directory
/// behaviour (RFC-013 §5 "Threshold Policy"). All byte values are inclusive
/// upper bounds for the named class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceLimits {
    /// Upper bound (inclusive) for `FileSizeClass::Small` (bytes).
    pub max_eager_text_bytes:           u64,
    /// Upper bound (inclusive) for `FileSizeClass::Medium` (bytes).
    pub medium_text_threshold_bytes:    u64,
    /// Upper bound (inclusive) for `FileSizeClass::Large` (bytes).
    /// Files above this are `VeryLarge`.
    pub large_text_threshold_bytes:     u64,
    /// Maximum number of characters in a hunk for eager inline diff.
    pub max_inline_diff_chars_per_hunk: usize,
    /// Maximum directory entries to compare without backgrounding.
    pub max_directory_entries_eager:    usize,
    /// Maximum lines in a text document before disabling some UI features.
    pub max_eager_lines:                usize,
}

impl Default for PerformanceLimits {
    fn default() -> Self {
        Self {
            max_eager_text_bytes:           512 * 1024,      //  512 KiB
            medium_text_threshold_bytes:    4 * 1024 * 1024, //    4 MiB
            large_text_threshold_bytes:     64 * 1024 * 1024,//   64 MiB
            max_inline_diff_chars_per_hunk: 2_000,
            max_directory_entries_eager:    500,
            max_eager_lines:                50_000,
        }
    }
}

