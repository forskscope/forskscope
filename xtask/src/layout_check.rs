//! RFC-098 stage B (handoff 101, after review 216's answer to `188`):
//! `cargo xtask layout-check` — the `layout-probe`'s own driver.
//!
//! Generates fixtures, writes the plans, builds `forskscope` with the
//! `layout-probe` feature, runs the matrix, and reports every finding
//! plus the cost figures.
//!
//! **Two axis groups, review 216 §2's own split** — "layout checks need
//! text size × language × window × view at one colour setting; rendered
//! contrast needs the full colour axes at two layouts" — so this driver
//! writes two different shapes of plan, one window (or layout) at a
//! time, rather than one plan trying to be both:
//! - [`write_layout_checks_plan`]: one per [`DEFAULT_WINDOWS`] entry,
//!   the full text-size/language sweep, Light/Standard/Standard fixed.
//! - [`write_contrast_plan`]: one per [`CONTRAST_LAYOUTS`] entry, the
//!   full theme/contrast/colour-vision sweep, one fixed text size/
//!   window pair, English only.
//!
//! Both run every check the probe computes (it has no notion of "this
//! check belongs to that axis group" - the economy is entirely in which
//! combinations each plan enumerates, not in the measurement script).
//!
//! **Not yet implemented here:** `--inject <defect>`; the release-binary
//! marker check; CI wiring on every push; the retroactive proof.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The RFC's own matrix (§3), minus the colour axes the layout checks
/// don't need (neither overflow nor clipped text, nor any of stage B's
/// own six, depends on colour). `"WxH"`, read back by the probe itself
/// (`layout_probe::parse_window_size`); the driver no longer parses or
/// applies these numbers itself, since review 216 §4 moved the resize
/// in process, through the desktop context, rather than an external
/// resize after launch.
const DEFAULT_WINDOWS: &[&str] = &["1000x700", "800x560", "1366x768"];
const DEFAULT_TEXT_SIZES: &[u32] = &[100, 125, 150, 175, 200];
const DEFAULT_LANGUAGES: &[&str] = &["en", "ja"];

/// Rendered contrast's own two layouts (RFC-098 §3): side by side at
/// the listing's own size, and stacked - "where elements appear and
/// hide" - at a narrow window and a large text size together, review
/// 216 §2's exact pair. `(window, text_size)`.
const CONTRAST_LAYOUTS: &[(&str, u32)] = &[("1000x700", 100), ("800x560", 200)];
const CONTRAST_THEMES: &[&str] = &["dark", "light", "night"];
const CONTRAST_CONTRASTS: &[&str] = &["standard", "high"];
const CONTRAST_COLOUR_VISIONS: &[&str] = &["standard", "red-green-safe"];

pub struct RunOptions {
    pub release: bool,
    pub out_dir: PathBuf,
    /// RFC-098 §4's own "falsifiable: an `--inject <defect>` flag" -
    /// one of [`INJECTABLE_DEFECTS`]'s own names. When set, `run` skips
    /// the full matrix entirely and instead plants that one defect,
    /// confirms the matching check reports it, then reverts the patch
    /// (a `Drop` guard, so a panic mid-check still restores the source
    /// tree) and exits - local and CI-dispatch only, per the RFC, never
    /// the default push-triggered job.
    pub inject: Option<String>,
}

/// `(name, check)` - every `--inject` name `run_inject` understands,
/// and which [`ResultRow`] field (by name, matching `report`'s own
/// list) the planted defect must show up in. Printed by `--inject` with
/// no name, or an unrecognized one, so the flag is self-documenting.
const INJECTABLE_DEFECTS: &[(&str, &str)] = &[
    ("overflow", "overflow"),
    ("clipped", "clipped"),
    ("off-screen", "off_screen"),
    ("hit-area", "hit_area"),
    ("text-size", "small_text"),
    ("rendered-contrast", "rendered_contrast"),
    ("label-placement", "label_placement"),
    ("dialog-occlusion", "dialog_occlusion"),
    ("overlap", "overlap"),
];

/// Every fixture path stage B's own views need, generated once and
/// reused by every plan this run writes.
struct Fixtures {
    file_left: PathBuf,
    file_right: PathBuf,
    folder_left: PathBuf,
    folder_right: PathBuf,
    merge_local: PathBuf,
    merge_remote: PathBuf,
    merge_merged: PathBuf,
}

/// Review 220 §2.5: a run invoked bare (no `session.sh`, no `xvfb-run
/// dbus-run-session` wrapper around it) inherits the *caller's* own
/// `DBUS_SESSION_BUS_ADDRESS` - the real desktop's session bus, locally,
/// since `Command::new` passes the parent's environment through
/// unchanged by default. RFC-097 stage 2's own startup portal read
/// (`org.freedesktop.portal.Settings`, for the colour scheme) then asks
/// the *maintainer's real desktop*, not the isolated plan - exactly the
/// gap review 185's own portal investigation found and named
/// `.git-exclude/gui/no-activation-session.conf` for. That file can't be
/// read here: CI's checkout never has `.git-exclude/` at all. Writing
/// the identical rules into `target/` from code, the same way fixtures
/// already are, is the equivalent the review itself asked for - no
/// `<standard_session_servicedirs/>` at all, so a call to an unowned
/// name fails immediately (`ServiceUnknown`) instead of activating any
/// real service, portal included, on what `dbus-run-session` believes
/// is a fresh, empty bus.
const NO_ACTIVATION_SESSION_CONF: &str = r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <keep_umask/>
  <listen>unix:tmpdir=/tmp</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#;

fn write_no_activation_conf(out_dir: &Path) -> PathBuf {
    let path = out_dir.join("no-activation-session.conf");
    fs::write(&path, NO_ACTIVATION_SESSION_CONF)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    path
}

pub fn run(root: &Path, opts: &RunOptions) {
    if let Some(defect) = &opts.inject {
        run_inject(root, opts, defect);
        return;
    }
    // Review 216 §4: "every run starts empty" - a stale prior run's own
    // data dirs (tabs accumulated on disk) must never carry forward, not
    // just between window sizes within one run (handled per-window
    // below already) but across separate invocations at the same
    // `--out-dir`, which is exactly the shape that hid F218 on a first
    // retroactive-proof attempt.
    let _ = fs::remove_dir_all(&opts.out_dir);
    fs::create_dir_all(&opts.out_dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", opts.out_dir.display()));
    let bus_conf = write_no_activation_conf(&opts.out_dir);

    let fixtures_dir = opts.out_dir.join("fixtures");
    let (file_left, file_right) = write_file_comparison_fixture(&fixtures_dir);
    let (folder_left, folder_right) = write_folder_fixture(&fixtures_dir);
    let (merge_local, merge_remote, merge_merged) = write_mergetool_fixture(&fixtures_dir);
    let fixtures = Fixtures {
        file_left,
        file_right,
        folder_left,
        folder_right,
        merge_local,
        merge_remote,
        merge_merged,
    };

    let bin = build_binary(root, opts.release);

    let mut all_rows: Vec<serde_json::Value> = Vec::new();
    let mut per_window_ms: Vec<(String, u128)> = Vec::new();

    for &window_label in DEFAULT_WINDOWS {
        let plan_path = opts
            .out_dir
            .join(format!("plan-layout-{window_label}.json"));
        let results_path = opts
            .out_dir
            .join(format!("results-layout-{window_label}.jsonl"));
        let data_dir = opts.out_dir.join(format!("data-layout-{window_label}"));

        write_layout_checks_plan(
            &plan_path,
            &results_path,
            &fixtures,
            DEFAULT_TEXT_SIZES,
            DEFAULT_LANGUAGES,
            window_label,
        );

        println!("layout-check: running layout checks, window {window_label}...");
        let elapsed = launch_and_wait(&bin, &plan_path, &data_dir, &bus_conf);
        per_window_ms.push((format!("layout {window_label}"), elapsed.as_millis()));

        for row in read_jsonl(&results_path) {
            all_rows.push(row);
        }
    }

    for &(window_label, text_size) in CONTRAST_LAYOUTS {
        let plan_path = opts
            .out_dir
            .join(format!("plan-contrast-{window_label}-{text_size}.json"));
        let results_path = opts
            .out_dir
            .join(format!("results-contrast-{window_label}-{text_size}.jsonl"));
        let data_dir = opts
            .out_dir
            .join(format!("data-contrast-{window_label}-{text_size}"));

        write_contrast_plan(
            &plan_path,
            &results_path,
            &fixtures,
            text_size,
            window_label,
        );

        println!(
            "layout-check: running rendered contrast, window {window_label} at {text_size}%..."
        );
        let elapsed = launch_and_wait(&bin, &plan_path, &data_dir, &bus_conf);
        per_window_ms.push((
            format!("contrast {window_label}@{text_size}%"),
            elapsed.as_millis(),
        ));

        for row in read_jsonl(&results_path) {
            all_rows.push(row);
        }
    }

    report(&all_rows, &per_window_ms);
}

// ── --inject ─────────────────────────────────────────────────────────────

/// Appends to `03-components.css` (already loaded by every view) - never
/// a new file, so there's nothing to delete, only a known suffix to cut
/// back off. Reverted by [`InjectedDefect`]'s own `Drop`, panic or not.
const INJECT_CSS_PATH: &str = "crates/forskscope-ui/assets/css/03-components.css";

/// One CSS rule per [`INJECTABLE_DEFECTS`] entry. Most are scoped to
/// the Settings dialog's own *Advanced* section (`.modal .actions
/// button` - its "Close" button - or `#text-size-help`), since Settings
/// is the view every `--inject` run opens by default: real, reachable
/// elements, not a synthetic fixture of their own.
///
/// `label-placement` is the one exception (review 220 §2.4/§4):
/// `label_placement` no longer examines Settings' own form fields at
/// all, only the diff view's own `data-layout-labels` regions - so
/// [`run_inject_guarded`] opens file comparison instead, just for this
/// one defect, and the rule below targets `#pane-label-left-0`, the
/// side-by-side header's own label (the layout's active one at this
/// view's default, wide window size).
fn injected_rule(defect: &str) -> Option<&'static str> {
    Some(match defect {
        // "Close" (5 glyphs) doesn't fit 10px either way - the same
        // rule proves both: an overflowing box (`overflow`) with its
        // own text actually cut off (`clipped`), not ellipsized.
        "overflow" | "clipped" => {
            ".modal .actions button { max-width: 10px !important; \
             white-space: nowrap !important; overflow: hidden !important; }"
        }
        "off-screen" => {
            ".modal .actions button { position: fixed !important; \
             left: -9999px !important; top: -9999px !important; }"
        }
        "hit-area" => {
            ".modal .actions button { min-width: 1px !important; min-height: 1px !important; \
             width: 10px !important; height: 10px !important; padding: 0 !important; }"
        }
        "text-size" => ".modal .actions button { font-size: 8px !important; }",
        "rendered-contrast" => {
            ".modal .actions button { color: #808080 !important; \
             background: #808080 !important; }"
        }
        "label-placement" => {
            "#pane-label-left-0 { position: relative !important; top: 500px !important; }"
        }
        // The regression this whole round started from (review 217/218),
        // reproduced directly: nothing above the scrim to answer "what's
        // on top" once something else claims a higher stacking value -
        // here, a full-viewport `::before` on `body`, not even a real
        // element, which `elementFromPoint` still resolves to `body`
        // itself over the dialog's own box.
        "dialog-occlusion" => {
            "body::before { content: ''; position: fixed; inset: 0; z-index: 99999; }"
        }
        // Review 220 §2.1: the previous target, `#ignore-extensions-
        // label`, sits far enough down Settings' own *Advanced* section
        // that `many_ignore_entries`' own long badge lists scroll it
        // out of the modal's visible area entirely - exactly the
        // clipped-away shape `visibleRect` now correctly excludes from
        // every check, this one included, so the old target stopped
        // proving anything. `#text-size-help` sits right under the
        // dialog's own, always-visible first field instead.
        "overlap" => "#text-size-help { position: relative !important; top: 60px !important; }",
        _ => return None,
    })
}

struct InjectedDefect {
    root: PathBuf,
    path: PathBuf,
    original: String,
}

impl Drop for InjectedDefect {
    fn drop(&mut self) {
        // Found live, first run: reverting only the source file left
        // `main.css` - a generated, but still tracked, artifact - with
        // the injected rule baked in and no further step to notice.
        // `cargo xtask css --check` would have failed on the very next
        // unrelated commit, for a reason nothing in that commit caused.
        if fs::write(&self.path, &self.original).is_err() {
            eprintln!(
                "layout-check: FAILED to revert the injected defect at {} - restore it by hand \
                 from git (git checkout -- {})",
                self.path.display(),
                self.path.display()
            );
            return;
        }
        let status = Command::new("cargo")
            .current_dir(&self.root)
            .arg("xtask")
            .arg("css")
            .status();
        if !matches!(status, Ok(s) if s.success()) {
            eprintln!(
                "layout-check: reverted {} but could not regenerate main.css ({status:?}) - run \
                 `cargo xtask css` by hand",
                self.path.display()
            );
        }
    }
}

/// The thin, guard-free half of `--inject`: resolve the name, then exit.
/// `std::process::exit` never runs a `Drop` impl - calling it from
/// anywhere still holding [`InjectedDefect`] would skip the revert
/// entirely (found live, the first run that failed to catch anything:
/// the injected rule stayed in the tree, since the early `exit(1)` on
/// that path never gave the guard a chance to run). So every exit call
/// lives here, *outside* [`run_inject_guarded`]'s own scope - that
/// function only ever returns a `bool`, letting its guard drop by
/// ordinary scope exit before this caller decides whether to exit at
/// all.
fn run_inject(root: &Path, opts: &RunOptions, defect: &str) {
    let Some(rule) = injected_rule(defect) else {
        eprintln!("layout-check: unknown --inject defect {defect:?}. Known defects:");
        for (name, _) in INJECTABLE_DEFECTS {
            eprintln!("  {name}");
        }
        process::exit(2);
    };
    let field = INJECTABLE_DEFECTS
        .iter()
        .find(|(name, _)| *name == defect)
        .map(|(_, field)| *field)
        .expect("injected_rule and INJECTABLE_DEFECTS name the same set");

    if !run_inject_guarded(root, opts, defect, rule, field) {
        process::exit(1);
    }
}

/// Plants `rule`, builds and runs Settings-only, and reports whether
/// `field` came back non-empty - `true` only on a clean catch. Every
/// return path here (`?`-shaped early returns included) drops its own
/// `_guard` on the way out, reverting the CSS and regenerating
/// `main.css` to match, so the caller is free to exit however it likes
/// once this returns.
fn run_inject_guarded(
    root: &Path,
    opts: &RunOptions,
    defect: &str,
    rule: &str,
    field: &str,
) -> bool {
    let css_path = root.join(INJECT_CSS_PATH);
    let original = fs::read_to_string(&css_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", css_path.display()));
    fs::write(
        &css_path,
        format!("{original}\n/* --inject {defect}, reverted after this run */\n{rule}\n"),
    )
    .unwrap_or_else(|e| panic!("cannot write {}: {e}", css_path.display()));
    let _guard = InjectedDefect {
        root: root.to_path_buf(),
        path: css_path,
        original,
    };

    let status = Command::new("cargo")
        .current_dir(root)
        .arg("xtask")
        .arg("css")
        .status()
        .expect("failed to run cargo xtask css");
    if !status.success() {
        eprintln!("layout-check: --inject could not regenerate main.css");
        return false;
    }

    let _ = fs::remove_dir_all(&opts.out_dir);
    fs::create_dir_all(&opts.out_dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", opts.out_dir.display()));
    let bus_conf = write_no_activation_conf(&opts.out_dir);
    let bin = build_binary(root, opts.release);
    let plan_path = opts.out_dir.join("plan-inject.json");
    let results_path = opts.out_dir.join("results-inject.jsonl");
    let data_dir = opts.out_dir.join("data-inject");
    // One view only, every injected rule's own target - not
    // `all_views_json`'s full six: a prior view's own DOM found live to
    // still answer `querySelectorAll` from inside `#app-root` after
    // switching away from it (nothing unmounts it, only the active
    // view changes), which doesn't belong in a one-check-at-a-time
    // falsification at all. `label-placement` opens file comparison
    // instead of Settings (see `injected_rule`'s own doc comment) -
    // every other defect still targets Settings' own *Advanced*
    // section.
    let (view_kind, view_name) = if defect == "label-placement" {
        let fixtures_dir = opts.out_dir.join("fixtures");
        let (left, right) = write_file_comparison_fixture(&fixtures_dir);
        (
            serde_json::json!({"kind": "file-comparison", "left": left, "right": right}),
            "file-comparison",
        )
    } else {
        (
            serde_json::json!({
                "kind": "settings",
                "ignore_extensions": many_ignore_entries("ext"),
                "ignore_dirs": many_ignore_entries("dir"),
            }),
            "settings",
        )
    };
    let plan = serde_json::json!({
        "results_path": results_path,
        "views": [view_kind],
        "text_sizes": [100],
        "languages": ["en"],
        "window_size": "1000x700",
    });
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap())
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", plan_path.display()));

    println!("layout-check: --inject {defect} - running {view_name} only...");
    launch_and_wait(&bin, &plan_path, &data_dir, &bus_conf);
    let rows = read_jsonl(&results_path);

    let settings_row = rows
        .iter()
        .find(|r| r.get("view").and_then(|v| v.as_str()) == Some(view_name));
    let Some(row) = settings_row else {
        eprintln!("layout-check: --inject {defect}: the {view_name} row itself is missing");
        return false;
    };
    let found = row
        .get(field)
        .and_then(|v| v.as_array())
        .is_some_and(|a| !a.is_empty());

    if found {
        println!(
            "layout-check: --inject {defect} - CAUGHT. {field}: {}",
            serde_json::to_string(&row[field]).unwrap_or_default()
        );
        true
    } else {
        eprintln!(
            "layout-check: --inject {defect} - NOT caught. {field} was empty. Full row: {row}"
        );
        false
    }
}

fn build_binary(root: &Path, release: bool) -> PathBuf {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root)
        .arg("build")
        .arg("-p")
        .arg("forskscope")
        .arg("--features")
        .arg("layout-probe");
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status().expect("failed to run cargo build");
    if !status.success() {
        eprintln!("layout-check: build failed");
        std::process::exit(1);
    }
    let profile = if release { "release" } else { "debug" };
    root.join("target").join(profile).join("forskscope")
}

// ── Fixtures ─────────────────────────────────────────────────────────────

/// A file pair with a collapsible unchanged region (more than
/// `2*context_lines+1` = 7 equal lines in a row, the default
/// `context_lines: 3`) *and* a replace hunk, as the RFC's own §"stage A"
/// point 2 names. Returns `(left, right)`.
fn write_file_comparison_fixture(dir: &Path) -> (PathBuf, PathBuf) {
    fs::create_dir_all(dir).unwrap_or_else(|e| panic!("cannot create {}: {e}", dir.display()));
    let left = dir.join("file-comparison-left.txt");
    let right = dir.join("file-comparison-right.txt");
    let mut left_lines = Vec::new();
    let mut right_lines = Vec::new();
    for i in 1..=8 {
        left_lines.push(format!("unchanged line {i}"));
        right_lines.push(format!("unchanged line {i}"));
    }
    left_lines.push("const CLOSE_HOUR = 17;".to_string());
    right_lines.push("const CLOSE_HOUR = 18;".to_string());
    right_lines.push("const CLOSED_DAYS = [0]; // Sunday".to_string());
    for i in 9..=16 {
        left_lines.push(format!("unchanged line {i}"));
        right_lines.push(format!("unchanged line {i}"));
    }
    fs::write(&left, left_lines.join("\n") + "\n")
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", left.display()));
    fs::write(&right, right_lines.join("\n") + "\n")
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", right.display()));
    (left, right)
}

/// A folder pair exercising every status a fixture can produce, built the
/// same shape as `.git-exclude/fixtures/dirs/status-all` (not reused
/// directly - CI cannot see `.git-exclude/`, the RFC's own point 2).
/// Returns `(left, right)`, opened by stage B's own Deep Compare view.
fn write_folder_fixture(dir: &Path) -> (PathBuf, PathBuf) {
    let left = dir.join("folder-comparison-left");
    let right = dir.join("folder-comparison-right");
    for d in [&left, &right] {
        fs::create_dir_all(d.join("subdir"))
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", d.display()));
    }
    fs::write(left.join("same.txt"), "identical on both sides\n").unwrap();
    fs::write(right.join("same.txt"), "identical on both sides\n").unwrap();
    fs::write(left.join("changed.txt"), "left version\n").unwrap();
    fs::write(right.join("changed.txt"), "right version, different\n").unwrap();
    fs::write(left.join("left-only.txt"), "only on the left\n").unwrap();
    fs::write(right.join("right-only.txt"), "only on the right\n").unwrap();
    fs::write(left.join("subdir/nested.txt"), "nested, identical\n").unwrap();
    fs::write(right.join("subdir/nested.txt"), "nested, identical\n").unwrap();
    fs::write(left.join("binary.bin"), [0u8, 159, 146, 150]).unwrap();
    fs::write(right.join("binary.bin"), [0u8, 159, 146, 151]).unwrap();
    // `unreadable.txt`: deliberately left world-readable here (CI's own
    // checkout has no guarantee of the uid a `chmod 000` would need to
    // matter under) - a driver wanting the `⊘` glyph's own status chmods
    // its session copy, the same way `.git-exclude/gui/session.sh` does
    // for `dirs/status-all`, never this canonical fixture.
    fs::write(
        left.join("unreadable.txt"),
        "would be chmod 000 by a driver\n",
    )
    .unwrap();
    fs::write(
        right.join("unreadable.txt"),
        "would be chmod 000 by a driver\n",
    )
    .unwrap();
    (left, right)
}

/// A three-way merge triple (local/remote/merged), one conflicting region.
/// Returns `(local, remote, merged)`, opened by stage B's own mergetool
/// view.
fn write_mergetool_fixture(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let base_lines = ["one", "two", "three", "four", "five"];
    let local = dir.join("mergetool-local.txt");
    let remote = dir.join("mergetool-remote.txt");
    let merged = dir.join("mergetool-merged.txt");
    let mut local_lines = base_lines.to_vec();
    local_lines[1] = "two, changed locally";
    let mut remote_lines = base_lines.to_vec();
    remote_lines[1] = "two, changed remotely";
    fs::write(&local, local_lines.join("\n") + "\n")
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", local.display()));
    fs::write(&remote, remote_lines.join("\n") + "\n")
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", remote.display()));
    // The merge target: starts as a copy of `local`, same as the app's own
    // mergetool CLI mode does before any conflict is resolved by hand.
    fs::write(&merged, local_lines.join("\n") + "\n")
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", merged.display()));
    (local, remote, merged)
}

// ── Plan ─────────────────────────────────────────────────────────────────

/// RFC-099 §3's own verification: the ignore-list badges "at 200 % holding
/// many entries" - twenty of each, long enough to need several wrapped
/// lines at every window size this matrix covers.
fn many_ignore_entries(prefix: &str) -> String {
    (1..=20)
        .map(|i| format!("{prefix}-entry-{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// RFC-098 §3's own view list - the Explorer, a file comparison with a
/// collapsed region, Deep Compare with a selected row (automatic, RFC-097
/// stage 3 §3), the mergetool layout, Settings with *Advanced* open, and
/// *Shortcuts* - shared by both plan shapes below, since every check runs
/// against every view regardless of which axis group is sweeping.
fn all_views_json(fixtures: &Fixtures) -> serde_json::Value {
    serde_json::json!([
        {"kind": "explorer"},
        {"kind": "file-comparison", "left": fixtures.file_left, "right": fixtures.file_right},
        {"kind": "deep-compare", "left": fixtures.folder_left, "right": fixtures.folder_right},
        {
            "kind": "merge-tool",
            "local": fixtures.merge_local,
            "remote": fixtures.merge_remote,
            "merged": fixtures.merge_merged,
        },
        {
            "kind": "settings",
            "ignore_extensions": many_ignore_entries("ext"),
            "ignore_dirs": many_ignore_entries("dir"),
        },
        {"kind": "shortcuts"},
    ])
}

/// The layout-checks axis group (review 216 §2): text size × language ×
/// window × view, at one fixed colour setting (Light/Standard/Standard -
/// the probe's own defaults when a plan omits these axes entirely).
fn write_layout_checks_plan(
    plan_path: &Path,
    results_path: &Path,
    fixtures: &Fixtures,
    text_sizes: &[u32],
    languages: &[&str],
    window_label: &str,
) {
    let plan = serde_json::json!({
        "results_path": results_path,
        "views": all_views_json(fixtures),
        "text_sizes": text_sizes,
        "languages": languages,
        "window_size": window_label,
    });
    fs::write(plan_path, serde_json::to_string_pretty(&plan).unwrap())
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", plan_path.display()));
}

/// The rendered-contrast axis group (review 216 §2): theme × colour
/// vision × contrast × view, at one fixed (window, text size) layout -
/// English only, since the contrast check doesn't read any text.
fn write_contrast_plan(
    plan_path: &Path,
    results_path: &Path,
    fixtures: &Fixtures,
    text_size: u32,
    window_label: &str,
) {
    let plan = serde_json::json!({
        "results_path": results_path,
        "views": all_views_json(fixtures),
        "text_sizes": [text_size],
        "languages": ["en"],
        "themes": CONTRAST_THEMES,
        "contrasts": CONTRAST_CONTRASTS,
        "colour_visions": CONTRAST_COLOUR_VISIONS,
        "window_size": window_label,
    });
    fs::write(plan_path, serde_json::to_string_pretty(&plan).unwrap())
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", plan_path.display()));
}

// ── Launch ───────────────────────────────────────────────────────────────

/// Launches `bin` with the plan env var set and an isolated `--data-dir`/
/// XDG set, and waits for it to exit - the probe calls
/// `std::process::exit(0)` itself once its own matrix is measured, so
/// this never sends a signal on the success path. The window size comes
/// from the plan itself, applied in process (review 216 §4) - no
/// external resize step, no `xdotool` dependency, no `DISPLAY` check.
/// Returns the wall-clock time from launch to exit.
///
/// Review 220 §2.5: wrapped in `dbus-run-session --config-file
/// <bus_conf>`, every launch, locally and in CI alike - a bare
/// `Command::new(bin)` inherits the caller's own
/// `DBUS_SESSION_BUS_ADDRESS` unchanged, which is the real desktop's
/// session bus on a plain local run. `dbus-run-session` gives this
/// child its own fresh bus and sets the env var to it; `--config-file`
/// (see [`NO_ACTIVATION_SESSION_CONF`]) is what then stops that fresh
/// bus from activating any real service, the portal included, the
/// moment something asks for a name nobody on it owns yet.
fn launch_and_wait(bin: &Path, plan_path: &Path, data_dir: &Path, bus_conf: &Path) -> Duration {
    fs::create_dir_all(data_dir.join("xdg/config")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/state")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/cache")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/data")).unwrap();

    let start = Instant::now();
    let mut child = Command::new("dbus-run-session")
        .arg("--config-file")
        .arg(bus_conf)
        .arg("--")
        .arg(bin)
        .arg("--data-dir")
        .arg(data_dir)
        .env("FORSKSCOPE_LAYOUT_PROBE_PLAN", plan_path)
        .env("XDG_CONFIG_HOME", data_dir.join("xdg/config"))
        .env("XDG_STATE_HOME", data_dir.join("xdg/state"))
        .env("XDG_CACHE_HOME", data_dir.join("xdg/cache"))
        .env("XDG_DATA_HOME", data_dir.join("xdg/data"))
        // Review 202 (this sandbox): AT-SPI is unreachable here and the
        // probe doesn't use it anyway - `none` avoids the same multi-
        // second stall `session.sh` already found waiting on a
        // connection attempt that can never complete.
        .env("GTK_A11Y", "none")
        .env("GDK_BACKEND", "x11")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch forskscope under dbus-run-session");

    let timeout = Duration::from_secs(60);
    let exited = wait_with_timeout(&mut child, timeout);
    if !exited {
        eprintln!(
            "layout-check: forskscope did not exit within {}s - killing it (an unfinished plan \
             means fewer rows than expected, not a crash)",
            timeout.as_secs()
        );
        let _ = child.kill();
        let _ = child.wait();
    }
    start.elapsed()
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => return true,
            Ok(None) => {
                if start.elapsed() > timeout {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                eprintln!("layout-check: error waiting for forskscope: {e}");
                return false;
            }
        }
    }
}

fn read_jsonl(path: &Path) -> Vec<serde_json::Value> {
    let Ok(file) = fs::File::open(path) else {
        eprintln!(
            "layout-check: no results at {} (app may not have run)",
            path.display()
        );
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| match serde_json::from_str(&l) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("layout-check: could not parse a result row: {e} ({l:?})");
                None
            }
        })
        .collect()
}

// ── Report ───────────────────────────────────────────────────────────────

/// One triaged, unfixed finding (review 221 §3's own decision) that
/// the gate must not fail the build over, matched by check kind and
/// the element's own description with its trailing `#<index>` cut off
/// (the index shifts across builds and views; the tag and classes
/// before it don't - confirmed by the seven-run determinism check
/// this same reply's §2 also required). `view`, when set, narrows a
/// match that would otherwise be ambiguous - F228's own bare `input`
/// (no class of its own) only means one of its three checkboxes
/// inside Settings' own hit-area list, at exactly their 12×12 box;
/// `input.profile-radio` carries its own class and never collides.
struct ExpectedFailure {
    f_number: &'static str,
    kind: &'static str,
    view: Option<&'static str>,
    element_base: &'static str,
    detail_contains: Option<&'static str>,
}

/// Review 221 §3's own decided triage: four findings, registered
/// (`ROADMAP.md`) and not yet fixed - the owner's scope call, not a
/// probe decision. Each stays on this list exactly as long as the
/// finding itself does; removing the fix removes the finding, and
/// `report`'s own "an expected failure stopped happening" check (not
/// this list - a gate silently drifting out of date would defeat the
/// entire point of naming them) then requires removing the entry too.
const EXPECTED_FAILURES: &[ExpectedFailure] = &[
    ExpectedFailure {
        f_number: "F227",
        kind: "hit_area",
        view: None,
        element_base: "button.bc-root",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F227",
        kind: "hit_area",
        view: None,
        element_base: "button.path-btn",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F227",
        kind: "hit_area",
        view: None,
        element_base: "button.apply-btn",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F228",
        kind: "hit_area",
        view: Some("settings"),
        element_base: "input",
        detail_contains: Some("12.0x12.0px"),
    },
    ExpectedFailure {
        f_number: "F229",
        kind: "rendered_contrast",
        view: None,
        element_base: "span.stats",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F229",
        kind: "rendered_contrast",
        view: None,
        element_base: "button.bc-seg",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F229",
        kind: "rendered_contrast",
        view: None,
        element_base: "button.filter-btn.active",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F229",
        kind: "rendered_contrast",
        view: None,
        element_base: "span.tree-status.status-equal",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F229",
        kind: "rendered_contrast",
        view: None,
        element_base: "span.tree-status.status-different",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F229",
        kind: "rendered_contrast",
        view: None,
        element_base: "span.status-key-glyph.status-equal",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F229",
        kind: "rendered_contrast",
        view: None,
        element_base: "span.status-key-glyph.status-different",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F229",
        kind: "rendered_contrast",
        view: None,
        element_base: "span.dir-status.status-different",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F230",
        kind: "overflow",
        view: None,
        element_base: "div.explorer-footer",
        detail_contains: None,
    },
    ExpectedFailure {
        f_number: "F230",
        kind: "overflow",
        view: None,
        element_base: "div.explorer-status-key",
        detail_contains: None,
    },
];

/// `"button.bc-root#18"` -> `"button.bc-root"` - the index past the
/// last `#` is never part of an [`ExpectedFailure`]'s own match.
fn element_base(element: &str) -> &str {
    element.rsplit_once('#').map_or(element, |(base, _)| base)
}

fn matches_expected(
    expected: &ExpectedFailure,
    kind: &str,
    view: &str,
    element: &str,
    detail: Option<&str>,
) -> bool {
    expected.kind == kind
        && expected.view.is_none_or(|v| v == view)
        && expected.element_base == element_base(element)
        && expected
            .detail_contains
            .is_none_or(|needle| detail.is_some_and(|d| d.contains(needle)))
}

fn report(rows: &[serde_json::Value], per_window_ms: &[(String, u128)]) {
    let mut failures = 0usize;
    let mut probe_ms_values: Vec<f64> = Vec::new();
    let mut total_recv_retries = 0u64;
    let mut expected_seen = vec![false; EXPECTED_FAILURES.len()];

    for row in rows {
        let view = row.get("view").and_then(|v| v.as_str()).unwrap_or("?");
        let text_size = row.get("text_size").and_then(|v| v.as_u64()).unwrap_or(0);
        let language = row.get("language").and_then(|v| v.as_str()).unwrap_or("?");
        let window = row
            .get("window_size")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let theme = row.get("theme").and_then(|v| v.as_str()).unwrap_or("?");
        let contrast = row.get("contrast").and_then(|v| v.as_str()).unwrap_or("?");
        let colour_vision = row
            .get("colour_vision")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let combo =
            format!("{view} {window} {text_size}% {language} {theme}/{contrast}/{colour_vision}");

        if let Some(ms) = row.get("probe_ms").and_then(|v| v.as_f64()) {
            probe_ms_values.push(ms);
        }
        if let Some(retries) = row.get("recv_retries").and_then(|v| v.as_u64()) {
            total_recv_retries += retries;
        }

        if let Some(err) = row.get("error").and_then(|v| v.as_str()) {
            failures += 1;
            println!("FAIL [{combo}]: probe error: {err}");
            continue;
        }
        if row.get("settled").and_then(|v| v.as_bool()) == Some(false) {
            failures += 1;
            println!("FAIL [{combo}]: never settled within the timeout");
        }
        // RFC-098 §2's own nine checks - every one reported the same
        // generic way: a list of findings, each already carrying its
        // own human-readable description (`element`/`text`/`detail`,
        // or `element`/`text`/the scroll-vs-client box for the two
        // stage A checks), so the report never needs to know a given
        // check's own shape beyond its name here.
        for kind in [
            "overflow",
            "clipped",
            "off_screen",
            "hit_area",
            "small_text",
            "rendered_contrast",
            "label_placement",
            "dialog_occlusion",
            "overlap",
        ] {
            if let Some(list) = row.get(kind).and_then(|v| v.as_array()) {
                for finding in list {
                    let element = finding
                        .get("element")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let detail = finding.get("detail").and_then(|v| v.as_str());
                    let matched = EXPECTED_FAILURES
                        .iter()
                        .enumerate()
                        .find(|(_, e)| matches_expected(e, kind, view, element, detail));
                    if let Some((idx, expected)) = matched {
                        expected_seen[idx] = true;
                        println!(
                            "EXPECTED [{combo}] {kind} ({}): {}",
                            expected.f_number,
                            serde_json::to_string(finding).unwrap_or_default()
                        );
                    } else {
                        failures += 1;
                        println!(
                            "FAIL [{combo}] {kind}: {}",
                            serde_json::to_string(finding).unwrap_or_default()
                        );
                    }
                }
            }
        }
        if let Some(toast) = row.get("toast_check")
            && toast.get("occluded").and_then(|v| v.as_bool()) == Some(true)
        {
            failures += 1;
            println!(
                "FAIL [{combo}] toast_check: {}",
                serde_json::to_string(toast).unwrap_or_default()
            );
        }
    }

    // Review 221 §3's own rule: "the gate fails if one stops failing" -
    // an expected-failure list that can silently go stale would defeat
    // the entire point of naming findings on it, so an entry nothing
    // in this run ever matched is itself a failure, loud about exactly
    // which F-number and why.
    for (idx, expected) in EXPECTED_FAILURES.iter().enumerate() {
        if !expected_seen[idx] {
            failures += 1;
            println!(
                "FAIL: expected failure {} ({} {}{}) did not occur this run - remove it from EXPECTED_FAILURES",
                expected.f_number,
                expected.kind,
                expected.element_base,
                expected
                    .view
                    .map(|v| format!(", view {v}"))
                    .unwrap_or_default()
            );
        }
    }

    println!();
    println!(
        "layout-check: {} combinations measured, {failures} failures.",
        rows.len()
    );
    // Review 220 §3: `recv_retries` (the "eval did not return" mitigation,
    // review 216 §4) was already recorded per measurement but never
    // summed anywhere a run's own log shows it - a retry firing at all
    // means the race it works around is still there, so the total is
    // worth seeing on every run, not just when something failed outright.
    println!("layout-check: total recv_retries across the run: {total_recv_retries}");
    if !probe_ms_values.is_empty() {
        let sum: f64 = probe_ms_values.iter().sum();
        let mean = sum / probe_ms_values.len() as f64;
        let max = probe_ms_values.iter().cloned().fold(f64::MIN, f64::max);
        let min = probe_ms_values.iter().cloned().fold(f64::MAX, f64::min);
        println!(
            "layout-check: per-measurement cost - mean {mean:.1}ms, min {min:.1}ms, max {max:.1}ms, over {} measurements",
            probe_ms_values.len()
        );
    }
    for (window, ms) in per_window_ms {
        println!("layout-check: window {window} - whole app run took {ms}ms");
    }

    if failures > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_base_cuts_the_trailing_index_only() {
        assert_eq!(element_base("button.bc-root#18"), "button.bc-root");
        assert_eq!(element_base("input#48"), "input");
        assert_eq!(element_base("div.a.b.c#1149"), "div.a.b.c");
        // No `#` at all - nothing to cut.
        assert_eq!(element_base("point(273,136)"), "point(273,136)");
    }

    #[test]
    fn matches_expected_requires_kind_and_base_and_honours_view_and_detail() {
        let f227 = &EXPECTED_FAILURES[0];
        assert_eq!(f227.f_number, "F227");
        assert!(matches_expected(
            f227,
            "hit_area",
            "explorer",
            "button.bc-root#18",
            None
        ));
        // Wrong kind.
        assert!(!matches_expected(
            f227,
            "overflow",
            "explorer",
            "button.bc-root#18",
            None
        ));
        // Wrong base (a different control entirely).
        assert!(!matches_expected(
            f227,
            "hit_area",
            "explorer",
            "button.path-btn#16",
            None
        ));

        let f228 = EXPECTED_FAILURES
            .iter()
            .find(|e| e.f_number == "F228")
            .expect("F228 is on the list");
        assert_eq!(f228.view, Some("settings"));
        assert!(matches_expected(
            f228,
            "hit_area",
            "settings",
            "input#48",
            Some("12.0x12.0px, needs 24x24")
        ));
        // Right base, wrong view - F228 is Settings-only.
        assert!(!matches_expected(
            f228,
            "hit_area",
            "shortcuts",
            "input#48",
            Some("12.0x12.0px, needs 24x24")
        ));
        // Right base and view, but the wrong size - a different,
        // untriaged bare `input` finding must not slip through as F228.
        assert!(!matches_expected(
            f228,
            "hit_area",
            "settings",
            "input#48",
            Some("18.0x18.0px, needs 24x24")
        ));
        // `input.profile-radio` carries its own class, so its own base
        // never collides with F228's bare `input`.
        assert!(!matches_expected(
            f228,
            "hit_area",
            "settings",
            "input.profile-radio#206",
            Some("12.0x12.0px, needs 24x24")
        ));
    }

    /// Every entry's own `f_number` is one of the four review 221 §3
    /// actually decided - catches a typo or a fifth, undecided entry
    /// added without a review to back it.
    #[test]
    fn every_expected_failure_names_a_decided_f_number() {
        for e in EXPECTED_FAILURES {
            assert!(
                ["F227", "F228", "F229", "F230"].contains(&e.f_number),
                "unexpected F-number on the list: {}",
                e.f_number
            );
        }
    }
}
