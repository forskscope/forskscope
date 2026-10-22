//! Review 189 §3.2: `cargo xtask metainfo-release <tag>` — verifies the
//! AppStream metainfo's first `<release version="…">` entry names the tag
//! being released.
//!
//! **Not part of `version-sync`:** `main` carries the next patch version
//! between releases (`Cargo.toml`'s own version is always one ahead), but
//! the metainfo only names releases that have actually shipped — the two
//! files are supposed to disagree most of the time, so one check covering
//! both would either be wrong on `main` or blind to a stale metainfo at
//! the cut. This check runs only against the tag being released, not
//! against `Cargo.toml`.
//!
//! A plain text search, not an XML parser (DEC-005's reasoning: this
//! codebase already has a `rust_xlsxwriter`/`serde_json`-sized bar for a
//! new xtask dependency, and the metainfo's own shape, hand-written by
//! this project, is simple and stable enough that a text-pattern search is
//! exact here, the same call `rfc_sync.rs` and `crate_boundaries.rs` make
//! for their own structured-but-simple inputs).

use std::fs;
use std::path::Path;
use std::process;

const METAINFO_RELATIVE_PATH: &str = "packaging/linux/io.github.forskscope.forskscope.metainfo.xml";
const RELEASES_OPEN_TAG: &str = "<releases>";
const RELEASE_VERSION_MARKER: &str = "<release version=\"";

pub fn run(root: &Path, tag: &str) {
    let metainfo_path = root.join(METAINFO_RELATIVE_PATH);
    let metainfo = fs::read_to_string(&metainfo_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", metainfo_path.display()));

    let Some(releases_start) = metainfo.find(RELEASES_OPEN_TAG) else {
        eprintln!(
            "::error::{} has no <releases> element",
            metainfo_path.display()
        );
        process::exit(1);
    };
    let after_releases = &metainfo[releases_start..];

    let Some(marker_start) = after_releases.find(RELEASE_VERSION_MARKER) else {
        eprintln!(
            "::error::{} has a <releases> element with no <release version=\"…\"> entry",
            metainfo_path.display()
        );
        process::exit(1);
    };
    let after_marker = &after_releases[marker_start + RELEASE_VERSION_MARKER.len()..];
    let Some(quote_end) = after_marker.find('"') else {
        eprintln!(
            "::error::{} has a <release version=\"…\"> entry with no closing quote",
            metainfo_path.display()
        );
        process::exit(1);
    };
    let first_release_version = &after_marker[..quote_end];

    if first_release_version != tag {
        eprintln!(
            "::error::{}'s first <release> entry names {first_release_version}, but this tag is {tag} - add the {tag} <release> entry before cutting it",
            metainfo_path.display()
        );
        process::exit(1);
    }

    println!(
        "metainfo-release: {} matches tag {tag}",
        metainfo_path.display()
    );
}
