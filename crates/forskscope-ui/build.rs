//! F180 (handoff 073): `about.rs` reads `FORSKSCOPE_CHANNEL` through
//! `option_env!` at compile time. Without this, Cargo has no reason to
//! know that env var affects this crate's output, and can skip rebuilding
//! it when only that var changes between two builds of the same tree —
//! which is exactly how `PKGBUILD` and `store-build.ps1` select the
//! channel. In practice both are one-shot clean builds, so this would
//! rarely bite in CI, but it is cheap and correct to declare.
//!
//! RFC-101 §3 (handoff 114): also parses every `i18n/<lang>.catalog` file
//! into the `CATALOG_<LANG>` tables `src/i18n/mod.rs` [`include!`]s from
//! `OUT_DIR` — the "parsed at build time" half of the design; "a
//! malformed line fails the build" is this file's `panic!` below, which
//! Cargo turns into exactly that.
//!
//! `include!`s `src/i18n/catalog.rs` rather than depending on this crate's
//! own (not yet built) library — the parser that runs here is the exact
//! same code [`catalog`]'s own unit tests exercise under `cargo test`,
//! not a hand-kept twin of it.
#[path = "src/i18n/catalog.rs"]
mod catalog;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed=FORSKSCOPE_CHANNEL");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let i18n_dir = Path::new(&manifest_dir).join("i18n");
    println!("cargo:rerun-if-changed={}", i18n_dir.display());
    println!("cargo:rerun-if-changed=src/i18n/catalog.rs");

    let mut entries: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(&i18n_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", i18n_dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|e| panic!("cannot read entry: {e}"))
                .path()
        })
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("catalog"))
        .map(|path| {
            let lang = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_else(|| panic!("{}: not a valid catalog file name", path.display()))
                .to_string();
            (lang, path)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        !entries.is_empty(),
        "{}: no *.catalog files found - at least en.catalog and ja.catalog are required",
        i18n_dir.display()
    );

    let mut generated = String::new();
    for (lang, path) in &entries {
        println!("cargo:rerun-if-changed={}", path.display());
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let parsed = catalog::parse(&source).unwrap_or_else(|e| {
            panic!(
                "{}: {e}\n\nFix the line above - a malformed catalog line fails the build \
                 rather than silently dropping a message or guessing its shape.",
                path.display()
            )
        });

        let mut sorted: Vec<(&String, &catalog::MessageForms)> = parsed
            .messages
            .iter()
            .map(|(id, forms)| (id, forms))
            .collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        let const_name = format!("CATALOG_{}", lang.to_uppercase());
        writeln!(
            generated,
            "pub(crate) static {const_name}: &[(&str, StaticForms)] = &["
        )
        .unwrap();
        for (id, forms) in &sorted {
            match forms {
                catalog::MessageForms::Single(text) => {
                    writeln!(generated, "    ({id:?}, StaticForms::Single({text:?})),",).unwrap();
                }
                catalog::MessageForms::Plural(forms) => {
                    write!(generated, "    ({id:?}, StaticForms::Plural(&[").unwrap();
                    for (form, text) in forms {
                        write!(generated, "({form:?}, {text:?}), ").unwrap();
                    }
                    writeln!(generated, "])),").unwrap();
                }
            }
        }
        writeln!(generated, "];\n").unwrap();
    }

    // A quick, build-time sanity check that every language defines the
    // same id set - not a replacement for `cargo xtask i18n`'s own fuller
    // audit (placeholders, plural forms, unused ids), but this one is
    // free to add here and fails a build immediately, rather than waiting
    // for a separate gate step, for the single most common slip (adding
    // a message to one language's file and forgetting the other).
    let mut ids_by_lang: BTreeMap<&str, std::collections::BTreeSet<&str>> = BTreeMap::new();
    let mut parsed_catalogs: Vec<(String, catalog::ParsedCatalog)> = Vec::new();
    for (lang, path) in &entries {
        let source = std::fs::read_to_string(path).unwrap();
        let parsed = catalog::parse(&source).unwrap();
        parsed_catalogs.push((lang.clone(), parsed));
    }
    for (lang, parsed) in &parsed_catalogs {
        ids_by_lang.insert(
            lang.as_str(),
            parsed.messages.iter().map(|(id, _)| id.as_str()).collect(),
        );
    }
    if let Some(english) = ids_by_lang.get("en") {
        for (lang, ids) in &ids_by_lang {
            if lang == &"en" {
                continue;
            }
            let missing: Vec<&&str> = english.difference(ids).collect();
            let extra: Vec<&&str> = ids.difference(english).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "{lang}.catalog and en.catalog define different message ids - \
                 missing from {lang}: {missing:?}; extra in {lang}: {extra:?}"
            );
        }
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is set");
    std::fs::write(Path::new(&out_dir).join("i18n_tables.rs"), generated)
        .expect("writing i18n_tables.rs to OUT_DIR");
}
