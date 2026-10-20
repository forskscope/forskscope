//! `cargo xtask json-schema` (RFC-094 §3): builds `forskscope-cli`, runs
//! `compare --format json` against a real file pair and a real folder
//! pair, and validates both emitted documents against
//! `docs/schemas/forskscope.comparison.v1.json` — a minimal JSON Schema
//! subset interpreter, not a hardcoded duplicate of the schema's rules, so
//! a change to the schema file changes what this gate accepts. Falsify by
//! renaming a field in `forskscope-report::report::json`'s output (the
//! schema still requires the old name) or in the schema file itself (it
//! then requires a field the program never emits).
//!
//! Only the subset of JSON Schema (2020-12) the schema file actually
//! uses: `$ref` (local `#/$defs/...` only), `oneOf`, `const`, `enum`,
//! `type` (a string or an array of strings, for a nullable field),
//! `properties`/`required`/`additionalProperties`, `items`, `minimum`.

use std::fs;
use std::path::Path;
use std::process::{self, Command};

use serde_json::Value;

pub fn run(root: &Path) {
    let schema_path = root.join("docs/schemas/forskscope.comparison.v1.json");
    let schema_text = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", schema_path.display()));
    let schema: Value = serde_json::from_str(&schema_text)
        .unwrap_or_else(|e| panic!("{}: invalid JSON: {e}", schema_path.display()));

    let build = Command::new("cargo")
        .args(["build", "--quiet", "-p", "forskscope-cli"])
        .current_dir(root)
        .status()
        .unwrap_or_else(|e| panic!("failed to run cargo build -p forskscope-cli: {e}"));
    if !build.success() {
        eprintln!("cargo build -p forskscope-cli failed");
        process::exit(1);
    }
    let bin = root.join("target/debug/forskscope-cli");

    let scratch = root.join("target/json-schema-check");
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).expect("creating the scratch directory");

    let mut problems = Vec::new();

    // A file comparison document.
    fs::write(scratch.join("left.txt"), "one\ntwo\n").unwrap();
    fs::write(scratch.join("right.txt"), "one\nTWO\n").unwrap();
    let file_doc = run_compare_json(&bin, &scratch.join("left.txt"), &scratch.join("right.txt"));
    validate(
        &schema,
        &schema,
        &file_doc,
        "<file document>",
        &mut problems,
    );

    // A directory comparison document, with every one-sided/modified shape
    // the entries array can hold, so the walk below exercises every branch
    // of directory_entry's schema, not just "changed".
    let left_dir = scratch.join("left_dir");
    let right_dir = scratch.join("right_dir");
    fs::create_dir_all(&left_dir).unwrap();
    fs::create_dir_all(&right_dir).unwrap();
    fs::write(left_dir.join("changed.txt"), "a\n").unwrap();
    fs::write(right_dir.join("changed.txt"), "b\n").unwrap();
    fs::write(left_dir.join("left_only.txt"), "c\n").unwrap();
    fs::write(right_dir.join("right_only.txt"), "d\n").unwrap();
    let dir_doc = run_compare_json(&bin, &left_dir, &right_dir);
    validate(
        &schema,
        &schema,
        &dir_doc,
        "<directory document>",
        &mut problems,
    );

    let _ = fs::remove_dir_all(&scratch);

    if !problems.is_empty() {
        for problem in &problems {
            eprintln!("{problem}");
        }
        eprintln!(
            "json-schema check failed: {} problem(s) against docs/schemas/forskscope.comparison.v1.json",
            problems.len()
        );
        process::exit(1);
    }

    println!(
        "json-schema check passed: a real file comparison and a real directory comparison, \
         both run through forskscope-cli compare --format json, validate against \
         docs/schemas/forskscope.comparison.v1.json."
    );
}

fn run_compare_json(bin: &Path, left: &Path, right: &Path) -> Value {
    let output = Command::new(bin)
        .args([
            "compare",
            left.to_str().expect("utf-8 scratch path"),
            right.to_str().expect("utf-8 scratch path"),
            "--format",
            "json",
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    // Exit code 1 ("different") is expected and not a failure here - only
    // a missing/unparseable document is.
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{} compare --format json did not print valid JSON: {e}\nstdout: {}\nstderr: {}",
            bin.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

// ── Minimal JSON Schema subset validator ────────────────────────────────────

fn validate(
    schema_root: &Value,
    schema: &Value,
    value: &Value,
    path: &str,
    problems: &mut Vec<String>,
) {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        validate(
            schema_root,
            resolve_ref(schema_root, reference),
            value,
            path,
            problems,
        );
        return;
    }

    if let Some(one_of) = schema.get("oneOf").and_then(Value::as_array) {
        let mut matched = 0usize;
        let mut closest: Option<Vec<String>> = None;
        for branch in one_of {
            let mut branch_problems = Vec::new();
            validate(schema_root, branch, value, path, &mut branch_problems);
            if branch_problems.is_empty() {
                matched += 1;
            } else if closest
                .as_ref()
                .is_none_or(|c| branch_problems.len() < c.len())
            {
                closest = Some(branch_problems);
            }
        }
        match matched {
            1 => {}
            0 => {
                problems.push(format!(
                    "{path}: no `oneOf` branch matched; the closest branch's own problems:"
                ));
                problems.extend(closest.unwrap_or_default());
            }
            n => problems.push(format!(
                "{path}: expected exactly one `oneOf` branch to match, {n} did"
            )),
        }
        return;
    }

    if let Some(expected) = schema.get("const") {
        if value != expected {
            problems.push(format!(
                "{path}: expected the constant {expected}, got {value}"
            ));
        }
        return;
    }

    if let Some(allowed) = schema.get("enum").and_then(Value::as_array) {
        if !allowed.contains(value) {
            problems.push(format!(
                "{path}: {value} is not one of the allowed enum values"
            ));
        }
        return;
    }

    if let Some(ty) = schema.get("type")
        && !type_matches(ty, value)
    {
        problems.push(format!(
            "{path}: wrong type (schema wants {ty}, value is {value})"
        ));
        return;
    }

    if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64)
        && let Some(n) = value.as_f64()
        && n < minimum
    {
        problems.push(format!(
            "{path}: {n} is below the schema's minimum of {minimum}"
        ));
    }

    if let Some(obj) = value.as_object() {
        let properties = schema.get("properties").and_then(Value::as_object);
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required {
                let key = key.as_str().expect("schema `required` entries are strings");
                if !obj.contains_key(key) {
                    problems.push(format!("{path}: missing required field `{key}`"));
                }
            }
        }
        for (key, child) in obj {
            if let Some(child_schema) = properties.and_then(|p| p.get(key)) {
                validate(
                    schema_root,
                    child_schema,
                    child,
                    &format!("{path}.{key}"),
                    problems,
                );
                continue;
            }
            match schema.get("additionalProperties") {
                Some(Value::Bool(false)) => {
                    problems.push(format!(
                        "{path}: unexpected field `{key}` (schema renamed or removed it?)"
                    ));
                }
                Some(additional) if additional.is_object() => {
                    validate(
                        schema_root,
                        additional,
                        child,
                        &format!("{path}.{key}"),
                        problems,
                    );
                }
                _ => {}
            }
        }
    }

    if let Some(arr) = value.as_array()
        && let Some(items_schema) = schema.get("items")
    {
        for (i, item) in arr.iter().enumerate() {
            validate(
                schema_root,
                items_schema,
                item,
                &format!("{path}[{i}]"),
                problems,
            );
        }
    }
}

/// `type` is either one JSON Schema type name, or an array of names (a
/// nullable field, e.g. `["string", "null"]`) - matches if `value` fits
/// any one of them.
fn type_matches(ty: &Value, value: &Value) -> bool {
    let matches_one = |name: &str| match name {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        other => panic!("unsupported JSON Schema type name `{other}` in this minimal validator"),
    };
    match ty {
        Value::String(s) => matches_one(s),
        Value::Array(names) => names.iter().filter_map(Value::as_str).any(matches_one),
        other => panic!("unsupported JSON Schema `type` shape: {other}"),
    }
}

/// Resolves a local `#/$defs/...`-style reference against `root`. This
/// validator supports only same-document refs - the only kind the schema
/// file uses.
fn resolve_ref<'a>(root: &'a Value, reference: &str) -> &'a Value {
    let pointer = reference
        .strip_prefix('#')
        .unwrap_or_else(|| panic!("unsupported non-local $ref `{reference}`"));
    root.pointer(pointer)
        .unwrap_or_else(|| panic!("$ref `{reference}` does not resolve in the schema document"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tiny_schema() -> Value {
        json!({
            "$defs": {
                "side": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "size"],
                    "properties": {
                        "name": { "type": "string" },
                        "size": { "type": ["integer", "null"], "minimum": 0 }
                    }
                }
            },
            "oneOf": [{ "$ref": "#/$defs/side" }]
        })
    }

    #[test]
    fn a_document_matching_the_schema_validates_clean() {
        let schema = tiny_schema();
        let doc = json!({ "name": "a.txt", "size": 3 });
        let mut problems = Vec::new();
        validate(&schema, &schema, &doc, "doc", &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_nullable_field_accepts_null() {
        let schema = tiny_schema();
        let doc = json!({ "name": "a.txt", "size": null });
        let mut problems = Vec::new();
        validate(&schema, &schema, &doc, "doc", &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_renamed_field_is_rejected() {
        // RFC-094 §3 acceptance criterion 3's own falsification: "a gate
        // validates emitted documents ... falsify by a renamed field".
        let schema = tiny_schema();
        let doc = json!({ "full_name": "a.txt", "size": 3 });
        let mut problems = Vec::new();
        validate(&schema, &schema, &doc, "doc", &mut problems);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("missing required field `name`")),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains("unexpected field `full_name`")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_wrong_type_is_rejected() {
        let schema = tiny_schema();
        let doc = json!({ "name": "a.txt", "size": "three" });
        let mut problems = Vec::new();
        validate(&schema, &schema, &doc, "doc", &mut problems);
        assert!(
            problems.iter().any(|p| p.contains("wrong type")),
            "{problems:?}"
        );
    }

    #[test]
    fn the_real_schema_file_parses_and_its_defs_all_resolve() {
        let root = crate::workspace_root();
        let schema_text =
            fs::read_to_string(root.join("docs/schemas/forskscope.comparison.v1.json")).unwrap();
        let schema: Value = serde_json::from_str(&schema_text).unwrap();
        // Every $ref in the document resolves - walk the whole tree.
        fn walk(root: &Value, node: &Value) {
            if let Some(r) = node.get("$ref").and_then(Value::as_str) {
                resolve_ref(root, r);
            }
            if let Some(obj) = node.as_object() {
                for v in obj.values() {
                    walk(root, v);
                }
            }
            if let Some(arr) = node.as_array() {
                for v in arr {
                    walk(root, v);
                }
            }
        }
        walk(&schema, &schema);
    }
}
