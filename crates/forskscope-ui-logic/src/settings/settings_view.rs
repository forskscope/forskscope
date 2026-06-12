//! Settings form view-model (RFC-009, Slice 5).
//!
//! Provides the *picker metadata* the settings dialog needs: what options
//! appear in each `<select>` dropdown, what their display labels are, and
//! validation helpers for numeric fields. No I/O; pure derivation from
//! `UserSettings` types.
//!
//! ## Why a separate module?
//!
//! The settings dialog must render stable option lists (theme, density, font
//! family, compare profile). Hard-coding these in the Dioxus component means
//! the lists are untestable and duplicated. This module makes them testable:
//! every option list has a test that verifies completeness and uniqueness.
//!
//! ## Scope
//!
//! This module covers *display* metadata — labels and identifiers. It does not
//! re-implement persistence (`UserSettings::to_json`) or CSS injection
//! (`ThemeId::css_var_names`), which live in core.

use forskscope_core::diff::CompareProfile;
use forskscope_core::settings::{Density, FontFamilySetting, ThemeId};

// ── Generic choice ────────────────────────────────────────────────────────────

/// One option in a `<select>` picker.
///
/// `value` is passed to the `<option value="…">` attribute; `label` is the
/// visible text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectChoice {
    pub value:  &'static str,
    pub label:  &'static str,
}

// ── Theme choices ─────────────────────────────────────────────────────────────

/// All available theme choices, in display order.
pub fn theme_choices() -> Vec<SelectChoice> {
    vec![
        SelectChoice { value: ThemeId::Dark .as_str(), label: "Dark"  },
        SelectChoice { value: ThemeId::Light.as_str(), label: "Light" },
        SelectChoice { value: ThemeId::Night.as_str(), label: "Night" },
    ]
}

// ── Density choices ───────────────────────────────────────────────────────────

/// All available density choices, in display order.
pub fn density_choices() -> Vec<SelectChoice> {
    vec![
        SelectChoice { value: Density::Comfortable.as_str(), label: "Comfortable" },
        SelectChoice { value: Density::Compact    .as_str(), label: "Compact"     },
        SelectChoice { value: Density::Spacious   .as_str(), label: "Spacious"    },
    ]
}

// ── Font family choices ───────────────────────────────────────────────────────

/// All available font family choices, in display order.
pub fn font_family_choices() -> Vec<SelectChoice> {
    vec![
        SelectChoice { value: FontFamilySetting::SystemMono .as_str(), label: "Monospace"   },
        SelectChoice { value: FontFamilySetting::SystemSans .as_str(), label: "Sans-serif"  },
        SelectChoice { value: FontFamilySetting::SystemSerif.as_str(), label: "Serif"       },
    ]
}

// ── Compare profile choices ───────────────────────────────────────────────────

/// One compare profile option.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileChoice {
    /// Display name shown in the picker.
    pub name:    String,
    /// The full profile — passed to `to_diff_options()` when selected.
    pub profile: CompareProfile,
}

/// All built-in compare profile presets, in display order.
///
/// These are the options shown in the profile picker. Custom profiles added by
/// the user are appended after the presets in the component.
pub fn profile_presets() -> Vec<ProfileChoice> {
    CompareProfile::all_presets().into_iter().map(|p| {
        let name = p.name.clone();
        ProfileChoice { name, profile: p }
    }).collect()
}

// ── Font size validation ──────────────────────────────────────────────────────

/// Validate and clamp a font size to the allowed range (6–50 pt).
///
/// Returns `Ok(clamped)` when the value is in range, `Err(min, max)` when it
/// is out of range (so the caller can show a validation message).
pub fn validate_font_size(size: u32) -> Result<u8, (u32, u32)> {
    const MIN: u32 = 6;
    const MAX: u32 = 50;
    if (MIN..=MAX).contains(&size) {
        Ok(size as u8)
    } else {
        Err((MIN, MAX))
    }
}

/// Clamp a raw font size to the valid range without returning an error.
pub fn clamp_font_size(size: u32) -> u8 {
    size.clamp(6, 50) as u8
}

// ── Context lines validation ──────────────────────────────────────────────────

/// Validate the context-lines setting (0–20 is reasonable).
pub fn validate_context_lines(n: usize) -> Result<usize, (usize, usize)> {
    const MIN: usize = 0;
    const MAX: usize = 20;
    if n <= MAX { Ok(n) } else { Err((MIN, MAX)) }
}

// ── Active choice helper ──────────────────────────────────────────────────────

/// Find the currently selected choice by its value string.
///
/// Returns `None` if the stored value doesn't match any choice — the
/// component should then fall back to the first option.
pub fn find_active<'a>(choices: &'a [SelectChoice], value: &str) -> Option<&'a SelectChoice> {
    choices.iter().find(|c| c.value == value)
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
        assert!(values.contains(&"dark"),  "must include dark");
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

    // ── density_choices ───────────────────────────────────────────────────────

    #[test]
    fn density_choices_covers_all_densities() {
        let choices = density_choices();
        let values: Vec<_> = choices.iter().map(|c| c.value).collect();
        assert!(values.contains(&"comfortable"));
        assert!(values.contains(&"compact"));
        assert!(values.contains(&"spacious"));
    }

    #[test]
    fn density_choice_values_round_trip() {
        for choice in density_choices() {
            let d = Density::from_id(choice.value)
                .expect("every density value must parse");
            assert_eq!(d.as_str(), choice.value);
        }
    }

    // ── font_family_choices ───────────────────────────────────────────────────

    #[test]
    fn font_family_choices_covers_all_families() {
        let choices = font_family_choices();
        let values: Vec<_> = choices.iter().map(|c| c.value).collect();
        assert!(values.contains(&"system-mono"));
        assert!(values.contains(&"system-sans"));
        assert!(values.contains(&"system-serif"));
    }

    #[test]
    fn font_family_choice_values_round_trip() {
        for choice in font_family_choices() {
            let f = FontFamilySetting::from_id(choice.value)
                .expect("every font value must parse");
            assert_eq!(f.as_str(), choice.value);
        }
    }

    // ── profile_presets ───────────────────────────────────────────────────────

    #[test]
    fn profile_presets_is_non_empty() {
        assert!(!profile_presets().is_empty());
    }

    #[test]
    fn profile_presets_names_match_profile_names() {
        for choice in profile_presets() {
            assert_eq!(choice.name, choice.profile.name,
                "ProfileChoice.name must equal profile.name");
        }
    }

    #[test]
    fn profile_presets_all_have_non_empty_names() {
        for choice in profile_presets() {
            assert!(!choice.name.is_empty());
        }
    }

    #[test]
    fn profile_presets_count_matches_all_presets() {
        assert_eq!(profile_presets().len(), CompareProfile::all_presets().len());
    }

    // ── validate_font_size ────────────────────────────────────────────────────

    #[test]
    fn valid_font_size_returns_ok() {
        assert_eq!(validate_font_size(14), Ok(14u8));
        assert_eq!(validate_font_size(6),  Ok(6u8));
        assert_eq!(validate_font_size(50), Ok(50u8));
    }

    #[test]
    fn too_small_font_size_returns_err() {
        assert!(validate_font_size(5).is_err());
        assert!(validate_font_size(0).is_err());
    }

    #[test]
    fn too_large_font_size_returns_err() {
        assert!(validate_font_size(51).is_err());
    }

    #[test]
    fn clamp_font_size_stays_in_range() {
        assert_eq!(clamp_font_size(0),   6u8);
        assert_eq!(clamp_font_size(14),  14u8);
        assert_eq!(clamp_font_size(100), 50u8);
    }

    // ── validate_context_lines ────────────────────────────────────────────────

    #[test]
    fn zero_context_lines_is_valid() {
        assert_eq!(validate_context_lines(0), Ok(0));
    }

    #[test]
    fn twenty_context_lines_is_valid() {
        assert_eq!(validate_context_lines(20), Ok(20));
    }

    #[test]
    fn too_many_context_lines_is_error() {
        assert!(validate_context_lines(21).is_err());
    }

    // ── find_active ───────────────────────────────────────────────────────────

    #[test]
    fn find_active_returns_matching_choice() {
        let choices = theme_choices();
        let found = find_active(&choices, "dark").unwrap();
        assert_eq!(found.value, "dark");
    }

    #[test]
    fn find_active_returns_none_for_unknown_value() {
        let choices = theme_choices();
        assert!(find_active(&choices, "neon").is_none());
    }

    // ── No duplicate values in any list ───────────────────────────────────────

    #[test]
    fn no_duplicate_values_in_any_choice_list() {
        for (name, choices) in [
            ("theme",       theme_choices()),
            ("density",     density_choices()),
            ("font_family", font_family_choices()),
        ] {
            let mut values: Vec<_> = choices.iter().map(|c| c.value).collect();
            let before = values.len();
            values.dedup();
            assert_eq!(values.len(), before,
                "{name} choices must have unique values");
        }
    }
}
