//! Display primitive settings: theme, density, font family, and locale (RFC-018).


/// Schema version for the settings file.
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

// ── Theme ─────────────────────────────────────────────────────────────────────

/// Named theme. Drives the CSS variable set injected by the Dioxus app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeId {
    #[default]
    Dark,
    Light,
    Night,
}

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark  => "dark",
            Self::Light => "light",
            Self::Night => "night",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "dark"  => Some(Self::Dark),
            "light" => Some(Self::Light),
            "night" => Some(Self::Night),
            _       => None,
        }
    }

    /// All built-in themes in display order.
    pub fn all() -> &'static [Self] {
        &[Self::Dark, Self::Light, Self::Night]
    }
}

/// Semantic CSS variable names for one theme (RFC-009 §6).
///
/// The Dioxus app converts these to `--var-name: value` injected at the
/// `:root` level. Core does not render CSS; it only maps the token names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeTokens {
    pub app_bg:          String,
    pub panel_bg:        String,
    pub text_primary:    String,
    pub text_muted:      String,
    pub border_subtle:   String,
    pub diff_equal_bg:   String,
    pub diff_insert_bg:  String,
    pub diff_delete_bg:  String,
    pub diff_replace_bg: String,
    pub focus_ring:      String,
    pub warning:         String,
    pub error:           String,
}

impl ThemeTokens {
    /// Returns the CSS variable names for the given theme.
    ///
    /// The actual color values are applied by the Dioxus app's CSS layer.
    /// Core exposes only the stable variable names so tests can verify
    /// completeness without depending on color values.
    pub fn css_var_names(theme: ThemeId) -> Vec<(&'static str, &'static str)> {
        let prefix = match theme {
            ThemeId::Dark  => "dark",
            ThemeId::Light => "light",
            ThemeId::Night => "night",
        };
        vec![
            ("--fsk-app-bg",          prefix),
            ("--fsk-panel-bg",        prefix),
            ("--fsk-text-primary",    prefix),
            ("--fsk-text-muted",      prefix),
            ("--fsk-border-subtle",   prefix),
            ("--fsk-diff-equal-bg",   prefix),
            ("--fsk-diff-insert-bg",  prefix),
            ("--fsk-diff-delete-bg",  prefix),
            ("--fsk-diff-replace-bg", prefix),
            ("--fsk-focus-ring",      prefix),
            ("--fsk-warning",         prefix),
            ("--fsk-error",           prefix),
        ]
    }
}

// ── Density / display ─────────────────────────────────────────────────────────

/// UI layout density (RFC-009 §4 `AppearanceSettings`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
            Self::Compact     => "compact",
            Self::Spacious    => "spacious",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "comfortable" => Some(Self::Comfortable),
            "compact"     => Some(Self::Compact),
            "spacious"    => Some(Self::Spacious),
            _             => None,
        }
    }
}

/// Font family setting for UI and diff panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontFamilySetting {
    #[default]
    SystemMono,
    SystemSans,
    SystemSerif,
}

impl FontFamilySetting {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemMono  => "system-mono",
            Self::SystemSans  => "system-sans",
            Self::SystemSerif => "system-serif",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "system-mono"  => Some(Self::SystemMono),
            "system-sans"  => Some(Self::SystemSans),
            "system-serif" => Some(Self::SystemSerif),
            _              => None,
        }
    }
}

// ── Locale ────────────────────────────────────────────────────────────────────

/// Language / locale identifier (RFC-009 §9 "Localization Model").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocaleId(pub String);

impl LocaleId {
    pub fn english()  -> Self { Self("en".into()) }
    pub fn japanese() -> Self { Self("ja".into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

