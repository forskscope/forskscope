//! RFC-098 stage A (handoff 101): `cargo xtask layout-check` — the
//! `layout-probe`'s own driver.
//!
//! Generates fixtures, writes a plan, builds `forskscope` with the
//! `layout-probe` feature, runs it once per window size (the RFC's own
//! design: "the window size is the one thing set at launch"), and reports
//! every finding plus the cost figures the RFC's own matrix decision
//! needs.
//!
//! **Not yet implemented here (stage B, after the architect's answer to
//! `188`):** `--inject <defect>`; the full check set (off-screen controls,
//! hit area, rendered contrast, text size); the remaining views (Explorer,
//! Deep Compare, mergetool, *Shortcuts*); CI wiring on every push.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The RFC's own matrix (§3), minus the colour axes this one check
/// doesn't need (neither overflow nor clipped text depends on colour).
const DEFAULT_WINDOWS: &[(&str, u32, u32)] = &[
    ("1000x700", 1000, 700),
    ("800x560", 800, 560),
    ("1366x768", 1366, 768),
];
const DEFAULT_TEXT_SIZES: &[u32] = &[100, 125, 150, 175, 200];
const DEFAULT_LANGUAGES: &[&str] = &["en", "ja"];

pub struct RunOptions {
    pub release: bool,
    pub out_dir: PathBuf,
}

pub fn run(root: &Path, opts: &RunOptions) {
    fs::create_dir_all(&opts.out_dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", opts.out_dir.display()));

    let fixtures_dir = opts.out_dir.join("fixtures");
    let (left, right) = write_file_comparison_fixture(&fixtures_dir);
    // Stage B's own views need these two too; generated now so the
    // generator itself (and its determinism) is proven before anything
    // depends on it, even though this round's one check doesn't open
    // either yet.
    write_folder_fixture(&fixtures_dir);
    write_mergetool_fixture(&fixtures_dir);

    let bin = build_binary(root, opts.release);

    let mut all_rows: Vec<serde_json::Value> = Vec::new();
    let mut per_window_ms: Vec<(String, u128)> = Vec::new();

    for &(window_label, w, h) in DEFAULT_WINDOWS {
        let plan_path = opts.out_dir.join(format!("plan-{window_label}.json"));
        let results_path = opts.out_dir.join(format!("results-{window_label}.jsonl"));
        let data_dir = opts.out_dir.join(format!("data-{window_label}"));
        let _ = fs::remove_dir_all(&data_dir); // F57-style: never a stale prior run's tabs.

        write_plan(
            &plan_path,
            &results_path,
            &left,
            &right,
            DEFAULT_TEXT_SIZES,
            DEFAULT_LANGUAGES,
            window_label,
        );

        println!("layout-check: running window {window_label} ({w}x{h})...");
        let elapsed = launch_and_wait(&bin, &plan_path, &data_dir, w, h);
        per_window_ms.push((window_label.to_string(), elapsed.as_millis()));

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
/// directly - CI cannot see `.git-exclude/`, the RFC's own point 2). Not
/// opened by this round's own plan (no view for it yet); generated so the
/// generator itself is proven ahead of stage B needing it.
fn write_folder_fixture(dir: &Path) {
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
}

/// A three-way merge triple (local/remote/merged), one conflicting region.
/// Not opened by this round's own plan; see `write_folder_fixture`'s own
/// note.
fn write_mergetool_fixture(dir: &Path) {
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
}

// ── Plan ─────────────────────────────────────────────────────────────────

fn write_plan(
    plan_path: &Path,
    results_path: &Path,
    left: &Path,
    right: &Path,
    text_sizes: &[u32],
    languages: &[&str],
    window_label: &str,
) {
    let plan = serde_json::json!({
        "results_path": results_path,
        "views": [
            {"kind": "file-comparison", "left": left, "right": right},
            {"kind": "settings"},
        ],
        "text_sizes": text_sizes,
        "languages": languages,
        "window_size": window_label,
    });
    fs::write(plan_path, serde_json::to_string_pretty(&plan).unwrap())
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", plan_path.display()));
}

// ── Launch ───────────────────────────────────────────────────────────────

/// Launches `bin` with the plan env var set and an isolated `--data-dir`/
/// XDG set, resizes its window to `w`x`h` if a `DISPLAY` exists (CI's own
/// `xvfb-run`, or a nested session locally), and waits for it to exit -
/// the probe calls `std::process::exit(0)` itself once its own matrix is
/// measured, so this never sends a signal on the success path. Returns
/// the wall-clock time from launch to exit.
fn launch_and_wait(bin: &Path, plan_path: &Path, data_dir: &Path, w: u32, h: u32) -> Duration {
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

    if std::env::var("DISPLAY").is_ok() {
        resize_via_xdotool(&child, w, h);
    } else {
        eprintln!(
            "layout-check: DISPLAY not set - window stays at its own default size, not {w}x{h}"
        );
    }

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

/// Best-effort: polls for the launched process's own window for up to 5s,
/// then asks the X server to resize it directly - verified live (handoff
/// 101 §Stage A) to work with **no window manager running at all**: a
/// bare `xdotool windowsize` resize request reaches the window over core
/// X11 regardless of whether anything is managing it, which is exactly
/// `xvfb-run`'s own shape (a virtual framebuffer, no window manager).
fn resize_via_xdotool(child: &Child, w: u32, h: u32) {
    let pid = child.id().to_string();
    let mut win_id = None;
    for _ in 0..25 {
        let out = Command::new("xdotool")
            .args(["search", "--pid", &pid, "--onlyvisible"])
            .output();
        if let Ok(out) = out
            && out.status.success()
        {
            let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !id.is_empty() {
                win_id = Some(id);
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let Some(win_id) = win_id else {
        eprintln!("layout-check: could not find forskscope's own window to resize (pid {pid})");
        return;
    };
    let status = Command::new("xdotool")
        .args(["windowsize", &win_id, &w.to_string(), &h.to_string()])
        .status();
    match status {
        Ok(s) if s.success() => {}
        other => eprintln!("layout-check: xdotool windowsize failed: {other:?}"),
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

        if let Some(ms) = row.get("probe_ms").and_then(|v| v.as_f64()) {
            probe_ms_values.push(ms);
        }

        if let Some(err) = row.get("error").and_then(|v| v.as_str()) {
            failures += 1;
            println!("FAIL [{view} {window} {text_size}% {language}]: probe error: {err}");
            continue;
        }
        if row.get("settled").and_then(|v| v.as_bool()) == Some(false) {
            failures += 1;
            println!(
                "FAIL [{view} {window} {text_size}% {language}]: never settled within the timeout"
            );
        }
        for kind in ["overflow", "clipped"] {
            if let Some(list) = row.get(kind).and_then(|v| v.as_array()) {
                for finding in list {
                    failures += 1;
                    println!(
                        "FAIL [{view} {window} {text_size}% {language}] {kind}: {}",
                        serde_json::to_string(finding).unwrap_or_default()
                    );
                }
            }
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
