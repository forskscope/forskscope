//! RFC-076 F26: per-variant serialization assertions for the ten schema
//! enums [`crate::persist::v2::settings::PersistedSettingsV2`] reuses
//! directly as canonical v2 wire types.
//!
//! Five of these enums (`WhitespaceMode`, `NewlineCompareMode`,
//! `CaseSensitivity`, `InlineMode`, `DiffAlgorithm`) only ever appear on
//! `profiles`, a list field — the golden fixture's multiple profile entries
//! already exercise every variant. The other five (`ThemeId`, `Density`,
//! `FontFamilySetting`, `DiffFontFamilySetting`, `NewlinePolicy`) appear
//! only on scalar fields, where a payload holds exactly one value: the
//! fixture can pin only whichever variant happens to be on disk, so twelve
//! variants were provably unpinned (review 036's mutation-testing finding,
//! registered as F26) — renaming `ThemeId::Dark` passed all 969 tests before
//! this file existed. These tests assert every variant's exact wire string
//! directly, independent of any fixture, so a rename of any variant —
//! scalar-field or list-field — fails here regardless of which value a
//! fixture happens to hold.

use crate::diff::{CaseSensitivity, DiffAlgorithm, InlineMode, NewlineCompareMode, WhitespaceMode};
use crate::encoding::NewlinePolicy;
use crate::settings::display::{Density, DiffFontFamilySetting, FontFamilySetting, ThemeId};

fn assert_wire_string<T>(value: T, expected: &str)
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq + std::fmt::Debug + Copy,
{
    let serialized = serde_json::to_value(value).unwrap();
    assert_eq!(
        serialized,
        serde_json::json!(expected),
        "{value:?} must serialize to {expected:?}"
    );
    let round_tripped: T = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_tripped, value);
}

// ── Scalar-field enums (previously unpinned variants) ──────────────────────

#[test]
fn theme_id_variants_have_exact_wire_strings() {
    assert_wire_string(ThemeId::Dark, "dark");
    assert_wire_string(ThemeId::Light, "light");
    assert_wire_string(ThemeId::Night, "night");
}

#[test]
fn density_variants_have_exact_wire_strings() {
    assert_wire_string(Density::Comfortable, "comfortable");
    assert_wire_string(Density::Compact, "compact");
    assert_wire_string(Density::Spacious, "spacious");
}

#[test]
fn font_family_setting_variants_have_exact_wire_strings() {
    assert_wire_string(FontFamilySetting::SystemMono, "system-mono");
    assert_wire_string(FontFamilySetting::SystemSans, "system-sans");
    assert_wire_string(FontFamilySetting::SystemSerif, "system-serif");
}

#[test]
fn diff_font_family_setting_variants_have_exact_wire_strings() {
    assert_wire_string(DiffFontFamilySetting::SystemMono, "system-mono");
    assert_wire_string(DiffFontFamilySetting::SystemSans, "system-sans");
    assert_wire_string(DiffFontFamilySetting::SystemSerif, "system-serif");
    assert_wire_string(DiffFontFamilySetting::CourierNew, "courier-new");
    assert_wire_string(DiffFontFamilySetting::Consolas, "consolas");
}

#[test]
fn newline_policy_variants_have_exact_wire_strings() {
    assert_wire_string(NewlinePolicy::Preserve, "preserve");
    assert_wire_string(NewlinePolicy::ForceLf, "force-lf");
    assert_wire_string(NewlinePolicy::ForceCrlf, "force-crlf");
}

// ── List-field enums (already fully covered by the golden fixture, pinned
//    here too so this file is a complete, fixture-independent reference for
//    all ten schema enums rather than five) ─────────────────────────────────

#[test]
fn diff_algorithm_variants_have_exact_wire_strings() {
    assert_wire_string(DiffAlgorithm::Myers, "myers");
    assert_wire_string(DiffAlgorithm::Patience, "patience");
    assert_wire_string(DiffAlgorithm::Lcs, "lcs");
    assert_wire_string(DiffAlgorithm::Histogram, "histogram");
}

#[test]
fn inline_mode_variants_have_exact_wire_strings() {
    assert_wire_string(InlineMode::None, "none");
    assert_wire_string(InlineMode::Lazy, "lazy");
    assert_wire_string(InlineMode::EagerForSmallHunks, "eager-for-small-hunks");
}

#[test]
fn whitespace_mode_variants_have_exact_wire_strings() {
    assert_wire_string(WhitespaceMode::Significant, "significant");
    assert_wire_string(WhitespaceMode::IgnoreTrailing, "ignore-trailing");
    assert_wire_string(WhitespaceMode::IgnoreAll, "ignore-all");
    assert_wire_string(WhitespaceMode::IgnoreBlankLines, "ignore-blank-lines");
}

#[test]
fn newline_compare_mode_variants_have_exact_wire_strings() {
    assert_wire_string(NewlineCompareMode::Significant, "significant");
    assert_wire_string(NewlineCompareMode::IgnoreDifference, "ignore-difference");
}

#[test]
fn case_sensitivity_variants_have_exact_wire_strings() {
    assert_wire_string(CaseSensitivity::Sensitive, "sensitive");
    assert_wire_string(CaseSensitivity::Insensitive, "insensitive");
}
