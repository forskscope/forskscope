//! Session schema v2 (RFC-076 §"Session schema v2").
//!
//! The canonical payload stores restorable path state, not live task
//! identity or unsaved content. RFC-075's [`crate::session::tab::TabId`]-like
//! runtime concurrency identity is always freshly allocated on restore and is
//! never populated from a persisted identifier — installing a legacy ID as a
//! runtime token would let a restored value validate a task from another
//! process lifetime. This module knows nothing about that runtime identity;
//! it only produces path pairs for the caller to open.

mod repository;
pub mod runtime;

pub use repository::SessionRepository;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{PersistenceError, PersistenceLoad};
use crate::session::{WorkspaceRoot, WorkspaceSession};

pub const SESSION_SCHEMA_NAME: &str = "session";
pub const SESSION_SCHEMA_VERSION_V2: u32 = 2;

// ── Canonical v2 payload ────────────────────────────────────────────────────

/// `#[derive(Default)]` is exactly the "no session yet" state — empty tabs,
/// no active tab, no explorer roots — which is what a repository returns for
/// [`PersistenceLoad::Missing`] when no session file exists yet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersistedSessionV2 {
    pub tabs: Vec<PersistedComparePairV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_tab: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explorer_roots: Option<PersistedDirectoryPairV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedComparePairV2 {
    pub left: PathBuf,
    pub right: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedDirectoryPairV2 {
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
pub fn load_session_v2(raw: &str) -> PersistenceLoad<PersistedSessionV2> {
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
            match serde_json::from_value::<PersistedSessionV2>(payload.clone()) {
                Ok(v2) => PersistenceLoad::Current {
                    value: normalize(v2),
                },
                Err(_) => PersistenceLoad::Corrupt {
                    detail: PersistenceError::MalformedPayload,
                },
            }
        }
        1 => match WorkspaceSession::from_payload_json(&payload.to_string()) {
            Ok(v1) => PersistenceLoad::MigratedVersion {
                value: migrate_from_v1(v1),
                from: 1,
            },
            Err(_) => PersistenceLoad::Corrupt {
                detail: PersistenceError::MalformedPayload,
            },
        },
        _ => PersistenceLoad::Corrupt {
            detail: PersistenceError::MalformedVersion,
        },
    }
}

// ── Migration ────────────────────────────────────────────────────────────

fn migrate_from_v0(v0: LegacySessionStateV0) -> PersistedSessionV2 {
    let tabs = v0
        .tabs
        .into_iter()
        .map(|(left, right)| PersistedComparePairV2 {
            left: PathBuf::from(left),
            right: PathBuf::from(right),
        })
        .collect();
    // v0 tracks an ordered list only, no active-tab index and no directory
    // comparison root; the UI re-derives an active tab after restore.
    normalize(PersistedSessionV2 {
        tabs,
        active_tab: None,
        explorer_roots: None,
    })
}

/// v1's own parser always returns an empty `tabs` list — restoring the full
/// tab list was never wired up in v1 (see its own doc comment). The only
/// restorable path information v1 actually carries is `root`, so that is all
/// this can recover: a `FilePair` root becomes the sole compare tab, a
/// `DirectoryPair` root becomes `explorer_roots`, and `Empty` recovers
/// nothing. This is a deliberate, bounded discard, not a bug reintroduced
/// here — v1 files were never written by the shipping UI in the first place.
fn migrate_from_v1(v1: WorkspaceSession) -> PersistedSessionV2 {
    let (tabs, explorer_roots) = match v1.root {
        WorkspaceRoot::Empty => (vec![], None),
        WorkspaceRoot::FilePair(pair) => (
            vec![PersistedComparePairV2 {
                left: pair.left,
                right: pair.right,
            }],
            None,
        ),
        WorkspaceRoot::DirectoryPair(pair) => (
            vec![],
            Some(PersistedDirectoryPairV2 {
                left: pair.left,
                right: pair.right,
            }),
        ),
    };
    let active_tab = if tabs.is_empty() { None } else { Some(0) };
    normalize(PersistedSessionV2 {
        tabs,
        active_tab,
        explorer_roots,
    })
}

/// Clamps `active_tab` to a valid index without dropping the tab list.
fn normalize(mut v2: PersistedSessionV2) -> PersistedSessionV2 {
    if let Some(i) = v2.active_tab
        && i >= v2.tabs.len()
    {
        v2.active_tab = if v2.tabs.is_empty() { None } else { Some(0) };
    }
    v2
}
