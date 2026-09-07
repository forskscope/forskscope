//! Explorer status row view-model (RFC-054, RFC-037, RFC-059), and the
//! shared status vocabulary both the Explorer and Deep Compare render
//! through (F82).
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
//!
//! ## Shared vocabulary (F82)
//!
//! [`RowStatusKind`] (this module, Explorer) and `RecStatus`
//! (`forskscope_core::dir`, Deep Compare) describe different things — an
//! aligned-tree row versus a recursive scan result — and stay separate
//! types; they are not merged here. What both views need to be the same
//! is the *presentation* for the concepts they share, so [`StatusGlyph`]
//! is the one table both map into for glyph, CSS class, and the English
//! fallback label. A concept present in only one view (`NotCompared` here,
//! `Symlink` in Deep Compare) still goes through this table — there is
//! exactly one definition of every glyph, never a second copy for the view
//! that doesn't currently need it.

use forskscope_core::dir::{EqualityEvidence, RecStatus};

// ── Shared status vocabulary (F82) ──────────────────────────────────────────

/// The presentation both directory-compare views render through: glyph,
/// CSS class, and English fallback label, one row per concept. See the
/// module doc comment for why this is a separate type from
/// [`RowStatusKind`] and `RecStatus` rather than a merge of the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusGlyph {
    Equal,
    Different,
    Computing,
    /// Nothing could be read for this entry — not a verdict (F79). Explorer
    /// calls this concept `Error`; Deep Compare's own name, `Unreadable`,
    /// is the one adopted here, and its glyph `⊘` ("prohibited") replaces
    /// Explorer's generic `!` (F82 §2: `⚠`-style alarm iconography is
    /// wrong for "nothing was measured", and `⊘` already says that
    /// correctly in Deep Compare).
    Unreadable,
    LeftOnly,
    RightOnly,
    /// Comparison was never attempted (Explorer only — a same-named
    /// directory pair, F74).
    NotCompared,
    /// One or both sides is a symlink, not followed (Deep Compare only).
    Symlink,
}

impl StatusGlyph {
    /// Single-character glyph (non-colour indicator, RFC-009 §7).
    pub fn glyph(self) -> char {
        match self {
            Self::Equal => '=',
            Self::Different => '≠',
            Self::Computing => '…',
            Self::Unreadable => '⊘',
            Self::LeftOnly => '←',
            Self::RightOnly => '→',
            Self::NotCompared => '–',
            Self::Symlink => '↗',
        }
    }

    /// Stable CSS class token for the status badge.
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Equal => "status-equal",
            Self::Different => "status-different",
            Self::Computing => "status-computing",
            Self::Unreadable => "status-unreadable",
            Self::LeftOnly => "status-left-only",
            Self::RightOnly => "status-right-only",
            Self::NotCompared => "status-not-compared",
            Self::Symlink => "status-symlink",
        }
    }

    /// English fallback screen-reader label (ARIA). Each view's Dioxus
    /// component renders its own `Lang`-translated label instead when one
    /// is available (`ui-logic` has no i18n system) — this is the value
    /// used when this type is consumed directly, and every variant using
    /// the same wording that reaches production kept it unchanged going
    /// through this consolidation (F82 falsification 3: labels move with
    /// the glyphs, never dropped).
    pub fn aria_label(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Different => "different",
            Self::Computing => "computing",
            Self::Unreadable => "unreadable",
            Self::LeftOnly => "left only",
            Self::RightOnly => "right only",
            Self::NotCompared => "not compared",
            Self::Symlink => "symlink",
        }
    }

    /// The shared concept for one `RecStatus` (Deep Compare, RFC-037).
    pub fn for_rec_status(status: RecStatus) -> Self {
        match status {
            RecStatus::Equal => Self::Equal,
            RecStatus::Changed => Self::Different,
            RecStatus::Computing => Self::Computing,
            RecStatus::Unreadable => Self::Unreadable,
            RecStatus::LeftOnly => Self::LeftOnly,
            RecStatus::RightOnly => Self::RightOnly,
            RecStatus::Symlink => Self::Symlink,
        }
    }
}

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
    /// The shared concept (F82) this Explorer status renders as. `Error`
    /// maps to [`StatusGlyph::Unreadable`] — the same "nothing was
    /// measured" concept Deep Compare's `RecStatus::Unreadable` names,
    /// under Deep Compare's glyph and CSS class rather than Explorer's
    /// former `!`/`status-error`.
    pub fn status_glyph(self) -> StatusGlyph {
        match self {
            Self::Equal => StatusGlyph::Equal,
            Self::Different => StatusGlyph::Different,
            Self::LeftOnly => StatusGlyph::LeftOnly,
            Self::RightOnly => StatusGlyph::RightOnly,
            Self::Computing => StatusGlyph::Computing,
            Self::Error => StatusGlyph::Unreadable,
            Self::NotCompared => StatusGlyph::NotCompared,
        }
    }

    /// Single-character glyph (non-colour indicator, RFC-009 §7).
    pub fn glyph(self) -> char {
        self.status_glyph().glyph()
    }

    /// Stable CSS class token for the status badge.
    pub fn css_class(self) -> &'static str {
        self.status_glyph().css_class()
    }

    /// Screen-reader label (ARIA).
    pub fn aria_label(self) -> &'static str {
        self.status_glyph().aria_label()
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

    // ── F82: shared status vocabulary ────────────────────────────────────────

    #[test]
    fn all_status_glyph_glyphs_are_distinct() {
        let concepts = [
            StatusGlyph::Equal,
            StatusGlyph::Different,
            StatusGlyph::Computing,
            StatusGlyph::Unreadable,
            StatusGlyph::LeftOnly,
            StatusGlyph::RightOnly,
            StatusGlyph::NotCompared,
            StatusGlyph::Symlink,
        ];
        let glyphs: std::collections::HashSet<char> = concepts.iter().map(|c| c.glyph()).collect();
        assert_eq!(glyphs.len(), concepts.len(), "all glyphs must be distinct");
    }

    #[test]
    fn all_status_glyph_aria_labels_are_non_empty() {
        for concept in [
            StatusGlyph::Equal,
            StatusGlyph::Different,
            StatusGlyph::Computing,
            StatusGlyph::Unreadable,
            StatusGlyph::LeftOnly,
            StatusGlyph::RightOnly,
            StatusGlyph::NotCompared,
            StatusGlyph::Symlink,
        ] {
            assert!(
                !concept.aria_label().is_empty(),
                "{concept:?} must have a non-empty aria label"
            );
        }
    }

    // F82 falsification 1: `RowStatusKind::glyph/css_class/aria_label` must
    // be real delegations to `StatusGlyph`, not independently maintained
    // copies that happen to agree today — falsify by hardcoding any one of
    // them back to a literal and this fails.
    #[test]
    fn row_status_kind_delegates_to_status_glyph_for_every_variant() {
        for kind in [
            RowStatusKind::Equal,
            RowStatusKind::Different,
            RowStatusKind::LeftOnly,
            RowStatusKind::RightOnly,
            RowStatusKind::Computing,
            RowStatusKind::Error,
            RowStatusKind::NotCompared,
        ] {
            let concept = kind.status_glyph();
            assert_eq!(kind.glyph(), concept.glyph());
            assert_eq!(kind.css_class(), concept.css_class());
            assert_eq!(kind.aria_label(), concept.aria_label());
        }
    }

    // F82 falsification 1, the one that matters: this does not compare the
    // shared table to itself. It compares what the *Explorer* renders
    // (`RowStatusKind`) against what *Deep Compare* renders (`RecStatus`)
    // for every concept both views can produce, so a regression in either
    // view's mapping — not just a drift inside the shared table — fails
    // this test. Falsify by changing one arm of either
    // `RowStatusKind::status_glyph` or `StatusGlyph::for_rec_status`.
    #[test]
    fn explorer_and_deep_compare_render_overlapping_concepts_identically() {
        use forskscope_core::dir::RecStatus;

        let pairs = [
            (RowStatusKind::Equal, RecStatus::Equal),
            (RowStatusKind::Different, RecStatus::Changed),
            (RowStatusKind::Computing, RecStatus::Computing),
            (RowStatusKind::Error, RecStatus::Unreadable),
            (RowStatusKind::LeftOnly, RecStatus::LeftOnly),
            (RowStatusKind::RightOnly, RecStatus::RightOnly),
        ];
        for (explorer_kind, deep_status) in pairs {
            let explorer = explorer_kind.status_glyph();
            let deep = StatusGlyph::for_rec_status(deep_status);
            assert_eq!(
                explorer, deep,
                "{explorer_kind:?} (Explorer) and {deep_status:?} (Deep \
                 Compare) must render the same concept"
            );
        }
    }
}
