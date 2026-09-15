//! Settings form view-model (RFC-009, Slice 5).
//!
//! Provides the *picker metadata* the settings dialog needs: what options
//! appear in the theme `<select>` dropdown, and the shared font-size clamp
//! (F53). No I/O; pure derivation from core's `ThemeId`.
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

use forskscope_core::settings::ThemeId;

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

/// All available theme choices, in display order.
pub fn theme_choices() -> Vec<SelectChoice> {
    vec![
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── theme_choices ─────────────────────────────────────────────────────────

    #[test]
    fn theme_choices_covers_all_three_themes() {
        let choices = theme_choices();
        assert_eq!(choices.len(), 3, "must have exactly three themes");
        let values: Vec<_> = choices.iter().map(|c| c.value).collect();
        assert!(values.contains(&"dark"), "must include dark");
        assert!(values.contains(&"light"), "must include light");
        assert!(values.contains(&"night"), "must include night");
    }

    #[test]
    fn theme_choice_values_match_theme_id_as_str() {
        for choice in theme_choices() {
            let tid = ThemeId::from_id(choice.value)
                .expect("every theme value must parse with ThemeId::from_id");
            assert_eq!(tid.as_str(), choice.value);
        }
    }

    #[test]
    fn theme_choices_labels_are_non_empty() {
        for c in theme_choices() {
            assert!(!c.label.is_empty(), "theme {} must have a label", c.value);
        }
    }

    #[test]
    fn no_duplicate_values_in_theme_choices() {
        let mut values: Vec<_> = theme_choices().iter().map(|c| c.value).collect();
        let before = values.len();
        values.dedup();
        assert_eq!(
            values.len(),
            before,
            "theme choices must have unique values"
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
}
