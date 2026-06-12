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
    /// When `true`, newline-style differences (LF vs CRLF) are ignored during
    /// line comparison (RFC-028 `NewlineCompareMode::IgnoreDifference`).
    pub ignore_newlines: bool,
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
            ignore_newlines: false,
            inline_mode: InlineMode::Lazy,
            algorithm: DiffAlgorithm::Myers,
            max_inline_chars_per_hunk: 16 * 1024,
            max_file_bytes_for_full_diff: 16 * 1024 * 1024,
            deadline_ms: Some(5_000),
        }
    }
}

// ── RFC-028: Richer compare option types and named profiles ───────────────────

/// How whitespace is treated during comparison (RFC-028).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhitespaceMode {
    /// Every whitespace character is significant. Default.
    #[default]
    Significant,
    /// Trailing whitespace on a line is ignored.
    IgnoreTrailing,
    /// All whitespace differences (leading, trailing, internal) are ignored.
    IgnoreAll,
    /// Lines that are entirely blank are ignored.
    IgnoreBlankLines,
}

/// How newline style differences are treated during comparison (RFC-028).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewlineCompareMode {
    /// CRLF vs LF vs CR are considered different. Default.
    #[default]
    Significant,
    /// Newline style differences are ignored (LF == CRLF for diff purposes).
    IgnoreDifference,
}

/// Case sensitivity for comparison (RFC-028).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseSensitivity {
    /// Case differences are significant. Default.
    #[default]
    Sensitive,
    /// Case differences are ignored (maps to `DiffOptions::ignore_case`).
    Insensitive,
}

/// A named comparison profile: a preset combination of options intended for a
/// specific use case (RFC-028 §"Default profiles").
///
/// Profiles are the UI-layer concept; `DiffOptions` is the engine-layer
/// concept. `CompareProfile::to_diff_options()` bridges them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareProfile {
    /// Human-readable name shown in the UI toolbar.
    pub name: String,
    pub whitespace:  WhitespaceMode,
    pub newlines:    NewlineCompareMode,
    pub case:        CaseSensitivity,
    pub inline_mode: InlineMode,
    pub algorithm:   DiffAlgorithm,
}

impl CompareProfile {
    // ── Named presets ──────────────────────────────────────────────────────

    /// Default: all differences are significant, Myers algorithm.
    pub fn default_profile() -> Self {
        Self {
            name:        "Default".into(),
            whitespace:  WhitespaceMode::Significant,
            newlines:    NewlineCompareMode::Significant,
            case:        CaseSensitivity::Sensitive,
            inline_mode: InlineMode::Lazy,
            algorithm:   DiffAlgorithm::Myers,
        }
    }

    /// Code Review: whitespace significant, newline preserved, inline on.
    pub fn code_review() -> Self {
        Self {
            name:        "Code Review".into(),
            whitespace:  WhitespaceMode::Significant,
            newlines:    NewlineCompareMode::Significant,
            case:        CaseSensitivity::Sensitive,
            inline_mode: InlineMode::Lazy,
            algorithm:   DiffAlgorithm::Histogram,
        }
    }

    /// Loose Text: ignore trailing whitespace and newline differences.
    pub fn loose_text() -> Self {
        Self {
            name:        "Loose Text".into(),
            whitespace:  WhitespaceMode::IgnoreTrailing,
            newlines:    NewlineCompareMode::IgnoreDifference,
            case:        CaseSensitivity::Sensitive,
            inline_mode: InlineMode::Lazy,
            algorithm:   DiffAlgorithm::Myers,
        }
    }

    /// Large File Safe: line diff only, inline disabled.
    pub fn large_file_safe() -> Self {
        Self {
            name:        "Large File Safe".into(),
            whitespace:  WhitespaceMode::Significant,
            newlines:    NewlineCompareMode::Significant,
            case:        CaseSensitivity::Sensitive,
            inline_mode: InlineMode::None,
            algorithm:   DiffAlgorithm::Myers,
        }
    }

    /// All built-in profiles, in display order.
    pub fn all_presets() -> Vec<Self> {
        vec![
            Self::default_profile(),
            Self::code_review(),
            Self::loose_text(),
            Self::large_file_safe(),
        ]
    }

    // ── Conversion ─────────────────────────────────────────────────────────

    /// Derive `DiffOptions` from this profile.
    ///
    /// `WhitespaceMode` and `NewlineCompareMode` map onto `ignore_whitespace`
    /// and `ignore_case` today; future engine work may expose finer controls.
    pub fn to_diff_options(&self) -> DiffOptions {
        DiffOptions {
            ignore_whitespace: !matches!(self.whitespace, WhitespaceMode::Significant),
            ignore_case:       self.case == CaseSensitivity::Insensitive,
            ignore_newlines:   self.newlines == NewlineCompareMode::IgnoreDifference,
            inline_mode:       self.inline_mode,
            algorithm:         self.algorithm,
            ..DiffOptions::default()
        }
    }
}

impl Default for CompareProfile {
    fn default() -> Self {
        Self::default_profile()
    }
}
