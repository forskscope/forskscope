//! RFC-098 stage A (handoff 101): a test-only, in-process layout
//! measurement probe, built only with the `layout-probe` Cargo feature.
//!
//! With the feature off (every normal build, including every release),
//! this module still compiles but [`maybe_start`] is never called —
//! `app.rs`'s own hook that calls it is itself behind
//! `#[cfg(feature = "layout-probe")]`. With the feature on and
//! [`PLAN_ENV_VAR`] set to a plan file's path, the app drives itself
//! through every view/text-size/language combination the plan names, in
//! process, through the same `Store` signals the Settings dialog itself
//! writes — no clicking, no AT-SPI, no external accessibility bridge.
//!
//! ## Why in-process, not AT-SPI (`render_check.py`'s own approach)
//!
//! `packaging/render_check.py` (F34) already starts the real app headless
//! under `xvfb-run`/`dbus-run-session` and walks its AT-SPI tree from
//! outside — confirmed unreachable for click-driven interaction in this
//! *sandbox* specifically (review 202), but that limit is about this one
//! development environment, not about AT-SPI as a mechanism; CI's own
//! real X server is a different question this handoff's own CI run
//! answers directly. This probe still doesn't use it, because AT-SPI
//! reports accessible geometry (`Component.get_extents`), not the box
//! model layout itself needs (`scrollWidth`/`clientWidth`, computed
//! `overflow`/`text-overflow`) — the RFC's own checks are DOM/CSS
//! questions, answered directly and faster by the same web engine that
//! renders the page, through [`dioxus::document::eval`], than by walking
//! an accessibility tree built from that same DOM a layer away.
//!
//! ## Plan format
//!
//! [`PLAN_ENV_VAR`] names a JSON file (its path, not its content — kept
//! out of the environment's own size limits and easy to inspect/diff as
//! a CI artifact). See [`Plan`] for the exact shape. The window size is
//! set once, in process, before the matrix starts (review 216 §4:
//! through the same [`dioxus_desktop::DesktopContext`] `app.rs` already
//! holds for the window title and the zoom level, not by an external
//! driver resizing the real OS window after launch) — "one app run per
//! window size" is what keeps the matrix from paying a process launch
//! for every combination.
//!
//! ## Settle condition
//!
//! Not a sleep: each combination's own measurement script first awaits,
//! in one poll loop (once per animation frame, bounded by
//! [`SETTLE_TIMEOUT_MS`]):
//! - the view's own [`ViewPlan::ready_selector`] actually existing in
//!   the page (the navigation call that opened it is itself a `Store`
//!   signal write, no more instantly reflected in the DOM than any
//!   other one — found live, first retroactive-proof run: a bare,
//!   un-polled check measured it absent);
//! - then a quiet period of [`SETTLE_QUIET_MS`] with neither a DOM
//!   mutation (`MutationObserver`) nor a change in the document root's
//!   own `clientWidth`/`clientHeight` — the second half catches a
//!   text-size change settling, which applies as a native webview zoom
//!   level (`wry`'s `set_zoom_level`, driven from `app.rs`'s own render
//!   body), never a DOM mutation a `MutationObserver` alone would see.
//!
//! `settled: false` in a result row means the loop hit its timeout
//! before both conditions held at once — reported, not silently
//! measured anyway against a view that may never have opened or never
//! stopped moving. A throwaway `document::eval` round trip runs before
//! every combination's real measurement too, forcing whatever render a
//! prior `Store` signal write scheduled to flush before either condition
//! starts being watched — see `measure_one`'s own doc comment.
//!
//! ## Results
//!
//! One JSON object per line (JSONL), appended to [`Plan::results_path`]
//! as each combination finishes, so a crash mid-matrix still leaves every
//! prior result on disk. See [`ResultRow`].

use std::io::Write as _;
use std::path::PathBuf;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::state::{Lang, Modal, Store, open_compare};

/// The only environment variable this build reads outside normal
/// settings/session paths, and the literal string `release.yml`'s own
/// "never in a release" check (RFC-098 §"The probe", stage B) greps the
/// release binary for — if this string can be found in
/// `target/release/forskscope`, the probe was compiled in. Stage A does
/// not add that CI step; the marker is named here so stage B's check
/// greps the real constant, not a hand-copied string.
pub const PLAN_ENV_VAR: &str = "FORSKSCOPE_LAYOUT_PROBE_PLAN";

/// How long a view's own DOM must go without a mutation before the probe
/// considers it settled (milliseconds, polled once per animation frame
/// inside the measurement script itself — see the module doc's own
/// "Settle condition").
const SETTLE_QUIET_MS: f64 = 120.0;
/// Upper bound on how long one combination's settle-wait may run before
/// giving up and measuring anyway, with `settled: false` in its own
/// result row (milliseconds).
const SETTLE_TIMEOUT_MS: f64 = 5000.0;

/// A probe run, read whole from the file [`PLAN_ENV_VAR`] names.
#[derive(Debug, Deserialize)]
pub struct Plan {
    /// Where [`ResultRow`]s are appended, one JSON object per line. Created
    /// if missing; truncated if it already exists, so a re-run of the same
    /// plan never mixes two runs' rows together.
    pub results_path: PathBuf,
    /// Every view to measure, each opened once, then cycled through every
    /// text-size/language combination before moving to the next view.
    pub views: Vec<ViewPlan>,
    /// `AppSettings::text_size` values to measure (e.g. `[100, 125, 150,
    /// 175, 200]`) — not normalized here; the driver's own plan is
    /// expected to name real steps directly.
    pub text_sizes: Vec<u32>,
    /// `"en"` / `"ja"`, mapped to [`Lang`] by [`parse_lang`].
    pub languages: Vec<String>,
    /// `"WxH"` in logical pixels (e.g. `"1000x700"`), applied once via
    /// [`dioxus_desktop::DesktopContext::window`]'s own `set_inner_size`
    /// before the first view opens, and recorded into every
    /// [`ResultRow`] for traceability. `None` leaves the window at
    /// whatever size it already launched with.
    #[serde(default)]
    pub window_size: Option<String>,
}

/// One view to open and measure, with whatever its own fixture needs.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ViewPlan {
    /// Opens a file comparison from two real paths (the driver's own
    /// generated fixture files), the same call every in-app "Compare"
    /// button makes (`state::open_compare`).
    FileComparison { left: PathBuf, right: PathBuf },
    /// Opens the Settings dialog (`store.modal.set(Modal::Settings)`),
    /// the same call the tab bar's own "Settings" button makes, with
    /// *Advanced* opened (a real DOM click on `.advanced-toggle`, run
    /// once before the text-size/language loop starts — `show_advanced`
    /// is a plain local UI signal, not one of `Store`'s, so this is the
    /// one view the plan can't drive purely through signal writes).
    /// `ignore_extensions`/`ignore_dirs`, if given, seed the two
    /// ignore-list fields before opening — RFC-099 §3's own verification
    /// asks for the badge lists "at 200% holding many entries".
    ///
    /// `raise_toast`, if set, calls [`Store::notify`] right after opening
    /// the dialog — an error toast, the same severity and the same
    /// insert-with-text timing as F62's real "Could not save settings"
    /// (review 218 §2: a probe-level proof that no visible surface sits
    /// above `.toast`, falsified against the regression `b723d41`
    /// shipped — `.scrim`'s own `z-index: 100` with nothing on `.toast`
    /// to answer it). An error [`Notice`](crate::state::Notice) never
    /// auto-dismisses, so it stays up for this view's whole text-size/
    /// language loop, not just the combination that raised it.
    Settings {
        #[serde(default)]
        ignore_extensions: Option<String>,
        #[serde(default)]
        ignore_dirs: Option<String>,
        #[serde(default)]
        raise_toast: bool,
    },
}

/// Where the measurement script walks every combination's own DOM from —
/// `app.rs`'s own outermost wrapper (`id: "app-root"`), not any one
/// view's narrower root.
///
/// Found live, first retroactive-proof run (handoff 101 §Stage A):
/// scoping to a view's own content root (`.diff-wrap`, originally)
/// missed F218 entirely — its own defect was the status bar
/// (`ui/layout/statusbar.rs`'s `StatusBar`), rendered as `.diff-wrap`'s
/// *sibling* in `app.rs` (`StatusBar {}` after the body's own
/// `ControlZone`), never its descendant. `#app-root` wraps the tab bar,
/// the body and the status bar together, so a chrome defect and a
/// view-content defect are both reachable from the same walk. Stage B's
/// own per-view presence check ([`ViewPlan::ready_selector`]) stays
/// scoped narrowly on purpose - confirming a view actually opened is a
/// different question from where the DOM walk itself starts.
const MEASURE_ROOT_SELECTOR: &str = "#app-root";

impl ViewPlan {
    /// The name this view's own rows are tagged with in [`ResultRow::view`].
    fn name(&self) -> &'static str {
        match self {
            Self::FileComparison { .. } => "file-comparison",
            Self::Settings { .. } => "settings",
        }
    }

    /// A selector confirming this view actually opened, checked before
    /// measuring — `role`/`class`/`id`, never a localized `aria-label`
    /// (neither is translated, unlike the text inside them), so one
    /// selector works across every language in the plan. Deliberately
    /// narrower than [`MEASURE_ROOT_SELECTOR`]: this only answers "did
    /// the navigation call this combination made actually land", not
    /// "where does the DOM walk start".
    fn ready_selector(&self) -> &'static str {
        match self {
            // `ui/view/diff.rs`'s `DiffWorkspace` root (`div.diff-wrap`).
            Self::FileComparison { .. } => ".diff-wrap",
            // `ui/view/settings/modal.rs`'s own dialog heading id.
            Self::Settings { .. } => "#settings-title",
        }
    }
}

/// One element the measurement script found over its own box or text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// A short, human-readable description of the element (tag, class,
    /// and an index among siblings sharing both) — not a full CSS
    /// selector, which the script does not attempt to compute uniquely.
    pub element: String,
    /// The first 80 characters of the element's own trimmed text content,
    /// for a clipped-text finding; empty for an overflow finding with no
    /// text of its own.
    #[serde(default)]
    pub text: String,
    pub scroll_width: f64,
    pub client_width: f64,
    pub scroll_height: f64,
    pub client_height: f64,
}

/// Whether `.toast` (if raised — [`ViewPlan::Settings`]'s own
/// `raise_toast`) is actually on top at its own centre point — review
/// 218 §2's probe-level proof that no visible surface, in particular an
/// open dialog's own scrim, sits above it. `present: false` means no
/// toast was raised at all (`raise_toast` was unset, or it auto-
/// dismissed — never true for an error [`Notice`](crate::state::Notice),
/// the only severity this field is ever set with).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToastCheck {
    pub present: bool,
    /// `true` when `document.elementFromPoint` at the toast's own centre
    /// returns something other than the toast or one of its descendants
    /// — meaning some other box paints above it there. Always `false`
    /// when `present` is `false`.
    #[serde(default)]
    pub occluded: bool,
    /// A short description of whatever `elementFromPoint` actually hit,
    /// when `occluded` — the same `tag.class` shape [`Finding::element`]
    /// uses. `None` when `present` is `false` or the toast itself was hit.
    #[serde(default)]
    pub hit: Option<String>,
}

/// One combination's own measurement, appended as one line of
/// [`Plan::results_path`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultRow {
    pub view: String,
    pub text_size: u32,
    pub language: String,
    #[serde(default)]
    pub window_size: Option<String>,
    /// `false` means the settle wait hit [`SETTLE_TIMEOUT_MS`] without a
    /// quiet period — the measurement below still ran, against a view
    /// that may still have been mutating.
    pub settled: bool,
    pub settle_ms: f64,
    /// Wall-clock time for this whole combination's `document::eval`
    /// round trip, measured from the Rust side (`Instant`, not the page's
    /// own `performance.now()`) — includes `settle_ms`, JS execution and
    /// the Dioxus eval channel's own overhead. `probe_ms - settle_ms` is
    /// the cost the DOM walk itself adds once the view has already
    /// settled, which is the number stage A's own cost report uses: a
    /// view resettling on every push is a layout-check design question
    /// for stage B, not a per-measurement cost.
    pub probe_ms: f64,
    /// How many times [`retry_recv`] had to re-issue the real measurement
    /// eval after `EvalError::Finished` before one came back (0 means
    /// the first attempt succeeded). See `retry_recv`'s own doc comment
    /// for the mechanism; surfaced here, not just logged, so a report
    /// reading [`Plan::results_path`] can see exactly how often the race
    /// bit and be honest about it rather than a silent retry hiding it.
    #[serde(default)]
    pub recv_retries: u32,
    /// Boxes with content wider/taller than themselves, excluding:
    /// - any element whose own computed `overflow-x`/`overflow-y` is
    ///   `auto` or `scroll` (a real scroll owner, not a defect — the
    ///   RFC's own "known scroll owners" allowance; stage A applies it
    ///   generically by computed style rather than a named selector
    ///   list, which stage B's fuller check will narrow where that
    ///   over- or under-allows);
    /// - any element the measurement script's own `isVisuallyHidden`
    ///   finds clipped to nothing on purpose (`.sr-only` and friends;
    ///   found live, first run — an accessible-only label's own deliberate
    ///   1×1 clip box reads exactly like an overflow otherwise).
    pub overflow: Vec<Finding>,
    /// Boxes with their own direct text clipped (`scrollWidth >
    /// clientWidth` on an element with a non-empty direct text node),
    /// excluding any element whose own computed `text-overflow` is
    /// `ellipsis` (the RFC's own base allowance) or that `isVisuallyHidden`
    /// finds deliberately clipped, the same as `overflow` above. F220's
    /// own status-bar rules are a stage B refinement, not modeled here.
    pub clipped: Vec<Finding>,
    /// Set when the view's own root selector was never found at all
    /// (e.g. a navigation call that didn't actually open it) — a probe
    /// defect, not a layout finding, reported rather than silently
    /// producing an empty, misleadingly-clean row.
    #[serde(default)]
    pub error: Option<String>,
    /// See [`ToastCheck`]. Always present (not an `Option`) once a
    /// measurement actually ran — `Default` only for the early-return
    /// rows above, where the script never ran at all.
    #[serde(default)]
    pub toast_check: ToastCheck,
}

fn parse_lang(s: &str) -> Lang {
    match s {
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

/// Parses a plan's own `"WxH"` window size into logical pixels. `None`
/// on anything else, rather than a default — a malformed plan value
/// should be reported (`run`'s own caller does), not silently measured
/// against the wrong size.
fn parse_window_size(s: &str) -> Option<(f64, f64)> {
    let (w, h) = s.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

/// Starts the probe if [`PLAN_ENV_VAR`] is set to a readable plan file;
/// otherwise does nothing, so a normal `layout-probe` build with the
/// variable unset behaves exactly like a build without the feature at
/// all. Called once from `app.rs`'s own startup `use_hook`, after the
/// window exists (the probe's own navigation and the measurement script
/// both need a live page).
pub fn maybe_start(store: Store, desktop: dioxus_desktop::DesktopContext) {
    let Ok(plan_path) = std::env::var(PLAN_ENV_VAR) else {
        return;
    };
    // `spawn_forever`, not `spawn` (review 216 §4; the same F208 shape
    // `keyboard::focus_handle` and `state::compare` already found): a
    // plain `spawn` ties this task to whichever component scope is
    // current at the moment it's called, and the probe's own loop calls
    // `store.modal.set` to open and close views as it goes - once a view
    // it opened unmounts, a `spawn`-ed task following it is dropped
    // mid-combination, which is the intermittent "eval did not return:
    // Finished" the stage A reply reported honestly rather than hid.
    // `spawn_forever` runs detached, at `ScopeId(0)` (`Store::new`'s own
    // doc comment), so no view opening or closing anywhere in the plan
    // can ever tear down the probe's own task.
    dioxus_core::spawn_forever(async move {
        run(store, desktop, PathBuf::from(plan_path)).await;
        // A probe run has nothing left to do once its matrix is measured;
        // there is no interactive session to hand control back to, unlike
        // every normal launch. `main()`'s own `atexit` cleanup (removing
        // a private-mode data root) still runs on this `exit()`, same as
        // any other - see `startup_resolve.rs`'s own doc comment on why.
        std::process::exit(0);
    });
}

async fn run(mut store: Store, desktop: dioxus_desktop::DesktopContext, plan_path: PathBuf) {
    let plan: Plan = match std::fs::read_to_string(&plan_path) {
        Ok(raw) => match serde_json::from_str(&raw) {
            Ok(plan) => plan,
            Err(e) => {
                eprintln!("layout-probe: could not parse plan {plan_path:?}: {e}");
                return;
            }
        },
        Err(e) => {
            eprintln!("layout-probe: could not read plan {plan_path:?}: {e}");
            return;
        }
    };

    if let Some(size) = &plan.window_size {
        match parse_window_size(size) {
            Some((w, h)) => desktop
                .window
                .set_inner_size(dioxus_desktop::tao::dpi::LogicalSize::new(w, h)),
            None => eprintln!(
                "layout-probe: window_size {size:?} is not \"WxH\" - window left at its launch size"
            ),
        }
    }

    let mut results_file = match std::fs::File::create(&plan.results_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "layout-probe: could not create results file {:?}: {e}",
                plan.results_path
            );
            return;
        }
    };

    for view in &plan.views {
        open_view(&mut store, view);
        open_view_followup(view).await;
        for &text_size in &plan.text_sizes {
            store.settings.write().text_size = text_size;
            for language in &plan.languages {
                store.settings.write().language = parse_lang(language);
                let row = measure_one(view, text_size, language, plan.window_size.clone()).await;
                let line = match serde_json::to_string(&row) {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("layout-probe: could not serialize a result row: {e}");
                        continue;
                    }
                };
                if let Err(e) = writeln!(results_file, "{line}") {
                    eprintln!("layout-probe: could not write a result row: {e}");
                }
            }
        }
        close_view(&mut store, view);
    }
}

fn open_view(store: &mut Store, view: &ViewPlan) {
    match view {
        ViewPlan::FileComparison { left, right } => {
            store.modal.set(Modal::None);
            open_compare(store, left.clone(), right.clone());
        }
        ViewPlan::Settings {
            ignore_extensions,
            ignore_dirs,
            raise_toast,
        } => {
            if let Some(v) = ignore_extensions {
                store.settings.write().ignore_extensions = v.clone();
            }
            if let Some(v) = ignore_dirs {
                store.settings.write().ignore_dirs = v.clone();
            }
            store.modal.set(Modal::Settings);
            if *raise_toast {
                store.notify("layout-probe: Could not save settings: test toast");
            }
        }
    }
}

/// Run once, right after [`open_view`], for a view whose own content
/// needs a real DOM click, not a signal write, to appear at all —
/// today, only `Settings`' own `.advanced-toggle` (its `show_advanced`
/// is a plain local UI signal). A no-op `document::eval` for every other
/// view.
async fn open_view_followup(view: &ViewPlan) {
    if let ViewPlan::Settings { .. } = view {
        retry_eval("document.querySelector('.advanced-toggle')?.click();").await;
    }
}

fn close_view(store: &mut Store, view: &ViewPlan) {
    match view {
        ViewPlan::FileComparison { .. } => {}
        ViewPlan::Settings { .. } => store.modal.set(Modal::None),
    }
}

async fn measure_one(
    view: &ViewPlan,
    text_size: u32,
    language: &str,
    window_size: Option<String>,
) -> ResultRow {
    // A `store.settings.write()` just before this call schedules a
    // re-render; Dioxus flushes it on this same async executor's own
    // next tick, not synchronously inside the write. A round trip through
    // `document::eval` needs at least one tick of its own to reach the
    // page at all, so by the time a *second* eval's result comes back,
    // any render the first one's own tick triggered is guaranteed flushed
    // - this one-line throwaway eval is that guaranteed first tick, kept
    // separate from the real measurement script below (which still runs
    // its own settle wait afterward, for whatever the render itself then
    // mutates asynchronously - a zoom level change in particular, applied
    // via `wry`'s own `set_zoom_level` rather than a DOM mutation the
    // settle wait's `MutationObserver` would otherwise never see).
    retry_eval_true().await;
    let start = std::time::Instant::now();
    let script = measurement_script(view.ready_selector());
    let (raw, recv_retries) = match retry_recv(&script).await {
        Ok(r) => r,
        Err((e, recv_retries)) => {
            return ResultRow {
                view: view.name().to_string(),
                text_size,
                language: language.to_string(),
                window_size,
                settled: false,
                settle_ms: 0.0,
                probe_ms: start.elapsed().as_secs_f64() * 1000.0,
                recv_retries,
                overflow: Vec::new(),
                clipped: Vec::new(),
                error: Some(format!("eval did not return: {e:?}")),
                toast_check: ToastCheck::default(),
            };
        }
    };
    let probe_ms = start.elapsed().as_secs_f64() * 1000.0;
    match serde_json::from_str::<MeasurementOutcome>(&raw) {
        Ok(m) => ResultRow {
            view: view.name().to_string(),
            text_size,
            language: language.to_string(),
            window_size,
            settled: m.settled,
            settle_ms: m.settle_ms,
            probe_ms,
            recv_retries,
            overflow: m.overflow,
            clipped: m.clipped,
            error: m.error,
            toast_check: m.toast_check,
        },
        Err(e) => ResultRow {
            view: view.name().to_string(),
            text_size,
            language: language.to_string(),
            window_size,
            settled: false,
            settle_ms: 0.0,
            probe_ms,
            recv_retries,
            overflow: Vec::new(),
            clipped: Vec::new(),
            error: Some(format!("could not parse measurement JSON {raw:?}: {e}")),
            toast_check: ToastCheck::default(),
        },
    }
}

/// How many times [`retry_recv`] (and the throwaway tick before it) will
/// re-issue a fresh `document::eval` after `EvalError::Finished` before
/// giving up. See [`retry_recv`]'s own doc comment for why a retry, not a
/// one-line fix, is the right shape here.
const RECV_RETRIES: u32 = 5;

/// The guaranteed-first-tick throwaway eval `measure_one`'s own doc
/// comment describes, retried the same way [`retry_recv`] is - see that
/// function's doc comment for why. Its own result is never read either
/// way, only whether a round trip happened at all.
async fn retry_eval_true() {
    retry_eval("true").await;
}

/// Runs `script` for its side effect only, retried the same way
/// [`retry_recv`] is (a fresh `document::eval` each attempt, since a
/// failed one's own `GenerationalBox` is gone) — shared by the
/// throwaway tick above and [`open_view_followup`]'s own real DOM
/// click, neither of which has a return value worth reading.
async fn retry_eval(script: &str) {
    for attempt in 0..=RECV_RETRIES {
        if dioxus::document::eval(script).await.is_ok() {
            return;
        }
        if attempt < RECV_RETRIES {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}

/// Issues `script` as a fresh `document::eval` and awaits its result,
/// retrying from scratch (a new `Eval`, since a failed one's own
/// `GenerationalBox` is gone) up to [`RECV_RETRIES`] times on
/// `EvalError::Finished` specifically.
///
/// Found live (stage B reliability work, review 216 §4): `Finished` means
/// `dioxus_document::Eval`'s own backing `GenerationalBox` - owned by a
/// fresh, scope-independent `generational_box::Owner` created per query,
/// never by the Dioxus component scope the `spawn_forever` fix already
/// ruled out - has already been invalidated by the time this code tries
/// to read it. Tracing `dioxus-desktop`'s own `native_eval.js`: the
/// per-query JS channel object's cleanup IPC message
/// (`QueryResultData::Drop`, which drops that `Owner`) is posted from a
/// `window.FinalizationRegistry` callback - fired whenever the JS engine
/// happens to garbage-collect that now-unreferenced object, with no
/// guaranteed ordering against this code's own poll of the result. A
/// script run many times in a row, as this probe's own loop does, makes
/// that collection more likely to land mid-measurement than a page that
/// evals rarely. There is no single-line fix for *when* a JS engine
/// decides to collect garbage; retrying with a fresh query is the
/// resilient shape until or unless `dioxus_document`/`dioxus-desktop`
/// close this race upstream.
async fn retry_recv(script: &str) -> Result<(String, u32), (dioxus::document::EvalError, u32)> {
    let mut last_err = None;
    for attempt in 0..=RECV_RETRIES {
        let mut eval = dioxus::document::eval(script);
        match eval.recv().await {
            Ok(raw) => return Ok((raw, attempt)),
            Err(e) => last_err = Some(e),
        }
        if attempt < RECV_RETRIES {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    Err((last_err.expect("loop runs at least once"), RECV_RETRIES))
}

#[derive(Debug, Deserialize)]
struct MeasurementOutcome {
    settled: bool,
    settle_ms: f64,
    #[serde(default)]
    overflow: Vec<Finding>,
    #[serde(default)]
    clipped: Vec<Finding>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    toast_check: ToastCheck,
}

/// The measurement script run in the page for one combination. Returns
/// (via `dioxus.send`) one JSON string matching [`MeasurementOutcome`].
/// `ready_selector` only confirms the expected view actually opened
/// (checked once, before the settle wait, so a navigation that silently
/// failed is reported as `error`, not measured as a suspiciously clean
/// row); the DOM walk itself always starts at [`MEASURE_ROOT_SELECTOR`].
///
/// `describe(el)` below gives each finding a short, readable label
/// (`tag.class#n`) rather than a full unique CSS selector — sufficient
/// for a human reading a failure, and far cheaper than computing one
/// precisely for every candidate element.
fn measurement_script(ready_selector: &str) -> String {
    format!(
        r#"(function() {{
  function settle(readySelector, quietMs, timeoutMs) {{
    return new Promise((resolve) => {{
      let last = performance.now();
      let lastWidth = document.documentElement.clientWidth;
      let lastHeight = document.documentElement.clientHeight;
      const obs = new MutationObserver(() => {{ last = performance.now(); }});
      obs.observe(document.body, {{childList: true, subtree: true, attributes: true, characterData: true}});
      const deadline = performance.now() + timeoutMs;
      function tick() {{
        const now = performance.now();
        // A text-size change applies as a native webview zoom level
        // (`wry`'s own `set_zoom_level`), not a DOM mutation - the
        // MutationObserver above would never see it settle on its own.
        // Polling the root's own client box alongside it catches a zoom
        // still landing the same way a mutation does.
        const w = document.documentElement.clientWidth;
        const h = document.documentElement.clientHeight;
        if (w !== lastWidth || h !== lastHeight) {{
          last = now; lastWidth = w; lastHeight = h;
        }}
        // The view's own navigation call (`open_compare`/`modal.set`) is
        // a signal write too, same as text-size/language - a render it
        // schedules is not guaranteed to have landed on the very first
        // tick this script itself runs on, found live (first retroactive-
        // proof run, handoff 101 §Stage A: `document.querySelector`,
        // called with no wait at all right after the throwaway sync eval,
        // still measured `.diff-wrap` as absent). Polling for it here,
        // inside the same loop the quiet-period check already runs in,
        // costs nothing extra and needs no second round trip.
        const ready = !readySelector || document.querySelector(readySelector) !== null;
        if (ready && now - last > quietMs) {{ obs.disconnect(); resolve(true); }}
        else if (now > deadline) {{ obs.disconnect(); resolve(false); }}
        else {{ requestAnimationFrame(tick); }}
      }}
      requestAnimationFrame(tick);
    }});
  }}
  function isScrollOwner(el) {{
    const cs = getComputedStyle(el);
    return cs.overflowX === 'auto' || cs.overflowX === 'scroll' ||
           cs.overflowY === 'auto' || cs.overflowY === 'scroll';
  }}
  function isVisuallyHidden(el) {{
    // `.sr-only`, and `.local-only`/`.mode-indicator` under their own
    // narrow-width queries (02-layout-shell.css), all use this exact
    // clip-rect technique on purpose (review 204 §2.2's own doc comment:
    // "never display:none/visibility:hidden, which would also drop the
    // accessible name") - a 1x1, fully clipped box is the deliberate
    // shape of an accessible-only label, not a box something failed to
    // fit into.
    return getComputedStyle(el).clip === 'rect(0px, 0px, 0px, 0px)';
  }}
  function ownText(el) {{
    let s = '';
    for (const n of el.childNodes) {{
      if (n.nodeType === Node.TEXT_NODE) {{ s += n.textContent; }}
    }}
    return s.trim();
  }}
  function describe(el, index) {{
    const cls = el.className && typeof el.className === 'string'
      ? '.' + el.className.trim().split(/\s+/).join('.') : '';
    return el.tagName.toLowerCase() + cls + '#' + index;
  }}
  return (async () => {{
    const t0 = performance.now();
    const settled = await settle('{ready_selector}', {SETTLE_QUIET_MS}, {SETTLE_TIMEOUT_MS});
    const settle_ms = performance.now() - t0;
    if (!document.querySelector('{ready_selector}')) {{
      dioxus.send(JSON.stringify({{
        settled, settle_ms, overflow: [], clipped: [],
        error: 'view did not open: {ready_selector} not found',
      }}));
      return;
    }}
    const root = document.querySelector('{MEASURE_ROOT_SELECTOR}');
    if (!root) {{
      const out = JSON.stringify({{settled, settle_ms, overflow: [], clipped: [], error: 'measure root not found: {MEASURE_ROOT_SELECTOR}'}});
      dioxus.send(out);
      return;
    }}
    const overflow = [];
    const clipped = [];
    const all = root.querySelectorAll('*');
    for (let i = 0; i < all.length; i++) {{
      const el = all[i];
      const cs = getComputedStyle(el);
      if (isVisuallyHidden(el)) {{ continue; }}
      const wOverRaw = el.scrollWidth > el.clientWidth + 1;
      const hOver = el.scrollHeight > el.clientHeight + 1;
      // A deliberately ellipsized box is narrower than its content by
      // design - that's the whole point of `text-overflow: ellipsis` - so
      // it is excluded here exactly as it already is below for `clipped`,
      // not just from the text-clip check.
      const ellipsized = cs.textOverflow === 'ellipsis';
      const wOver = wOverRaw && !ellipsized;
      if ((wOver || hOver) && !isScrollOwner(el)) {{
        overflow.push({{
          element: describe(el, i), text: '',
          scroll_width: el.scrollWidth, client_width: el.clientWidth,
          scroll_height: el.scrollHeight, client_height: el.clientHeight,
        }});
      }}
      if (wOver) {{
        const txt = ownText(el);
        if (txt.length > 0) {{
          clipped.push({{
            element: describe(el, i), text: txt.slice(0, 80),
            scroll_width: el.scrollWidth, client_width: el.clientWidth,
            scroll_height: el.scrollHeight, client_height: el.clientHeight,
          }});
        }}
      }}
    }}
    // Review 218 §2: a probe-level proof that `.toast`, when raised, is
    // actually hit at its own centre point - not covered by `.scrim` or
    // anything else. `elementFromPoint` answers "what's really on top
    // here", the same question a screenshot leaves to a human eye.
    let toast_check = {{present: false, occluded: false, hit: null}};
    const toastEl = document.querySelector('.toast');
    if (toastEl) {{
      const r = toastEl.getBoundingClientRect();
      const cx = r.left + r.width / 2;
      const cy = r.top + r.height / 2;
      const hit = document.elementFromPoint(cx, cy);
      const isToast = hit === toastEl || toastEl.contains(hit);
      toast_check = {{
        present: true,
        occluded: !isToast,
        hit: isToast || !hit ? null : describe(hit, -1),
      }};
    }}
    dioxus.send(JSON.stringify({{settled, settle_ms, overflow, clipped, toast_check}}));
  }})();
}})()"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lang_defaults_to_english_for_an_unrecognized_code() {
        assert_eq!(parse_lang("en"), Lang::En);
        assert_eq!(parse_lang("ja"), Lang::Ja);
        assert_eq!(parse_lang("xx"), Lang::En);
    }

    #[test]
    fn parse_window_size_reads_w_x_h_and_rejects_the_rest() {
        assert_eq!(parse_window_size("1000x700"), Some((1000.0, 700.0)));
        assert_eq!(parse_window_size(" 800 x 560 "), Some((800.0, 560.0)));
        assert_eq!(parse_window_size("1000"), None);
        assert_eq!(parse_window_size("1000x700x3"), None);
        assert_eq!(parse_window_size("widexhigh"), None);
    }

    #[test]
    fn plan_deserializes_from_the_documented_shape() {
        let raw = r#"{
            "results_path": "/tmp/results.jsonl",
            "views": [
                {"kind": "file-comparison", "left": "/a.txt", "right": "/b.txt"},
                {"kind": "settings"}
            ],
            "text_sizes": [100, 200],
            "languages": ["en", "ja"],
            "window_size": "1000x700"
        }"#;
        let plan: Plan = serde_json::from_str(raw).expect("plan must parse");
        assert_eq!(plan.views.len(), 2);
        assert_eq!(plan.views[0].name(), "file-comparison");
        assert_eq!(plan.views[1].name(), "settings");
        assert_eq!(plan.text_sizes, vec![100, 200]);
        assert_eq!(plan.window_size.as_deref(), Some("1000x700"));
    }

    #[test]
    fn view_plan_ready_selectors_are_class_or_id_not_text() {
        // A selector built from `class`/`id` works across every language
        // in the plan; one built from `aria-label` text would not, since
        // that text is translated and `settings.rs`'s label literally
        // reads "Settings" in English only. Falsify by switching either
        // selector to an `[aria-label=...]` form: this still compiles but
        // the assertion below catches it.
        let fc = ViewPlan::FileComparison {
            left: "a".into(),
            right: "b".into(),
        };
        let settings = ViewPlan::Settings {
            ignore_extensions: None,
            ignore_dirs: None,
            raise_toast: false,
        };
        for selector in [fc.ready_selector(), settings.ready_selector()] {
            assert!(
                selector.starts_with('.') || selector.starts_with('#'),
                "{selector} must select by class or id, not an attribute value"
            );
        }
    }

    #[test]
    fn measurement_script_embeds_the_given_selector_once() {
        let script = measurement_script(".diff-wrap");
        assert!(script.contains("querySelector('.diff-wrap')"));
    }

    #[test]
    fn measurement_script_checks_the_toast_with_element_from_point() {
        let script = measurement_script(".diff-wrap");
        assert!(script.contains("elementFromPoint"));
        assert!(script.contains("querySelector('.toast')"));
        assert!(script.contains("toast_check"));
    }

    #[test]
    fn settings_plan_deserializes_raise_toast_and_defaults_it_false() {
        let raw = r#"{"kind": "settings", "raise_toast": true}"#;
        let view: ViewPlan = serde_json::from_str(raw).expect("must parse");
        let ViewPlan::Settings { raise_toast, .. } = view else {
            panic!("expected Settings");
        };
        assert!(raise_toast);

        let raw = r#"{"kind": "settings"}"#;
        let view: ViewPlan = serde_json::from_str(raw).expect("must parse");
        let ViewPlan::Settings { raise_toast, .. } = view else {
            panic!("expected Settings");
        };
        assert!(!raise_toast);
    }

    #[test]
    fn toast_check_default_is_not_present_and_not_occluded() {
        let check = ToastCheck::default();
        assert!(!check.present);
        assert!(!check.occluded);
        assert_eq!(check.hit, None);
    }
}
