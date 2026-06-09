//! Diff options (RFC-002 §7).

/// Diff algorithm selection, mapped onto `similar` v3 algorithms inside the
/// engine. UI layers must use this enum, never `similar::Algorithm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffAlgorithm {
    #[default]
    Myers,
    Patience,
    Lcs,
    /// Git-style histogram diff (new in `similar` v3).
    Histogram,
}

/// When inline (character-level) refinement is computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InlineMode {
    /// Never compute inline spans.
    None,
    /// Compute on request only (UI toggle). MVP default.
    #[default]
    Lazy,
    /// Compute immediately for replace hunks under the size threshold.
    EagerForSmallHunks,
}

/// Options controlling one diff computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffOptions {
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    pub inline_mode: InlineMode,
    pub algorithm: DiffAlgorithm,
    /// Hunks whose combined text exceeds this are skipped by inline diff.
    pub max_inline_chars_per_hunk: usize,
    /// Files larger than this fall back to the large-file policy
    /// (line diff with inline disabled + `DiffWarning::LargeFilePolicyApplied`).
    pub max_file_bytes_for_full_diff: u64,
    /// Soft deadline for the line diff; on expiry `similar` degrades
    /// gracefully and `DiffWarning::DeadlineExpired` is reported.
    pub deadline_ms: Option<u64>,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            ignore_whitespace: false,
            ignore_case: false,
            inline_mode: InlineMode::Lazy,
            algorithm: DiffAlgorithm::Myers,
            max_inline_chars_per_hunk: 16 * 1024,
            max_file_bytes_for_full_diff: 16 * 1024 * 1024,
            deadline_ms: Some(5_000),
        }
    }
}
