//! RFC-076 session schema v2: routing, migration, and validation tests.

use crate::persist::v2::PersistenceLoad;
use crate::persist::v2::session::{
    PersistedComparePairV2, PersistedDirectoryPairV2, PersistedSessionV2, load_session_v2,
};

const SESSION_V0_FIXTURE: &str = include_str!("fixtures/persistence/session-v0.json");
const SESSION_V1_FILEPAIR_FIXTURE: &str =
    include_str!("fixtures/persistence/session-v1-filepair-envelope.json");
const SESSION_V1_DIRPAIR_FIXTURE: &str =
    include_str!("fixtures/persistence/session-v1-dirpair-envelope.json");
const SESSION_V2_FIXTURE: &str = include_str!("fixtures/persistence/session-v2.json");

fn sample_v2() -> PersistedSessionV2 {
    PersistedSessionV2 {
        tabs: vec![PersistedComparePairV2 {
            left: "/tmp/fixtures/left.txt".into(),
            right: "/tmp/fixtures/right.txt".into(),
        }],
        active_tab: Some(0),
        explorer_roots: Some(PersistedDirectoryPairV2 {
            left: "/tmp/fixtures/left-dir".into(),
            right: "/tmp/fixtures/right-dir".into(),
        }),
    }
}

fn envelope_for(schema_version: u32, payload: &serde_json::Value) -> String {
    serde_json::json!({
        "schema_name": "session",
        "schema_version": schema_version,
        "app_version": "0.165.1",
        "created_unix": 1_700_000_000u64,
        "updated_unix": 1_700_000_000u64,
        "payload": payload,
    })
    .to_string()
}

// ── Current (v2) ─────────────────────────────────────────────────────────

#[test]
fn current_v2_envelope_round_trips() {
    let payload = serde_json::to_value(sample_v2()).unwrap();
    let raw = envelope_for(2, &payload);
    match load_session_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value, sample_v2()),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn current_v2_tolerates_unknown_payload_fields() {
    let mut payload = serde_json::to_value(sample_v2()).unwrap();
    payload
        .as_object_mut()
        .unwrap()
        .insert("some_future_field".into(), serde_json::json!(42));
    let raw = envelope_for(2, &payload);
    match load_session_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value, sample_v2()),
        other => panic!("expected Current despite unknown field, got {other:?}"),
    }
}

/// Pins the v2 wire *format* (review 035 C2) — see the settings-side
/// counterpart for why a struct round-trip test cannot substitute for this.
#[test]
fn current_v2_golden_fixture_parses_to_the_exact_expected_struct() {
    let expected = PersistedSessionV2 {
        tabs: vec![
            PersistedComparePairV2 {
                left: "/tmp/fixtures/left.txt".into(),
                right: "/tmp/fixtures/right.txt".into(),
            },
            PersistedComparePairV2 {
                left: "/tmp/fixtures/left-b.txt".into(),
                right: "/tmp/fixtures/right-b.txt".into(),
            },
        ],
        active_tab: Some(1),
        explorer_roots: Some(PersistedDirectoryPairV2 {
            left: "/tmp/fixtures/left-dir".into(),
            right: "/tmp/fixtures/right-dir".into(),
        }),
    };
    match load_session_v2(SESSION_V2_FIXTURE) {
        PersistenceLoad::Current { value } => assert_eq!(value, expected),
        other => panic!("expected Current, got {other:?}"),
    }
}

// ── Legacy v0 ────────────────────────────────────────────────────────────

#[test]
fn legacy_v0_fixture_migrates_every_field_exactly() {
    match load_session_v2(SESSION_V0_FIXTURE) {
        PersistenceLoad::MigratedLegacy {
            value,
            source_backup_required,
        } => {
            assert!(source_backup_required);
            assert_eq!(value.tabs.len(), 2);
            assert_eq!(
                value.tabs[0].left,
                std::path::PathBuf::from("/tmp/fixtures/left-a.txt")
            );
            assert_eq!(
                value.tabs[0].right,
                std::path::PathBuf::from("/tmp/fixtures/right-a.txt")
            );
            assert_eq!(
                value.tabs[1].left,
                std::path::PathBuf::from("/tmp/fixtures/left-b.txt")
            );
            // v0 has no active-tab-index or explorer-root concept.
            assert_eq!(value.active_tab, None);
            assert_eq!(value.explorer_roots, None);
        }
        other => panic!("expected MigratedLegacy, got {other:?}"),
    }
}

#[test]
fn unrecognized_legacy_shape_is_corrupt_not_defaults() {
    let raw = r#"{"totally":"unrelated","shape":true}"#;
    match load_session_v2(raw) {
        PersistenceLoad::Corrupt { .. } => {}
        other => panic!("expected Corrupt for an unrecognized no-schema_name shape, got {other:?}"),
    }
}

// ── Core v1 ──────────────────────────────────────────────────────────────

#[test]
fn core_v1_filepair_envelope_migrates_root_as_the_sole_tab() {
    match load_session_v2(SESSION_V1_FILEPAIR_FIXTURE) {
        PersistenceLoad::MigratedVersion { value, from } => {
            assert_eq!(from, 1);
            assert_eq!(value.tabs.len(), 1);
            assert_eq!(
                value.tabs[0].left,
                std::path::PathBuf::from("/tmp/fixtures/left.txt")
            );
            assert_eq!(
                value.tabs[0].right,
                std::path::PathBuf::from("/tmp/fixtures/right.txt")
            );
            assert_eq!(value.active_tab, Some(0));
            assert_eq!(value.explorer_roots, None);
        }
        other => panic!("expected MigratedVersion{{from: 1}}, got {other:?}"),
    }
}

#[test]
fn core_v1_dirpair_envelope_migrates_root_as_explorer_roots() {
    match load_session_v2(SESSION_V1_DIRPAIR_FIXTURE) {
        PersistenceLoad::MigratedVersion { value, from } => {
            assert_eq!(from, 1);
            assert!(value.tabs.is_empty());
            assert_eq!(value.active_tab, None);
            let roots = value
                .explorer_roots
                .expect("directory_pair root should migrate");
            assert_eq!(
                roots.left,
                std::path::PathBuf::from("/tmp/fixtures/left-dir")
            );
            assert_eq!(
                roots.right,
                std::path::PathBuf::from("/tmp/fixtures/right-dir")
            );
        }
        other => panic!("expected MigratedVersion{{from: 1}}, got {other:?}"),
    }
}

// ── Future / corrupt ─────────────────────────────────────────────────────

#[test]
fn future_schema_version_is_preserved_not_defaulted() {
    let payload = serde_json::to_value(sample_v2()).unwrap();
    let raw = envelope_for(3, &payload);
    match load_session_v2(&raw) {
        PersistenceLoad::FutureVersion { schema, version } => {
            assert_eq!(schema, "session");
            assert_eq!(version, 3);
        }
        other => panic!("expected FutureVersion, got {other:?}"),
    }
}

#[test]
fn wrong_schema_name_is_corrupt() {
    let payload = serde_json::to_value(sample_v2()).unwrap();
    let raw = serde_json::json!({
        "schema_name": "settings",
        "schema_version": 2,
        "app_version": "0.165.1",
        "created_unix": 0,
        "updated_unix": 0,
        "payload": payload,
    })
    .to_string();
    match load_session_v2(&raw) {
        PersistenceLoad::Corrupt { .. } => {}
        other => panic!("expected Corrupt for a schema-name mismatch, got {other:?}"),
    }
}

#[test]
fn malformed_json_is_corrupt() {
    match load_session_v2("not json at all") {
        PersistenceLoad::Corrupt { .. } => {}
        other => panic!("expected Corrupt for malformed JSON, got {other:?}"),
    }
}

// ── Normalization ─────────────────────────────────────────────────────────

#[test]
fn out_of_range_active_tab_normalizes() {
    let mut invalid = sample_v2();
    invalid.active_tab = Some(99);
    let payload = serde_json::to_value(&invalid).unwrap();
    let raw = envelope_for(2, &payload);
    match load_session_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value.active_tab, Some(0)),
        other => panic!("expected Current, got {other:?}"),
    }
}

#[test]
fn active_tab_normalizes_to_none_when_tabs_empty() {
    let mut invalid = sample_v2();
    invalid.tabs = vec![];
    invalid.active_tab = Some(0);
    let payload = serde_json::to_value(&invalid).unwrap();
    let raw = envelope_for(2, &payload);
    match load_session_v2(&raw) {
        PersistenceLoad::Current { value } => assert_eq!(value.active_tab, None),
        other => panic!("expected Current, got {other:?}"),
    }
}
