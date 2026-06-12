//! Scroll synchronisation view-model for the two-pane diff view (RFC-035).
//!
//! Both diff panes share the same [`ScrollAnchor`] — a `(row_index,
//! row_fraction)` pair. When the user scrolls one pane, the component
//! converts the raw pixel scroll position to a `ScrollAnchor`, stores it,
//! and derives the other pane's `scrollTop` from it.
//!
//! This module contains the pure arithmetic: no DOM, no Dioxus.
//!
//! ## Scroll model
//!
//! All aligned rows have the same height (`row_height_px`). Given:
//!
//! ```text
//! scroll_top_px  = row_index * row_height_px + row_fraction * row_height_px
//!                = (row_index + row_fraction) * row_height_px
//! ```
//!
//! Inversion (pixel → anchor):
//!
//! ```text
//! row_index    = floor(scroll_top_px / row_height_px)
//! row_fraction = frac(scroll_top_px  / row_height_px)
//! ```
//!
//! This model is valid when row heights are uniform — which is guaranteed by
//! the current diff CSS (`height: calc(var(--line-height) * N)` per hunk,
//! uniform line height). If variable-height rows are introduced in future,
//! this module needs to be updated.

use forskscope_core::line_map::ScrollAnchor;

// ── ScrollSyncState ───────────────────────────────────────────────────────────

/// The shared scroll state for the two diff panes.
///
/// Stored once in the diff workspace component. Both panes derive their
/// `scrollTop` from this value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollSyncState {
    /// The current scroll anchor — shared across left and right panes.
    pub anchor:         ScrollAnchor,
    /// Uniform row height in CSS pixels. Comes from `--diff-fs * line-height`.
    pub row_height_px:  f32,
    /// Total number of aligned rows in the diff.
    pub total_rows:     usize,
}

impl ScrollSyncState {
    /// Create a state at the top of the diff.
    pub fn at_top(row_height_px: f32, total_rows: usize) -> Self {
        Self {
            anchor: ScrollAnchor::at_top(),
            row_height_px,
            total_rows,
        }
    }

    /// Update the anchor from a raw `scrollTop` pixel value reported by one pane.
    ///
    /// The returned `ScrollSyncState` should replace the stored state; the
    /// other pane can then call [`scroll_top_px`] to get its target position.
    pub fn from_scroll_top(scroll_top_px: f32, row_height_px: f32, total_rows: usize) -> Self {
        let row_height = row_height_px.max(1.0); // guard against zero
        let fractional_row = (scroll_top_px / row_height).max(0.0);
        let row_index = fractional_row.floor() as usize;
        let row_fraction = fractional_row.fract();

        let clamped_index = row_index.min(total_rows.saturating_sub(1));

        Self {
            anchor: ScrollAnchor::clamped(clamped_index, row_fraction),
            row_height_px,
            total_rows,
        }
    }

    /// Compute the `scrollTop` pixel value to apply to the *other* pane.
    pub fn scroll_top_px(&self) -> f32 {
        let row_height = self.row_height_px.max(1.0);
        (self.anchor.row_index as f32 + self.anchor.row_fraction) * row_height
    }

    /// Move the anchor to point at `hunk_first_row` (used by hunk navigation).
    pub fn scroll_to_row(self, row_index: usize) -> Self {
        Self {
            anchor: ScrollAnchor::clamped(
                row_index.min(self.total_rows.saturating_sub(1)),
                0.0,
            ),
            ..self
        }
    }

    /// `true` when both panes are scrolled to the very top.
    pub fn is_at_top(&self) -> bool {
        self.anchor.row_index == 0 && self.anchor.row_fraction < f32::EPSILON
    }

    /// Maximum valid `scrollTop` for the diff pane (in pixels).
    pub fn max_scroll_px(&self, visible_height_px: f32) -> f32 {
        let total_height = self.total_rows as f32 * self.row_height_px.max(1.0);
        (total_height - visible_height_px).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW_H: f32 = 20.0;  // 20 px per row — easy arithmetic

    fn state(total: usize) -> ScrollSyncState {
        ScrollSyncState::at_top(ROW_H, total)
    }

    // ── at_top ────────────────────────────────────────────────────────────────

    #[test]
    fn at_top_is_row_zero_fraction_zero() {
        let s = state(50);
        assert_eq!(s.anchor.row_index, 0);
        assert!(s.anchor.row_fraction < f32::EPSILON);
        assert!(s.is_at_top());
        assert_eq!(s.scroll_top_px(), 0.0);
    }

    // ── from_scroll_top ───────────────────────────────────────────────────────

    #[test]
    fn scroll_top_at_row_boundary_gives_exact_row() {
        let s = ScrollSyncState::from_scroll_top(60.0, ROW_H, 100);
        // 60 / 20 = 3.0 exactly → row 3, fraction 0
        assert_eq!(s.anchor.row_index, 3);
        assert!(s.anchor.row_fraction < f32::EPSILON);
    }

    #[test]
    fn scroll_top_mid_row_gives_correct_fraction() {
        let s = ScrollSyncState::from_scroll_top(70.0, ROW_H, 100);
        // 70 / 20 = 3.5 → row 3, fraction 0.5
        assert_eq!(s.anchor.row_index, 3);
        assert!((s.anchor.row_fraction - 0.5).abs() < 1e-4);
    }

    #[test]
    fn scroll_top_zero_is_at_top() {
        let s = ScrollSyncState::from_scroll_top(0.0, ROW_H, 100);
        assert!(s.is_at_top());
    }

    #[test]
    fn negative_scroll_top_clamps_to_zero() {
        let s = ScrollSyncState::from_scroll_top(-10.0, ROW_H, 100);
        assert_eq!(s.anchor.row_index, 0);
        assert!(s.anchor.row_fraction < f32::EPSILON);
    }

    // ── scroll_top_px round-trip ──────────────────────────────────────────────

    #[test]
    fn from_scroll_top_and_back_is_identity() {
        let input_px = 135.0_f32;
        let s = ScrollSyncState::from_scroll_top(input_px, ROW_H, 100);
        let back = s.scroll_top_px();
        assert!((back - input_px).abs() < 0.5,
            "round-trip: input={input_px} back={back}");
    }

    #[test]
    fn scroll_top_px_at_row_5_is_100() {
        let s = ScrollSyncState::from_scroll_top(100.0, ROW_H, 50);
        assert_eq!(s.scroll_top_px(), 100.0);
    }

    // ── scroll_to_row ─────────────────────────────────────────────────────────

    #[test]
    fn scroll_to_row_sets_anchor_to_row() {
        let s = state(50).scroll_to_row(7);
        assert_eq!(s.anchor.row_index, 7);
        assert!(s.anchor.row_fraction < f32::EPSILON);
    }

    #[test]
    fn scroll_to_row_beyond_total_clamps() {
        let s = state(10).scroll_to_row(99);
        assert_eq!(s.anchor.row_index, 9); // last valid row
    }

    #[test]
    fn scroll_to_row_preserves_row_height_and_total() {
        let s = state(50).scroll_to_row(3);
        assert_eq!(s.row_height_px, ROW_H);
        assert_eq!(s.total_rows, 50);
    }

    // ── clamp at max row ──────────────────────────────────────────────────────

    #[test]
    fn from_scroll_top_past_end_clamps_to_last_row() {
        let s = ScrollSyncState::from_scroll_top(10_000.0, ROW_H, 10);
        assert_eq!(s.anchor.row_index, 9);
    }

    // ── max_scroll_px ─────────────────────────────────────────────────────────

    #[test]
    fn max_scroll_px_is_total_height_minus_visible() {
        let s = state(100);
        // total = 100 * 20 = 2000, visible = 400 → max = 1600
        assert_eq!(s.max_scroll_px(400.0), 1600.0);
    }

    #[test]
    fn max_scroll_px_clamped_to_zero_when_content_fits() {
        let s = state(5);
        // total = 5 * 20 = 100, visible = 400 → max = 0 (no scroll needed)
        assert_eq!(s.max_scroll_px(400.0), 0.0);
    }

    // ── zero row_height guard ─────────────────────────────────────────────────

    #[test]
    fn zero_row_height_does_not_panic() {
        let s = ScrollSyncState::from_scroll_top(100.0, 0.0, 50);
        // row_height guarded to 1.0 → row_index = 100
        let _ = s.scroll_top_px(); // must not panic
    }
}
