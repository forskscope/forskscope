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
use std::process::{Child, Command, Stdio};
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
}

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

pub fn run(root: &Path, opts: &RunOptions) {
    // Review 216 §4: "every run starts empty" - a stale prior run's own
    // data dirs (tabs accumulated on disk) must never carry forward, not
    // just between window sizes within one run (handled per-window
    // below already) but across separate invocations at the same
    // `--out-dir`, which is exactly the shape that hid F218 on a first
    // retroactive-proof attempt.
    let _ = fs::remove_dir_all(&opts.out_dir);
    fs::create_dir_all(&opts.out_dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", opts.out_dir.display()));

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
        let elapsed = launch_and_wait(&bin, &plan_path, &data_dir);
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
        let elapsed = launch_and_wait(&bin, &plan_path, &data_dir);
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
fn launch_and_wait(bin: &Path, plan_path: &Path, data_dir: &Path) -> Duration {
    fs::create_dir_all(data_dir.join("xdg/config")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/state")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/cache")).unwrap();
    fs::create_dir_all(data_dir.join("xdg/data")).unwrap();

    let start = Instant::now();
    let mut child = Command::new(bin)
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
        .expect("failed to launch forskscope");

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

fn report(rows: &[serde_json::Value], per_window_ms: &[(String, u128)]) {
    let mut failures = 0usize;
    let mut probe_ms_values: Vec<f64> = Vec::new();

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
                    failures += 1;
                    println!(
                        "FAIL [{combo}] {kind}: {}",
                        serde_json::to_string(finding).unwrap_or_default()
                    );
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

    println!();
    println!(
        "layout-check: {} combinations measured, {failures} failures.",
        rows.len()
    );
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
