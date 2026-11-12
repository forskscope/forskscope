//! Settings form view-model (RFC-009, Slice 5).
//!
//! Provides the *picker metadata* the settings dialog needs: what options
//! appear in the theme `<select>` dropdown, the shared font-size clamp
//! (F53), and the text-size step logic Ctrl+=/Ctrl+- use (RFC-097 stage 1,
//! handoff 095 §3). No I/O; pure derivation from core's `ThemeId`/
//! `TEXT_SIZE_STEPS`.
//!
//! ## Why a separate module?
//!
//! The settings dialog must render a stable option list (theme). Hard-coding
//! it in the Dioxus component means the list is untestable and duplicated.
//! This module makes it testable: the option list has a test that verifies
//! completeness and uniqueness.
//!
//! ## F75/F53: what used to live here and does not anymore
//!
//! This module previously also carried `density_choices`, `font_family_choices`,
//! `find_active`, `ProfileChoice`/`profile_presets`, `validate_font_size`, and
//! `validate_context_lines` — none consumed by `forskscope-ui` (F75's
//! connectivity audit). Each was checked against the actual UI rather than
//! assumed dead:
//!
//! - `density_choices`/`font_family_choices` describe settings
//!   (`Density`/`FontFamilySetting`, "appearance" font — distinct from the
//!   diff-pane font the settings modal does expose) that persist in
//!   `PersistedSettings` but have no control anywhere in the shipped UI.
//!   Wiring them would mean building that UI, which F75's handoff put out
//!   of scope; deleted rather than left as an untested promise.
//! - `find_active` had no caller once `density_choices`/`font_family_choices`
//!   were gone — the theme `<select>` below needs no "find the active
//!   option" helper; a native `<select>` highlights the matching `value`
//!   itself.
//! - `ProfileChoice`/`profile_presets` were explicitly documented (by their
//!   own since-deleted doc comment) as view-models for RFC-028's toolbar
//!   profile picker, deferred post-v1 — the same "delete it, it returns
//!   with its feature" call the handoff made explicitly for the conflict
//!   workspace and command palette view-models.
//! - `validate_font_size` returned `Result<u8, (u32, u32)>` for a
//!   validation-with-error-message UI that was never built — the modal
//!   clamps silently (see `clamp_font_size` below), which is the behaviour
//!   actually shipped.
//! - `validate_context_lines` checked a 0-20 range no caller needed: the
//!   modal's context-lines control is a fixed `<select>` (0/3/5/10), not a
//!   free-entry field, so nothing was ever out of range to validate.
//!
//! ## Scope
//!
//! This module covers *display* metadata — labels and identifiers. It does not
//! re-implement persistence (`persist::schema::settings::SettingsRepository`)
//! or CSS injection (`ThemeId::css_var_names`), which live in core.

use forskscope_desktop::settings::ThemeId;
use forskscope_desktop::settings::display::{Contrast, TEXT_SIZE_STEPS, nearest_text_size_step};

// ── Generic choice ────────────────────────────────────────────────────────────

/// One option in a `<select>` picker.
///
/// `value` is passed to the `<option value="…">` attribute; `label` is the
/// visible text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectChoice {
    pub value: &'static str,
    pub label: &'static str,
}

// ── Theme choices ─────────────────────────────────────────────────────────────

/// All available theme choices, in display order. `system_offered` is the
/// caller's own platform/runtime decision (RFC-097 stage 2 §2.1, decided
/// from measurement: Windows always; Linux once the settings portal has
/// answered this session; not yet on macOS) — this function stays pure
/// derivation, with no OS check of its own.
///
/// `saved_is_unavailable_system` (review 211 §3): the *saved* theme is
/// `System`, but it isn't offered in *this* session (no portal answered
/// in time, or an unmeasured platform) — the choice still has to appear,
/// selected, with its value unchanged (never silently reset to `Dark`,
/// the class of bug 0.193.0 fixed: "dropdowns show the current value"),
/// but its label says what will actually happen: `Dark` is what renders
/// here. Mutually exclusive with `system_offered` in practice (the
/// caller only ever has one true at a time), but accepting both as plain
/// bools keeps this a pure, exhaustively-testable derivation rather than
/// a three-way enum only one caller builds.
pub fn theme_choices(system_offered: bool, saved_is_unavailable_system: bool) -> Vec<SelectChoice> {
    let mut choices = vec![];
    if system_offered {
        choices.push(SelectChoice {
            value: ThemeId::System.as_str(),
            label: "System",
        });
    } else if saved_is_unavailable_system {
        choices.push(SelectChoice {
            value: ThemeId::System.as_str(),
            label: "System (not available here; Dark is used)",
        });
    }
    choices.extend([
        SelectChoice {
            value: ThemeId::Dark.as_str(),
            label: "Dark",
        },
        SelectChoice {
            value: ThemeId::Light.as_str(),
            label: "Light",
        },
        SelectChoice {
            value: ThemeId::Night.as_str(),
            label: "Night",
        },
    ]);
    choices
}

// ── Contrast choices ──────────────────────────────────────────────────────────

/// All available contrast choices, in display order. Unlike [`theme_choices`],
/// *System* is offered unconditionally — every platform measured
/// `prefers-contrast: more` reaching the web view (reply 184/review 210).
pub fn contrast_choices() -> Vec<SelectChoice> {
    vec![
        SelectChoice {
            value: Contrast::System.as_str(),
            label: "System",
        },
        SelectChoice {
            value: Contrast::Standard.as_str(),
            label: "Standard",
        },
        SelectChoice {
            value: Contrast::High.as_str(),
            label: "High",
        },
    ]
}

// ── Font size clamp ───────────────────────────────────────────────────────────

/// The diff-pane font-size bound the product actually ships (F53): the
/// settings modal's `<input type="number" min="8" max="32">` and this
/// function are now the only two places a diff-font-size bound is declared
/// — the modal's HTML `min`/`max` are display hints for the number input's
/// spinner; this is the bound actually enforced in Rust, on every edit.
pub const FONT_SIZE_MIN: u32 = 8;
pub const FONT_SIZE_MAX: u32 = 32;

/// Clamp a raw font size to the valid range.
pub fn clamp_font_size(size: u32) -> u8 {
    size.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX) as u8
}

// ── Text size (RFC-097 stage 1, handoff 095 §3) ────────────────────────────────

/// Steps `current` to the next/previous entry in `TEXT_SIZE_STEPS` (Ctrl+=/
/// Ctrl+-), clamping at the ends rather than wrapping: a step past 200% or
/// below 100% returns 200%/100% unchanged, not the opposite end. `current`
/// is rounded to its nearest step first (the same rule persistence applies
/// to a loaded value), so a value that drifted off-step by any means still
/// steps from the step it is closest to, rather than snapping somewhere
/// surprising.
pub fn step_text_size(current: u32, delta: i32) -> u32 {
    let nearest = nearest_text_size_step(current);
    let idx = TEXT_SIZE_STEPS
        .iter()
        .position(|&step| step == nearest)
        .unwrap_or(0);
    let new_idx = (idx as i32 + delta).clamp(0, TEXT_SIZE_STEPS.len() as i32 - 1);
    TEXT_SIZE_STEPS[new_idx as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── theme_choices ─────────────────────────────────────────────────────────

    #[test]
    fn theme_choices_covers_all_three_themes_when_system_is_not_offered() {
        let choices = theme_choices(false, false);
        assert_eq!(choices.len(), 3, "must have exactly three themes");
        let values: Vec<_> = choices.iter().map(|c| c.value).collect();
        assert!(values.contains(&"dark"), "must include dark");
        assert!(values.contains(&"light"), "must include light");
        assert!(values.contains(&"night"), "must include night");
        assert!(
            !values.contains(&"system"),
            "system must not appear when not offered and nothing is saved as system"
        );
    }

    #[test]
    fn theme_choices_adds_system_when_offered() {
        let choices = theme_choices(true, false);
        assert_eq!(choices.len(), 4, "must have exactly four themes");
        let values: Vec<_> = choices.iter().map(|c| c.value).collect();
        assert!(values.contains(&"system"), "must include system");
        assert!(values.contains(&"dark"), "must include dark");
        assert!(values.contains(&"light"), "must include light");
        assert!(values.contains(&"night"), "must include night");
    }

    /// Review 211 §3: a saved `System` that isn't offered here must still
    /// appear, selected, with the real outcome in its own label - never
    /// silently absent, which would show `Dark` as selected in the
    /// `<select>` while the persisted value still says `system` (the
    /// class of bug 0.193.0 fixed).
    #[test]
    fn theme_choices_keeps_a_saved_system_value_with_an_unavailable_label() {
        let choices = theme_choices(false, true);
        assert_eq!(
            choices.len(),
            4,
            "system must still appear, alongside the other three"
        );
        let system = choices
            .iter()
            .find(|c| c.value == "system")
            .expect("a saved, unavailable system choice must still be listed");
        assert_eq!(
            system.label, "System (not available here; Dark is used)",
            "the label must say what will actually render, not just \"System\""
        );
    }

    #[test]
    fn system_offered_wins_over_saved_unavailable_when_somehow_both_are_true() {
        // Not a real caller shape (system_theme_offered and "saved system,
        // unavailable" are mutually exclusive in practice), but the
        // function must still resolve to exactly one System entry, not
        // two, if it's ever called this way.
        let choices = theme_choices(true, true);
        assert_eq!(
            choices.iter().filter(|c| c.value == "system").count(),
            1,
            "must never list System twice"
        );
        assert_eq!(
            choices.iter().find(|c| c.value == "system").unwrap().label,
            "System",
            "offered must win over the unavailable label"
        );
    }

    #[test]
    fn theme_choice_values_match_theme_id_as_str() {
        for choice in theme_choices(true, false) {
            let tid = ThemeId::from_id(choice.value)
                .expect("every theme value must parse with ThemeId::from_id");
            assert_eq!(tid.as_str(), choice.value);
        }
    }

    #[test]
    fn theme_choices_labels_are_non_empty() {
        for c in theme_choices(true, false) {
            assert!(!c.label.is_empty(), "theme {} must have a label", c.value);
        }
    }

    #[test]
    fn no_duplicate_values_in_theme_choices() {
        let mut values: Vec<_> = theme_choices(true, false).iter().map(|c| c.value).collect();
        let before = values.len();
        values.dedup();
        assert_eq!(
            values.len(),
            before,
            "theme choices must have unique values"
        );
    }

    // ── contrast_choices ──────────────────────────────────────────────────────

    #[test]
    fn contrast_choices_covers_all_three_levels() {
        let choices = contrast_choices();
        assert_eq!(choices.len(), 3, "must have exactly three contrast choices");
        let values: Vec<_> = choices.iter().map(|c| c.value).collect();
        assert!(values.contains(&"system"), "must include system");
        assert!(values.contains(&"standard"), "must include standard");
        assert!(values.contains(&"high"), "must include high");
    }

    #[test]
    fn contrast_choice_values_match_contrast_as_str() {
        for choice in contrast_choices() {
            let c = Contrast::from_id(choice.value)
                .expect("every contrast value must parse with Contrast::from_id");
            assert_eq!(c.as_str(), choice.value);
        }
    }

    #[test]
    fn contrast_choices_labels_are_non_empty() {
        for c in contrast_choices() {
            assert!(
                !c.label.is_empty(),
                "contrast {} must have a label",
                c.value
            );
        }
    }

    #[test]
    fn no_duplicate_values_in_contrast_choices() {
        let mut values: Vec<_> = contrast_choices().iter().map(|c| c.value).collect();
        let before = values.len();
        values.dedup();
        assert_eq!(
            values.len(),
            before,
            "contrast choices must have unique values"
        );
    }

    // ── clamp_font_size ───────────────────────────────────────────────────────

    #[test]
    fn clamp_font_size_stays_in_range() {
        assert_eq!(clamp_font_size(0), 8u8);
        assert_eq!(clamp_font_size(14), 14u8);
        assert_eq!(clamp_font_size(100), 32u8);
    }

    #[test]
    fn clamp_font_size_bounds_match_product_shipped_range() {
        // F53: the modal's own <input min="8" max="32"> - if this drifts,
        // the modal and the helper disagree about what the product ships.
        assert_eq!(FONT_SIZE_MIN, 8);
        assert_eq!(FONT_SIZE_MAX, 32);
    }

    // ── step_text_size ────────────────────────────────────────────────────────

    #[test]
    fn step_text_size_moves_one_step_at_a_time() {
        assert_eq!(step_text_size(100, 1), 125);
        assert_eq!(step_text_size(125, 1), 150);
        assert_eq!(step_text_size(150, -1), 125);
    }

    #[test]
    fn step_text_size_stops_at_the_ends_rather_than_wrapping() {
        assert_eq!(
            step_text_size(200, 1),
            200,
            "a step up from the top must stay at the top, not wrap to the bottom"
        );
        assert_eq!(
            step_text_size(100, -1),
            100,
            "a step down from the bottom must stay at the bottom, not wrap to the top"
        );
    }

    #[test]
    fn step_text_size_rounds_an_off_step_value_before_stepping() {
        // 113 is nearest to 125; stepping up from there lands on 150, not
        // on 125 (which a naive "round after stepping" could also produce
        // from a different off-step input, so this specifically checks the
        // round-first order).
        assert_eq!(step_text_size(113, 1), 150);
    }

    #[test]
    fn step_text_size_never_returns_a_value_outside_the_step_list() {
        for &start in &TEXT_SIZE_STEPS {
            for delta in [-1, 1] {
                assert!(TEXT_SIZE_STEPS.contains(&step_text_size(start, delta)));
            }
        }
    }
}
