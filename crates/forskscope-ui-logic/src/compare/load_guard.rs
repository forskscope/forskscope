//! File size load guard view-model (RFC-013 §"Large file prompt").
//!
//! [`LoadGuard`] tells the diff workspace component whether it should
//! proceed immediately, show a non-blocking warning banner, or block with
//! a confirmation prompt before starting the diff. It is derived from
//! [`FileSizeClass`] for both files in a comparison pair.
//!
//! This replaces the ad-hoc `DiffWarning::LargeFilePolicyApplied` banner
//! (which fires *after* the diff is already done) with a *pre-diff* decision
//! the UI can act on before triggering the expensive computation.
//!
//! ## Design
//!
//! - No I/O. The caller provides `left_bytes` and `right_bytes`; this module
//!   applies [`FileSizeClass::classify`] and produces the guard.
//! - [`LoadGuard::Proceed`] — diff immediately, no user interaction needed.
//! - [`LoadGuard::WarnBanner`] — proceed but show an informational banner.
//! - [`LoadGuard::ConfirmPrompt`] — block and ask the user to confirm.
//! - All text in the guard is owned `String` so the component holds a
//!   snapshot without lifetime coupling.

use forskscope_core::job::{FileSizeClass, PerformanceLimits};

// ── LoadGuard ─────────────────────────────────────────────────────────────────

/// Pre-diff action the component should take based on file sizes.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadGuard {
    /// Both files are small — proceed with full diff immediately.
    Proceed,
    /// At least one file is medium-sized — proceed but show a warning banner.
    WarnBanner {
        /// Short message for the yellow banner, e.g. `"Large file — inline
        /// diff disabled."`.
        message:        String,
        /// Whether character-level inline diff should be suppressed.
        suppress_inline: bool,
    },
    /// At least one file is large or very large — block and ask the user.
    ConfirmPrompt {
        /// Title line for the confirmation dialog.
        title:          String,
        /// Body text explaining what will happen if the user confirms.
        body:           String,
        /// Label for the confirm button, e.g. `"Diff anyway"`.
        confirm_label:  String,
        /// Whether the file is so large that text diff is impossible.
        too_large:      bool,
    },
}

impl LoadGuard {
    /// `true` when the component should proceed without showing any UI.
    pub fn is_proceed(&self) -> bool { matches!(self, Self::Proceed) }

    /// `true` when the component needs user confirmation before diffing.
    pub fn needs_confirm(&self) -> bool { matches!(self, Self::ConfirmPrompt { .. }) }

    /// `true` when inline character diff should be suppressed for this pair.
    pub fn suppress_inline(&self) -> bool {
        match self {
            Self::WarnBanner { suppress_inline, .. } => *suppress_inline,
            Self::ConfirmPrompt { .. } => true,
            Self::Proceed => false,
        }
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Derive the load guard for a file pair from their byte sizes.
///
/// `left_bytes` and `right_bytes` are the sizes reported by the filesystem
/// before any decoding. Uses [`PerformanceLimits::default()`] thresholds.
pub fn guard_for_sizes(left_bytes: u64, right_bytes: u64) -> LoadGuard {
    guard_for_sizes_with_limits(left_bytes, right_bytes, &PerformanceLimits::default())
}

/// Like [`guard_for_sizes`] but with explicit thresholds (for testing).
pub fn guard_for_sizes_with_limits(
    left_bytes:  u64,
    right_bytes: u64,
    limits:      &PerformanceLimits,
) -> LoadGuard {
    let left_class  = FileSizeClass::classify(left_bytes,  limits);
    let right_class = FileSizeClass::classify(right_bytes, limits);
    let worst       = worst_class(left_class, right_class);

    match worst {
        FileSizeClass::Small => LoadGuard::Proceed,

        FileSizeClass::Medium => LoadGuard::WarnBanner {
            message:         "Large file — inline diff disabled.".into(),
            suppress_inline: true,
        },

        FileSizeClass::Large => LoadGuard::ConfirmPrompt {
            title:         "File is large".into(),
            body:          format!(
                "One or both files exceed the recommended diff limit ({} MiB). \
                 Diffing may be slow or produce an approximate result.",
                limits.medium_text_threshold_bytes / (1024 * 1024)
            ),
            confirm_label: "Diff anyway".into(),
            too_large:     false,
        },

        FileSizeClass::VeryLarge => LoadGuard::ConfirmPrompt {
            title:         "File too large".into(),
            body:          format!(
                "One or both files exceed {} MiB. Only metadata and binary \
                 summary will be shown; text diff is not available.",
                limits.large_text_threshold_bytes / (1024 * 1024)
            ),
            confirm_label: "Show metadata".into(),
            too_large:     true,
        },
    }
}

/// Return the more severe of two [`FileSizeClass`] values.
fn worst_class(a: FileSizeClass, b: FileSizeClass) -> FileSizeClass {
    if severity(a) >= severity(b) { a } else { b }
}

fn severity(c: FileSizeClass) -> u8 {
    match c {
        FileSizeClass::Small    => 0,
        FileSizeClass::Medium   => 1,
        FileSizeClass::Large    => 2,
        FileSizeClass::VeryLarge => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Limits with tight thresholds for easy testing.
    fn test_limits() -> PerformanceLimits {
        PerformanceLimits {
            max_eager_text_bytes:           1_000,       // Small  ≤ 1 KB
            medium_text_threshold_bytes:    10_000,      // Medium ≤ 10 KB
            large_text_threshold_bytes:     100_000,     // Large  ≤ 100 KB
            max_inline_diff_chars_per_hunk: 200,
            max_directory_entries_eager:    50,
            max_eager_lines:                1_000,
        }
    }

    // ── Single-file classification ─────────────────────────────────────────────

    #[test]
    fn both_small_produces_proceed() {
        let g = guard_for_sizes_with_limits(500, 800, &test_limits());
        assert_eq!(g, LoadGuard::Proceed);
        assert!(g.is_proceed());
        assert!(!g.needs_confirm());
        assert!(!g.suppress_inline());
    }

    #[test]
    fn medium_left_produces_warn_banner() {
        let g = guard_for_sizes_with_limits(5_000, 100, &test_limits());
        assert!(matches!(g, LoadGuard::WarnBanner { .. }));
        assert!(!g.is_proceed());
        assert!(!g.needs_confirm());
        assert!(g.suppress_inline());
    }

    #[test]
    fn medium_right_produces_warn_banner() {
        let g = guard_for_sizes_with_limits(100, 5_000, &test_limits());
        assert!(matches!(g, LoadGuard::WarnBanner { .. }));
    }

    #[test]
    fn large_file_produces_confirm_prompt() {
        let g = guard_for_sizes_with_limits(50_000, 100, &test_limits());
        assert!(matches!(g, LoadGuard::ConfirmPrompt { too_large: false, .. }));
        assert!(g.needs_confirm());
        assert!(g.suppress_inline());
    }

    #[test]
    fn very_large_file_produces_confirm_too_large() {
        let g = guard_for_sizes_with_limits(200_000, 100, &test_limits());
        assert!(matches!(g, LoadGuard::ConfirmPrompt { too_large: true, .. }));
    }

    // ── Worst-of-pair logic ────────────────────────────────────────────────────

    #[test]
    fn worst_class_is_taken_from_larger_file() {
        // Left is VeryLarge, right is Small — should use VeryLarge
        let g = guard_for_sizes_with_limits(200_000, 100, &test_limits());
        assert!(matches!(g, LoadGuard::ConfirmPrompt { too_large: true, .. }));
    }

    #[test]
    fn both_medium_stays_warn_banner() {
        let g = guard_for_sizes_with_limits(5_000, 5_000, &test_limits());
        assert!(matches!(g, LoadGuard::WarnBanner { .. }));
    }

    #[test]
    fn left_large_right_medium_takes_large() {
        let g = guard_for_sizes_with_limits(50_000, 5_000, &test_limits());
        assert!(matches!(g, LoadGuard::ConfirmPrompt { too_large: false, .. }));
    }

    // ── Default limits ─────────────────────────────────────────────────────────

    #[test]
    fn tiny_files_use_default_limits_proceed() {
        let g = guard_for_sizes(1_024, 1_024); // 1 KB each
        assert_eq!(g, LoadGuard::Proceed);
    }

    #[test]
    fn five_mib_file_uses_default_limits_confirm() {
        let g = guard_for_sizes(5 * 1024 * 1024, 1_024);
        assert!(g.needs_confirm(), "5 MiB should require confirmation with defaults");
    }

    // ── Message content ────────────────────────────────────────────────────────

    #[test]
    fn warn_banner_message_is_non_empty() {
        let g = guard_for_sizes_with_limits(5_000, 100, &test_limits());
        if let LoadGuard::WarnBanner { message, .. } = g {
            assert!(!message.is_empty());
        } else {
            panic!("expected WarnBanner");
        }
    }

    #[test]
    fn confirm_prompt_has_non_empty_title_body_label() {
        let g = guard_for_sizes_with_limits(50_000, 100, &test_limits());
        if let LoadGuard::ConfirmPrompt { title, body, confirm_label, .. } = g {
            assert!(!title.is_empty());
            assert!(!body.is_empty());
            assert!(!confirm_label.is_empty());
        } else {
            panic!("expected ConfirmPrompt");
        }
    }

    #[test]
    fn very_large_prompt_has_different_confirm_label_than_large() {
        let large      = guard_for_sizes_with_limits(50_000,  100, &test_limits());
        let very_large = guard_for_sizes_with_limits(200_000, 100, &test_limits());
        let large_label = if let LoadGuard::ConfirmPrompt { confirm_label, .. } = large {
            confirm_label
        } else { panic!() };
        let very_label = if let LoadGuard::ConfirmPrompt { confirm_label, .. } = very_large {
            confirm_label
        } else { panic!() };
        assert_ne!(large_label, very_label,
            "large and very-large should have distinct confirm labels");
    }

    // ── Boundary values ────────────────────────────────────────────────────────

    #[test]
    fn exactly_at_small_limit_is_proceed() {
        let g = guard_for_sizes_with_limits(1_000, 1_000, &test_limits());
        assert_eq!(g, LoadGuard::Proceed);
    }

    #[test]
    fn one_byte_over_small_limit_is_warn_banner() {
        let g = guard_for_sizes_with_limits(1_001, 100, &test_limits());
        assert!(matches!(g, LoadGuard::WarnBanner { .. }));
    }

    #[test]
    fn exactly_at_medium_limit_is_warn_banner() {
        let g = guard_for_sizes_with_limits(10_000, 100, &test_limits());
        assert!(matches!(g, LoadGuard::WarnBanner { .. }));
    }

    #[test]
    fn one_byte_over_medium_limit_is_confirm() {
        let g = guard_for_sizes_with_limits(10_001, 100, &test_limits());
        assert!(matches!(g, LoadGuard::ConfirmPrompt { too_large: false, .. }));
    }

    #[test]
    fn exactly_at_large_limit_is_confirm_not_too_large() {
        let g = guard_for_sizes_with_limits(100_000, 100, &test_limits());
        assert!(matches!(g, LoadGuard::ConfirmPrompt { too_large: false, .. }));
    }

    #[test]
    fn one_byte_over_large_limit_is_too_large() {
        let g = guard_for_sizes_with_limits(100_001, 100, &test_limits());
        assert!(matches!(g, LoadGuard::ConfirmPrompt { too_large: true, .. }));
    }
}
