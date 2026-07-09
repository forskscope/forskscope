//! ForskScope build tasks.
//!
//! Usage:
//!   cargo xtask css           — regenerate assets/main.css from assets/css/*.css
//!   cargo xtask css --check   — verify main.css is current (exits non-zero if stale)
//!   cargo xtask audit-deps    — verify reviewed security dependency paths
//!
//! CSS source files under assets/css/ are assembled in alphabetical order.
//! The numeric prefix on each filename (00-, 01-, …) encodes the cascade order.
//! To add a file: create it with the appropriate prefix; run `cargo xtask css`.

use std::{
    fs,
    path::PathBuf,
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
    assert_quick_xml_path_is_reviewed();
    println!("security dependency path check passed.");
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
