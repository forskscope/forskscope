//! Display primitive settings: theme, density, font family, and locale (RFC-018).

use serde::{Deserialize, Serialize};

// ── Theme ─────────────────────────────────────────────────────────────────────

/// Named theme. Drives the CSS variable set injected by the Dioxus app.
///
/// Part of the settings v2 on-disk schema (RFC-076): renaming a variant or
/// changing its serde representation is a schema change and requires a
/// version bump, not just a Rust-level rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeId {
    #[default]
    Dark,
    Light,
    Night,
    /// RFC-097 stage 2 §2.1: follows the platform's light/dark preference
    /// where the measurement (reply 184, review 210) found it reaches the
    /// app — Windows always, Linux once the settings portal has answered
    /// this session, never yet on macOS. Resolves to `Dark` or `Light`;
    /// `Night` is never auto-selected. Offered only where measured, and
    /// is the default only for a settings file that never saved a theme
    /// (an existing `Dark`/`Light`/`Night`/`System` is always kept).
    System,
}

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Night => "night",
            Self::System => "system",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "night" => Some(Self::Night),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    /// All built-in themes in display order.
    pub fn all() -> &'static [Self] {
        &[Self::Dark, Self::Light, Self::Night, Self::System]
    }
}

/// Semantic CSS variable names for one theme (RFC-009 §6).
///
/// The Dioxus app converts these to `--var-name: value` injected at the
/// `:root` level. Core does not render CSS; it only maps the token names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeTokens {
    pub app_bg: String,
    pub panel_bg: String,
    pub text_primary: String,
    pub text_muted: String,
    pub border_subtle: String,
    pub diff_equal_bg: String,
    pub diff_insert_bg: String,
    pub diff_delete_bg: String,
    pub diff_replace_bg: String,
    pub focus_ring: String,
    pub warning: String,
    pub error: String,
}

impl ThemeTokens {
    /// Returns the CSS variable names for the given theme.
    ///
    /// The actual color values are applied by the Dioxus app's CSS layer.
    /// Core exposes only the stable variable names so tests can verify
    /// completeness without depending on color values.
    pub fn css_var_names(theme: ThemeId) -> Vec<(&'static str, &'static str)> {
        let prefix = match theme {
            ThemeId::Dark => "dark",
            ThemeId::Light => "light",
            ThemeId::Night => "night",
            // `System` has no token set of its own - it always resolves to
            // `Dark` or `Light` before anything reads CSS variables (see
            // `forskscope_ui::state::settings::resolved_theme_css_class`).
            // This function is otherwise unused (dead per F75's connectivity
            // audit); the arm exists only so the match stays exhaustive.
            ThemeId::System => "dark",
        };
        vec![
            ("--fsk-app-bg", prefix),
            ("--fsk-panel-bg", prefix),
            ("--fsk-text-primary", prefix),
            ("--fsk-text-muted", prefix),
            ("--fsk-border-subtle", prefix),
            ("--fsk-diff-equal-bg", prefix),
            ("--fsk-diff-insert-bg", prefix),
            ("--fsk-diff-delete-bg", prefix),
            ("--fsk-diff-replace-bg", prefix),
            ("--fsk-focus-ring", prefix),
            ("--fsk-warning", prefix),
            ("--fsk-error", prefix),
        ]
    }
}

// ── Colour vision ────────────────────────────────────────────────────────────

/// Colour-vision palette choice (RFC-088): a second, independent axis beside
/// [`ThemeId`] — it combines with every theme rather than adding new ones.
///
/// Part of the settings v2 on-disk schema (RFC-076): renaming a variant or
/// changing its serde representation is a schema change and requires a
/// version bump, not just a Rust-level rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColourVision {
    #[default]
    Standard,
    RedGreenSafe,
}

impl ColourVision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::RedGreenSafe => "red-green-safe",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "standard" => Some(Self::Standard),
            "red-green-safe" => Some(Self::RedGreenSafe),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Standard, Self::RedGreenSafe]
    }
}

// ── Contrast ──────────────────────────────────────────────────────────────────

/// Contrast-level choice (RFC-097 stage 2 §2.2): a third, independent axis
/// beside [`ThemeId`] and [`ColourVision`] — it combines with every theme
/// and colour-vision mode rather than adding new ones, the same way
/// `ColourVision` does (RFC-088).
///
/// `System` is a setting choice, not a render level: it resolves to
/// `Standard` or `High` from `prefers-contrast: more` where the platform
/// reports it (measured on every platform, reply 184/review 210), exactly
/// as `ThemeId::System` resolves to `Dark`/`Light`. `Standard` is the
/// default for both a new install and a settings file saved before this
/// field existed — an accessibility boost is never applied without an
/// explicit choice, matching `ColourVision`'s own default.
///
/// Part of the settings v2 on-disk schema (RFC-076): renaming a variant or
/// changing its serde representation is a schema change and requires a
/// version bump, not just a Rust-level rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Contrast {
    #[default]
    Standard,
    High,
    System,
}

impl Contrast {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::High => "high",
            Self::System => "system",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "standard" => Some(Self::Standard),
            "high" => Some(Self::High),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Standard, Self::High, Self::System]
    }
}

// ── Text size (RFC-097 stage 1) ────────────────────────────────────────────────

/// Whole-app zoom steps, as a percentage of ForskScope's own default — not a
/// reflection of the system's own text scale. Handoff 095's measurement
/// (reply 177) found the platform already applies its own text-scaling
/// factor automatically, on both measured platforms (Windows' *Make text
/// bigger*, GNOME on Wayland's `text-scaling-factor`), with no app code at
/// all; a second, app-level *System* option would have applied it twice,
/// and would otherwise have rendered identically to 100% anyway (review
/// 199) — so there is no *System* choice here, only this plain, relative
/// scale on top of whatever the platform already does.
pub const TEXT_SIZE_STEPS: [u32; 5] = [100, 125, 150, 175, 200];
pub const TEXT_SIZE_DEFAULT: u32 = 100;

/// Snaps an arbitrary persisted value to the nearest step (handoff 095 §4:
/// "a value outside the steps loads as the nearest step"). The five steps
/// are all 25 apart, so no input is ever exactly equidistant between two of
/// them — there is no tie to break.
pub fn nearest_text_size_step(value: u32) -> u32 {
    TEXT_SIZE_STEPS
        .iter()
        .copied()
        .min_by_key(|&step| step.abs_diff(value))
        .unwrap_or(TEXT_SIZE_DEFAULT)
}

// ── Density / display ─────────────────────────────────────────────────────────

/// UI layout density (RFC-009 §4 `AppearanceSettings`).
///
/// Part of the settings v2 on-disk schema (RFC-076); a variant rename is a
/// schema change, not just a Rust-level rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Density {
    /// Default spacing, comfortable for general use.
    #[default]
    Comfortable,
    /// Reduced spacing for power users with many open panes.
    Compact,
    /// Increased spacing for accessibility or high-DPI use.
    Spacious,
}

impl Density {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Comfortable => "comfortable",
            Self::Compact => "compact",
            Self::Spacious => "spacious",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "comfortable" => Some(Self::Comfortable),
            "compact" => Some(Self::Compact),
            "spacious" => Some(Self::Spacious),
            _ => None,
        }
    }
}

/// Font family setting for the application chrome (RFC-018 "appearance font").
/// Distinct from [`DiffFontFamilySetting`], which governs the diff panes.
///
/// Part of the settings v2 on-disk schema (RFC-076); a variant rename is a
/// schema change, not just a Rust-level rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontFamilySetting {
    #[default]
    SystemMono,
    SystemSans,
    SystemSerif,
}

impl FontFamilySetting {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemMono => "system-mono",
            Self::SystemSans => "system-sans",
            Self::SystemSerif => "system-serif",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "system-mono" => Some(Self::SystemMono),
            "system-sans" => Some(Self::SystemSans),
            "system-serif" => Some(Self::SystemSerif),
            _ => None,
        }
    }
}

/// Font family setting for the diff panes (RFC-070, RFC-076 §"Settings schema
/// v2"). Distinct from [`FontFamilySetting`], which governs the app chrome.
/// `CourierNew` and `Consolas` are exact, non-normalized choices — they must
/// not collapse to `SystemMono` during migration from the shipping UI's
/// five-way `DiffFontFamily` setting. A variant rename here is a schema
/// change, not just a Rust-level rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiffFontFamilySetting {
    #[default]
    SystemMono,
    SystemSans,
    SystemSerif,
    CourierNew,
    Consolas,
}

impl DiffFontFamilySetting {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemMono => "system-mono",
            Self::SystemSans => "system-sans",
            Self::SystemSerif => "system-serif",
            Self::CourierNew => "courier-new",
            Self::Consolas => "consolas",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "system-mono" => Some(Self::SystemMono),
            "system-sans" => Some(Self::SystemSans),
            "system-serif" => Some(Self::SystemSerif),
            "courier-new" => Some(Self::CourierNew),
            "consolas" => Some(Self::Consolas),
            _ => None,
        }
    }
}

// ── Locale ────────────────────────────────────────────────────────────────────

/// Language / locale identifier (RFC-009 §9 "Localization Model").
///
/// Part of the settings v2 on-disk schema (RFC-076).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LocaleId(pub String);

impl LocaleId {
    pub fn english() -> Self {
        Self("en".into())
    }
    pub fn japanese() -> Self {
        Self("ja".into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_text_size_step_is_exact_on_a_real_step() {
        for &step in &TEXT_SIZE_STEPS {
            assert_eq!(nearest_text_size_step(step), step);
        }
    }

    #[test]
    fn nearest_text_size_step_snaps_a_value_outside_the_steps() {
        assert_eq!(nearest_text_size_step(0), 100);
        assert_eq!(nearest_text_size_step(110), 100);
        assert_eq!(
            nearest_text_size_step(113),
            125,
            "113 is closer to 125 than 100"
        );
        assert_eq!(nearest_text_size_step(190), 200);
        assert_eq!(nearest_text_size_step(9999), 200);
    }
}
