//! F111/F149: the ignore rules reach the recursive directory walk (RFC-056
//! §"Where ignore rules apply": "ignored entries are not walked or reported").
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

fn hide_dotfiles() -> IgnoreRules {
    IgnoreRules {
        hide_dotfiles: true,
        ..Default::default()
    }
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

// ── F149: hide_dotfiles ─────────────────────────────────────────────────────
//
// Issue #146: core's walk sees dotfiles the Explorer tree does not show, so a
// pair differing only in a dotfile was reported as differing while every
// visible row matched. The product's answer is that both halves can be made to
// agree — the Explorer gets a setting, and `hide_dotfiles` is the walk's half
// of it, built on the same `IgnoreRules` mechanism F111 already threads through
// every entry point, not a parallel one.

/// The reproduction itself (before/after in one test, both walks): a pair
/// differing only in a dotfile reports the difference by default — matching
/// what core's walk gave before this handoff existed — and reports nothing
/// once `hide_dotfiles` is set.
#[test]
fn a_pair_differing_only_in_a_dotfile_reports_nothing_once_hidden_entries_are_excluded() {
    let base = tmp("dotfile-diff");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "src/main.rs", "same\n");
    write(&r, "src/main.rs", "same\n");
    write(&l, ".foo/x.txt", "hi");

    // The premise, unfiltered: this is the exact contradiction issue #146
    // reports, and it must still be true before the setting is used.
    let t = CancellationToken::new();
    let unfiltered = recursive_diff_with_cancel(&l, &r, &t);
    assert_eq!(
        entry(&unfiltered, ".foo/x.txt").map(|e| e.status),
        Some(RecStatus::LeftOnly),
        "the premise: core's walk sees the dotfile, got {:?}",
        unfiltered.entries
    );

    for (label, scan) in both(&l, &r, &hide_dotfiles()) {
        assert!(
            names(&scan).iter().all(|n| !n.contains(".foo")),
            "{label}: a hidden entry was listed: {:?}",
            scan.entries
        );
        assert_eq!(names(&scan), vec!["src/main.rs"], "{label}");
    }
    let _ = fs::remove_dir_all(&base);
}

/// A hidden entry is excluded whatever it is — directory or plain file — not
/// only the directory shape the reproduction above uses.
#[test]
fn a_hidden_file_is_excluded_as_well_as_a_hidden_directory() {
    let base = tmp("dotfile-kinds");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, ".env", "SECRET=1");
    write(&r, ".env", "SECRET=2");
    write(&l, ".config/x", "a");
    write(&l, "keep.txt", "k");
    write(&r, "keep.txt", "k");

    for (label, scan) in both(&l, &r, &hide_dotfiles()) {
        assert_eq!(
            names(&scan),
            vec!["keep.txt"],
            "{label}: {:?}",
            scan.entries
        );
    }
    let _ = fs::remove_dir_all(&base);
}

/// A hidden directory is **not descended into** — same proof as F111's
/// unreadable-directory test: made unreadable, and the walk neither fails on
/// it, nor flags it, nor flags a root. Without the setting the same tree
/// reports it `Unreadable` (the premise).
#[cfg(unix)]
#[test]
fn a_hidden_directory_is_not_walked_not_merely_hidden() {
    use std::os::unix::fs::PermissionsExt;
    let base = tmp("dotfile-notwalked");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "keep.txt", "k");
    write(&r, "keep.txt", "k");
    write(&l, ".git/HEAD", "x");
    write(&r, ".git/HEAD", "x");
    let blocked = l.join(".git");
    let _ = fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000));
    if fs::read_dir(&blocked).is_ok() {
        eprintln!("skipping a_hidden_directory_is_not_walked: chmod had no effect (root?)");
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

    for (label, scan) in both(&l, &r, &hide_dotfiles()) {
        assert!(
            scan.entries
                .iter()
                .all(|e| e.status != RecStatus::Unreadable),
            "{label}: a hidden, unreadable directory was flagged: {:?}",
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
        "the premise: without hide_dotfiles the unreadable .git is flagged"
    );
}

/// Issue #145's exact case, pinned again under F149: name-based ignore and
/// `hide_dotfiles` are independent mechanisms on the same struct, and must not
/// interact. `.git` ignored by name alone (hide_dotfiles off, as issue #145's
/// own fix shipped it) still reports equal; `hide_dotfiles` alone (no name
/// rule) also reports equal, since `.git` is itself hidden; neither needs the
/// other.
#[test]
fn issue_145s_dot_git_case_is_unaffected_by_hide_dotfiles_either_way() {
    let base = tmp("issue-145");
    let (l, r) = (base.join("l"), base.join("r"));
    write(&l, "src/main.rs", "same\n");
    write(&r, "src/main.rs", "same\n");
    write(&l, ".git/HEAD", "ref: refs/heads/main\n");
    write(&r, ".git/HEAD", "ref: refs/heads/other\n");

    // Named-ignore alone (hide_dotfiles stays off): still reports equal.
    for (label, scan) in both(&l, &r, &rules("", ".git")) {
        assert_eq!(
            names(&scan),
            vec!["src/main.rs"],
            "{label}: name rule alone"
        );
    }
    // hide_dotfiles alone (no name rule): also reports equal, independently.
    for (label, scan) in both(&l, &r, &hide_dotfiles()) {
        assert_eq!(
            names(&scan),
            vec!["src/main.rs"],
            "{label}: hide_dotfiles alone"
        );
    }
    // Not ignored at all: the difference is reported (today's behaviour,
    // unchanged by this handoff).
    let t = CancellationToken::new();
    let unfiltered = recursive_diff_with_cancel(&l, &r, &t);
    assert!(
        unfiltered
            .entries
            .iter()
            .any(|e| e.status != RecStatus::Equal),
        "the premise: unignored, the difference is reported"
    );
    let _ = fs::remove_dir_all(&base);
}

/// `IgnoreRules::is_empty()` must not let `hide_dotfiles` alone look empty —
/// the early return in `is_ignored` would silently skip the check it guards.
#[test]
fn hide_dotfiles_alone_is_not_an_empty_ruleset() {
    assert!(!hide_dotfiles().is_empty());
    assert!(IgnoreRules::default().is_empty());
}

/// Review 133 §1: a dotfile whose name is not valid UTF-8 is still a dotfile —
/// the tree crate's own rule reasons on the first raw byte, not a decoded
/// character, and `is_ignored` must check `hide_dotfiles` the same way, before
/// its `to_str()` guard. Unix-only: Windows paths cannot hold this name.
#[cfg(unix)]
#[test]
fn a_non_utf8_dotfile_is_excluded_once_hidden_entries_are_excluded() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let base = tmp("dotfile-non-utf8");
    let (l, r) = (base.join("l"), base.join("r"));
    fs::create_dir_all(&l).unwrap();
    fs::create_dir_all(&r).unwrap();
    write(&l, "keep.txt", "k");
    write(&r, "keep.txt", "k");
    // ".\xFFx" - a dotfile whose name is not valid UTF-8.
    let bad_name = OsStr::from_bytes(b".\xFFx");
    fs::write(l.join(bad_name), "hi").unwrap();

    // The premise, unfiltered: the walk sees it, by whatever lossy name
    // `rel_path` renders it as - not asserted on the name, only that *some*
    // extra entry beyond keep.txt is reported.
    let t = CancellationToken::new();
    let unfiltered = recursive_diff_with_cancel(&l, &r, &t);
    assert_eq!(
        unfiltered.entries.len(),
        2,
        "the premise: the non-UTF-8 dotfile is seen unfiltered, got {:?}",
        names(&unfiltered)
    );

    for (label, scan) in both(&l, &r, &hide_dotfiles()) {
        assert_eq!(
            names(&scan),
            vec!["keep.txt"],
            "{label}: a non-UTF-8 dotfile was still reported: {:?}",
            scan.entries
        );
    }
    let _ = fs::remove_dir_all(&base);
}
