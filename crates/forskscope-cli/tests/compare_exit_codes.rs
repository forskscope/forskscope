//! Exit codes follow `diff` (RFC-094 §2 acceptance criterion 2): 0 same, 1
//! different, 2 error or refusal. Runs the actual compiled binary — the
//! only honest way to test a program's own exit code and stdout/stderr
//! split, matching this project's other "run the real program" test files
//! (e.g. `tests/patch_apply.rs`'s round trip through the real `patch`/
//! `git apply` binaries).

use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_forskscope-cli")
}

fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("forskscope-cli should run")
}

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

#[test]
fn identical_files_exit_zero_and_print_no_patch() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a.txt", "same\n");
    write(tmp.path(), "b.txt", "same\n");
    let a = tmp.path().join("a.txt").display().to_string();
    let b = tmp.path().join("b.txt").display().to_string();

    let out = run(&["compare", &a, &b, "--format", "text"]);
    assert_eq!(out.status.code(), Some(0));

    let out = run(&["compare", &a, &b, "--format", "patch"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        out.stdout.is_empty(),
        "identical files have nothing to patch"
    );
}

#[test]
fn different_files_exit_one() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a.txt", "one\ntwo\n");
    write(tmp.path(), "b.txt", "one\nTWO\n");
    let a = tmp.path().join("a.txt").display().to_string();
    let b = tmp.path().join("b.txt").display().to_string();

    let out = run(&["compare", &a, &b]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn a_missing_path_exits_two_with_one_stderr_line() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a.txt", "x\n");
    let a = tmp.path().join("a.txt").display().to_string();
    let missing = tmp.path().join("does-not-exist.txt").display().to_string();

    let out = run(&["compare", &a, &missing]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        stderr.lines().count(),
        1,
        "exactly one stderr line naming the path and the reason, got: {stderr}"
    );
    assert!(stderr.contains("does-not-exist.txt"));
}

#[test]
fn a_file_against_a_folder_is_refused_with_exit_two() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a.txt", "x\n");
    std::fs::create_dir(tmp.path().join("folder")).unwrap();
    let a = tmp.path().join("a.txt").display().to_string();
    let dir = tmp.path().join("folder").display().to_string();

    let out = run(&["compare", &a, &dir]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("folder"));
}

#[test]
fn identical_folders_exit_zero() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    write(left.path(), "f.txt", "x\n");
    write(right.path(), "f.txt", "x\n");

    let out = run(&[
        "compare",
        &left.path().display().to_string(),
        &right.path().display().to_string(),
    ]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn different_folders_exit_one() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    write(left.path(), "f.txt", "x\n");
    write(right.path(), "f.txt", "y\n");

    let out = run(&[
        "compare",
        &left.path().display().to_string(),
        &right.path().display().to_string(),
    ]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn an_unrecognized_option_is_an_exit_two_refusal_before_anything_is_read() {
    let out = run(&["compare", "a", "b", "--not-a-real-flag"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn json_output_parses_as_the_versioned_schema() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a.txt", "one\ntwo\n");
    write(tmp.path(), "b.txt", "one\nTWO\n");
    let a = tmp.path().join("a.txt").display().to_string();
    let b = tmp.path().join("b.txt").display().to_string();

    let out = run(&["compare", &a, &b, "--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(doc["schema"], "forskscope.comparison/1");
    assert_eq!(doc["kind"], "file");
    assert_eq!(doc["identical"], false);
}

// F211: a file that exists on both sides but cannot be read must be
// `unreadable`/"Unreadable", never a verdict of "different" - exit code 1
// either way (review 183's exit-code clarification: "not compared" is not
// "the same"), but the status itself must say why, not assert a measurement
// that was never taken. `chmod 000` does not stop root, so this is
// `#[cfg(unix)]` and skips, with an explicit message, if run as root -
// `forskscope-compare`'s own permission-guard tests follow the same rule,
// for the same reason (a suite running as root would otherwise pass this
// for the wrong reason).
#[cfg(unix)]
#[test]
fn an_unreadable_file_inside_a_folder_is_reported_unreadable_never_different() {
    use std::os::unix::fs::PermissionsExt;

    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    // Same length, different bytes - were the content actually read, this
    // would be "different", so passing here is proof the read never
    // happened, not a coincidence of equal content.
    write(left.path(), "n.txt", "left-content");
    write(right.path(), "n.txt", "right-conten");

    let blocked = right.path().join("n.txt");
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&blocked).is_ok() {
        eprintln!(
            "skipping an_unreadable_file_inside_a_folder_is_reported_unreadable_never_different: \
             chmod 000 had no effect (running as root?)"
        );
        std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o644)).unwrap();
        return;
    }

    let json_out = run(&[
        "compare",
        &left.path().display().to_string(),
        &right.path().display().to_string(),
        "--format",
        "json",
    ]);
    let text_out = run(&[
        "compare",
        &left.path().display().to_string(),
        &right.path().display().to_string(),
        "--format",
        "text",
    ]);
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(
        json_out.status.code(),
        Some(1),
        "not compared is not the same"
    );
    let doc: serde_json::Value = serde_json::from_slice(&json_out.stdout).expect("valid JSON");
    let entries = doc["entries"].as_array().expect("a directory document");
    let entry = entries
        .iter()
        .find(|e| e["path"] == "n.txt")
        .unwrap_or_else(|| panic!("n.txt must be listed: {doc}"));
    assert_eq!(entry["status"], "unreadable", "{doc}");

    assert_eq!(text_out.status.code(), Some(1));
    let text = String::from_utf8_lossy(&text_out.stdout);
    assert!(text.contains("n.txt"), "{text}");
    assert!(text.contains("Unreadable"), "{text}");
    assert!(!text.contains("Different"), "{text}");
}

#[test]
fn version_and_help_exit_zero_and_write_nothing_to_stderr() {
    let out = run(&["--version"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
    assert!(String::from_utf8_lossy(&out.stdout).contains("forskscope-cli"));

    let out = run(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
}
