//! ForskScope build tasks.
//!
//! Usage:
//!   cargo xtask css           — regenerate assets/main.css from assets/css/*.css
//!   cargo xtask css --check   — verify main.css is current (exits non-zero if stale)
//!   cargo xtask audit-deps    — verify reviewed security dependency paths
//!   cargo xtask i18n          — verify Japanese translations cover UI keys
//!   cargo xtask version-sync [expected] — verify version metadata is in sync (no-arg mode also rejects an already-published version; [expected] mode additionally requires non-empty CHANGELOG content, F24)
//!   cargo xtask rfc-sync      — verify ROADMAP.md's RFC table agrees with rfcs/proposed/ (F83)
//!
//! CSS source files under assets/css/ are assembled in alphabetical order.
//! The numeric prefix on each filename (00-, 01-, …) encodes the cascade order.
//! To add a file: create it with the appropriate prefix; run `cargo xtask css`.

mod rfc_sync;

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

fn run_audit_deps() {
    assert_package_inactive("dioxus-devtools");
    assert_external_network_crates_absent();
    assert_quick_xml_path_is_reviewed();
    assert_network_paths_are_reviewed();
    // RFC-085: sheets-diff -> calamine -> quick-xml/zip is a deliberately
    // re-added, reviewed path — RFC-058 suspended it (quick-xml 0.39 XML
    // DoS advisories); sheets-diff 2.5.0's chain (quick-xml 0.41.0, zip
    // 8.6.0) carries none, verified against the versions actually
    // resolved here, not inherited from an earlier check (xlsx.rs's
    // module doc has the full account). Each assertion below replaces
    // this pair's old `assert_package_absent` — a gate that passed
    // because the dependency was absent, not because the path was
    // reviewed, is exactly the failure mode F65 records.
    assert_immediate_dependents("sheets-diff", &["forskscope-core "]);
    assert_immediate_dependents("calamine", &["sheets-diff "]);
    assert_immediate_dependents("zip", &["calamine "]);
    println!("security dependency path check passed.");
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

/// RFC-085: `wayland-scanner` (pre-existing, reviewed) and `calamine <-
/// sheets-diff` (re-added by RFC-085) now depend on two different quick-xml
/// majors at once, so `cargo tree -i quick-xml` is ambiguous by package name
/// alone — `assert_immediate_dependents`'s version resolution handles that;
/// this just supplies both accepted paths.
fn assert_quick_xml_path_is_reviewed() {
    assert_immediate_dependents("quick-xml", &["wayland-scanner ", "calamine "]);
}

fn assert_external_network_crates_absent() {
    for package in ["reqwest", "hyper", "ureq"] {
        assert_package_absent(package);
    }
}

fn assert_network_paths_are_reviewed() {
    assert_immediate_dependents("tungstenite", &["dioxus-desktop "]);
    assert_immediate_dependents("native-tls", &["tungstenite "]);
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

fn cargo_tree(args: &[&str]) -> Output {
    Command::new("cargo")
        .args(args)
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
