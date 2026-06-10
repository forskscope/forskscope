//! Versioned envelope and migration policy tests (RFC-031).
//!
//! Covers: envelope serialisation, round-trip parse, migration policy
//! decisions, schema name mapping, error cases, and the payload extraction
//! for nested objects and arrays.

use crate::persist::{
    EnvelopeError, MigrationPolicy, ParsedEnvelope, SchemaName, VersionedEnvelope,
};

// ── Schema name ───────────────────────────────────────────────────────────────

#[test]
fn all_known_schema_names_round_trip_through_as_str_and_from_str() {
    for name in [
        SchemaName::Settings, SchemaName::Profiles, SchemaName::Session,
        SchemaName::BatchManifest, SchemaName::Report,
    ] {
        let s = name.as_str().to_string();
        // Round-trip: from_str produces the same variant.
        let env = VersionedEnvelope::new(SchemaName::from_str_pub(&s), 1, "{}");
        let parsed = VersionedEnvelope::parse(&env.to_json()).unwrap();
        assert_eq!(parsed.schema_name.as_str(), name.as_str(),
            "schema name {s} must round-trip");
    }
}

#[test]
fn unknown_schema_name_preserved_as_unknown() {
    let env = VersionedEnvelope::new(
        SchemaName::Unknown("future_thing".into()), 1, "{}",
    );
    let parsed = VersionedEnvelope::parse(&env.to_json()).unwrap();
    assert!(matches!(parsed.schema_name, SchemaName::Unknown(ref s) if s == "future_thing"));
}

// ── Serialisation ─────────────────────────────────────────────────────────────

#[test]
fn to_json_produces_valid_envelope_json() {
    let env = VersionedEnvelope::new(SchemaName::Settings, 1, "{\"theme\":\"dark\"}");
    let json = env.to_json();
    assert!(json.trim().starts_with('{'), "must start with {{");
    assert!(json.trim().ends_with('}'),   "must end with }}");
    assert!(json.contains("\"schema_name\""),    "must have schema_name");
    assert!(json.contains("\"schema_version\""), "must have schema_version");
    assert!(json.contains("\"app_version\""),    "must have app_version");
    assert!(json.contains("\"payload\""),        "must have payload");
}

#[test]
fn to_json_embeds_payload_verbatim() {
    let payload = r#"{"key":"value","n":42}"#;
    let json = VersionedEnvelope::new(SchemaName::Settings, 1, payload).to_json();
    assert!(json.contains(payload), "payload JSON must appear verbatim in output");
}

// ── Parse ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_round_trips_all_envelope_fields() {
    let env = VersionedEnvelope::with_timestamps(
        SchemaName::Session, 3, "0.50.0", 1_700_000_000, 1_700_000_001,
        "{\"tabs\":[]}",
    );
    let json = env.to_json();
    let parsed = VersionedEnvelope::parse(&json).unwrap();

    assert_eq!(parsed.schema_name.as_str(), "session");
    assert_eq!(parsed.schema_version, 3);
    assert_eq!(parsed.app_version, "0.50.0");
    assert_eq!(parsed.payload_json, "{\"tabs\":[]}");
}

#[test]
fn parse_preserves_nested_object_payload() {
    let payload = r#"{"a":{"b":{"c":1}}}"#;
    let env = VersionedEnvelope::new(SchemaName::Profiles, 1, payload);
    let parsed = VersionedEnvelope::parse(&env.to_json()).unwrap();
    assert_eq!(parsed.payload_json, payload);
}

#[test]
fn parse_preserves_array_payload() {
    let payload = r#"[{"id":1},{"id":2}]"#;
    let env = VersionedEnvelope::new(SchemaName::Report, 1, payload);
    let parsed = VersionedEnvelope::parse(&env.to_json()).unwrap();
    assert_eq!(parsed.payload_json, payload);
}

#[test]
fn parse_empty_payload_object() {
    let env = VersionedEnvelope::new(SchemaName::Settings, 1, "{}");
    let parsed = VersionedEnvelope::parse(&env.to_json()).unwrap();
    assert_eq!(parsed.payload_json, "{}");
}

#[test]
fn parse_missing_schema_name_returns_error() {
    let json = r#"{"schema_version":1,"app_version":"1.0","payload":{}}"#;
    assert!(matches!(
        VersionedEnvelope::parse(json),
        Err(EnvelopeError::MissingField("schema_name"))
    ));
}

#[test]
fn parse_missing_schema_version_returns_error() {
    let json = r#"{"schema_name":"settings","app_version":"1.0","payload":{}}"#;
    assert!(matches!(
        VersionedEnvelope::parse(json),
        Err(EnvelopeError::MissingField("schema_version"))
    ));
}

#[test]
fn parse_missing_payload_returns_error() {
    let json = r#"{"schema_name":"settings","schema_version":1,"app_version":"1.0"}"#;
    assert!(matches!(
        VersionedEnvelope::parse(json),
        Err(EnvelopeError::MissingField("payload"))
    ));
}

// ── Migration policy ──────────────────────────────────────────────────────────

fn parsed(schema_version: u32) -> ParsedEnvelope {
    ParsedEnvelope {
        schema_name:    SchemaName::Settings,
        schema_version,
        app_version:    "0.50.0".into(),
        payload_json:   "{}".into(),
    }
}

#[test]
fn same_version_is_compatible_read() {
    assert_eq!(
        parsed(2).migration_policy(2),
        MigrationPolicy::CompatibleRead
    );
}

#[test]
fn older_schema_requires_forward_migration() {
    let policy = parsed(1).migration_policy(3);
    assert!(matches!(
        policy,
        MigrationPolicy::ForwardMigration { from_version: 1 }
    ));
}

#[test]
fn newer_schema_is_refused() {
    let policy = parsed(5).migration_policy(3);
    assert!(matches!(
        policy,
        MigrationPolicy::NewerSchema { file_version: 5, app_version: 3 }
    ));
}

#[test]
fn unknown_schema_name_yields_unknown_schema_policy() {
    let p = ParsedEnvelope {
        schema_name:    SchemaName::Unknown("mystery".into()),
        schema_version: 1,
        app_version:    "0.50.0".into(),
        payload_json:   "{}".into(),
    };
    assert!(matches!(
        p.migration_policy(1),
        MigrationPolicy::UnknownSchema { ref schema_name } if schema_name == "mystery"
    ));
}

// ── Migration policy predicates ───────────────────────────────────────────────

#[test]
fn compatible_read_is_compatible_and_writable() {
    assert!(MigrationPolicy::CompatibleRead.is_compatible());
    assert!(MigrationPolicy::CompatibleRead.can_write());
}

#[test]
fn forward_migration_is_not_compatible_but_writable() {
    let p = MigrationPolicy::ForwardMigration { from_version: 1 };
    assert!(!p.is_compatible(), "forward migration requires migration, not direct use");
    assert!(p.can_write(), "app may overwrite after migration");
}

#[test]
fn newer_schema_is_not_compatible_and_not_writable() {
    let p = MigrationPolicy::NewerSchema { file_version: 5, app_version: 3 };
    assert!(!p.is_compatible());
    assert!(!p.can_write(), "must not overwrite a file from a newer version");
}

#[test]
fn unknown_schema_is_not_compatible_and_not_writable() {
    let p = MigrationPolicy::UnknownSchema { schema_name: "x".into() };
    assert!(!p.is_compatible());
    assert!(!p.can_write());
}
