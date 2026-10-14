//! Session schema v2 (RFC-076 §"Session schema v2").
//!
//! The canonical payload stores restorable path state, not live task
//! identity or unsaved content. RFC-075's runtime concurrency identity
//! (`forskscope_ui_logic::CompareTabId`) is always freshly allocated on
//! restore and is never populated from a persisted identifier — installing a
//! legacy ID as a runtime token would let a restored value validate a task
//! from another process lifetime. This module knows nothing about that
//! runtime identity; it only produces path pairs for the caller to open.
//!
//! Schema v1 (RFC-031's `WorkspaceSession`) was never shipped to users — no
//! released version ever wrote one — and its migration path was removed by
//! RFC-076's 2026-08-03 amendment (patch 5). A v1 envelope is preserved and
//! reported as `Corrupt`, the same as any other unrecognized version.

mod repository;
pub mod runtime;

pub use repository::SessionRepository;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{PersistenceError, PersistenceLoad};

pub const SESSION_SCHEMA_NAME: &str = "session";
pub const SESSION_SCHEMA_VERSION_V2: u32 = 2;

// ── Canonical v2 payload ────────────────────────────────────────────────────

/// `#[derive(Default)]` is exactly the "no session yet" state — empty tabs,
/// no active tab — which is what a repository returns for
/// [`PersistenceLoad::Missing`] when no session file exists yet.
///
/// F67: there is no `explorer_roots` field. One existed through 0.188.x,
/// always written `None` and never read — directory comparisons were never
/// restored, so it recorded nothing a restore could use. Rather than keep
/// writing a value nobody reads, the field is gone; restoring directory
/// comparisons is a design question for a later release (handoff 076 §1),
/// and the field returns, carrying real data, if that release needs it. A
/// file written by an older build that still has `explorer_roots` loads
/// unaffected — an unrecognized payload field is ignored, not an error — see
/// `a_stale_explorer_roots_field_from_an_older_build_is_ignored_not_an_error`
/// in the session schema tests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersistedSession {
    pub tabs: Vec<PersistedComparePair>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_tab: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedComparePair {
    pub left: PathBuf,
    pub right: PathBuf,
}

// ── Legacy v0 DTO (exact mirror of the shipping UI's plain-JSON shape) ─────

/// Mirrors `forskscope_ui::state::session::SessionState` field-for-field.
/// `forskscope-core` cannot depend on `forskscope-ui`, so this is a private,
/// intentionally frozen copy.
#[derive(Debug, Deserialize)]
struct LegacySessionStateV0 {
    tabs: Vec<(String, String)>,
}

// ── Routing ──────────────────────────────────────────────────────────────

/// Load and route a raw session file. Pure: no file I/O, no side effects.
pub fn load_session(raw: &str) -> PersistenceLoad<PersistedSession> {
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => {
            return PersistenceLoad::Corrupt {
                detail: PersistenceError::MalformedJson,
            };
        }
    };
    let Some(obj) = value.as_object() else {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedJson,
        };
    };

    let Some(name_value) = obj.get("schema_name") else {
        return match serde_json::from_value::<LegacySessionStateV0>(value.clone()) {
            Ok(v0) => PersistenceLoad::MigratedLegacy {
                value: migrate_from_v0(v0),
                source_backup_required: true,
            },
            Err(_) => PersistenceLoad::Corrupt {
                detail: PersistenceError::UnrecognizedLegacyShape,
            },
        };
    };
    let Some(name) = name_value.as_str() else {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedJson,
        };
    };
    if name != SESSION_SCHEMA_NAME {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::SchemaNameMismatch {
                found: name.to_string(),
            },
        };
    }
    let Some(version) = obj.get("schema_version").and_then(|v| v.as_u64()) else {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedVersion,
        };
    };
    let version = version as u32;
    if version > SESSION_SCHEMA_VERSION_V2 {
        return PersistenceLoad::FutureVersion {
            schema: name.to_string(),
            version,
        };
    }
    let Some(payload) = obj.get("payload") else {
        return PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedPayload,
        };
    };
    match version {
        SESSION_SCHEMA_VERSION_V2 => {
            match serde_json::from_value::<PersistedSession>(payload.clone()) {
                Ok(v2) => PersistenceLoad::Current {
                    value: normalize(v2),
                },
                Err(_) => PersistenceLoad::Corrupt {
                    detail: PersistenceError::MalformedPayload,
                },
            }
        }
        // Core schema v1 (RFC-031) was never shipped to users. Its migration
        // path is removed (RFC-076's 2026-08-03 amendment); a v1 envelope is
        // preserved and reported as Corrupt like any other unrecognized
        // version, never silently reinterpreted.
        _ => PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedVersion,
        },
    }
}

// ── Migration ────────────────────────────────────────────────────────────

fn migrate_from_v0(v0: LegacySessionStateV0) -> PersistedSession {
    let tabs = v0
        .tabs
        .into_iter()
        .map(|(left, right)| PersistedComparePair {
            left: PathBuf::from(left),
            right: PathBuf::from(right),
        })
        .collect();
    // v0 tracks an ordered list only, no active-tab index; the UI re-derives
    // an active tab after restore.
    normalize(PersistedSession {
        tabs,
        active_tab: None,
    })
}

/// Clamps `active_tab` to a valid index without dropping the tab list.
fn normalize(mut v2: PersistedSession) -> PersistedSession {
    if let Some(i) = v2.active_tab
        && i >= v2.tabs.len()
    {
        v2.active_tab = if v2.tabs.is_empty() { None } else { Some(0) };
    }
    v2
}
