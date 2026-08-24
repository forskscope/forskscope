//! Explorer status row view-model (RFC-054, RFC-037, RFC-059).
//!
//! Maps `EqualityEvidence` (core truth) to the display model the Explorer
//! tree row component needs: a status icon glyph, a CSS class, and a
//! screen-reader label. Replaces the ad-hoc `DigestState` enum in
//! `ui/dir_pane.rs` with a tested, core-connected type.
//!
//! ## Accessibility (RFC-009 §7)
//!
//! Every status has both a glyph (for sighted users) and a text label (for
//! screen readers). The CSS class is for styling only; it is never the sole
//! indicator of status.

use forskscope_core::dir::EqualityEvidence;

// ── Status display kind ───────────────────────────────────────────────────────

/// The visual status of one explorer row (RFC-054 §"Status badges").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowStatusKind {
    /// Digest or metadata comparison confirmed equal.
    Equal,
    /// Content differs (size, digest, or type mismatch).
    Different,
    /// Only present on the left side.
    LeftOnly,
    /// Only present on the right side.
    RightOnly,
    /// Digest computation is still running.
    Computing,
    /// An error prevented comparison.
    Error,
    /// Comparison was never attempted for this entry (handoff 007 §4/§7a) -
    /// distinct from `Computing`: *not attempted* and *in progress* are
    /// different claims, and rendering `Computing` for `Unknown` gives a
    /// row a spinner that never resolves. This is also the state a
    /// same-named directory pair must show (F74, `16c35f1`) - the Explorer
    /// never examines directory contents, so it can prove neither equality
    /// nor difference for one.
    NotCompared,
}

impl RowStatusKind {
    /// Single-character glyph (non-colour indicator, RFC-009 §7).
    pub fn glyph(self) -> char {
        match self {
            Self::Equal => '=',
            Self::Different => '≠',
            Self::LeftOnly => '←',
            Self::RightOnly => '→',
            Self::Computing => '…',
            Self::Error => '!',
            Self::NotCompared => '–',
        }
    }

    /// Stable CSS class token for the status badge.
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Equal => "status-equal",
            Self::Different => "status-different",
            Self::LeftOnly => "status-left-only",
            Self::RightOnly => "status-right-only",
            Self::Computing => "status-computing",
            Self::Error => "status-error",
            Self::NotCompared => "status-not-compared",
        }
    }

    /// Screen-reader label (ARIA).
    pub fn aria_label(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Different => "different",
            Self::LeftOnly => "left only",
            Self::RightOnly => "right only",
            Self::Computing => "computing",
            Self::Error => "error",
            Self::NotCompared => "not compared",
        }
    }

    /// `true` when the entry needs user attention (is a change or one-sided).
    pub fn needs_action(self) -> bool {
        matches!(
            self,
            Self::Different | Self::LeftOnly | Self::RightOnly | Self::Error
        )
    }
}

// ── Derive from EqualityEvidence ──────────────────────────────────────────────

impl RowStatusKind {
    /// Derive the display kind from core `EqualityEvidence`.
    pub fn from_evidence(evidence: &EqualityEvidence) -> Self {
        match evidence {
            EqualityEvidence::DigestEqual | EqualityEvidence::MetadataEqual => Self::Equal,
            EqualityEvidence::MetadataOnly => Self::Computing,
            EqualityEvidence::DigestDifferent
            | EqualityEvidence::SizeDifferent { .. }
            | EqualityEvidence::TypeMismatch { .. } => Self::Different,
            EqualityEvidence::LeftOnly => Self::LeftOnly,
            EqualityEvidence::RightOnly => Self::RightOnly,
            EqualityEvidence::Error { .. } => Self::Error,
            EqualityEvidence::Unknown => Self::NotCompared,
        }
    }
}

// ── Status row ────────────────────────────────────────────────────────────────

/// Fully-resolved display data for one explorer row's status badge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRow {
    pub kind: RowStatusKind,
    pub glyph: char,
    pub css_class: &'static str,
    pub aria_label: &'static str,
}

impl StatusRow {
    pub fn from_evidence(evidence: &EqualityEvidence) -> Self {
        let kind = RowStatusKind::from_evidence(evidence);
        Self {
            kind,
            glyph: kind.glyph(),
            css_class: kind.css_class(),
            aria_label: kind.aria_label(),
        }
    }

    pub fn computing() -> Self {
        let kind = RowStatusKind::Computing;
        Self {
            kind,
            glyph: kind.glyph(),
            css_class: kind.css_class(),
            aria_label: kind.aria_label(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::dir::{EntryType, EqualityEvidence};

    // ── RowStatusKind from EqualityEvidence ───────────────────────────────────

    #[test]
    fn digest_equal_maps_to_equal() {
        assert_eq!(
            RowStatusKind::from_evidence(&EqualityEvidence::DigestEqual),
            RowStatusKind::Equal
        );
    }

    #[test]
    fn metadata_equal_maps_to_equal() {
        assert_eq!(
            RowStatusKind::from_evidence(&EqualityEvidence::MetadataEqual),
            RowStatusKind::Equal
        );
    }

    #[test]
    fn metadata_only_maps_to_computing() {
        assert_eq!(
            RowStatusKind::from_evidence(&EqualityEvidence::MetadataOnly),
            RowStatusKind::Computing
        );
    }

    #[test]
    fn digest_different_maps_to_different() {
        assert_eq!(
            RowStatusKind::from_evidence(&EqualityEvidence::DigestDifferent),
            RowStatusKind::Different
        );
    }

    #[test]
    fn size_different_maps_to_different() {
        let e = EqualityEvidence::SizeDifferent {
            left_size: 100,
            right_size: 200,
        };
        assert_eq!(RowStatusKind::from_evidence(&e), RowStatusKind::Different);
    }

    #[test]
    fn type_mismatch_maps_to_different() {
        let e = EqualityEvidence::TypeMismatch {
            left: EntryType::File,
            right: EntryType::Directory,
        };
        assert_eq!(RowStatusKind::from_evidence(&e), RowStatusKind::Different);
    }

    #[test]
    fn left_only_maps_to_left_only() {
        assert_eq!(
            RowStatusKind::from_evidence(&EqualityEvidence::LeftOnly),
            RowStatusKind::LeftOnly
        );
    }

    #[test]
    fn right_only_maps_to_right_only() {
        assert_eq!(
            RowStatusKind::from_evidence(&EqualityEvidence::RightOnly),
            RowStatusKind::RightOnly
        );
    }

    #[test]
    fn error_maps_to_error() {
        let e = EqualityEvidence::Error {
            message: "io error".into(),
        };
        assert_eq!(RowStatusKind::from_evidence(&e), RowStatusKind::Error);
    }

    // Handoff 007 §4: "not attempted" and "in progress" are different
    // claims. Mapping `Unknown` to `Computing` renders a spinner that
    // never resolves - this was the defect found while checking whether
    // wiring this module was mechanical (it was not).
    #[test]
    fn unknown_maps_to_not_compared_not_computing() {
        assert_eq!(
            RowStatusKind::from_evidence(&EqualityEvidence::Unknown),
            RowStatusKind::NotCompared
        );
        assert_ne!(
            RowStatusKind::from_evidence(&EqualityEvidence::Unknown),
            RowStatusKind::Computing
        );
    }

    // F74 (`16c35f1`): a same-named directory pair must render as
    // not-compared, sharing neither `Computing`'s glyph/label (a spinner
    // that never resolves) nor `Equal`'s (the false-equal claim F74 fixed).
    #[test]
    fn not_compared_has_its_own_distinct_glyph_and_label() {
        assert_ne!(
            RowStatusKind::NotCompared.glyph(),
            RowStatusKind::Equal.glyph()
        );
        assert_ne!(
            RowStatusKind::NotCompared.glyph(),
            RowStatusKind::Computing.glyph()
        );
        assert_ne!(
            RowStatusKind::NotCompared.aria_label(),
            RowStatusKind::Equal.aria_label()
        );
        assert_ne!(
            RowStatusKind::NotCompared.aria_label(),
            RowStatusKind::Computing.aria_label()
        );
    }

    // ── Display contract ──────────────────────────────────────────────────────

    #[test]
    fn all_css_classes_start_with_status_prefix() {
        for kind in [
            RowStatusKind::Equal,
            RowStatusKind::Different,
            RowStatusKind::LeftOnly,
            RowStatusKind::RightOnly,
            RowStatusKind::Computing,
            RowStatusKind::Error,
            RowStatusKind::NotCompared,
        ] {
            assert!(
                kind.css_class().starts_with("status-"),
                "{kind:?} css class must start with status-"
            );
        }
    }

    #[test]
    fn all_glyphs_are_distinct() {
        let kinds = [
            RowStatusKind::Equal,
            RowStatusKind::Different,
            RowStatusKind::LeftOnly,
            RowStatusKind::RightOnly,
            RowStatusKind::Computing,
            RowStatusKind::Error,
            RowStatusKind::NotCompared,
        ];
        let glyphs: std::collections::HashSet<char> = kinds.iter().map(|k| k.glyph()).collect();
        assert_eq!(glyphs.len(), kinds.len(), "all glyphs must be distinct");
    }

    #[test]
    fn all_aria_labels_are_non_empty() {
        for kind in [
            RowStatusKind::Equal,
            RowStatusKind::Different,
            RowStatusKind::LeftOnly,
            RowStatusKind::RightOnly,
            RowStatusKind::Computing,
            RowStatusKind::Error,
            RowStatusKind::NotCompared,
        ] {
            assert!(
                !kind.aria_label().is_empty(),
                "{kind:?} must have aria label"
            );
        }
    }

    #[test]
    fn needs_action_is_true_for_actionable_states() {
        assert!(!RowStatusKind::Equal.needs_action());
        assert!(!RowStatusKind::Computing.needs_action());
        assert!(RowStatusKind::Different.needs_action());
        assert!(RowStatusKind::LeftOnly.needs_action());
        assert!(RowStatusKind::RightOnly.needs_action());
        assert!(RowStatusKind::Error.needs_action());
        assert!(!RowStatusKind::NotCompared.needs_action());
    }

    #[test]
    fn status_row_from_evidence_matches_kind() {
        let row = StatusRow::from_evidence(&EqualityEvidence::DigestEqual);
        assert_eq!(row.kind, RowStatusKind::Equal);
        assert_eq!(row.glyph, RowStatusKind::Equal.glyph());
        assert_eq!(row.css_class, RowStatusKind::Equal.css_class());
    }

    #[test]
    fn status_row_computing_is_computing() {
        let row = StatusRow::computing();
        assert_eq!(row.kind, RowStatusKind::Computing);
    }
}
