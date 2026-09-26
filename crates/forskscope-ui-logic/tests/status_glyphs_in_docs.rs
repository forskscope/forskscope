//! F115: the status glyphs the user documentation teaches must be the ones
//! `StatusGlyph` renders.
//!
//! Four documents teach the Explorer's and Directory Report's status icons.
//! All four taught `✓` `⚠` `⊙` for a year after F82 replaced them, and nothing
//! failed. `ui/overlay/keybindings.rs` pins the in-app shortcut list against
//! the code; this does the same for the documents, and it checks the
//! **document**, not the code: the expected glyphs and labels are read from
//! `StatusGlyph`, and each document's marked table is parsed and compared to
//! them.
//!
//! Each document holds exactly one table between
//! `<!-- status-glyphs:begin -->` and `<!-- status-glyphs:end -->` (invisible
//! in the rendered book). A row is `| `<glyph>` | <meaning> |`. The test
//! requires, per document:
//! - exactly one marked table;
//! - exactly one row per `StatusGlyph` variant, keyed by the character
//!   `glyph()` returns, with no other character and no duplicate;
//! - each row's meaning to contain that variant's `aria_label()`, so a glyph
//!   cannot be documented under the wrong meaning.
//!
//! The retired alarm glyphs `⚠` and `⊙` must not appear anywhere in these four
//! documents (prose included). `✓` is not checked: features.md and others use
//! it, correctly, for unrelated tables.

use std::collections::BTreeMap;
use std::path::PathBuf;

use forskscope_ui_logic::StatusGlyph;

const DOCS: [&str; 4] = [
    "users/explorer.md",
    "users/directory-compare.md",
    "users/features.md",
    "users/faq.md",
];

const BEGIN: &str = "<!-- status-glyphs:begin -->";
const END: &str = "<!-- status-glyphs:end -->";

/// Every `StatusGlyph` variant. The `match` makes adding a variant a compile
/// error here, so a new glyph cannot be added without the documents being
/// checked for it.
fn all_variants() -> [StatusGlyph; 9] {
    fn exhaustive(g: StatusGlyph) {
        match g {
            StatusGlyph::Equal
            | StatusGlyph::Different
            | StatusGlyph::Computing
            | StatusGlyph::Unreadable
            | StatusGlyph::LeftOnly
            | StatusGlyph::RightOnly
            | StatusGlyph::NotCompared
            | StatusGlyph::Symlink
            | StatusGlyph::MetadataMatch => {}
        }
    }
    let all = [
        StatusGlyph::Equal,
        StatusGlyph::Different,
        StatusGlyph::Computing,
        StatusGlyph::Unreadable,
        StatusGlyph::LeftOnly,
        StatusGlyph::RightOnly,
        StatusGlyph::NotCompared,
        StatusGlyph::Symlink,
        StatusGlyph::MetadataMatch,
    ];
    all.iter().for_each(|g| exhaustive(*g));
    all
}

fn read_doc(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/src")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The rows of the single marked table in `doc`: glyph character -> meaning
/// text, or a description of what is wrong with the markers or a row.
fn documented_rows(doc: &str) -> Result<Vec<(char, String)>, String> {
    let begins = doc.matches(BEGIN).count();
    let ends = doc.matches(END).count();
    if begins != 1 || ends != 1 {
        return Err(format!(
            "expected exactly one {BEGIN} ... {END} region, found {begins} begin and {ends} end markers"
        ));
    }
    let start = doc.find(BEGIN).unwrap() + BEGIN.len();
    let stop = doc.find(END).unwrap();
    if stop < start {
        return Err("the end marker comes before the begin marker".into());
    }

    let mut rows = Vec::new();
    for line in doc[start..stop].lines().map(str::trim) {
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 2 || cells[0].chars().all(|c| matches!(c, '-' | ':' | ' ')) {
            continue; // separator row
        }
        let icon = cells[0];
        let mut chars = icon.trim_matches('`').chars();
        match (
            icon.starts_with('`') && icon.ends_with('`'),
            chars.next(),
            chars.next(),
        ) {
            (true, Some(glyph), None) => rows.push((glyph, cells[1..].join("|").to_lowercase())),
            _ => {
                if icon != "Icon" {
                    return Err(format!(
                        "row `{line}` does not start with a single backticked glyph"
                    ));
                }
            }
        }
    }
    Ok(rows)
}

/// The comparison itself, separated from the file reads so the falsification
/// tests below can feed it a deliberately wrong document.
fn check_doc(name: &str, doc: &str) -> Result<(), String> {
    for retired in ['⚠', '⊙'] {
        if doc.contains(retired) {
            return Err(format!("{name} still uses the retired glyph {retired}"));
        }
    }

    let rows = documented_rows(doc).map_err(|e| format!("{name}: {e}"))?;
    let mut by_glyph: BTreeMap<char, String> = BTreeMap::new();
    for (glyph, meaning) in rows {
        if by_glyph.insert(glyph, meaning).is_some() {
            return Err(format!("{name}: glyph {glyph} is documented twice"));
        }
    }

    let variants = all_variants();
    for variant in variants {
        let glyph = variant.glyph();
        let Some(meaning) = by_glyph.remove(&glyph) else {
            return Err(format!(
                "{name} does not document {glyph} ({})",
                variant.aria_label()
            ));
        };
        let label = variant.aria_label();
        if !meaning.contains(label) {
            return Err(format!(
                "{name} documents {glyph} as {meaning:?}, which does not contain {label:?}"
            ));
        }
    }
    if let Some((glyph, meaning)) = by_glyph.into_iter().next() {
        return Err(format!(
            "{name} documents {glyph} ({meaning:?}), which StatusGlyph does not render"
        ));
    }
    Ok(())
}

#[test]
fn the_four_documents_teach_the_glyphs_the_product_renders() {
    let mut failures = Vec::new();
    for rel in DOCS {
        if let Err(e) = check_doc(rel, &read_doc(rel)) {
            failures.push(e);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The pre-F115 table, as `faq.md` taught it, inside the markers. The checker
/// must reject it — this is what keeps the check from passing vacuously.
#[test]
fn the_old_alarm_glyph_table_is_rejected() {
    let old = format!(
        "{BEGIN}\n| Icon | Meaning |\n|---|---|\n| `✓` | Identical |\n| `⚠` | Different |\n| `⊙` | Running |\n{END}\n"
    );
    let err = check_doc("old-faq.md", &old).unwrap_err();
    assert!(err.contains("retired glyph"), "{err}");
}

#[test]
fn a_glyph_documented_under_the_wrong_meaning_is_rejected() {
    let rows: String = all_variants()
        .iter()
        .map(|v| {
            // Swap the meanings of `=` and `≠`.
            let meaning = match v {
                StatusGlyph::Equal => StatusGlyph::Different.aria_label(),
                StatusGlyph::Different => StatusGlyph::Equal.aria_label(),
                other => other.aria_label(),
            };
            format!("| `{}` | {meaning} |\n", v.glyph())
        })
        .collect();
    let doc = format!("{BEGIN}\n| Icon | Meaning |\n|---|---|\n{rows}{END}\n");
    let err = check_doc("swapped.md", &doc).unwrap_err();
    assert!(err.contains("does not contain"), "{err}");
}

#[test]
fn a_missing_glyph_a_missing_table_and_an_extra_glyph_are_each_rejected() {
    let variants = all_variants();
    let table = |skip: Option<char>, extra: Option<&str>| {
        let mut rows = String::new();
        for v in variants {
            if Some(v.glyph()) != skip {
                rows.push_str(&format!("| `{}` | {} |\n", v.glyph(), v.aria_label()));
            }
        }
        if let Some(e) = extra {
            rows.push_str(e);
        }
        format!("{BEGIN}\n| Icon | Meaning |\n|---|---|\n{rows}{END}\n")
    };

    assert!(check_doc("full.md", &table(None, None)).is_ok());

    let missing = check_doc("m.md", &table(Some(StatusGlyph::Symlink.glyph()), None));
    assert!(missing.unwrap_err().contains("does not document"));

    let extra = check_doc("e.md", &table(None, Some("| `!` | alarm |\n")));
    assert!(extra.unwrap_err().contains("does not render"));

    let none = check_doc("n.md", "no markers here\n");
    assert!(none.unwrap_err().contains("exactly one"));
}
