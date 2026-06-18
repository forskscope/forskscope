//! `Notice` — a styled inline message strip used across views and overlays.
//!
//! Variants:
//! - Default (`notice`)           — neutral/info; used for read-only labels and
//!                                  short status lines.
//! - Ok (`notice notice-ok`)      — positive; used for success or equality
//!                                  confirmations.
//! - Warning (`notice notice-warn`) — amber; used for alerts that don't block
//!                                   work.
//! - Error (`notice notice-err`)  — red; used for failure messages.
//!
//! Call sites that previously wrote `p { class: "notice notice-ok", … }` inline
//! now use `Notice { kind: NoticeKind::Ok, … }` for consistency.

use dioxus::prelude::*;

/// Severity / styling variant for [`Notice`].
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum NoticeKind {
    #[default]
    Info,
    Ok,
    Warning,
    Error,
}

impl NoticeKind {
    fn css_class(self) -> &'static str {
        match self {
            Self::Info    => "notice",
            Self::Ok      => "notice notice-ok",
            Self::Warning => "notice notice-warn",
            Self::Error   => "notice notice-err",
        }
    }

    /// ARIA role appropriate for this severity.
    fn role(self) -> Option<&'static str> {
        match self {
            Self::Warning | Self::Error => Some("alert"),
            _ => None,
        }
    }
}

/// A styled single-line or short-paragraph notice strip.
///
/// ```rust,ignore
/// Notice { kind: NoticeKind::Ok, "Files are identical" }
/// Notice { kind: NoticeKind::Warning, "Large file — inline diff disabled." }
/// ```
#[component]
pub fn Notice(kind: NoticeKind, children: Element) -> Element {
    let class = kind.css_class();
    match kind.role() {
        Some(role) => rsx! { p { class, role, {children} } },
        None       => rsx! { p { class, {children} } },
    }
}
