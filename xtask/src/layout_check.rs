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

use std::collections::BTreeSet;
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
    bidi_left: PathBuf,
    bidi_right: PathBuf,
    bidi_names_left: PathBuf,
    bidi_names_right: PathBuf,
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

/// Review 223 §2.2's own two private displays, review 224 §2's own
/// three corrections to how each is proven:
/// - **CI:** `$GITHUB_ACTIONS == "true"`, set only by GitHub Actions
///   itself - review 224 §2.2 found the generic `$CI` (the previous
///   check) is common enough that a developer's own shell can set it
///   too, passing this check on a real desktop. Paired with `$DISPLAY`
///   (which `xvfb-run` sets, CI's own private Xvfb).
/// - **Locally:** inside `.git-exclude/gui/session.sh`'s own nested
///   `niri` - both `$FORSKSCOPE_GUI_SESSION` and `$NIRI_SOCKET` must be
///   set (neither alone proves the other), paired with
///   `$SESSION_X11_DISPLAY` (session.sh's own nested X11 socket).
///
/// Returns the one `DISPLAY` value every launch must now set
/// explicitly (review 224 §2.1): `GDK_BACKEND=x11` alone never set
/// `DISPLAY` on its own - a launch with no explicit `.env("DISPLAY",
/// ...)` silently keeps whatever `$DISPLAY` the caller's own shell
/// had, real desktop included, even once every check above already
/// passed. Refuses outright otherwise - never falls back to the
/// caller's own display for any reason, called before anything else
/// runs or builds.
fn require_private_display() -> String {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && let Ok(display) = std::env::var("DISPLAY")
        && !display.is_empty()
    {
        return display;
    }
    if std::env::var_os("FORSKSCOPE_GUI_SESSION").is_some()
        && std::env::var_os("NIRI_SOCKET").is_some()
        && let Ok(display) = std::env::var("SESSION_X11_DISPLAY")
        && !display.is_empty()
    {
        return display;
    }
    eprintln!(
        "layout-check: refusing to run - no private display.\n\
         \n\
         This launches real forskscope windows, and on an unexpected finding,\n\
         screenshots one. From a normal desktop shell there is no way to tell\n\
         this run's own window from your own real session, so it never runs\n\
         there.\n\
         \n\
         Run it in CI (already wired, .github/workflows/ci.yml), or locally\n\
         inside this project's own nested session, which starts no app of its\n\
         own (review 224 §2.3 - this driver launches its own):\n\
         \n\
         .git-exclude/gui/session.sh --fixture <any> --backend x11 --no-app -- \\\n\
           cargo xtask layout-check [--inject <defect>]"
    );
    process::exit(2);
}

pub fn run(root: &Path, opts: &RunOptions) {
    let display = require_private_display();
    if let Some(defect) = &opts.inject {
        run_inject(root, opts, defect, &display);
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
    let (bidi_left, bidi_right) = write_bidi_fixture(&fixtures_dir);
    let (bidi_names_left, bidi_names_right) = write_bidi_names_fixture(&fixtures_dir);
    let fixtures = Fixtures {
        file_left,
        file_right,
        folder_left,
        folder_right,
        merge_local,
        merge_remote,
        merge_merged,
        bidi_left,
        bidi_right,
        bidi_names_left,
        bidi_names_right,
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
        let elapsed = launch_and_wait(&bin, &plan_path, &data_dir, &bus_conf, &display);
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
        let elapsed = launch_and_wait(&bin, &plan_path, &data_dir, &bus_conf, &display);
        per_window_ms.push((
            format!("contrast {window_label}@{text_size}%"),
            elapsed.as_millis(),
        ));

        for row in read_jsonl(&results_path) {
            all_rows.push(row);
        }
    }

    let (any_failures, failing_rows) = report(&all_rows, &per_window_ms);
    if !failing_rows.is_empty() {
        capture_failures(
            &opts.out_dir,
            &fixtures,
            &bin,
            &bus_conf,
            &failing_rows,
            &display,
        );
    }
    if any_failures {
        std::process::exit(1);
    }
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
fn run_inject(root: &Path, opts: &RunOptions, defect: &str, display: &str) {
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

    if !run_inject_guarded(root, opts, defect, rule, field, display) {
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
    display: &str,
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
        // Review 220 §5's own falsification: one `--inject` run, one
        // combination, always worth a look - not conditioned on
        // `found` below the way the real matrix's capture is, since
        // there is nothing else here to filter by.
        "pause_before_exit_ms": CAPTURE_PAUSE_MS,
    });
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap())
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", plan_path.display()));

    println!("layout-check: --inject {defect} - running {view_name} only...");
    let captures_dir = opts.out_dir.join("failure-captures");
    fs::create_dir_all(&captures_dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", captures_dir.display()));
    let capture_path = captures_dir.join(format!("inject-{defect}.png"));
    launch_and_capture(
        &bin,
        &plan_path,
        &data_dir,
        &bus_conf,
        &results_path,
        &capture_path,
        display,
    );
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

/// RFC-100 §1's own twelve bidirectional control characters, duplicated
/// here rather than depended on: xtask deliberately stays outside the
/// main workspace (DEC-005/F119, this file's own `[workspace]` table),
/// so it cannot depend on `forskscope-compare` the way the engine and UI
/// crates do. This literal must stay in sync with `forskscope-compare::
/// bidi::BIDI_CONTROLS` by hand - the same relationship `TEXT_SIZE_ALLOWLIST`
/// already has with `xtask::css_shared::FONT_SIZE_ALLOWLIST` (layout_probe.rs's
/// own doc comment).
const BIDI_CONTROL_CHARS: [char; 12] = [
    '\u{061C}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}',
    '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
];

/// RFC-100's own layout-matrix fixture: a file pair holding every one of
/// §1's bidirectional control characters, generated here rather than
/// stored (the same "never copied from elsewhere" rule the handoff gives
/// for the engine's own test fixtures) - one unchanged line carrying all
/// twelve (exercises the plain-render path's markers), one changed line
/// that drops one of them on the right side (exercises the
/// character-refined path's markers, and gives the matrix an actual
/// change to measure). Returns `(left, right)`.
fn write_bidi_fixture(dir: &Path) -> (PathBuf, PathBuf) {
    fs::create_dir_all(dir).unwrap_or_else(|e| panic!("cannot create {}: {e}", dir.display()));
    let left = dir.join("bidi-left.txt");
    let right = dir.join("bidi-right.txt");
    let mut all_controls = String::from("all twelve: ");
    for c in BIDI_CONTROL_CHARS {
        all_controls.push(c);
    }
    let changed_left = format!("changed line: one{}two", '\u{202E}');
    let changed_right = "changed line: one two".to_string();
    fs::write(
        &left,
        format!("{all_controls}\n{changed_left}\nunchanged tail\n"),
    )
    .unwrap_or_else(|e| panic!("cannot write {}: {e}", left.display()));
    fs::write(
        &right,
        format!("{all_controls}\n{changed_right}\nunchanged tail\n"),
    )
    .unwrap_or_else(|e| panic!("cannot write {}: {e}", right.display()));
    (left, right)
}

/// F235's own layout-matrix fixture: a folder pair, each holding one file
/// per [`BIDI_CONTROL_CHARS`] entry (same name on both sides, since the
/// *content* is irrelevant here - this fixture is about what Deep
/// Compare's own rows do with a bidi-control character in a NAME) plus
/// the classic Trojan Source spoofing shape, `invoice<U+202E>fdp.exe`
/// (an RLO reorders the rest of the name for display). Generated here
/// rather than stored, same reasoning as `write_bidi_fixture`. Returns
/// `(left_dir, right_dir)`.
///
/// Never created outside `target/` (this fixture lives under
/// `dir`, always a path inside the run's own `--out-dir`, itself inside
/// `target/` by this xtask's own convention) - a name a filesystem
/// refuses (none do on Linux; untested on Windows, where some of these
/// characters may not be refused either, since none of them are in
/// Windows' own reserved-character set) is reported by panicking with
/// the OS error, never silently worked around by renaming or skipping
/// it.
fn write_bidi_names_fixture(dir: &Path) -> (PathBuf, PathBuf) {
    let left = dir.join("bidi-names-left");
    let right = dir.join("bidi-names-right");
    fs::create_dir_all(&left).unwrap_or_else(|e| panic!("cannot create {}: {e}", left.display()));
    fs::create_dir_all(&right)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", right.display()));

    let spoofed_name = format!("invoice{}fdp.exe", '\u{202E}');
    fs::write(left.join(&spoofed_name), b"left\n")
        .unwrap_or_else(|e| panic!("cannot write {}/{spoofed_name}: {e}", left.display()));
    fs::write(right.join(&spoofed_name), b"right, changed\n")
        .unwrap_or_else(|e| panic!("cannot write {}/{spoofed_name}: {e}", right.display()));

    for (i, c) in BIDI_CONTROL_CHARS.iter().enumerate() {
        let name = format!("name-{i:02}-{c}-control.txt");
        fs::write(left.join(&name), b"same\n")
            .unwrap_or_else(|e| panic!("cannot write {}/{name}: {e}", left.display()));
        fs::write(right.join(&name), b"same\n")
            .unwrap_or_else(|e| panic!("cannot write {}/{name}: {e}", right.display()));
    }
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
        // RFC-100 §Fixtures: the marker's layout and contrast, covered by
        // every check this matrix already runs against every other view.
        {"kind": "bidi-fixture", "left": fixtures.bidi_left, "right": fixtures.bidi_right},
        // F235: the same coverage, for a bidi control in a NAME - Deep
        // Compare's own rows are exactly where a folder's entry names are
        // shown, the same check every other view already runs against.
        {
            "kind": "deep-compare",
            "left": fixtures.bidi_names_left,
            "right": fixtures.bidi_names_right,
        },
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
///
/// `display`: [`require_private_display`]'s own return value, set
/// explicitly (review 224 §2.1) - `GDK_BACKEND=x11` alone never sets
/// `DISPLAY`, so without this a launch silently keeps whatever
/// `$DISPLAY` this process itself inherited, real desktop included.
fn launch_and_wait(
    bin: &Path,
    plan_path: &Path,
    data_dir: &Path,
    bus_conf: &Path,
    display: &str,
) -> Duration {
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
        .env("DISPLAY", display)
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

// ── Capture on failure (review 220 §5) ──────────────────────────────────

/// How long the probe keeps a captured combination's view open and
/// settled, after its own measurement, before closing and exiting
/// (`layout_probe::Plan::pause_before_exit_ms`) - long enough for
/// [`launch_and_capture`]'s own poll-then-`import` sequence to finish
/// comfortably, nowhere near `launch_and_wait`'s own 60s launch
/// timeout.
const CAPTURE_PAUSE_MS: u64 = 3000;

/// A filesystem-safe name for one result row's own combination,
/// unique enough across a run's full matrix to never collide: every
/// axis the row carries, in the same order [`report`]'s own `combo`
/// string prints them.
fn combo_slug(row: &serde_json::Value) -> String {
    let view = row.get("view").and_then(|v| v.as_str()).unwrap_or("view");
    let window = row
        .get("window_size")
        .and_then(|v| v.as_str())
        .unwrap_or("window")
        .replace('x', "-");
    let text_size = row.get("text_size").and_then(|v| v.as_u64()).unwrap_or(0);
    let language = row
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("lang");
    let theme = row.get("theme").and_then(|v| v.as_str()).unwrap_or("theme");
    let contrast = row
        .get("contrast")
        .and_then(|v| v.as_str())
        .unwrap_or("contrast");
    let colour_vision = row
        .get("colour_vision")
        .and_then(|v| v.as_str())
        .unwrap_or("cv");
    format!("{view}-{window}-{text_size}pct-{language}-{theme}-{contrast}-{colour_vision}")
}

/// Rebuilds, from one already-measured row, the one-combination plan
/// that reopens exactly that same view/text-size/language/theme/
/// contrast/colour-vision/window - the view JSON itself pulled back
/// out of [`all_views_json`] by its own `kind`, not reconstructed by
/// hand, so a `FileComparison`/`DeepCompare`/`MergeTool` view's own
/// fixture paths carry over correctly instead of a bare `{"kind":
/// ...}` that would open the wrong (or no) fixture.
fn write_capture_plan(
    plan_path: &Path,
    results_path: &Path,
    fixtures: &Fixtures,
    row: &serde_json::Value,
) {
    let view_kind = row
        .get("view")
        .and_then(|v| v.as_str())
        .unwrap_or("explorer");
    let views = all_views_json(fixtures);
    let view_json = views
        .as_array()
        .expect("all_views_json always returns a JSON array")
        .iter()
        .find(|v| v.get("kind").and_then(|k| k.as_str()) == Some(view_kind))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"kind": view_kind}));
    let text_size = row.get("text_size").and_then(|v| v.as_u64()).unwrap_or(100);
    let language = row.get("language").and_then(|v| v.as_str()).unwrap_or("en");
    let theme = row.get("theme").and_then(|v| v.as_str()).unwrap_or("light");
    let contrast = row
        .get("contrast")
        .and_then(|v| v.as_str())
        .unwrap_or("standard");
    let colour_vision = row
        .get("colour_vision")
        .and_then(|v| v.as_str())
        .unwrap_or("standard");

    let mut plan = serde_json::json!({
        "results_path": results_path,
        "views": [view_json],
        "text_sizes": [text_size],
        "languages": [language],
        "themes": [theme],
        "contrasts": [contrast],
        "colour_visions": [colour_vision],
        "pause_before_exit_ms": CAPTURE_PAUSE_MS,
    });
    if let Some(window_size) = row.get("window_size").and_then(|v| v.as_str()) {
        plan["window_size"] = serde_json::json!(window_size);
    }
    fs::write(plan_path, serde_json::to_string_pretty(&plan).unwrap())
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", plan_path.display()));
}

/// Review 220 §5's own two routes, picked by what's actually running
/// underneath, not by a fixed CI/local split - found live: CI's own
/// Xvfb is a plain, uncomposited X server, where the root window
/// really does hold every pixel on screen, so `import -window root`
/// reads it correctly; this project's own local sessions run under a
/// nested `niri` (`require_private_display`, review 223 §2.2), a
/// Wayland compositor whose X11 side (XWayland) never paints a
/// composited frame to its own root window at all - `import -window
/// root` under it reads nothing and fails outright ("missing an image
/// filename", confirmed live, not a guess). `WAYLAND_DISPLAY` being set
/// is what actually distinguishes the two, not a hostname or a CI env
/// var.
///
/// `before`: only used on the local route, to find the probe's own
/// window and no other - see [`capture_via_niri`].
fn capture_screen(capture_path: &Path, before: &BTreeSet<u64>) {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        capture_via_niri(capture_path, before);
    } else {
        capture_via_import(capture_path);
    }
}

/// The `id` of every currently open window with [`NIRI_APP_ID`]
/// (`niri msg -j windows`) - [`capture_via_niri`]'s own "before" and
/// "after" snapshot.
fn forskscope_window_ids() -> Option<BTreeSet<u64>> {
    let output = Command::new("niri")
        .arg("msg")
        .arg("-j")
        .arg("windows")
        .output();
    let windows: serde_json::Value = match output {
        Ok(o) if o.status.success() => serde_json::from_slice(&o.stdout).ok()?,
        _ => return None,
    };
    Some(
        windows
            .as_array()?
            .iter()
            .filter(|w| w.get("app_id").and_then(|v| v.as_str()) == Some(NIRI_APP_ID))
            .filter_map(|w| w.get("id").and_then(serde_json::Value::as_u64))
            .collect(),
    )
}

/// The app's own window title bar (`niri msg -j windows`, live check):
/// `app_id` comes back `"Forskscope"`, not the binary's own lowercase
/// name - wry/gtk's own doing, not this crate's, so matched exactly as
/// observed rather than guessed at or lowercased.
const NIRI_APP_ID: &str = "Forskscope";

/// Finds the app's own window as the one `id` that's new since
/// `before` - **not** `niri msg -j windows`'s own `pid` field, review
/// 223 §2.2's own first attempt, reverted after it was built and run
/// live: under the X11 backend this driver always launches with,
/// every X11 window's own `pid` comes back as **`xwayland-satellite`'s
/// own pid** (niri's own X11-compatibility proxy - a single, shared
/// process every X11 client connects through), not the client
/// application's - confirmed by `ps` naming that exact process at that
/// exact pid, with no real `forskscope` process sharing it, across two
/// separate forskscope windows that both reported it. `pid`-matching
/// can't disambiguate two X11 clients from each other at all under
/// this backend; a before/after `id` diff doesn't depend on `pid`
/// being meaningful in the first place - this run's own window is
/// whichever one with [`NIRI_APP_ID`] wasn't open before this launch,
/// the maintainer's own real one (if open) stays in `before` either
/// way. Refuses - no screenshot, not a guess - on zero or more than
/// one new window, same as an unreachable niri or an unparsable
/// response.
fn capture_via_niri(capture_path: &Path, before: &BTreeSet<u64>) {
    let Some(after) = forskscope_window_ids() else {
        eprintln!("layout-check: `niri msg -j windows` failed or was unparsable - no screenshot");
        return;
    };
    let new_windows: Vec<u64> = after.difference(before).copied().collect();
    let [window_id] = new_windows[..] else {
        eprintln!(
            "layout-check: expected exactly one new niri window with app id {NIRI_APP_ID:?} \
             since this launch began, found {new_windows:?} (before: {before:?}, after: \
             {after:?}) - no screenshot"
        );
        return;
    };
    let status = Command::new("niri")
        .arg("msg")
        .arg("action")
        .arg("screenshot-window")
        .arg("--id")
        .arg(window_id.to_string())
        .arg("--path")
        .arg(capture_path)
        .status();
    if !matches!(status, Ok(s) if s.success()) {
        eprintln!("layout-check: niri screenshot-window failed ({status:?}) - no screenshot");
    }
}

/// CI's own Xvfb route: the root window itself holds every composited
/// pixel on an uncomposited X server, against whichever `$DISPLAY`
/// this process itself inherited (the same one the child, with no
/// `DISPLAY` override of its own, is actually drawing to).
fn capture_via_import(capture_path: &Path) {
    let status = Command::new("import")
        .arg("-window")
        .arg("root")
        .arg(capture_path)
        .status();
    if !matches!(status, Ok(s) if s.success()) {
        eprintln!(
            "layout-check: `import -window root {}` failed ({status:?}) - no screenshot",
            capture_path.display()
        );
    }
}

/// Launches `bin` the same way [`launch_and_wait`] does, but doesn't
/// block on exit first: this plan's own single combination pauses,
/// settled, for [`CAPTURE_PAUSE_MS`] after writing its one result row
/// (`layout_probe::Plan::pause_before_exit_ms`), so polling
/// `results_path` for that row's arrival is this function's own
/// signal that there's something on screen worth capturing yet - see
/// [`capture_screen`]. Never panics on a capture failure (a missing
/// `import` binary locally, say) - a missing screenshot is a worse
/// report, not a worse run.
fn launch_and_capture(
    bin: &Path,
    plan_path: &Path,
    data_dir: &Path,
    bus_conf: &Path,
    results_path: &Path,
    capture_path: &Path,
    display: &str,
) {
    fs::create_dir_all(data_dir.join("xdg/config")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/state")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/cache")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/data")).unwrap();

    // Taken before this launch's own child exists, so this run's own
    // window is never among them - `capture_via_niri`'s own "before".
    let before_windows = forskscope_window_ids().unwrap_or_default();

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
        .env("GTK_A11Y", "none")
        .env("GDK_BACKEND", "x11")
        .env("DISPLAY", display)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch forskscope under dbus-run-session");

    let poll_start = Instant::now();
    let poll_timeout = Duration::from_secs(30);
    loop {
        if !read_jsonl(results_path).is_empty() {
            break;
        }
        if poll_start.elapsed() > poll_timeout {
            eprintln!(
                "layout-check: capture for {} never produced a result row within {}s - no \
                 screenshot",
                capture_path.display(),
                poll_timeout.as_secs()
            );
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    capture_screen(capture_path, &before_windows);

    if !wait_with_timeout(&mut child, Duration::from_secs(10)) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// One capture per failing row (review 220 §5): a screenshot is only
/// worth the time once the real matrix already knows which exact
/// combinations had an unexpected finding, not every one of the
/// hundreds that merely had an *expected* one - this is the second,
/// targeted re-launch that follows, never the matrix's own.
fn capture_failures(
    out_dir: &Path,
    fixtures: &Fixtures,
    bin: &Path,
    bus_conf: &Path,
    failing_rows: &[serde_json::Value],
    display: &str,
) {
    let captures_dir = out_dir.join("failure-captures");
    fs::create_dir_all(&captures_dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", captures_dir.display()));
    for (i, row) in failing_rows.iter().enumerate() {
        let slug = combo_slug(row);
        let plan_path = out_dir.join(format!("plan-capture-{i}.json"));
        let results_path = out_dir.join(format!("results-capture-{i}.jsonl"));
        let data_dir = out_dir.join(format!("data-capture-{i}"));
        let capture_path = captures_dir.join(format!("{slug}.png"));

        write_capture_plan(&plan_path, &results_path, fixtures, row);
        println!("layout-check: capturing a screenshot of the failing combination [{slug}]...");
        launch_and_capture(
            bin,
            &plan_path,
            &data_dir,
            bus_conf,
            &results_path,
            &capture_path,
            display,
        );
    }
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

/// Review 221 §3's own decided triage, F227–F231 (handoff 104) the
/// last five registered here - every one now fixed, so this list is
/// empty: the gate's own "an expected failure stopped happening" check
/// (below) would otherwise fail loudly on its own if a fix were ever
/// reverted without also restoring the matching entry here, the same
/// as it did for each of these five the moment its own fix landed.
const EXPECTED_FAILURES: &[ExpectedFailure] = &[];

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

/// Prints every finding, `EXPECTED` or `FAIL`, and the run's own cost
/// figures - unchanged from before review 220 §5. Returns `(any
/// failures at all, one clone of each row that had at least one
/// *unexpected* failure)` - the two differ when the only failure is
/// the "expected failure didn't occur" check below, which names an
/// `EXPECTED_FAILURES` entry, not a row, so there is nothing of its
/// own to screenshot. Never exits itself now, so the caller can act
/// on the row list (review 220 §5's own capture) before deciding
/// whether to exit non-zero.
fn report(
    rows: &[serde_json::Value],
    per_window_ms: &[(String, u128)],
) -> (bool, Vec<serde_json::Value>) {
    let mut failures = 0usize;
    let mut probe_ms_values: Vec<f64> = Vec::new();
    let mut total_recv_retries = 0u64;
    let mut expected_seen = vec![false; EXPECTED_FAILURES.len()];
    let mut failing_rows: Vec<serde_json::Value> = Vec::new();

    for row in rows {
        let mut row_has_unexpected_failure = false;
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
            failing_rows.push(row.clone());
            continue;
        }
        if row.get("settled").and_then(|v| v.as_bool()) == Some(false) {
            failures += 1;
            row_has_unexpected_failure = true;
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
                        row_has_unexpected_failure = true;
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
            row_has_unexpected_failure = true;
            println!(
                "FAIL [{combo}] toast_check: {}",
                serde_json::to_string(toast).unwrap_or_default()
            );
        }
        if row_has_unexpected_failure {
            failing_rows.push(row.clone());
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

    (failures > 0, failing_rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Review 223 §2.2's own before/after diff, as a pure set operation -
    /// the live niri/screenshot side (`capture_via_niri` itself) needs a
    /// real compositor to exercise at all, but the rule it applies to
    /// whatever `forskscope_window_ids` returns before and after does
    /// not. A pre-existing window (the maintainer's own, real
    /// ForskScope, say) must stay out of the result even when it is
    /// still open in `after` - it was already in `before`.
    #[test]
    fn new_windows_is_the_set_difference_not_the_after_set() {
        let before: BTreeSet<u64> = [7].into_iter().collect();
        let after: BTreeSet<u64> = [7, 12].into_iter().collect();
        let new_windows: Vec<u64> = after.difference(&before).copied().collect();
        assert_eq!(new_windows, vec![12]);
    }

    /// Two new windows (or zero) is never "the one this run opened" -
    /// `capture_via_niri`'s own refusal, exercised here as the same
    /// slice-pattern match it uses, since asserting on its own
    /// `eprintln!` output would mean parsing stderr instead of testing
    /// the actual decision.
    #[test]
    fn exactly_one_new_window_is_required() {
        fn matches_exactly_one(new_windows: &[u64]) -> bool {
            matches!(new_windows, [_])
        }
        assert!(!matches_exactly_one(&[]));
        assert!(matches_exactly_one(&[12]));
        assert!(!matches_exactly_one(&[12, 13]));
    }

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
        // `matches_expected` is pure - exercised here against its own,
        // local fixtures (the exact shape F227/F228 once had on
        // `EXPECTED_FAILURES`, now empty, handoff 104) rather than real
        // entries on the list, so this test stays meaningful whether or
        // not anything is currently registered there.
        let no_view_entry = ExpectedFailure {
            f_number: "FTEST",
            kind: "hit_area",
            view: None,
            element_base: "button.bc-root",
            detail_contains: None,
        };
        assert!(matches_expected(
            &no_view_entry,
            "hit_area",
            "explorer",
            "button.bc-root#18",
            None
        ));
        // Wrong kind.
        assert!(!matches_expected(
            &no_view_entry,
            "overflow",
            "explorer",
            "button.bc-root#18",
            None
        ));
        // Wrong base (a different control entirely).
        assert!(!matches_expected(
            &no_view_entry,
            "hit_area",
            "explorer",
            "button.path-btn#16",
            None
        ));

        let view_and_detail_entry = ExpectedFailure {
            f_number: "FTEST2",
            kind: "hit_area",
            view: Some("settings"),
            element_base: "input",
            detail_contains: Some("12.0x12.0px"),
        };
        assert!(matches_expected(
            &view_and_detail_entry,
            "hit_area",
            "settings",
            "input#48",
            Some("12.0x12.0px, needs 24x24")
        ));
        // Right base, wrong view.
        assert!(!matches_expected(
            &view_and_detail_entry,
            "hit_area",
            "shortcuts",
            "input#48",
            Some("12.0x12.0px, needs 24x24")
        ));
        // Right base and view, but the wrong size - a different,
        // untriaged bare `input` finding must not slip through.
        assert!(!matches_expected(
            &view_and_detail_entry,
            "hit_area",
            "settings",
            "input#48",
            Some("18.0x18.0px, needs 24x24")
        ));
        // `input.profile-radio` carries its own class, so its own base
        // never collides with a bare `input`.
        assert!(!matches_expected(
            &view_and_detail_entry,
            "hit_area",
            "settings",
            "input.profile-radio#206",
            Some("12.0x12.0px, needs 24x24")
        ));
    }

    /// `EXPECTED_FAILURES` is empty (handoff 104: F227-F231, the only
    /// entries it ever held, are all fixed) - falsify by adding any
    /// entry back without a review to back it.
    #[test]
    fn expected_failures_is_empty() {
        assert!(
            EXPECTED_FAILURES.is_empty(),
            "EXPECTED_FAILURES should be empty - every triaged finding is fixed; a new \
             entry needs its own review to back it, not a silent addition"
        );
    }
}
