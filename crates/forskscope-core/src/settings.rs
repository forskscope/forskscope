//! Settings display types (RFC-009 §4, §6, §9, §10).
//!
//! `UserSettings` (RFC-031's schema v1 record, wrapped in `VersionedEnvelope`)
//! was never shipped to users — no released version ever wrote one — and was
//! removed by RFC-076's 2026-08-03 amendment (patch 5) along with its
//! migration path. This module now only holds the display types
//! (`ThemeId`, `LocaleId`, `Density`, `FontFamilySetting`, `ThemeTokens`)
//! that `forskscope-ui`/`forskscope-ui-logic` and the canonical
//! `persist::schema::settings::PersistedSettings` still depend on.

pub mod display;
pub use display::{Density, FontFamilySetting, LocaleId, ThemeId, ThemeTokens};
