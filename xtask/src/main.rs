//! ForskScope build tasks.
//!
//! Usage:
//!   cargo xtask css           — regenerate assets/main.css from assets/css/*.css
//!   cargo xtask css --check   — verify main.css is current (exits non-zero if stale)
//!   cargo xtask audit-deps    — verify reviewed security dependency paths
//!   cargo xtask i18n          — verify Japanese translations cover UI keys
//!   cargo xtask version-sync [expected] — verify version metadata is in sync (no-arg mode also rejects an already-published version)
//!   cargo xtask archive-layout [archive] — verify source archive layout
//!
//! CSS source files under assets/css/ are assembled in alphabetical order.
//! The numeric prefix on each filename (00-, 01-, …) encodes the cascade order.
//! To add a file: create it with the appropriate prefix; run `cargo xtask css`.

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
        Some("version-sync") if args.len() <= 2 => run_version_sync(args.get(1).map(String::as_str)),
        Some("archive-layout") if args.len() <= 2 => {
            let archive = args.get(1).map(PathBuf::from);
            run_archive_layout_check(archive.as_deref());
        }
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
    eprintln!("       cargo xtask archive-layout [archive]");
}

fn workspace_root() -> PathBuf {
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
    assert_package_absent("sheets-diff");
    assert_package_absent("calamine");
    assert_package_inactive("dioxus-devtools");
    assert_external_network_crates_absent();
    assert_quick_xml_path_is_reviewed();
    assert_network_paths_are_reviewed();
    println!("security dependency path check passed.");
}

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

    let cargo_lock = read_file(&root.join("Cargo.lock"));
    for package in [
        "forskscope-core",
        "forskscope-ui",
        "forskscope-ui-logic",
    ] {
        let expected = format!("name = \"{package}\"\nversion = \"{version}\"");
        if !cargo_lock.contains(&expected) {
            fail(&format!("Cargo.lock package {package} is not version {version}"));
        }
    }

    println!("version sync passed for v{version}.");
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

fn run_archive_layout_check(archive: Option<&Path>) {
    let root = workspace_root();
    let cargo_toml = read_file(&root.join("Cargo.toml"));
    let version = extract_workspace_value(&cargo_toml, "version")
        .unwrap_or_else(|| fail("could not find [workspace.package] version in Cargo.toml"));
    let default_archive = root.join(format!("target/forskscope-v{version}.tar.gz"));
    let archive = archive.unwrap_or(&default_archive);

    let output = Command::new("tar")
        .args(["-tzf"])
        .arg(archive)
        .output()
        .unwrap_or_else(|e| panic!("failed to list {}: {e}", archive.display()));
    if !output.status.success() {
        eprintln!("could not list source archive {}", archive.display());
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        process::exit(1);
    }

    let archive_name = archive
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("source archive");
    let parent_prefix = format!("forskscope-v{version}");
    let mut has_root_cargo_toml = false;
    let mut has_parent_dir = false;
    let mut has_hygiene_violation = false;

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let path = line.strip_prefix("./").unwrap_or(line);
        has_root_cargo_toml |= path == "Cargo.toml";
        has_parent_dir |= path == parent_prefix || path.starts_with(&format!("{parent_prefix}/"));
        has_hygiene_violation |= path == archive_name
            || path == ".git-exclude"
            || path.starts_with(".git-exclude/")
            || path == ".git"
            || path.starts_with(".git/")
            || path == "target"
            || path.starts_with("target/");
    }

    if !has_root_cargo_toml {
        fail("source archive does not contain Cargo.toml at archive root");
    }
    if has_parent_dir {
        fail(&format!(
            "source archive contains forbidden top-level {parent_prefix}/ directory"
        ));
    }
    if has_hygiene_violation {
        fail("source archive contains generated, ignored, or local-only paths");
    }

    println!("source archive layout check passed for {}.", archive.display());
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

fn assert_quick_xml_path_is_reviewed() {
    let output = cargo_tree(&["tree", "--prefix", "depth", "-i", "quick-xml"]);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("did not match any packages") {
            println!("quick-xml is absent.");
            return;
        }

        eprintln!("could not inspect quick-xml dependency path");
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
            .any(|package| !package.starts_with("wayland-scanner "))
    {
        eprintln!("quick-xml has an unreviewed immediate dependency path:");
        eprintln!("{stdout}");
        process::exit(1);
    }

    println!("quick-xml path is limited to wayland-scanner.");
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

fn assert_immediate_dependents(package: &str, accepted_prefixes: &[&str]) {
    let output = cargo_tree(&["tree", "--prefix", "depth", "-i", package]);
    if !output.status.success() {
        eprintln!("could not inspect {package} dependency path");
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
        eprintln!("{package} has an unreviewed immediate dependency path:");
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
