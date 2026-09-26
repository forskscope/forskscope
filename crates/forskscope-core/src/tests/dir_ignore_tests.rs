//! F111: the ignore rules reach the recursive directory walk (RFC-056 §"Where
//! ignore rules apply": "ignored entries are not walked or reported").
//!
//! Real temporary trees. Every "not walked" claim is shown by making the ignored
//! subtree *unreadable*, so a walk that descended would fail or flag it; a walk
//! that merely hid the entries afterwards could not pass.

use std::fs;
use std::path::{Path, PathBuf};

use crate::CancellationToken;
use crate::IgnoreRules;
use crate::dir::{
    RecEntry, RecStatus, RecursiveScan, list_recursive_for_display_with_cancel,
    list_recursive_for_display_with_rules, recursive_diff_with_cancel, recursive_diff_with_rules,
};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fsk-dirignore-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn write(base: &Path, rel: &str, content: &str) {
    let p = base.join(rel);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(p, content).unwrap();
}

fn rules(exts: &str, dirs: &str) -> IgnoreRules {
    IgnoreRules::from_settings(exts, dirs)
}

fn names(scan: &RecursiveScan) -> Vec<String> {
    scan.entries
        .iter()
        .map(|e| e.rel_path.to_string_lossy().replace('\\', "/"))
        .collect()
}

fn entry<'a>(scan: &'a RecursiveScan, rel: &str) -> Option<&'a RecEntry> {
    scan.entries
        .iter()
        .find(|e| e.rel_path.to_string_lossy().replace('\\', "/") == rel)
}

/// Both walks, so the two entry points cannot drift: (label, scan).
fn both(left: &Path, right: &Path, r: &IgnoreRules) -> [(&'static str, RecursiveScan); 2] {
    let t = CancellationToken::new();
    [
        (
            "display",
            list_recursive_for_display_with_rules(left, right, &t, r),
        ),
        ("full", recursive_diff_with_rules(left, right, &t, r)),
    ]
}

/// Issue #145: two checkouts that differ **only inside `.git`**. With `.git`
/// ignored the comparison finds nothing and lists no `.git` entry; without the
/// rule the same pair reports the differences, so this is the check that would
/// have caught it.
#[test]
fn two_trees_that_differ_only_inside_dot_git_have_no_differences_once_it_is_ignored() {
    let base = tmp("dotgit");
    let (l, r) = (base.join("l"), base.join("r"));
    for side in [&l, &r] {
        write(side, "src/main.rs", "same\n");
        write(side, "README.md", "same\n");
    }
    write(&l, ".git/HEAD", "ref: refs/heads/main\n");
    write(&l, ".git/objects/aa/one", "left object");
    write(&r, ".git/HEAD", "ref: refs/heads/other\n");
    write(&r, ".git/index", "right only");

    // The control: with no rule, the differences are reported.
    let t = CancellationToken::new();
    let unfiltered = recursive_diff_with_cancel(&l, &r, &t);
    assert!(
        names(&unfiltered).iter().any(|n| n.starts_with(".git")),
        "the premise: without the rule .git is compared, got {:?}",
        names(&unfiltered)
    );
    assert!(
        unfiltered
            .entries
            .iter()
            .any(|e| e.status != RecStatus::Equal)
    );

    for (label, scan) in both(&l, &r, &rules("", ".git")) {
        assert!(
            names(&scan).iter().all(|n| !n.starts_with(".git")),
            "{label}: a .git entry was listed: {:?}",
            names(&scan)
        );
        if label == "full" {
            assert!(
                scan.entries.iter().all(|e| e.status == RecStatus::Equal),
                "full: only the ignored directory differed, got {:?}",
                scan.entries
            );
        } else {
            // The fast listing does not compute verdicts: common files are
            // `Computing`, and nothing is one-sided.
            assert!(
                scan.entries
                    .iter()
                    .all(|e| e.status == RecStatus::Computing),
                "display: {:?}",
                scan.entries
            );
        }
        assert_eq!(names(&scan), vec!["README.md", "src/main.rs"], "{label}");
    }
    let _ = fs::remove_dir_all(&base);
}

/// An ignored directory is **not descended into**: it is made unreadable, and
/// the walk neither fails on it, nor flags it, nor flags a root. Without the
/// rule the same tree reports it `Unreadable` (the premise).
#[cfg(unix)]
#[test]
fn an_ignored_directory_is_not_walked_not_merely_hidden() {
    use std::os::unix::fs::PermissionsExt;
    let base = tmp("notwalked");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "keep.txt", "k");
    write(&r, "keep.txt", "k");
    write(&l, ".git/HEAD", "x");
    write(&r, ".git/HEAD", "x");
    let blocked = l.join(".git");
    let _ = fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000));
    if fs::read_dir(&blocked).is_ok() {
        eprintln!("skipping an_ignored_directory_is_not_walked: chmod had no effect (root?)");
        let _ = fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755));
        let _ = fs::remove_dir_all(&base);
        return;
    }

    let t = CancellationToken::new();
    let premise = list_recursive_for_display_with_cancel(&l, &r, &t);
    let flagged = premise
        .entries
        .iter()
        .any(|e| e.status == RecStatus::Unreadable);

    for (label, scan) in both(&l, &r, &rules("", ".git")) {
        assert!(
            scan.entries
                .iter()
                .all(|e| e.status != RecStatus::Unreadable),
            "{label}: an ignored, unreadable directory was flagged: {:?}",
            scan.entries
        );
        assert!(
            !scan.left_root_unreadable && !scan.right_root_unreadable,
            "{label}"
        );
        assert_eq!(names(&scan), vec!["keep.txt"], "{label}");
    }

    let _ = fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755));
    let _ = fs::remove_dir_all(&base);
    assert!(
        flagged,
        "the premise: without the rule the unreadable .git is flagged"
    );
}

/// An ignored file is not reported on either side, including when it exists on
/// one side only, and a differing ignored file is not a difference.
#[test]
fn an_ignored_file_is_not_reported_even_when_one_sided_or_different() {
    let base = tmp("files");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "a.log", "left only");
    write(&r, "b.LOG", "right only"); // extensions are case-insensitive
    write(&l, "c.log", "one");
    write(&r, "c.log", "two");
    write(&l, "sub/d.log", "x");
    write(&l, "keep.txt", "same");
    write(&r, "keep.txt", "same");
    write(&l, "only.txt", "left only, not ignored");

    for (label, scan) in both(&l, &r, &rules("log", "")) {
        assert_eq!(
            names(&scan),
            vec!["keep.txt", "only.txt"],
            "{label}: {:?}",
            scan.entries
        );
        assert_eq!(
            entry(&scan, "only.txt").unwrap().status,
            RecStatus::LeftOnly,
            "{label}"
        );
    }
    let _ = fs::remove_dir_all(&base);
}

/// Each kind of rule applies to its own kind of entry: a directory rule does
/// not hide a *file* of that name, and an extension rule does not hide a
/// *directory* called `x.log` (whose contents are still compared).
#[test]
fn a_rule_applies_only_to_the_kind_of_entry_it_names() {
    let base = tmp("kinds");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "build", "a file called build");
    write(&r, "build", "a file called build");
    write(&l, "x.log/inner.txt", "one");
    write(&r, "x.log/inner.txt", "two");

    let t = CancellationToken::new();
    let scan = recursive_diff_with_rules(&l, &r, &t, &rules("log", "build"));
    assert_eq!(
        names(&scan),
        vec!["build", "x.log/inner.txt"],
        "{:?}",
        scan.entries
    );
    assert_eq!(
        entry(&scan, "x.log/inner.txt").unwrap().status,
        RecStatus::Changed
    );
    let _ = fs::remove_dir_all(&base);
}

/// Wildcards in directory patterns work in the walk as they do in the Explorer.
#[test]
fn a_directory_wildcard_pattern_is_applied() {
    let base = tmp("glob");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "build.cache/x", "1");
    write(&r, "build.cache/y", "2");
    write(&l, "src/a", "same");
    write(&r, "src/a", "same");
    for (label, scan) in both(&l, &r, &rules("", "*.cache")) {
        assert_eq!(names(&scan), vec!["src/a"], "{label}");
    }
    let _ = fs::remove_dir_all(&base);
}

/// The default (empty rules) changes nothing: the rule-taking entry points give
/// exactly what the unchanged ones give, on a tree with every status.
#[test]
fn empty_rules_give_exactly_the_unfiltered_result() {
    let base = tmp("empty");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "same.txt", "s");
    write(&r, "same.txt", "s");
    write(&l, "diff.txt", "1");
    write(&r, "diff.txt", "22");
    write(&l, "left.txt", "l");
    write(&r, "right.txt", "r");
    write(&l, ".git/HEAD", "x");
    write(&l, "d/e.log", "x");
    let t = CancellationToken::new();
    let none = IgnoreRules::default();
    assert_eq!(
        recursive_diff_with_rules(&l, &r, &t, &none).entries,
        recursive_diff_with_cancel(&l, &r, &t).entries
    );
    assert_eq!(
        list_recursive_for_display_with_rules(&l, &r, &t, &none).entries,
        list_recursive_for_display_with_cancel(&l, &r, &t).entries
    );
    let _ = fs::remove_dir_all(&base);
}

/// A symlink is judged by what it points at: one to a directory named by a
/// directory rule is ignored; without the rule it is reported.
#[cfg(unix)]
#[test]
fn a_symlink_to_an_ignored_directory_is_ignored() {
    let base = tmp("symlink");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "real/f", "x");
    fs::create_dir_all(&r).unwrap();
    std::os::unix::fs::symlink(l.join("real"), l.join("vendor")).unwrap();

    let t = CancellationToken::new();
    let plain = recursive_diff_with_cancel(&l, &r, &t);
    assert_eq!(
        entry(&plain, "vendor").map(|e| e.status),
        Some(RecStatus::Symlink)
    );
    let ignored = recursive_diff_with_rules(&l, &r, &t, &rules("", "vendor"));
    assert!(entry(&ignored, "vendor").is_none(), "{:?}", ignored.entries);
    let _ = fs::remove_dir_all(&base);
}
