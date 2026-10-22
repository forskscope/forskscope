//! RFC-094 §2 acceptance criterion 2: "`--format patch` for a file is
//! byte-identical to the GUI's *Export patch*." The GUI's own
//! `export_patch` (`crates/forskscope-ui/src/ui/view/diff_actions.rs`)
//! writes exactly `to_unified(&patch_from_file_diff(rel, &diff,
//! options).unwrap())` to the file it saves. This proves the compiled
//! `forskscope` binary's `compare --format patch` stdout for the same
//! inputs is the same bytes — not by re-deriving the logic, but by
//! calling those same two library functions directly here and comparing.
//!
//! RFC-096: moved here from `forskscope-cli`'s own `tests/`.

use std::process::Command;

use forskscope_compare::diff::{DiffOptions, compute_diff};
use forskscope_report::{PatchOptions, patch_from_file_diff, to_unified};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_forskscope")
}

#[test]
fn format_patch_output_is_byte_identical_to_patch_from_file_diff_plus_to_unified() {
    let tmp = tempfile::tempdir().unwrap();
    let left = tmp.path().join("a.txt");
    let right = tmp.path().join("b.txt");
    std::fs::write(&left, "one\ntwo\nthree\n").unwrap();
    std::fs::write(&right, "one\nTWO\nthree\n").unwrap();

    let output = Command::new(bin())
        .args([
            "compare",
            left.to_str().unwrap(),
            right.to_str().unwrap(),
            "--format",
            "patch",
        ])
        .output()
        .expect("forskscope should run");
    let cli_patch = String::from_utf8(output.stdout).expect("utf-8 output");

    // PatchOptions::default() is both the GUI's starting point (overridden
    // only by the user's own context-lines setting) and compare's own
    // default --context: this compares the two at their shared default.
    let diff = compute_diff(
        "one\ntwo\nthree\n",
        "one\nTWO\nthree\n",
        DiffOptions::default(),
    );
    let patch = patch_from_file_diff("b.txt", &diff, PatchOptions::default())
        .expect("the two files differ, so a patch exists");
    let expected = to_unified(&patch);

    assert_eq!(cli_patch, expected);
}

#[test]
fn format_patch_with_a_non_default_context_matches_the_same_patchoptions() {
    let tmp = tempfile::tempdir().unwrap();
    let left = tmp.path().join("a.txt");
    let right = tmp.path().join("b.txt");
    std::fs::write(&left, "l1\nl2\nl3\nl4\nl5\nCHANGED\nl7\nl8\nl9\nl10\nl11\n").unwrap();
    std::fs::write(
        &right,
        "l1\nl2\nl3\nl4\nl5\nchanged\nl7\nl8\nl9\nl10\nl11\n",
    )
    .unwrap();

    let output = Command::new(bin())
        .args([
            "compare",
            left.to_str().unwrap(),
            right.to_str().unwrap(),
            "--format",
            "patch",
            "--context",
            "1",
        ])
        .output()
        .expect("forskscope should run");
    let cli_patch = String::from_utf8(output.stdout).expect("utf-8 output");

    let diff = compute_diff(
        "l1\nl2\nl3\nl4\nl5\nCHANGED\nl7\nl8\nl9\nl10\nl11\n",
        "l1\nl2\nl3\nl4\nl5\nchanged\nl7\nl8\nl9\nl10\nl11\n",
        DiffOptions::default(),
    );
    let options = PatchOptions {
        context_lines: 1,
        ..PatchOptions::default()
    };
    let patch = patch_from_file_diff("b.txt", &diff, options).expect("the two files differ");
    let expected = to_unified(&patch);

    assert_eq!(cli_patch, expected);
}
