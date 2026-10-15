//! `cargo xtask keyboard-docs` (handoff 079 §3, F165): the in-app keyboard
//! reference (`ui/overlay/keybindings.rs`'s `KbRow` rows) and
//! `docs/src/intermediate/keyboard.md`'s tables must name the same set of
//! keys — every key documented in one place and not the other is exactly
//! the drift review 160 found twice (items 1 and 12: Enter's real
//! behaviour, and the Ctrl+/ gating note), the worst-effect class of
//! documentation rot this project has seen.
//!
//! **What this gate does NOT check: `F165`'s lesson, stated plainly so
//! nobody mistakes this for more than it is.** It compares two *lists* of
//! key names. It has no idea what either key actually does, whether the
//! in-app string correctly describes the real handler, or whether
//! `keyboard.md`'s prose is accurate. A key can appear in both places and
//! still be documented wrong on both sides at once — this gate would stay
//! green. It only prevents the narrower, cheaper failure: a key added or
//! renamed in one place and never carried to the other.
//!
//! **Matching.** Each source lists keys in a different shape — the in-app
//! reference groups several keys into one row's `keys: "A / B"` string;
//! `keyboard.md` sometimes puts multiple keys in one Markdown table cell
//! (`**A** / **B**`) and sometimes several keys inside one bold span
//! (`**A / B**`). Both are normalized to the same flat set: split every
//! `keys:` string and every bold span on `" / "`, then strip all
//! whitespace from what's left (so `"Ctrl + Z"` and `"Ctrl+Z"` compare
//! equal). The two sets must then be identical.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process;

/// Every individual key named in `keybindings.rs`'s `KbRow { keys: "...",
/// ... }` rows, split and normalized as the module doc describes.
fn keys_in_app_reference(src: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let marker = "keys:";
    let mut rest = src;
    while let Some(at) = rest.find(marker) {
        let after = &rest[at + marker.len()..];
        // A `KbRow { keys: "..." }` usage: the first non-whitespace
        // character after `keys:` is the opening quote. `fn KbRow(keys:
        // &'static str, ...)`'s own parameter declaration also contains
        // the literal substring `keys:`, immediately followed by `&`, not
        // whitespace-then-quote — skipped by requiring that shape, rather
        // than matched and then scanning forward into the component body
        // for the next unrelated string literal (`class: "kb-row"`,
        // caught live when this gate first ran against the real file).
        let trimmed = after.trim_start();
        if !trimmed.starts_with('"') {
            rest = after;
            continue;
        }
        let after_open = &trimmed[1..];
        let Some(close) = after_open.find('"') else {
            break;
        };
        let raw = &after_open[..close];
        for part in raw.split(" / ") {
            let normalized: String = part.chars().filter(|c| !c.is_whitespace()).collect();
            if !normalized.is_empty() {
                keys.insert(normalized);
            }
        }
        rest = &after_open[close + 1..];
    }
    keys
}

/// Every individual key named in a bold (`**...**`) span inside
/// `keyboard.md`'s Markdown tables' first (Shortcut) column, split and
/// normalized as the module doc describes. A row with no bold span at all
/// (the header row, the `---` separator row, any non-table prose line
/// that happens to start with `|`) contributes nothing, which is exactly
/// what's wanted — no special-casing those rows is needed.
fn keys_in_docs(src: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for line in src.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        // The Shortcut column is everything before the next unescaped `|`
        // after the first one.
        let Some(cell) = line.get(1..).and_then(|rest| rest.split('|').next()) else {
            continue;
        };
        let mut rest = cell;
        while let Some(open) = rest.find("**") {
            let after_open = &rest[open + 2..];
            let Some(close) = after_open.find("**") else {
                break;
            };
            let span = &after_open[..close];
            for part in span.split(" / ") {
                let normalized: String = part.chars().filter(|c| !c.is_whitespace()).collect();
                if !normalized.is_empty() {
                    keys.insert(normalized);
                }
            }
            rest = &after_open[close + 2..];
        }
    }
    keys
}

pub fn run(root: &Path) {
    let app_path = root.join("crates/forskscope-ui/src/ui/overlay/keybindings.rs");
    let docs_path = root.join("docs/src/intermediate/keyboard.md");
    let app_src = fs::read_to_string(&app_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", app_path.display()));
    let docs_src = fs::read_to_string(&docs_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", docs_path.display()));

    let app_keys = keys_in_app_reference(&app_src);
    let doc_keys = keys_in_docs(&docs_src);

    let missing_from_docs: Vec<&String> = app_keys.difference(&doc_keys).collect();
    let missing_from_app: Vec<&String> = doc_keys.difference(&app_keys).collect();

    if !missing_from_docs.is_empty() || !missing_from_app.is_empty() {
        eprintln!("keyboard-docs check failed:");
        for key in &missing_from_docs {
            eprintln!(
                "  - {key:?} is in the in-app keyboard reference ({}) but not in {}",
                app_path.display(),
                docs_path.display()
            );
        }
        for key in &missing_from_app {
            eprintln!(
                "  - {key:?} is in {} but not in the in-app keyboard reference ({})",
                docs_path.display(),
                app_path.display()
            );
        }
        process::exit(1);
    }
    println!(
        "keyboard-docs check passed: the in-app keyboard reference and {} name the same {} \
         keys. (This checks that the two lists agree, not that either is correct — see this \
         gate's own module doc.)",
        docs_path.display(),
        app_keys.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_grouped_keys_string_into_individual_normalized_keys() {
        let src = r#"KbRow { keys: "Ctrl + Tab / Ctrl + PageDown", desc: "x" }"#;
        let keys = keys_in_app_reference(src);
        assert_eq!(
            keys,
            BTreeSet::from(["Ctrl+Tab".to_string(), "Ctrl+PageDown".to_string()])
        );
    }

    #[test]
    fn a_single_key_with_no_slash_is_one_entry() {
        let src = r#"KbRow { keys: "Ctrl + Z", desc: "x" }"#;
        assert_eq!(
            keys_in_app_reference(src),
            BTreeSet::from(["Ctrl+Z".to_string()])
        );
    }

    #[test]
    fn docs_reads_two_separate_bold_spans_in_one_row() {
        let md = "| **Home** / **End** *(tab list focused)*  | First / last tab |";
        assert_eq!(
            keys_in_docs(md),
            BTreeSet::from(["Home".to_string(), "End".to_string()])
        );
    }

    #[test]
    fn docs_reads_two_keys_inside_one_bold_span() {
        let md = "| **↑ / ↓**     | Move focus between rows |";
        assert_eq!(
            keys_in_docs(md),
            BTreeSet::from(["↑".to_string(), "↓".to_string()])
        );
    }

    #[test]
    fn the_header_and_separator_rows_contribute_nothing() {
        let md = "| Shortcut | Action |\n|----------|--------|\n| **F7** | Previous change |";
        assert_eq!(keys_in_docs(md), BTreeSet::from(["F7".to_string()]));
    }

    /// Falsify by deleting one `KbRow` from `keybindings.rs` (or one row
    /// from `keyboard.md`): the real files then disagree and this fails.
    #[test]
    fn the_real_files_agree_today() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask/ is inside the workspace root");
        let app_src =
            fs::read_to_string(root.join("crates/forskscope-ui/src/ui/overlay/keybindings.rs"))
                .unwrap();
        let docs_src = fs::read_to_string(root.join("docs/src/intermediate/keyboard.md")).unwrap();
        let app_keys = keys_in_app_reference(&app_src);
        let doc_keys = keys_in_docs(&docs_src);
        let missing_from_docs: Vec<_> = app_keys.difference(&doc_keys).collect();
        let missing_from_app: Vec<_> = doc_keys.difference(&app_keys).collect();
        assert!(
            missing_from_docs.is_empty() && missing_from_app.is_empty(),
            "missing from docs: {missing_from_docs:?}, missing from app: {missing_from_app:?}"
        );
    }
}
