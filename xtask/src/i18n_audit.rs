//! RFC-101 stage B (handoff 114): the i18n audit, rewritten for the
//! catalog format (`crates/forskscope-ui/i18n/<lang>.catalog`) in place
//! of the old `i18n.rs` match table.
//!
//! A small, deliberate *duplicate* of `forskscope-ui`'s own catalog
//! parser (`crates/forskscope-ui/src/i18n/catalog.rs`) rather than a
//! dependency on it — DEC-005 (`xtask/Cargo.toml`): xtask never depends
//! on the crate it audits, so it keeps running (and reporting) even when
//! `forskscope-ui` itself fails to build.
//!
//! Checks (RFC-101 §3's own list):
//! - every supported language has every message;
//! - no message is unused;
//! - each translation uses exactly the English placeholders;
//! - plural forms match each language's rule;
//! - no code joins two translated results, except the named exceptions.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

/// Catalog ids reached only *dynamically* — a `&'static str` field in
/// another crate (`forskscope-compare::BIDI_CONTROLS`, `forskscope-ui-
/// logic`'s theme/contrast `SelectChoice`), never a literal
/// `t(lang, "id")`/`tf(lang, "id", …)` this scan can see. Named here,
/// with the reason, the same way every other gate's allowlist names its
/// exceptions (e.g. `assert_network_paths_are_reviewed`, below in
/// `main.rs`). A known limit of a static-scan tool living outside the
/// crate it checks (DEC-005): if one of these stops being referenced by
/// its own crate, this list alone would not notice.
const DYNAMIC_LOOKUP_IDS: &[&str] = &[
    "bidi-alm",
    "bidi-lrm",
    "bidi-rlm",
    "bidi-lre",
    "bidi-rle",
    "bidi-pdf",
    "bidi-lro",
    "bidi-rlo",
    "bidi-lri",
    "bidi-rli",
    "bidi-fsi",
    "bidi-pdi",
    "theme-choice-system",
    "theme-choice-system-unavailable",
    "theme-choice-dark",
    "theme-choice-light",
    "theme-choice-night",
    "contrast-choice-system",
    "contrast-choice-standard",
    "contrast-choice-high",
    // `forskscope_compare::dir::status_key` (review 174 §3): the one
    // source of a `RecStatus`'s status word, shared by Deep Compare's
    // live view and every report renderer, called with a *computed*
    // `RecStatus` - never a literal id at any of its own call sites.
    // Four of its ten possible ids ("different", "only-on-the-left",
    // "only-on-the-right", "comparing") are also reachable literally
    // elsewhere (`dir_pane.rs`'s own status labels) and would pass
    // without this entry; the other six are reached only this way.
    "only-on-the-left-empty-folder",
    "only-on-the-right-empty-folder",
    "identical",
    "not-compared",
    "unreadable",
    "symlink-not-followed",
    // RFC-101 stage B, review 232 §3.4: `recovery.rs`'s
    // `settings_dialog_text`/`session_dialog_text` return the body's id
    // as data (alongside its args and which ones to isolate), not as a
    // literal passed straight to `t`/`tf` - the caller renders it through
    // `TranslatedText { id: body_id, .. }` so `schema`/`detail` can be
    // wrapped in their own `<bdi>` (a name, a path, error text carried in
    // the file itself). The literal ids live only in each match arm's
    // return tuple.
    "settings-incompatible-schema",
    "settings-corrupt-preserved",
    "settings-migration-failed",
    "session-incompatible-schema",
    "session-corrupt-preserved",
    "session-migration-failed",
    // RFC-101 stage B, review 232 §3.5: forskscope-report's html.rs calls
    // a caller-supplied `tr: &dyn Fn(&str) -> String` closure
    // (`&|s| t(lang, s)`, from forskscope-ui) rather than `t`/`tf`
    // directly — forskscope-report cannot depend on forskscope-ui
    // (RFC-093's crate-boundaries rule) — so every id this closure can
    // receive is invisible to this scan by construction, the same as
    // the dynamic-lookup ids above.
    "report-compare-profile",
    "report-comparison-report",
    "report-files",
    "report-generated",
    "report-how",
    "report-left",
    "report-left-size",
    "report-no-differences-the-two-files-are-identical",
    "report-no-files",
    "report-notes",
    "report-path",
    "report-right",
    "report-right-size",
    "report-sha-256",
    "report-status",
    "report-summary",
    "report-the-changes",
    "report-what-was-compared",
    "report-run-cmd-on-a-file-to-confirm-it-is-the-file-this-report",
    "report-large-file-inline-diff-disabled",
    "report-some-hunks-were-too-large-for-inline-diff",
    "report-profile-name-default",
    "report-profile-name-custom",
    "report-whitespace",
    "report-case",
    "report-newlines",
    "report-algorithm",
    "report-ignored",
    "report-contains-1-direction-control-character",
    "report-contains-n-direction-control-characters",
    "report-n-change",
    "report-n-change-a-line-added",
    "report-n-change-a-lines-added",
    "report-n-change-d-line-removed",
    "report-n-change-d-line-removed-a-line-added",
    "report-n-change-d-line-removed-a-lines-added",
    "report-n-change-d-lines-removed",
    "report-n-change-d-lines-removed-a-line-added",
    "report-n-change-d-lines-removed-a-lines-added",
    "report-n-changes",
    "report-n-changes-a-line-added",
    "report-n-changes-a-lines-added",
    "report-n-changes-d-line-removed",
    "report-n-changes-d-line-removed-a-line-added",
    "report-n-changes-d-line-removed-a-lines-added",
    "report-n-changes-d-lines-removed",
    "report-n-changes-d-lines-removed-a-line-added",
    "report-n-changes-d-lines-removed-a-lines-added",
    "report-n-changed-file-s-content-is-not-shown-in-this-report-the",
    "report-n-changed-file-s-content-is-not-shown-in-this-report-the-2",
    "report-n-changed-files-content-is-not-shown-in-this-report-the-cap",
    "report-n-changed-files-content-is-not-shown-in-this-report-the-cap-2",
];

/// The plural forms each language's CLDR cardinal rule requires, for any
/// message that has forms at all (RFC-101 §2). A language with no entry
/// here would fail with a clear message rather than silently passing —
/// see `required_forms`.
fn required_forms(lang: &str) -> Option<&'static [&'static str]> {
    match lang {
        "en" => Some(&["one", "other"]),
        "ja" => Some(&["other"]),
        _ => None,
    }
}

#[derive(Debug, Clone)]
enum Forms {
    Single(String),
    Plural(Vec<(String, String)>),
}

impl Forms {
    fn texts(&self) -> Vec<&str> {
        match self {
            Self::Single(t) => vec![t.as_str()],
            Self::Plural(fs) => fs.iter().map(|(_, t)| t.as_str()).collect(),
        }
    }

    fn form_names(&self) -> Vec<&str> {
        match self {
            Self::Single(_) => Vec::new(),
            Self::Plural(fs) => fs.iter().map(|(f, _)| f.as_str()).collect(),
        }
    }
}

/// Deliberately permissive compared to `forskscope-ui`'s own parser: a
/// malformed *catalog* line already fails `forskscope-ui`'s own build
/// (its `build.rs`), so this audit's job is the checks above, not to be
/// a second strict parser. A line this can't parse is reported and
/// skipped, not a hard `panic!`, so one bad line doesn't hide every other
/// finding below it.
fn parse_catalog(path: &Path) -> BTreeMap<String, Forms> {
    let source =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut messages: BTreeMap<String, Forms> = BTreeMap::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, text)) = line.split_once(" = ") else {
            eprintln!(
                "{}:{line_no}: not a valid catalog line, skipped",
                path.display()
            );
            continue;
        };
        let (id, form) = match key.split_once('.') {
            Some((id, form)) => (id, Some(form)),
            None => (key, None),
        };
        match form {
            None => {
                messages.insert(id.to_string(), Forms::Single(text.to_string()));
            }
            Some(form) => match messages.get_mut(id) {
                Some(Forms::Plural(forms)) => forms.push((form.to_string(), text.to_string())),
                _ => {
                    messages.insert(
                        id.to_string(),
                        Forms::Plural(vec![(form.to_string(), text.to_string())]),
                    );
                }
            },
        }
    }
    messages
}

fn catalog_files(i18n_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut entries: Vec<(String, PathBuf)> = fs::read_dir(i18n_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", i18n_dir.display()))
        .map(|e| {
            e.unwrap_or_else(|e| panic!("cannot read entry: {e}"))
                .path()
        })
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("catalog"))
        .map(|p| {
            let lang = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_else(|| panic!("{}: not a valid catalog file name", p.display()))
                .to_string();
            (lang, p)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

/// Every `{ $name }` placeholder in `text` — the exact shape
/// `forskscope-ui`'s own `catalog::placeholders` used to define (now
/// this audit's own job, not shipped runtime code).
fn placeholders(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && text[i..].starts_with("{ $") {
            let name_start = i + 3;
            let mut j = name_start;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j > name_start && text[j..].starts_with(" }") {
                found.insert(text[name_start..j].to_string());
                i = j + 2;
                continue;
            }
        }
        i += 1;
    }
    found
}

/// True if `bytes[i..]` starts a standalone call to `prefix` (`b"t("` or
/// `b"tf("`) — not part of a longer identifier (`format(`, `shift(`, …).
/// Byte-level throughout, deliberately, rather than `str` slicing/
/// `chars()`: `i` is a byte offset the callers below reach mid-scan, not
/// necessarily on a UTF-8 char boundary, and `prefix`/identifier
/// characters are always plain ASCII either way.
fn is_call_start(bytes: &[u8], i: usize, prefix: &[u8]) -> bool {
    if !bytes[i..].starts_with(prefix) {
        return false;
    }
    match i.checked_sub(1).and_then(|p| bytes.get(p)) {
        None => true,
        Some(b) => !is_ident_byte(*b),
    }
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `ui::view::TranslatedText { id: "literal-id", .. }` (review 232
/// §3.4) is a second, direct way a literal id reaches a translation -
/// not a call to `t(`/`tf(` at all, so [`find_translate_calls`] can't
/// see it. Scans for the literal field-init shape `id: "..."` anywhere
/// in the file; a false match (some unrelated struct literal that
/// happens to have an `id` field holding a string) only ever adds a
/// spurious entry to the "used" set, never hides a real one, so this
/// stays a plain substring scan rather than requiring it to appear
/// inside a `TranslatedText { .. }` block specifically.
fn find_translated_text_literal_ids(source: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let bytes = source.as_bytes();
    let needle = b"id: \"";
    let mut i = 0;
    while let Some(found) = find_bytes(bytes, needle, i) {
        let start = found + needle.len();
        let mut j = start;
        while j < bytes.len() && bytes[j] != b'"' {
            if bytes[j] == b'\\' {
                j += 2;
            } else {
                j += 1;
            }
        }
        if j <= bytes.len() {
            ids.push(source[start..j.min(bytes.len())].to_string());
        }
        i = j + 1;
    }
    ids
}

/// Scans `source` for every call to `t(...)` or `tf(...)`, returning
/// `(call_start, call_end, id)` for each — `id` is the first string
/// literal inside the call's own balanced parens, which is always the
/// message id for both functions (the only other argument before it,
/// `lang`, is never itself a string literal).
fn find_translate_calls(source: &str) -> Vec<(usize, usize, String)> {
    let mut results = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let prefix = if is_call_start(bytes, i, b"tf(") {
            Some("tf(")
        } else if is_call_start(bytes, i, b"t(") {
            Some("t(")
        } else {
            None
        };
        let Some(prefix) = prefix else {
            i += 1;
            continue;
        };
        let mut cursor = i + prefix.len();
        let mut depth = 1usize;
        let mut id: Option<String> = None;
        while cursor < bytes.len() && depth > 0 {
            match bytes[cursor] {
                b'"' => {
                    let start = cursor + 1;
                    let mut j = start;
                    while j < bytes.len() && bytes[j] != b'"' {
                        if bytes[j] == b'\\' {
                            j += 2;
                        } else {
                            j += 1;
                        }
                    }
                    if id.is_none() {
                        id = Some(source[start..j.min(bytes.len())].to_string());
                    }
                    cursor = j + 1;
                }
                b'(' => {
                    depth += 1;
                    cursor += 1;
                }
                b')' => {
                    depth -= 1;
                    cursor += 1;
                }
                _ => cursor += 1,
            }
        }
        if let Some(id) = id {
            results.push((i, cursor, id));
        }
        // Advance past just the prefix, not the whole call's span: a
        // nested `t(`/`tf(` inside this call's own arguments (e.g.
        // `tf(lang, "id", &[("state", &t(lang, if x { "on" } else {
        // "off" }))], None)`) is itself a real, separate usage site and
        // must still be found — jumping straight to `cursor` would skip
        // right over it.
        i += prefix.len();
    }
    results
}

/// Every `.rs` file under `dir`, recursively — including
/// `src/i18n/mod.rs` itself: its own unit tests are real `t()`/`tf()`
/// call sites (the test-fixture ids below exist only for them), and
/// `catalog.rs`/`plural.rs` contain no `t(`/`tf(`-shaped text to false-
/// positive on, so there is nothing left to gain by excluding the
/// directory the way the old, single-file `i18n.rs` audit did.
fn all_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|e| panic!("cannot read entry: {e}"))
            .path();
        if path.is_dir() {
            all_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Check: no `format!`/`write!`/`push_str` call's own argument list
/// contains two or more `t(`/`tf(` calls — RFC-101 §1's own rule, "no
/// two translated pieces are ever joined in code". The Deep Compare
/// list (review 232 §3.3, `deep_compare.rs`) is the one named-by-rule
/// exception: it joins an *array* of whole messages with `.join()`, not
/// a `format!`/`write!`/`push_str` call, so it is structurally outside
/// what this scanner even looks at — not an allowlist entry, because the
/// pattern it uses never matches the shape this check forbids. The About
/// dialog's diagnostics (review 232 §3.5) call `t`/`tf` zero times
/// (hardcoded English), so they never reach this check either.
fn find_translated_joins(source: &str) -> Vec<usize> {
    let mut violations = Vec::new();
    let bytes = source.as_bytes();
    for macro_name in ["format!(", "write!(", "writeln!(", ".push_str("] {
        let needle = macro_name.as_bytes();
        let mut i = 0;
        while let Some(found) = find_bytes(bytes, needle, i) {
            let start = found;
            let args_start = start + needle.len();
            let (end, count) = count_translate_calls_in_span(source, args_start);
            if count >= 2 {
                violations.push(start);
            }
            i = end.max(args_start + 1);
        }
    }
    violations
}

/// Byte-level substring search from `from` — avoids `str::find` on a
/// re-sliced `&source[i..]`, which panics if `i` isn't a UTF-8 char
/// boundary (every position this module passes in is a byte offset
/// reached mid-scan, not guaranteed to be one).
fn find_bytes(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from > haystack.len() || needle.is_empty() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

/// From `start` (just past an opening `(` already consumed by the
/// caller), scans the balanced span up to its matching `)`, returning
/// that end position and how many `t(`/`tf(` calls begin within it
/// (not counting one found inside a *nested* string literal).
fn count_translate_calls_in_span(source: &str, start: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let mut depth = 1usize;
    let mut cursor = start;
    let mut count = 0;
    while cursor < bytes.len() && depth > 0 {
        match bytes[cursor] {
            b'"' => {
                let mut j = cursor + 1;
                while j < bytes.len() && bytes[j] != b'"' {
                    if bytes[j] == b'\\' {
                        j += 2;
                    } else {
                        j += 1;
                    }
                }
                cursor = j + 1;
            }
            b'(' => {
                depth += 1;
                cursor += 1;
            }
            b')' => {
                depth -= 1;
                cursor += 1;
            }
            _ => {
                if is_call_start(bytes, cursor, b"tf(") || is_call_start(bytes, cursor, b"t(") {
                    count += 1;
                }
                cursor += 1;
            }
        }
    }
    (cursor, count)
}

pub fn run(root: &Path) {
    let i18n_dir = root.join("crates/forskscope-ui/i18n");
    let files = catalog_files(&i18n_dir);
    assert!(
        !files.is_empty(),
        "{}: no *.catalog files found",
        i18n_dir.display()
    );

    let mut catalogs: BTreeMap<String, BTreeMap<String, Forms>> = BTreeMap::new();
    for (lang, path) in &files {
        catalogs.insert(lang.clone(), parse_catalog(path));
    }

    let mut failures: Vec<String> = Vec::new();

    // ── Every language has every message ───────────────────────────────
    let english_ids: BTreeSet<&String> = catalogs
        .get("en")
        .map(|m| m.keys().collect())
        .unwrap_or_default();
    for (lang, messages) in &catalogs {
        if lang == "en" {
            continue;
        }
        let ids: BTreeSet<&String> = messages.keys().collect();
        for missing in english_ids.difference(&ids) {
            failures.push(format!("{lang}.catalog is missing message {missing:?}"));
        }
        for extra in ids.difference(&english_ids) {
            failures.push(format!(
                "{lang}.catalog defines {extra:?}, which en.catalog does not"
            ));
        }
    }

    // ── No message is unused ────────────────────────────────────────────
    let ui_src = root.join("crates/forskscope-ui/src");
    let mut rs_files = Vec::new();
    all_rs_files(&ui_src, &mut rs_files);
    let mut used: BTreeSet<String> = DYNAMIC_LOOKUP_IDS.iter().map(|s| s.to_string()).collect();
    let mut join_violations: Vec<(PathBuf, usize)> = Vec::new();
    for path in &rs_files {
        let source = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        for (_, _, id) in find_translate_calls(&source) {
            used.insert(id);
        }
        for id in find_translated_text_literal_ids(&source) {
            used.insert(id);
        }
        for pos in find_translated_joins(&source) {
            join_violations.push((path.clone(), pos));
        }
    }
    if let Some(en) = catalogs.get("en") {
        for id in en.keys() {
            if !used.contains(id) {
                failures.push(format!("en.catalog's {id:?} is never used by any t()/tf() call site (and is not in DYNAMIC_LOOKUP_IDS)"));
            }
        }
    }

    // ── Placeholders and plural forms match ─────────────────────────────
    for (lang, messages) in &catalogs {
        let Some(required) = required_forms(lang) else {
            failures.push(format!(
                "{lang}.catalog: no required-plural-forms rule is registered for this language in xtask (add one to `required_forms`)"
            ));
            continue;
        };
        for (id, forms) in messages {
            if let Forms::Plural(_) = forms {
                let mut have: Vec<&str> = forms.form_names();
                have.sort_unstable();
                let mut want: Vec<&str> = required.to_vec();
                want.sort_unstable();
                if have != want {
                    failures.push(format!(
                        "{lang}.catalog's {id:?} has plural forms {have:?}, but {lang} requires exactly {want:?}"
                    ));
                }
            }

            if lang != "en"
                && let Some(Forms::Single(_) | Forms::Plural(_)) = Some(forms)
                && let Some(en_forms) = english_ids.contains(id).then(|| &catalogs["en"][id])
            {
                let en_placeholders: BTreeSet<String> = en_forms
                    .texts()
                    .iter()
                    .flat_map(|t| placeholders(t))
                    .collect();
                let this_placeholders: BTreeSet<String> =
                    forms.texts().iter().flat_map(|t| placeholders(t)).collect();
                if en_placeholders != this_placeholders {
                    failures.push(format!(
                        "{lang}.catalog's {id:?} uses placeholders {this_placeholders:?}, \
                         but en.catalog's uses {en_placeholders:?} - every translation must \
                         use exactly the English placeholders"
                    ));
                }
            }
        }
    }

    // ── No two translated pieces joined ─────────────────────────────────
    for (path, pos) in &join_violations {
        let line_no = fs::read_to_string(path)
            .map(|s| s[..*pos].matches('\n').count() + 1)
            .unwrap_or(0);
        failures.push(format!(
            "{}:{line_no}: a format!/write!/push_str call joins two or more t()/tf() results - \
             RFC-101 §1 forbids this except the Deep Compare list and the About dialog's \
             diagnostics (both already outside this scanner's own shape, see its doc comment)",
            path.display()
        ));
    }

    if !failures.is_empty() {
        eprintln!("i18n audit failed:");
        for f in &failures {
            eprintln!("  - {f}");
        }
        process::exit(1);
    }

    let total: usize = catalogs.values().map(|m| m.len()).sum();
    println!(
        "i18n audit passed: {} languages, {} messages in en.catalog, {total} catalog entries total.",
        catalogs.len(),
        english_ids.len(),
    );
}
