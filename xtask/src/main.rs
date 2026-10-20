//! ForskScope build tasks.
//!
//! Usage:
//!   cargo xtask css           — regenerate assets/main.css from assets/css/*.css
//!   cargo xtask css --check   — verify main.css is current (exits non-zero if stale)
//!   cargo xtask audit-deps    — verify sensitive dependencies are reached only through reviewed paths (dependency *shape*; no vulnerability-database lookup — see F140)
//!   cargo xtask i18n          — verify Japanese translations cover UI keys
//!   cargo xtask version-sync [expected] — verify version metadata is in sync (no-arg mode also rejects an already-published version; [expected] mode additionally requires non-empty CHANGELOG content, F24)
//!   cargo xtask rfc-sync      — verify ROADMAP.md's RFC table agrees with rfcs/proposed/ (F83)
//!   cargo xtask ui-logic-connectivity — verify every forskscope-ui-logic crate-root export has a consumer in forskscope-ui (F54)
//!   cargo xtask ui-logic-docs — verify architecture.md/testing.md's ui-logic module tables match disk (F93)
//!   cargo xtask xlsx-fixtures           — regenerate the .xlsx test fixtures under forskscope-compare (F134)
//!   cargo xtask xlsx-fixtures --check   — verify the committed fixtures match what the generator produces
//!   cargo xtask xlsx-fixtures --scale <dir> — write the size cases (a handful of rows against 40,000; 2,000 rows) to <dir>, never into the repository
//!   cargo xtask pages-origin-check <index.html> — verify a page loads nothing from another origin (handoff 075 §1)
//!   cargo xtask pages-link-check <site-dir>     — verify every relative link/image in a built site resolves (handoff 075 §3)
//!   cargo xtask colour-vision — verify every removed/added colour pair stays distinguishable under simulated colour vision deficiency (RFC-088 §3)
//!   cargo xtask keyboard-docs — verify the in-app keyboard reference and keyboard.md name the same set of keys (handoff 079 §3, F165)
//!
//! CSS source files under assets/css/ are assembled in alphabetical order.
//! The numeric prefix on each filename (00-, 01-, …) encodes the cascade order.
//! To add a file: create it with the appropriate prefix; run `cargo xtask css`.

mod colour_vision;
mod keyboard_docs;
mod pages_check;
mod rfc_sync;
mod ui_logic_connectivity;
mod ui_logic_docs;
mod xlsx_fixtures;

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{self, Command, Output},
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("css") => {
            let check = args.iter().any(|a| a == "--check");
            run_css(check);
        }
        Some("audit-deps") if args.len() == 1 => run_audit_deps(),
        Some("i18n") if args.len() == 1 => run_i18n_audit(),
        Some("version-sync") if args.len() <= 2 => {
            run_version_sync(args.get(1).map(String::as_str))
        }
        Some("rfc-sync") if args.len() == 1 => rfc_sync::run(&workspace_root()),
        Some("ui-logic-connectivity") if args.len() == 1 => {
            ui_logic_connectivity::run(&workspace_root())
        }
        Some("ui-logic-docs") if args.len() == 1 => ui_logic_docs::run(&workspace_root()),
        Some("xlsx-fixtures") if args.len() <= 2 => {
            let check = args.iter().any(|a| a == "--check");
            xlsx_fixtures::run(&workspace_root(), check);
        }
        // F137 / handoff 069 §9: the size cases, written outside the repository.
        Some("xlsx-fixtures") if args.len() == 3 && args[1] == "--scale" => {
            xlsx_fixtures::run_scale(std::path::Path::new(&args[2]));
        }
        Some("pages-origin-check") if args.len() == 2 => {
            pages_check::run_origin_check(Path::new(&args[1]));
        }
        Some("pages-link-check") if args.len() == 2 => {
            pages_check::run_link_check(Path::new(&args[1]));
        }
        Some("colour-vision") if args.len() == 1 => colour_vision::run(&workspace_root()),
        Some("keyboard-docs") if args.len() == 1 => keyboard_docs::run(&workspace_root()),
        Some(cmd) => {
            eprintln!("unknown command: {cmd}");
            print_usage();
            process::exit(1);
        }
        None => {
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("usage: cargo xtask css [--check]");
    eprintln!("       cargo xtask audit-deps");
    eprintln!("       cargo xtask i18n");
    eprintln!("       cargo xtask version-sync [expected]");
    eprintln!("       cargo xtask rfc-sync");
    eprintln!("       cargo xtask ui-logic-connectivity");
    eprintln!("       cargo xtask ui-logic-docs");
    eprintln!("       cargo xtask xlsx-fixtures [--check | --scale <dir>]");
    eprintln!("       cargo xtask pages-origin-check <index.html>");
    eprintln!("       cargo xtask pages-link-check <site-dir>");
    eprintln!("       cargo xtask colour-vision");
    eprintln!("       cargo xtask keyboard-docs");
}

pub(crate) fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is xtask/; workspace root is one level up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ must be inside the workspace root")
        .to_path_buf()
}

fn run_css(check: bool) {
    let root = workspace_root();
    let css_dir = root.join("crates/forskscope-ui/assets/css");
    let out_file = root.join("crates/forskscope-ui/assets/main.css");

    // Collect *.css files sorted alphabetically.
    // The numeric prefix on each filename (00-, 01-, …) encodes cascade order.
    let mut entries: Vec<PathBuf> = fs::read_dir(&css_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", css_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("css"))
        .collect();
    entries.sort();

    if entries.is_empty() {
        eprintln!("no *.css files found in {}", css_dir.display());
        process::exit(1);
    }

    // Assemble CSS.
    let mut assembled = String::from(
        "/*\n\
         * GENERATED FILE — DO NOT EDIT DIRECTLY.\n\
         * Source files live under assets/css/.\n\
         * Files are assembled in alphabetical order (numeric prefix = cascade order).\n\
         * Regenerate with: cargo xtask css\n\
         */\n\n",
    );

    for path in &entries {
        let name = path.file_name().unwrap().to_string_lossy();
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assembled.push_str(&format!("/* @source css/{name} */\n"));
        assembled.push_str(&content);
        if !assembled.ends_with('\n') {
            assembled.push('\n');
        }
        assembled.push('\n');
    }

    if check {
        let committed = fs::read_to_string(&out_file)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", out_file.display()));
        if assembled == committed {
            println!("assets/main.css is up to date.");
        } else {
            eprintln!("assets/main.css is STALE. Run `cargo xtask css` to regenerate.");
            process::exit(1);
        }
    } else {
        fs::write(&out_file, &assembled)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", out_file.display()));
        println!("wrote {}", out_file.display());
    }
}

/// F140: this check (and its own `println!`) used to claim more than it
/// verifies. It asserts dependency **shape** — that a named package is
/// reached only through the specific, reviewed hop(s) listed for it, so an
/// unreviewed second path to it (a new crate pulling `rustls` directly, say)
/// fails the build. **It does not consult a vulnerability database.** A path
/// passing here is not a claim that the crates on it carry no advisories —
/// that is `cargo audit`'s job, run separately in CI, and Dependabot alerts
/// (disabled on this repository; enabling them is the owner's call, not
/// this check's). The comment below used to assert the conclusion
/// ("carries none") rather than the shape assertion actually made; corrected.
fn run_audit_deps() {
    assert_package_inactive("dioxus-devtools");
    // F179: muda is vendored at vendor/muda (patched to drop its libxdo
    // dependency — see vendor/README.md) specifically to take libxdo out of
    // the graph, not merely out of use. This is the gate that guards that:
    // falsify by removing the `[patch.crates-io]` entry from the workspace
    // Cargo.toml and this must fail.
    assert_package_absent("libxdo");
    assert_dioxus_desktop_libxdo_not_yet_optional();
    assert_external_network_crates_absent();
    assert_quick_xml_path_is_reviewed();
    assert_network_paths_are_reviewed();
    assert_sha2_path_is_reviewed();
    // RFC-085: sheets-diff -> calamine -> quick-xml/zip is a deliberately
    // re-added, reviewed path — RFC-058 suspended it (quick-xml 0.39 XML
    // DoS advisories). What is asserted here is that each of the three
    // packages below is reached only through the single hop named for it —
    // not that the versions currently resolved carry no advisories, which
    // this check cannot see and does not claim (see the function doc
    // comment, F140). Each assertion below replaces this pair's old
    // `assert_package_absent` — a gate that passed because the dependency
    // was absent, not because the path was reviewed, is exactly the failure
    // mode F65 records.
    assert_immediate_dependents("sheets-diff", &["forskscope-compare "]);
    assert_immediate_dependents("calamine", &["sheets-diff "]);
    assert_immediate_dependents("zip", &["calamine "]);
    // RUSTSEC-2026-0367/-0368/-0369 (zip 8.6.0, ignored in .cargo/audit.toml,
    // reviewed 2026-10-17): the `assert_immediate_dependents` call above
    // already guards the resolved-graph half of that exception (zip reached
    // only through calamine). This guards the half it cannot see — a
    // manifest naming zip directly, which would not need to be *built* by a
    // default run to widen the exception's real scope — and reports,
    // without failing, once the resolved graph no longer needs the
    // exception at all.
    assert_no_direct_zip_dependency();
    report_if_zip_resolves_fixed();
    assert_xtask_zip_dependent_is_rust_xlsxwriter();
    println!(
        "security dependency path check passed: sensitive dependencies are reached only \
         through their reviewed paths. This checks dependency shape, not advisories — it does \
         not query a vulnerability database; that is cargo audit's job, run separately."
    );
    assert_threat_model_versions();
}

/// RUSTSEC-2026-0367/-0368/-0369's `.cargo/audit.toml` ignore is reviewed
/// against zip reached *only* through calamine's safe, in-memory
/// `ZipArchive` reads — never through this project's own code naming zip
/// directly, which would need its own review before the ignore could cover
/// it. Checked against every workspace crate's manifest and xtask's own
/// (a separate workspace, with its own `Cargo.lock` — `cargo tree`'s
/// resolved-graph check above never sees it, and it has its own zip
/// exposure today, via `rust_xlsxwriter`, reviewed the same way: no
/// `extract`/`ZipStreamReader` call in its source either).
/// Falsify by adding `zip = "..."` to any of these `[dependencies]`.
fn assert_no_direct_zip_dependency() {
    let manifests = [
        "Cargo.toml",
        "crates/forskscope-compare/Cargo.toml",
        "crates/forskscope-core/Cargo.toml",
        "crates/forskscope-ui/Cargo.toml",
        "crates/forskscope-ui-logic/Cargo.toml",
        "xtask/Cargo.toml",
    ];
    for manifest in manifests {
        let path = workspace_root().join(manifest);
        let text = read_file(&path);
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("zip ")
                || trimmed.starts_with("zip=")
                || trimmed.starts_with("zip.")
            {
                fail(&format!(
                    "{}:{} names `zip` as a direct dependency - RUSTSEC-2026-0367/-0368/-0369 \
                     are ignored in .cargo/audit.toml only because zip is reached exclusively \
                     through calamine's (or rust_xlsxwriter's) reviewed, extract-free usage; a \
                     direct dependency on zip needs its own review before the ignore can cover \
                     it:\n  {line}",
                    path.display(),
                    idx + 1
                ));
            }
        }
    }
    println!("no workspace crate or xtask names zip as a direct dependency.");
}

/// This exception's removal trigger: once zip resolves to >= 9 (the version
/// RUSTSEC-2026-0367/-0368/-0369 are fixed in) through calamine, a fixed
/// version is already in the graph and the ignore entries in
/// `.cargo/audit.toml` likely no longer apply. Reports, never fails — a
/// resolved fix is good news, not a gate to block on, and whether to drop
/// the ignore is a one-line review, not an emergency.
fn report_if_zip_resolves_fixed() {
    let resolved = resolve_version_via("zip", "calamine");
    let major: u32 = resolved
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if major >= 9 {
        println!(
            "NOTICE: zip now resolves to {resolved} (>= 9) via calamine — RUSTSEC-2026-0367/\
             -0368/-0369 are fixed there; .cargo/audit.toml's ignore entries for them were \
             written against zip 8.6.0 and should be reconsidered now."
        );
    }
}

/// F204 (review 170 §4): `.cargo/audit.toml`'s zip ignore was reviewed for
/// two reached-only-through-safe-usage paths, not one — xtask's own
/// `rust_xlsxwriter` dependency (the `.xlsx` fixture generator) pulls in
/// the same vulnerable zip 8.6.0, through a lockfile the main workspace's
/// `assert_immediate_dependents("zip", ...)` above never sees (xtask is a
/// separate workspace, run from the repo root this check is already in).
/// `rust_xlsxwriter` only writes archives and never calls
/// `extract`/`ZipStreamReader` (checked directly, 0.99.0, the same way
/// calamine's usage was). Falsify by adding a second crate to
/// `xtask/Cargo.toml` that also depends on zip.
fn assert_xtask_zip_dependent_is_rust_xlsxwriter() {
    let output = Command::new("cargo")
        .args(["tree", "--prefix", "depth", "-i", "zip", "--target", "all"])
        .current_dir(workspace_root().join("xtask"))
        .output()
        .unwrap_or_else(|e| panic!("failed to run cargo tree in xtask: {e}"));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("did not match any packages") {
            println!("zip is absent from xtask's dependency graph.");
            return;
        }
        eprintln!("could not inspect zip's dependency path in xtask");
        eprintln!("{stderr}");
        process::exit(1);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let immediate_dependents: Vec<&str> = stdout
        .lines()
        .filter_map(depth_prefixed_package)
        .filter_map(|(depth, package)| (depth == 1).then_some(package))
        .collect();

    if immediate_dependents.is_empty()
        || immediate_dependents
            .iter()
            .any(|package| !package.starts_with("rust_xlsxwriter "))
    {
        eprintln!("xtask's zip has an unreviewed immediate dependency path:");
        eprintln!("{stdout}");
        process::exit(1);
    }
    println!("xtask's zip path is limited to rust_xlsxwriter.");
}

/// F142: the versions `docs/src/maintainers/threat-model.md` quotes for the
/// `.xlsx` parser chain (`sheets-diff -> calamine -> quick-xml, zip`) must
/// match what `cargo` actually resolves. This check already resolves these
/// exact packages to assert their dependency *shape* (F140, above); the
/// version is one more resolution away, which is where a hand-quoted number
/// belongs once it has already drifted once (F153: `sheets-diff` 3.0.0 was
/// still quoted in this document days after the version it names had
/// already been superseded).
///
/// This document also narrates its own history - "closed in `sheets-diff`
/// 3.0.0 (F123, F130)", a changelog table of past bumps, and so on - quoting
/// older versions **correctly**, on purpose. A scan for every backtick
/// -quoted mention of a package name would flag every one of those as
/// drift; checked against that directly (an earlier draft of this check
/// did exactly that, and failed on `calamine`'s own reference-table row,
/// whose *description* cell happens to name `sheets-diff` 2.5.1 in passing).
/// So this checks exactly two places, each anchored on text specific to it,
/// not "every mention of the name": the one prose sentence describing the
/// current `.xlsx` parser chain, and each package's row in the current
/// -state dependency table (identified by starting with that package's own
/// name, which no history-table row does).
fn assert_threat_model_versions() {
    let path = workspace_root().join("docs/src/maintainers/threat-model.md");
    let doc = read_file(&path);

    let specs: [(&str, &str, Option<&str>); 4] = [
        ("sheets-diff", "forskscope-compare", None),
        ("calamine", "sheets-diff", None),
        // quick-xml also appears, correctly at a different version, as the
        // unrelated Wayland/GTK build-time dependency - its table row names
        // `calamine`, that one does not, so that is what distinguishes the
        // row this check owns from the one it must leave alone.
        ("quick-xml", "calamine", Some("calamine")),
        ("zip", "calamine", None),
    ];

    let prose = ProseWindow::find(&path, &doc);
    for (package, via, table_disambiguator) in specs {
        let resolved = resolve_version_via(package, via);
        prose.assert_quotes_version(&path, package, &resolved);
        assert_table_row_quotes_version(&path, &doc, package, &resolved, table_disambiguator);
    }

    println!(
        "threat model dependency versions check passed: sheets-diff, calamine, quick-xml and \
         zip all match what cargo actually resolves."
    );
}

/// The one prose sentence describing the `.xlsx` parser chain's current
/// state (" ... workbooks goes through `sheets-diff` 3.2.0 (`calamine
/// 0.36.1`, `quick-xml 0.41.0`, `zip 8.6.0`) under the bounds ... "),
/// isolated by its own anchor text so a version check inside it can never
/// stray into one of this document's many unrelated historical mentions of
/// the same package names.
struct ProseWindow {
    text: String,
    line_no: usize,
}

impl ProseWindow {
    const ANCHOR: &'static str = "workbooks goes through";

    fn find(path: &Path, doc: &str) -> Self {
        let start = doc.find(Self::ANCHOR).unwrap_or_else(|| {
            fail(&format!(
                "{}: could not find the \"{}\" sentence describing the .xlsx parser chain - \
                 update this check or restore the sentence",
                path.display(),
                Self::ANCHOR
            ))
        });
        let end = doc[start..]
            .find(')')
            .map(|i| start + i + 1)
            .unwrap_or_else(|| {
                fail(&format!(
                    "{}: the \"{}\" sentence has no closing ')' to bound this check's search",
                    path.display(),
                    Self::ANCHOR
                ))
            });
        Self {
            text: doc[start..end].to_string(),
            line_no: doc[..start].matches('\n').count() + 1,
        }
    }

    /// Checks `package`'s version within this window - either closed-form
    /// (`` `package` `` followed by a bare version, `sheets-diff`'s own
    /// style here) or open-form (the version inside the same backtick span,
    /// e.g. `` `calamine 0.36.1` ``, everything else's style here).
    fn assert_quotes_version(&self, path: &Path, package: &str, resolved: &str) {
        for marker in [format!("`{package}` "), format!("`{package} ")] {
            let Some(pos) = self.text.find(&marker) else {
                continue;
            };
            let after = self.text[pos + marker.len()..].trim_start();
            let quoted: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if !quoted.starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }
            if quoted != resolved {
                fail(&format!(
                    "{}:{} quotes {package} {quoted} in its .xlsx parser sentence, but the \
                     dependency graph resolves it to {resolved}",
                    path.display(),
                    self.line_no
                ));
            }
            return;
        }
        fail(&format!(
            "{}:{} no longer quotes a version for {package} in its .xlsx parser sentence - \
             update this check or restore the mention",
            path.display(),
            self.line_no
        ));
    }
}

/// Checks `package`'s row in the current-state dependency table - a line
/// starting with `` | `package` | `` (no history-table row does this; those
/// lead with a version tag or date, naming the package only in a later
/// cell), optionally requiring the row to also contain `same_line_requires`
/// (disambiguating `quick-xml`'s two rows). Fails with the exact line number
/// and text on a mismatch, and fails if no such row was found at all -
/// either this check or the table has gone stale.
fn assert_table_row_quotes_version(
    path: &Path,
    doc: &str,
    package: &str,
    resolved: &str,
    same_line_requires: Option<&str>,
) {
    let marker = format!("| `{package}` | ");
    let mut checked = 0usize;
    for (idx, line) in doc.lines().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with(&marker) {
            continue;
        }
        if let Some(req) = same_line_requires
            && !line.contains(req)
        {
            continue;
        }
        let after = trimmed[marker.len()..].trim_start();
        let quoted: String = after
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if !quoted.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        checked += 1;
        if quoted != resolved {
            fail(&format!(
                "{}:{} quotes {package} {quoted} in its dependency table, but the dependency \
                 graph resolves it to {resolved}:\n  {line}",
                path.display(),
                idx + 1
            ));
        }
    }
    if checked == 0 {
        fail(&format!(
            "{} no longer has a `{package}` row in its dependency table - update this check or \
             restore the row",
            path.display()
        ));
    }
}

/// The version `cargo` resolves for `package`, confirmed to be reached
/// through `via` (an immediate-dependent name prefix) rather than some other
/// version of the same package resolved elsewhere in the graph - `quick-xml`
/// exists at two versions at once (RFC-085: one via `wayland-scanner`, one
/// via `calamine`), so asking for "the" version of `quick-xml` without
/// saying through what is already the wrong question.
fn resolve_version_via(package: &str, via: &str) -> String {
    let specs = resolve_specs(package);
    if specs.is_empty() {
        fail(&format!(
            "{package} is absent from the dependency graph - the threat model's version claim \
             for it can no longer be checked"
        ));
    }
    for spec in &specs {
        let output = cargo_tree(&["tree", "--prefix", "depth", "-i", spec]);
        if !output.status.success() {
            eprintln!("could not inspect {spec} dependency path");
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            process::exit(1);
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = stdout.lines().filter_map(depth_prefixed_package);
        let Some((0, root)) = entries.next() else {
            fail(&format!(
                "unexpected `cargo tree -i {spec}` output:\n{stdout}"
            ));
        };
        let reached_via_expected = entries.any(|(depth, pkg)| depth == 1 && pkg.starts_with(via));
        if reached_via_expected {
            return root
                .rsplit_once(" v")
                .map(|(_, version)| version.to_string())
                .unwrap_or_else(|| fail(&format!("could not parse a version from `{root}`")));
        }
    }
    fail(&format!(
        "{package} has no path through {via} in the dependency graph"
    ));
}

/// Checks that every `t(lang, "key")` call site in `forskscope-ui` has a
/// Japanese translation. It does **not** check that every user-visible
/// string is routed through `t()` in the first place (F39, G-006 narrowed
/// 2026-08-13): a string that never reaches a `t(...)` call is invisible to
/// this scan by construction, not merely uncovered. Known bypasses today are
/// `store.notify(err.to_string())` sites carrying `CoreError`/`AppError`
/// `Display` output — filesystem/OS error text and other dependency-
/// generated messages, not authored UI copy — which cannot be pre-translated
/// because their content does not exist until the error occurs. See
/// `docs/src/maintainers/testing.md`'s i18n section for the full list and
/// reasoning.
fn run_i18n_audit() {
    let root = workspace_root();
    let i18n_file = root.join("crates/forskscope-ui/src/i18n.rs");
    let i18n = fs::read_to_string(&i18n_file)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", i18n_file.display()));
    let translated = extract_ja_translation_keys(&i18n);

    let ui_src = root.join("crates/forskscope-ui/src");
    let mut used = BTreeSet::new();
    collect_i18n_usage_keys(&ui_src, &mut used);

    let missing: Vec<&String> = used
        .iter()
        .filter(|key| !translated.contains(*key))
        .collect();
    if !missing.is_empty() {
        eprintln!("missing Japanese translations for UI keys:");
        for key in missing {
            eprintln!("  - {key}");
        }
        process::exit(1);
    }

    println!(
        "i18n audit passed: {} UI keys are covered by Japanese translations.",
        used.len()
    );
}

fn extract_ja_translation_keys(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix('"')
                .and_then(|rest| rest.find('"').map(|end| rest[..end].to_string()))
        })
        .collect()
}

fn collect_i18n_usage_keys(dir: &Path, keys: &mut BTreeSet<String>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|e| panic!("cannot read entry in {}: {e}", dir.display()))
            .path();
        if path.is_dir() {
            collect_i18n_usage_keys(&path, keys);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
            && path.file_name().and_then(|s| s.to_str()) != Some("i18n.rs")
        {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            extract_t_invocation_strings(&source, keys);
        }
    }
}

fn extract_t_invocation_strings(source: &str, keys: &mut BTreeSet<String>) {
    let bytes = source.as_bytes();
    let mut i = 0;
    while i + 2 <= bytes.len() {
        if bytes[i] == b't'
            && bytes.get(i + 1) == Some(&b'(')
            && i.checked_sub(1)
                .and_then(|prev| bytes.get(prev))
                .is_none_or(|ch| !is_ident_byte(*ch))
        {
            let mut cursor = i + 2;
            let mut depth = 1usize;
            while cursor < bytes.len() && depth > 0 {
                match bytes[cursor] {
                    b'"' => {
                        let (value, next) = parse_string_literal(source, cursor);
                        keys.insert(value);
                        cursor = next;
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
            i = cursor;
        } else {
            i += 1;
        }
    }
}

fn is_ident_byte(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn parse_string_literal(source: &str, start: usize) -> (String, usize) {
    let bytes = source.as_bytes();
    let mut out = String::new();
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => {
                if let Some(next) = bytes.get(cursor + 1) {
                    out.push(*next as char);
                    cursor += 2;
                } else {
                    cursor += 1;
                }
            }
            b'"' => return (out, cursor + 1),
            byte => {
                let ch = source[cursor..]
                    .chars()
                    .next()
                    .unwrap_or_else(|| panic!("invalid UTF-8 boundary at byte {cursor}"));
                out.push(ch);
                cursor += ch.len_utf8().max((byte as char).len_utf8());
            }
        }
    }
    (out, cursor)
}

fn run_version_sync(expected_version: Option<&str>) {
    let root = workspace_root();
    let cargo_toml = read_file(&root.join("Cargo.toml"));
    let version = extract_workspace_value(&cargo_toml, "version")
        .unwrap_or_else(|| fail("could not find [workspace.package] version in Cargo.toml"));

    if let Some(expected) = expected_version
        && expected != version
    {
        fail(&format!(
            "release version mismatch: expected {expected}, but [workspace.package] version is {version}"
        ));
    }
    if expected_version.is_none() {
        check_version_not_already_published(&root, &version);
    }

    assert_contains(
        &root.join("xtask/Cargo.toml"),
        &format!("version = \"{version}\""),
        "xtask package version",
    );
    assert_contains(
        &root.join("packaging/linux/PKGBUILD"),
        &format!("pkgver={version}"),
        "Arch PKGBUILD pkgver",
    );
    assert_contains(
        &root.join("packaging/windows/AppxManifest.xml"),
        &format!("Version=\"{version}.0\""),
        "Windows AppxManifest package version",
    );
    assert_contains(
        &root.join("CHANGELOG.md"),
        &format!("## [{version}]"),
        "CHANGELOG release section",
    );

    // F24: release mode only. The release workflow's own empty-section guard
    // used to run in its *last* job, after the tag, source archive (dropped
    // since, F43), and all three platform builds already existed — by then,
    // recovering from an empty section meant a re-cut. Dev mode (no
    // `expected_version`) must
    // keep accepting an empty section: the tree normally carries one for the
    // in-progress version between releases (opened by the post-release bump,
    // closed when release notes are actually written), and failing dev-mode
    // CI on that would turn every ordinary commit red.
    if expected_version.is_some() {
        let changelog = read_file(&root.join("CHANGELOG.md"));
        if changelog_section_is_empty(&changelog, &version) {
            fail(&format!(
                "CHANGELOG section for {version} has no content — release notes would ship blank"
            ));
        }
    }

    let cargo_lock = read_file(&root.join("Cargo.lock"));
    for package in ["forskscope-core", "forskscope-ui", "forskscope-ui-logic"] {
        let expected = format!("name = \"{package}\"\nversion = \"{version}\"");
        if !cargo_lock.contains(&expected) {
            fail(&format!(
                "Cargo.lock package {package} is not version {version}"
            ));
        }
    }

    println!("version sync passed for v{version}.");
}

// Mirrors the awk extraction `release.yml`'s "Compose release notes" step
// uses: everything between a line starting with `## [version]` and the next
// line starting with `## [`, matched the same way (`index($0, hdr) == 1`
// there, `starts_with` here — both prefix matches, so trailing text like
// " — Unreleased" or " — 2026-08-08" after the bracket doesn't break either).
// Kept as one Rust implementation rather than shelling out to `awk`, so the
// preflight check and the release-notes step can independently agree or
// disagree without one calling the other.
fn changelog_section_is_empty(changelog: &str, version: &str) -> bool {
    let header = format!("## [{version}]");
    let mut in_section = false;
    let mut content = String::new();
    for line in changelog.lines() {
        if line.starts_with(&header) {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with("## [") {
                break;
            }
            content.push_str(line);
            content.push('\n');
        }
    }
    content.trim().is_empty()
}

// No-arg mode only: fails if `version` is a tag that exists but not at HEAD.
// Fails safe — any git/tag problem is a SKIPPED notice, never a silent pass.
fn check_version_not_already_published(root: &Path, version: &str) {
    let skip = |reason: &str| println!("version-sync: SKIPPED published-tag check — {reason}.");
    let all = match git_lines(root, &["tag", "--list"]) {
        Ok(lines) if !lines.is_empty() => lines,
        Ok(_) => return skip("no tags found in this checkout (shallow clone?)"),
        Err(reason) => return skip(&reason),
    };
    if !all.iter().any(|t| t == version) {
        return;
    }
    match git_lines(root, &["tag", "--points-at", "HEAD"]) {
        Ok(here) if here.iter().any(|t| t == version) => {}
        Ok(_) => fail(&format!("version {version} is already tagged; bump it")),
        Err(reason) => skip(&reason),
    }
}

// Runs a git command, returning its stdout split into lines (as `git tag`
// emits them: one tag per line, no blank lines, no padding), or a reason it
// failed.
fn git_lines(root: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    let out = Command::new("git").args(args).current_dir(root).output();
    let out = out.map_err(|e| format!("git is unavailable: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text.lines().map(str::to_string).collect())
}

fn read_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn assert_contains(path: &Path, needle: &str, label: &str) {
    let content = read_file(path);
    if !content.contains(needle) {
        fail(&format!(
            "{label} is not in sync; expected `{needle}` in {}",
            path.display()
        ));
    }
}

fn extract_workspace_value(source: &str, key: &str) -> Option<String> {
    let mut in_workspace_package = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace_package = trimmed == "[workspace.package]";
            continue;
        }
        if in_workspace_package
            && let Some(rest) = trimmed.strip_prefix(key)
            && let Some(value) = rest.trim_start().strip_prefix('=')
        {
            return value
                .trim()
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .map(str::to_string);
        }
    }
    None
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    process::exit(1);
}

fn assert_package_absent(package: &str) {
    let output = cargo_tree(&["tree", "-i", package]);
    if output.status.success() {
        eprintln!("unexpected dependency present: {package}");
        eprintln!("{}", String::from_utf8_lossy(&output.stdout));
        process::exit(1);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.contains("did not match any packages") {
        eprintln!("could not verify absence of {package}");
        eprintln!("{stderr}");
        process::exit(1);
    }

    println!("{package} is absent.");
}

fn assert_package_inactive(package: &str) {
    let output = cargo_tree(&["tree", "-i", package]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        if stderr.contains("did not match any packages") {
            println!("{package} is absent.");
            return;
        }

        eprintln!("could not inspect {package}");
        eprintln!("{stderr}");
        process::exit(1);
    }

    if stdout.trim().is_empty() && stderr.contains("nothing to print") {
        println!("{package} is inactive.");
        return;
    }

    eprintln!("{package} is active in the dependency graph:");
    eprintln!("{stdout}");
    process::exit(1);
}

/// F179's removal condition, read from the dependency rather than
/// remembered: `vendor/muda`'s patch exists only because `dioxus-desktop`
/// 0.7.9 enables `muda`'s and `tray-icon`'s default features with no way for
/// us to opt out. Dioxus PR #5749 (merged to `main`, not yet on the `v0.7`
/// branch we depend on) fixes this upstream by declaring both with
/// `default-features = false` and adding a new `linux-libxdo` feature that
/// makes `libxdo` opt-in. Once that reaches a `dioxus-desktop` release we
/// depend on, this feature name appears in the resolved metadata and this
/// check fails, naming the fix: delete `vendor/muda/` and the
/// `[patch.crates-io]` entry.
fn assert_dioxus_desktop_libxdo_not_yet_optional() {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .unwrap_or_else(|e| panic!("failed to run cargo metadata: {e}"));

    if !output.status.success() {
        eprintln!("could not resolve dependency metadata");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        process::exit(1);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("linux-libxdo") {
        fail(
            "a package in the resolved dependency graph now defines a `linux-libxdo` feature, \
             most likely dioxus-desktop adopting DioxusLabs/dioxus#5749. Confirm that \
             dioxus-desktop is the package that defines it; if so, libxdo is opt-in upstream, \
             so delete vendor/muda/ and the `[patch.crates-io]` entry in the workspace \
             Cargo.toml (F179). If it was deleted too early, the `libxdo is absent` check \
             fails as soon as libxdo returns.",
        );
    }

    println!(
        "dioxus-desktop does not yet make libxdo opt-in (F179's vendor patch is still needed)."
    );
}

/// RFC-085: `wayland-scanner` (pre-existing, reviewed) and `calamine <-
/// sheets-diff` (re-added by RFC-085) now depend on two different quick-xml
/// majors at once, so `cargo tree -i quick-xml` is ambiguous by package name
/// alone — `assert_immediate_dependents`'s version resolution handles that;
/// this just supplies both accepted paths.
fn assert_quick_xml_path_is_reviewed() {
    assert_immediate_dependents("quick-xml", &["wayland-scanner ", "calamine "]);
}

/// RFC-092/handoff 084, updated by RFC-093 (handoff 086): `sha2` was already
/// resolved transitively (`wry` -> `forskscope-ui`); `forskscope-report`
/// depends on it directly too, for comparison-report checksums (SHA-256 of
/// each compared file's bytes) — moved there from `forskscope-core` as part
/// of the crate-boundary split. Same crate, same version, no new crypto
/// stack — both paths are accepted deliberately, a third would not be.
fn assert_sha2_path_is_reviewed() {
    assert_immediate_dependents("sha2", &["wry ", "forskscope-report "]);
}

fn assert_external_network_crates_absent() {
    for package in ["reqwest", "hyper", "ureq"] {
        assert_package_absent(package);
    }
}

fn assert_network_paths_are_reviewed() {
    assert_immediate_dependents("tungstenite", &["dioxus-desktop "]);
    // F180 (handoff 073): `forskscope-ui` depends on `native-tls` directly
    // now too, for the update-check request's TLS — no new TLS stack, the
    // same already-reviewed crate and version `tungstenite` pulls in. Both
    // paths are accepted deliberately; a third would not be.
    assert_immediate_dependents("native-tls", &["tungstenite ", "forskscope-ui "]);
    // F121 D: `tungstenite` compiles in both TLS backends, so the transport
    // has two reviewed paths, not one — and RUSTSEC-2026-0285 (0.171.0) was a
    // `rustls` advisory. Without these, a second crate pulling `rustls` would
    // pass a gate that reads as covering the transport.
    assert_immediate_dependents("rustls", &["tungstenite "]);
    assert_immediate_dependents("rustls-webpki", &["rustls "]);
    println!("network-capable dependency paths are reviewed.");
}

/// Resolves `package` to every fully-versioned spec `cargo tree` currently
/// has for it. A single unambiguous match returns the bare name unchanged
/// (cargo already accepts it as-is); two or more resolved versions in the
/// graph at once (e.g. quick-xml 0.39 via `wayland-scanner` alongside 0.41
/// via `calamine`, RFC-085) return one `name@version` spec per version, so a
/// caller checking dependency paths inspects every version present instead
/// of whichever one `cargo tree -i <bare name>` would otherwise fail to pick
/// between. Empty means the package is absent from the graph entirely.
fn resolve_specs(package: &str) -> Vec<String> {
    let output = cargo_tree(&["tree", "-i", package]);
    if output.status.success() {
        return vec![package.to_string()];
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("did not match any packages") {
        return Vec::new();
    }
    if stderr.contains("is ambiguous") {
        let prefix = format!("{package}@");
        return stderr
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with(&prefix))
            .map(str::to_string)
            .collect();
    }

    eprintln!("could not resolve {package}'s position in the dependency graph");
    eprintln!("{stderr}");
    process::exit(1);
}

fn assert_immediate_dependents(package: &str, accepted_prefixes: &[&str]) {
    let specs = resolve_specs(package);
    if specs.is_empty() {
        println!("{package} is absent.");
        return;
    }
    for spec in &specs {
        assert_immediate_dependents_for_spec(spec, accepted_prefixes);
    }
    let labels: Vec<&str> = accepted_prefixes.iter().map(|p| p.trim()).collect();
    println!("{package} path is limited to {}.", labels.join(" / "));
}

fn assert_immediate_dependents_for_spec(spec: &str, accepted_prefixes: &[&str]) {
    let output = cargo_tree(&["tree", "--prefix", "depth", "-i", spec]);
    if !output.status.success() {
        eprintln!("could not inspect {spec} dependency path");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        process::exit(1);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let immediate_dependents: Vec<&str> = stdout
        .lines()
        .filter_map(depth_prefixed_package)
        .filter_map(|(depth, package)| (depth == 1).then_some(package))
        .collect();

    if immediate_dependents.is_empty()
        || immediate_dependents.iter().any(|package| {
            !accepted_prefixes
                .iter()
                .any(|accepted| package.starts_with(accepted))
        })
    {
        eprintln!("{spec} has an unreviewed immediate dependency path:");
        eprintln!("{stdout}");
        process::exit(1);
    }
}

fn depth_prefixed_package(line: &str) -> Option<(usize, &str)> {
    let prefix_len = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_ascii_digit()).then_some(idx))?;
    if prefix_len == 0 {
        return None;
    }
    let depth = line[..prefix_len].parse().ok()?;
    Some((depth, &line[prefix_len..]))
}

/// Runs `cargo tree` over **every** target's dependency graph (`--target
/// all`), not just the host's. Without it the gate only saw the Linux graph:
/// `tungstenite`'s `rustls` dependency is compiled for the Windows and macOS
/// builds that ship, and was invisible here (F121 D) — a Windows-only crate
/// would have passed every assertion below.
fn cargo_tree(args: &[&str]) -> Output {
    Command::new("cargo")
        .args(args)
        .args(["--target", "all"])
        .current_dir(workspace_root())
        .output()
        .unwrap_or_else(|e| panic!("failed to run cargo {}: {e}", args.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::changelog_section_is_empty;

    #[test]
    fn empty_section_is_reported_empty() {
        let changelog = "\
# Changelog

## [0.166.1] — Unreleased

## [0.166.0] — 2026-08-08

Some prior content.
";
        assert!(changelog_section_is_empty(changelog, "0.166.1"));
    }

    #[test]
    fn whitespace_only_section_is_reported_empty() {
        let changelog = "\
## [0.166.1] — Unreleased


\t
## [0.166.0] — 2026-08-08
";
        assert!(changelog_section_is_empty(changelog, "0.166.1"));
    }

    #[test]
    fn section_with_real_content_is_not_empty() {
        let changelog = "\
## [0.166.1] — Unreleased

### Changed

- Something shipped.

## [0.166.0] — 2026-08-08
";
        assert!(!changelog_section_is_empty(changelog, "0.166.1"));
    }

    #[test]
    fn content_belonging_to_a_different_version_does_not_count() {
        // The 0.166.1 section itself is empty; real content lives only
        // under the *next* header. Must not bleed across the boundary.
        let changelog = "\
## [0.166.1] — Unreleased

## [0.166.0] — 2026-08-08

Real content here belongs to 0.166.0, not 0.166.1.
";
        assert!(changelog_section_is_empty(changelog, "0.166.1"));
    }

    #[test]
    fn missing_header_is_reported_empty() {
        // No `## [...]` header for this version at all — an absent section
        // has no content by definition. (The separate CHANGELOG-heading
        // presence check in run_version_sync is what catches a missing
        // header as its own distinct failure.)
        let changelog = "## [0.166.0] — 2026-08-08\n\nSome content.\n";
        assert!(changelog_section_is_empty(changelog, "0.166.1"));
    }

    #[test]
    fn trailing_text_after_the_bracket_does_not_prevent_a_match() {
        let changelog = "\
## [0.166.1] — Unreleased

### Changed

- Real content.

## [0.166.0] — 2026-08-08
";
        assert!(!changelog_section_is_empty(changelog, "0.166.1"));
    }
}
