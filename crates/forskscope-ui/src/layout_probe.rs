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
use forskscope_ui_logic::{CompareRequest, SaveDestination};
use serde::{Deserialize, Serialize};

use crate::state::{
    ColourVision, Contrast, Lang, Modal, Store, Theme, close_dir_tab, open_compare,
    open_compare_request, open_dir_compare,
};

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
    /// `"dark"`/`"light"`/`"night"`/`"system"`, mapped by [`parse_theme`].
    /// Stage B (handoff 101, review 216 §2): the layout checks only need
    /// one value here (their own axes are text size × language × window
    /// × view); the rendered-contrast check is what needs every theme.
    /// Defaults to `["light"]` so a stage-A-shaped plan, with this field
    /// absent, measures exactly what it always did.
    #[serde(default = "default_single_axis_light")]
    pub themes: Vec<String>,
    /// `"standard"`/`"high"`/`"system"`, mapped by [`Contrast::from_id`].
    /// Same reasoning and default (`["standard"]`) as `themes`.
    #[serde(default = "default_single_axis_standard")]
    pub contrasts: Vec<String>,
    /// `"standard"`/`"red-green-safe"`, mapped by [`ColourVision::from_id`].
    /// Same reasoning and default (`["standard"]`) as `themes`.
    #[serde(default = "default_single_axis_standard")]
    pub colour_visions: Vec<String>,
    /// `"WxH"` in logical pixels (e.g. `"1000x700"`), applied once via
    /// [`dioxus_desktop::DesktopContext::window`]'s own `set_inner_size`
    /// before the first view opens, and recorded into every
    /// [`ResultRow`] for traceability. `None` leaves the window at
    /// whatever size it already launched with.
    #[serde(default)]
    pub window_size: Option<String>,
}

fn default_single_axis_light() -> Vec<String> {
    vec!["light".to_string()]
}

fn default_single_axis_standard() -> Vec<String> {
    vec!["standard".to_string()]
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
    /// The Explorer workspace, the default view (`Modal::None`, no tab
    /// open) — nothing to navigate to, since it's where the app already
    /// is at startup.
    Explorer,
    /// Deep Compare, opened the same way the Explorer's own folder-compare
    /// action does (`state::open_dir_compare`). RFC-097 stage 3 §3's own
    /// first-row auto-selection ("the first row is selected when the list
    /// appears") means this view needs no DOM click of its own, unlike
    /// Settings' *Advanced* — the settle wait's own `ready_selector`
    /// below is what actually waits for the scan to finish and a row to
    /// land selected.
    DeepCompare { left: PathBuf, right: PathBuf },
    /// Git mergetool mode (RFC-077): local vs remote, saving to a distinct
    /// `merged` path — `open_compare_request` with `SaveDestination::
    /// Explicit`, the same conversion `main()`'s own three-argument
    /// startup path uses. Renders through the same `DiffWorkspace` as
    /// `FileComparison`, with the mergetool-only affordances RFC-077
    /// adds, so it is measured as a view of its own, not folded into
    /// `FileComparison`'s own rows.
    MergeTool {
        local: PathBuf,
        remote: PathBuf,
        merged: PathBuf,
    },
    /// *Shortcuts* (`store.modal.set(Modal::KeyboardRef)`), the tab bar's
    /// own "? Shortcuts" button.
    Shortcuts,
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
            Self::Explorer => "explorer",
            Self::DeepCompare { .. } => "deep-compare",
            Self::MergeTool { .. } => "mergetool",
            Self::Shortcuts => "shortcuts",
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
            // `ui/view/diff.rs`'s `DiffWorkspace` root (`div.diff-wrap`),
            // shared by plain compare and mergetool mode alike.
            Self::FileComparison { .. } | Self::MergeTool { .. } => ".diff-wrap",
            // `ui/view/settings/modal.rs`'s own dialog heading id.
            Self::Settings { .. } => "#settings-title",
            // `ui/view/explorer.rs`'s real (non-private-mode-empty) root.
            Self::Explorer => ".explorer-browse",
            // `ui/view/deep_compare.rs`: the scan has finished and RFC-097
            // stage 3 §3's own first-row auto-selection has landed, not
            // merely `.deep-compare` (present the instant the scanning
            // placeholder itself mounts, well before there is anything
            // to measure).
            Self::DeepCompare { .. } => ".deep-table [aria-selected=\"true\"]",
            // `ui/overlay/keybindings.rs`'s `KeyboardRefModal`.
            Self::Shortcuts => ".kb-table",
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

/// One finding from a stage B check whose own measured value doesn't fit
/// [`Finding`]'s scroll/client box shape — hit area, off-screen controls,
/// text size, rendered contrast, label placement and overlap all report
/// through this instead. `detail` is free text (e.g. "18×20px, needs
/// 24×24" or "contrast 3.1:1 against #1e1f22, needs 4.5:1") rather than
/// a fixed numeric shape, since each check's own "what's wrong" reads
/// differently and a human is who reads it, in `cargo xtask
/// layout-check`'s own report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub element: String,
    #[serde(default)]
    pub text: String,
    pub detail: String,
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
    /// `"dark"`/`"light"`/`"night"` — the resolved value the rendered-
    /// contrast check actually ran against this combination with.
    pub theme: String,
    pub contrast: String,
    pub colour_vision: String,
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
    /// Interactive elements (`button`, `[role=button]`, `a`, `input`,
    /// `select`, `[tabindex]`) with any part of their own box outside
    /// the viewport, excluding any with a scrollable ancestor (named the
    /// same way `isScrollOwner` already does for `overflow`) that could
    /// still bring them into view. No allowance (RFC-098 §2).
    #[serde(default)]
    pub off_screen: Vec<Measurement>,
    /// The same interactive-element selector as `off_screen`, under
    /// 24×24 CSS px. Allowed by [`HIT_AREA_ALLOWLIST`], WCAG 2.5.8's own
    /// spacing exception, by named selector.
    #[serde(default)]
    pub hit_area: Vec<Measurement>,
    /// Computed `font-size` under 12px at text size 100% (meaningless at
    /// any other text size, which scales every font — the script skips
    /// the check there rather than report it). Allowed by
    /// [`TEXT_SIZE_ALLOWLIST`], the same two non-text-glyph selectors
    /// `xtask::contrast`'s own `FONT_SIZE_ALLOWLIST` names. Named
    /// `small_text`, not `text_size` — that name is already
    /// [`ResultRow::text_size`], the axis value itself.
    #[serde(default)]
    pub small_text: Vec<Measurement>,
    /// Each text node's own computed `color` against its effective
    /// background (walking up for the first ancestor with its own fully
    /// opaque `background-color`, compositing partial alpha on the way),
    /// against 4.5:1 (7:1 under *High*) — or, for a control's own
    /// border, 3:1 against its adjacent background. Allowed by
    /// [`RENDERED_CONTRAST_ALLOWLIST`] (disabled controls; under *High*,
    /// their own 3:1 floor — the same exemptions `xtask::contrast`'s
    /// static gate already carries).
    #[serde(default)]
    pub rendered_contrast: Vec<Measurement>,
    /// A labelled region (`[aria-labelledby]`) whose own visible label
    /// doesn't sit where review 216 §3 requires: horizontal overlap with
    /// the region, at or above its top, with no other labelled region's
    /// box in between. No allowance.
    #[serde(default)]
    pub label_placement: Vec<Measurement>,
    /// With a dialog (`.modal`'s own containing `.scrim`) open, a grid of
    /// points sampled inside the dialog's box whose `elementFromPoint`
    /// hits something that is neither the dialog nor its descendant —
    /// review 217/218/219's own check, generalized from the one-point
    /// [`ToastCheck`] above to the RFC's full grid shape. Allowed by
    /// `.toast` (review 218 §2's own reasoning: transient, dismissed by
    /// a click, the app's only error notice).
    #[serde(default)]
    pub dialog_occlusion: Vec<Measurement>,
    /// Two visible boxes, each holding text or interactive, neither
    /// containing the other, both in normal flow (not sticky, fixed or
    /// absolute), intersecting by more than 1px in both axes. Parts
    /// clipped out by their own scroll container are skipped. No
    /// allowance (review 218 §4).
    #[serde(default)]
    pub overlap: Vec<Measurement>,
    /// Set when the view's own root selector was never found at all
    /// (e.g. a navigation call that didn't actually open it) — a probe
    /// defect, not a layout finding, reported rather than silently
    /// producing an empty, misleadingly-clean row.
    #[serde(default)]
    pub error: Option<String>,
    /// See [`ToastCheck`]. Always present (not an `Option`) once a
    /// measurement actually ran — `Default` only for the early-return
    /// rows above, where the script never ran at all. Kept alongside the
    /// more general `dialog_occlusion` above: review 219 §1 already
    /// approved this one-point proof's own shape specifically for
    /// `raise_toast`, so it stays as its own targeted check rather than
    /// being folded in and losing that approval's exact shape.
    #[serde(default)]
    pub toast_check: ToastCheck,
}

fn parse_lang(s: &str) -> Lang {
    match s {
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

/// Parses a plan's own `themes` entry. Defaults to `Theme::Light`, the
/// same value [`default_single_axis_light`] seeds a plan with, for
/// anything unrecognized rather than failing the whole run over one bad
/// value.
fn parse_theme(s: &str) -> Theme {
    match s {
        "dark" => Theme::Dark,
        "night" => Theme::Night,
        "system" => Theme::System,
        _ => Theme::Light,
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

/// Renders a fixed list of selectors as a JS array-of-strings literal,
/// for embedding one of the named allowlists directly into the
/// measurement script's own source — these never contain a quote or
/// backslash (they're all plain CSS attribute/class selectors this
/// module itself defines), so no escaping beyond the wrapping quotes is
/// needed.
fn json_string_array(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| format!("\"{s}\"")).collect();
    format!("[{}]", quoted.join(", "))
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

    // Review 222 §4: a deterministic signal for "has the zoom this
    // text-size change applies actually landed", in place of the
    // display-scale-dependent `expectedVw`/`expectedVh` review 221 §2.1
    // tried and reverted. Tracked once for the whole plan, across every
    // view - `set_zoom_level` is a webview-level setting, not scoped to
    // whatever view is currently open, so a text size already reached
    // under an earlier view needs no second wait under a later one.
    //
    // Found live: seeded at `None`, the plan's own first text size -
    // whenever it's 100, `app.rs`'s own startup render (its own "0u32
    // sentinel... first render always differs from a real step"
    // comment) has already applied that zoom before this loop's own
    // first write even runs, which is then a no-op `Signal` write that
    // never calls `set_zoom_level` again - nothing ever changes
    // `clientWidth` away from what this loop reads as "before", so
    // the wait ran out its own full timeout, every time, for no
    // reason. Seeded from the app's own actual current value instead,
    // read directly off the signal - no eval needed, this side of the
    // page already knows it.
    let mut last_text_size_applied: Option<u32> = Some(store.settings.read().text_size);

    for view in &plan.views {
        open_view(&mut store, view);
        open_view_followup(view).await;
        for &text_size in &plan.text_sizes {
            // Captured *before* the write just below schedules the
            // render that calls `set_zoom_level` (`app.rs`'s own
            // render-body guard) - `measure_one`'s own settle wait
            // then polls for this value to change, rather than for a
            // computed target it would otherwise have to get exactly
            // right on every display.
            let before_zoom_width = if last_text_size_applied == Some(text_size) {
                None
            } else {
                last_text_size_applied = Some(text_size);
                Some(read_client_width().await)
            };
            store.settings.write().text_size = text_size;
            for language in &plan.languages {
                store.settings.write().language = parse_lang(language);
                for theme in &plan.themes {
                    store.settings.write().theme = parse_theme(theme);
                    for contrast in &plan.contrasts {
                        store.settings.write().contrast =
                            Contrast::from_id(contrast).unwrap_or_default();
                        for colour_vision in &plan.colour_visions {
                            store.settings.write().colour_vision =
                                ColourVision::from_id(colour_vision).unwrap_or_default();
                            let row = measure_one(
                                view,
                                text_size,
                                language,
                                theme,
                                contrast,
                                colour_vision,
                                plan.window_size.clone(),
                                before_zoom_width,
                            )
                            .await;
                            let line = match serde_json::to_string(&row) {
                                Ok(l) => l,
                                Err(e) => {
                                    eprintln!(
                                        "layout-probe: could not serialize a result row: {e}"
                                    );
                                    continue;
                                }
                            };
                            if let Err(e) = writeln!(results_file, "{line}") {
                                eprintln!("layout-probe: could not write a result row: {e}");
                            }
                        }
                    }
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
        ViewPlan::Explorer => {
            store.modal.set(Modal::None);
        }
        ViewPlan::DeepCompare { left, right } => {
            store.modal.set(Modal::None);
            open_dir_compare(store, left.clone(), right.clone());
        }
        ViewPlan::MergeTool {
            local,
            remote,
            merged,
        } => {
            store.modal.set(Modal::None);
            open_compare_request(
                store,
                CompareRequest {
                    left_input: local.clone(),
                    right_input: remote.clone(),
                    save_destination: SaveDestination::Explicit(merged.clone()),
                },
            );
        }
        ViewPlan::Shortcuts => {
            store.modal.set(Modal::KeyboardRef);
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
        // Found live, first `--inject label-placement` attempt: a bare
        // `document.querySelector('.advanced-toggle')?.click()`, with
        // no wait of its own, ran before `open_view`'s own
        // `store.modal.set` had rendered anything at all - `?.` means a
        // still-missing toggle is not an error, so `retry_eval`'s own
        // retry-on-`EvalError` never fires either. The click silently
        // no-ops, *Advanced* never opens, and nothing downstream (the
        // ignore-list fields, their own badges) is ever there to
        // measure - reported as a clean row, not as a failure, which is
        // exactly the "suspiciously clean" shape this probe's own
        // settle condition was already built to avoid for the view
        // itself. Polls for the toggle the same way, then clicks once
        // it actually exists.
        //
        // Found live again, 1 run in 6-8 of repeated flake checks at
        // 800x560: resolving right after `toggle.click()` races the
        // signal write's own re-render - the click's mutation can
        // still be in flight when `settle()` starts observing
        // immediately after this awaits, so it can miss the mutation
        // entirely and report a clean-looking Settings row with
        // Advanced never actually open. A first fix (poll for
        // `#binary-cmp-help`, the *first* element the advanced section
        // renders) only narrowed the window: that element landing
        // doesn't mean the rest of the section's own, much larger
        // subtree (the ignore-list fields, their badges, the profile
        // list) has finished reconciling too - confirmed by the
        // flake's own shape, always a fraction of the expected count,
        // never zero. Now polls for `.new-profile-btn`, the advanced
        // section's own last static element (`modal.rs`'s own
        // `if *show_advanced.read()` block, right before `end
        // show_advanced`) - everything above it in source order is
        // already in the tree once this one is.
        retry_eval(
            r#"(function() {
  return new Promise((resolve) => {
    const deadline = performance.now() + 5000;
    function waitForToggle() {
      const toggle = document.querySelector('.advanced-toggle');
      if (toggle) { toggle.click(); waitForOpen(); }
      else if (performance.now() > deadline) { resolve(false); }
      else { requestAnimationFrame(waitForToggle); }
    }
    function waitForOpen() {
      if (document.querySelector('.new-profile-btn')) { resolve(true); }
      else if (performance.now() > deadline) { resolve(false); }
      else { requestAnimationFrame(waitForOpen); }
    }
    requestAnimationFrame(waitForToggle);
  });
})()"#,
        )
        .await;
    }
}

fn close_view(store: &mut Store, view: &ViewPlan) {
    match view {
        ViewPlan::FileComparison { .. } | ViewPlan::MergeTool { .. } | ViewPlan::Explorer => {}
        // Found live, first full-matrix run: `app.rs`'s own view match
        // picks Deep Compare whenever `active_dir` is `Some`, *before*
        // even looking at `active` (the file-tab index) - left set, it
        // silently kept every later view in the same plan run pinned to
        // Deep Compare, each one then failing its own `ready_selector`
        // (reported as "view did not open", not as a Deep-Compare-
        // shaped row, which is what made it non-obvious at first).
        ViewPlan::DeepCompare { .. } => close_dir_tab(store, 0),
        ViewPlan::Settings { .. } | ViewPlan::Shortcuts => store.modal.set(Modal::None),
    }
}

#[allow(clippy::too_many_arguments)]
async fn measure_one(
    view: &ViewPlan,
    text_size: u32,
    language: &str,
    theme: &str,
    contrast: &str,
    colour_vision: &str,
    window_size: Option<String>,
    before_zoom_width: Option<f64>,
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
    // 7:1 under *High* (RFC-098 §2's own text-contrast row); the
    // control-edge 3:1 floor is unaffected by *High*, per the same row.
    let required_text_contrast = if contrast == "high" { 7.0 } else { 4.5 };
    // Review 221 §3: `.diff-mark`'s own, lower floor - `xtask::contrast`'s
    // `CONTROL_MIN`/`HIGH_MARK_MIN` for this exact pair, not the text row
    // above. Computed here, not as a plain constant pair passed straight
    // through, so it stays next to `required_text_contrast`'s own *High*
    // branch - the two are meant to be read together, not independently.
    let required_mark_contrast = if contrast == "high" {
        RENDERED_CONTRAST_MARK_HIGH_MIN
    } else {
        RENDERED_CONTRAST_MARK_MIN
    };
    let zoom = f64::from(text_size) / 100.0;
    let script = measurement_script(
        view.ready_selector(),
        required_text_contrast,
        required_mark_contrast,
        zoom,
        before_zoom_width,
    );
    let (raw, recv_retries) = match retry_recv(&script).await {
        Ok(r) => r,
        Err((e, recv_retries)) => {
            return ResultRow {
                view: view.name().to_string(),
                text_size,
                language: language.to_string(),
                theme: theme.to_string(),
                contrast: contrast.to_string(),
                colour_vision: colour_vision.to_string(),
                window_size,
                settled: false,
                settle_ms: 0.0,
                probe_ms: start.elapsed().as_secs_f64() * 1000.0,
                recv_retries,
                overflow: Vec::new(),
                clipped: Vec::new(),
                off_screen: Vec::new(),
                hit_area: Vec::new(),
                small_text: Vec::new(),
                rendered_contrast: Vec::new(),
                label_placement: Vec::new(),
                dialog_occlusion: Vec::new(),
                overlap: Vec::new(),
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
            theme: theme.to_string(),
            contrast: contrast.to_string(),
            colour_vision: colour_vision.to_string(),
            window_size,
            settled: m.settled,
            settle_ms: m.settle_ms,
            probe_ms,
            recv_retries,
            overflow: m.overflow,
            clipped: m.clipped,
            off_screen: m.off_screen,
            hit_area: m.hit_area,
            small_text: m.small_text,
            rendered_contrast: m.rendered_contrast,
            label_placement: m.label_placement,
            dialog_occlusion: m.dialog_occlusion,
            overlap: m.overlap,
            error: m.error,
            toast_check: m.toast_check,
        },
        Err(e) => ResultRow {
            view: view.name().to_string(),
            text_size,
            language: language.to_string(),
            theme: theme.to_string(),
            contrast: contrast.to_string(),
            colour_vision: colour_vision.to_string(),
            window_size,
            settled: false,
            settle_ms: 0.0,
            probe_ms,
            recv_retries,
            overflow: Vec::new(),
            clipped: Vec::new(),
            off_screen: Vec::new(),
            hit_area: Vec::new(),
            small_text: Vec::new(),
            rendered_contrast: Vec::new(),
            label_placement: Vec::new(),
            dialog_occlusion: Vec::new(),
            overlap: Vec::new(),
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

/// Review 222 §4: `document.documentElement.clientWidth`, read before
/// a text-size change is applied - the "before" half of the
/// deterministic zoom wait `measurement_script`'s own settle condition
/// uses, in place of a computed, display-scale-dependent target
/// (review 221 §2.1's own `expectedVw`/`expectedVh`, tried and
/// reverted). `0.0` on a parse failure or a retry exhaustion: the
/// wait this feeds then degrades to "wait until it changes from 0",
/// which a real `clientWidth` always does, so a transient read
/// failure here costs one extra, harmless tick, not a hang.
async fn read_client_width() -> f64 {
    // `retry_recv`'s own `.recv()` waits for an explicit `dioxus.send`
    // call, the same channel every other script using it (the
    // measurement script itself) already ends with - found live, the
    // hard way: a bare expression here, with no `dioxus.send` of its
    // own, never delivered anything through that channel at all, so
    // `.recv()` hung indefinitely (it has no timeout of its own,
    // `retry_recv`'s own doc comment already says so) instead of
    // returning a parse failure.
    match retry_recv("dioxus.send(JSON.stringify(document.documentElement.clientWidth))").await {
        Ok((raw, _)) => raw.parse().unwrap_or(0.0),
        Err(_) => 0.0,
    }
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
    off_screen: Vec<Measurement>,
    #[serde(default)]
    hit_area: Vec<Measurement>,
    #[serde(default)]
    small_text: Vec<Measurement>,
    #[serde(default)]
    rendered_contrast: Vec<Measurement>,
    #[serde(default)]
    label_placement: Vec<Measurement>,
    #[serde(default)]
    dialog_occlusion: Vec<Measurement>,
    #[serde(default)]
    overlap: Vec<Measurement>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    toast_check: ToastCheck,
}

/// Selector for every element the hit-area/off-screen checks treat as an
/// interactive target — the RFC's own list, minus anything a sighted or
/// keyboard user could never reach anyway (`tabindex="-1"`, the tab ×'s
/// own RFC-087 §1 shape).
const INTERACTIVE_SELECTOR: &str = r#"button, [role="button"], a[href], input, select, [tabindex]"#;

/// Named selectors exempt from overflow/clipped once `isScrollOwner`
/// says no — RFC-098 §2's own words for this allowance, "the known
/// scroll owners... named in a list in the probe", which stage A's own
/// generic `isScrollOwner` check deliberately deferred ("stage B's
/// fuller check will narrow where that over- or under-allows", this
/// field's own doc comment).
///
/// `.tablist`: its responsive rule (`90-responsive.css`) sets `overflow:
/// visible` on purpose under the stacked layout, so nothing there is
/// ever lost — found live, and found *not* generalizable the same run
/// (`overflow: visible` is the unset default almost everywhere else in
/// the page, and WebKitGTK's own `scrollWidth` exceeds `clientWidth`
/// for plenty of those too).
///
/// Review 221 §2.2 refused a `.scrim` entry proposed here (reply
/// 189b): it would have exempted exactly the regression F224 is about
/// (a dialog taller than the window) rather than catching it.
/// `.scrim`'s own `scrollHeight` reflecting `.modal`'s full, unclipped
/// intrinsic size is real (confirmed live: a flex item's default
/// `min-height: auto` means a flex parent's layout still accounts for
/// its full content size even when the item's own `overflow-y: auto`
/// correctly clips what's actually painted) - but nothing paints
/// outside either box, so it names a measurement quirk, not a defect.
/// F224's own property - the dialog's border box inside the viewport -
/// is now checked directly instead, not through this allowlist-and-
/// scroll-size mechanism at all (the dedicated check right before
/// `dialog_occlusion`, below).
const OVERFLOW_ALLOWLIST: &[&str] = &[".tablist"];

/// Named selectors exempt from the hit-area 24×24 floor (WCAG 2.5.8's own
/// spacing exception) — kept empty, a named list ready for the first
/// real exception review decides, same shape as `xtask::contrast`'s own
/// allowlists.
const HIT_AREA_ALLOWLIST: &[&str] = &[];

/// Named selectors exempt from the 12px text-size floor — the exact two
/// non-text glyphs `xtask::contrast`'s own `FONT_SIZE_ALLOWLIST` names
/// (review 098 §2's own "Allowed by" column points at that list by
/// name); kept in sync by hand, since the probe runs in the page and the
/// static gate runs over CSS source, with no shared Rust type between
/// them to enforce it.
const TEXT_SIZE_ALLOWLIST: &[&str] = &[".dirty-dot", ".tab-loading-spinner"];

/// Named selectors exempt from the rendered-contrast floor — disabled
/// controls (the `contrast` gate's own exemption: a disabled control's
/// lower contrast is the platform's own convention for "inert", not a
/// defect).
// Single quotes around the attribute value, not double: `json_string_array`
// wraps each entry in double quotes for the JS array literal it embeds,
// and a double quote inside that would collide with its own.
const RENDERED_CONTRAST_ALLOWLIST: &[&str] = &["[disabled]", "[aria-disabled='true']"];

/// Review 221 §3's own decision on reply 189b's row 5: `.diff-mark`
/// ("−"/"+" on its own hunk-row background) is a mark, not text - not
/// an exemption from rendered contrast, a different, lower floor
/// within it, matching `xtask::contrast`'s own already-reviewed
/// classification for this exact use (`--del-mark on --del-bg`/
/// `--ins-mark on --ins-bg`, `CONTROL_MIN`/`HIGH_MARK_MIN`, not
/// `TEXT_MIN`/`HIGH_TEXT_MIN`) so the dynamic probe and the static
/// gate agree on what this glyph is. One list, `.diff-mark` only, per
/// review 221 §3's own instruction - a future entry is a review
/// decision, not something this probe adds on its own judgment.
const RENDERED_CONTRAST_MARK_SELECTOR: &[&str] = &[".diff-mark"];
/// `xtask::contrast::CONTROL_MIN` kept in step by hand, not shared
/// code - the probe measures rendered colour through the DOM, the
/// static gate parses CSS source text, and the two have never shared
/// a threshold constant. Changing one without the other is exactly
/// the "do both gates agree" question review 221 §3 raised; a stray
/// edit to just one is now the drift to watch for, not a borrow
/// checker error to catch it.
const RENDERED_CONTRAST_MARK_MIN: f64 = 3.0;
/// `xtask::contrast::HIGH_MARK_MIN`, same caveat as
/// [`RENDERED_CONTRAST_MARK_MIN`] just above.
const RENDERED_CONTRAST_MARK_HIGH_MIN: f64 = 4.5;

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
///
/// `required_text_contrast` is 7.0 under *High*, 4.5 otherwise (RFC-098
/// §2's own text-contrast row) — computed by the caller, which already
/// knows the plan's own `contrast` value; the control-edge floor is the
/// fixed `3.0` the same row gives, unaffected by *High*.
///
/// `zoom` (`text_size / 100`) is read only to gate the text-size check
/// to exactly 100% — not to normalize any measured pixel value. Found
/// live: `wry`'s own zoom does not change what `getBoundingClientRect`/
/// `getComputedStyle().fontSize` report for an individual element's own
/// absolute-px box (confirmed by comparing the same control's box at
/// 100% and 200% directly) — only the viewport's own `clientWidth`/
/// `clientHeight` shrinks in CSS-px terms, which is what the settle
/// condition's own poll already relies on. A normalizing division here
/// would be the bug, not its absence.
fn measurement_script(
    ready_selector: &str,
    required_text_contrast: f64,
    required_mark_contrast: f64,
    zoom: f64,
    before_zoom_width: Option<f64>,
) -> String {
    let interactive_selector = INTERACTIVE_SELECTOR;
    let overflow_allowlist = json_string_array(OVERFLOW_ALLOWLIST);
    let hit_area_allowlist = json_string_array(HIT_AREA_ALLOWLIST);
    let text_size_allowlist = json_string_array(TEXT_SIZE_ALLOWLIST);
    let rendered_contrast_allowlist = json_string_array(RENDERED_CONTRAST_ALLOWLIST);
    let rendered_contrast_mark_selector = json_string_array(RENDERED_CONTRAST_MARK_SELECTOR);
    let before_zoom_width_js = before_zoom_width.map_or("null".to_string(), |v| v.to_string());
    format!(
        r#"(function() {{
  function settle(readySelector, quietMs, timeoutMs) {{
    return new Promise((resolve) => {{
      let last = performance.now();
      let lastWidth = document.documentElement.clientWidth;
      let lastHeight = document.documentElement.clientHeight;
      // Review 221 §2.1's own lesson, found to extend past hit area:
      // a text-size change applies as a native webview zoom level
      // (`wry`'s own `set_zoom_level`), which isn't guaranteed to have
      // actually reflowed the page by the time this script's own first
      // tick runs, even though the Rust side already called it before
      // this script was even sent - found live (a five-run determinism
      // check after that fix): one run in several measured a Settings
      // field at close to its 100% width while every other run already
      // had it at 200%'s own narrower one, both window and text size
      // otherwise identical. An `expectedVw`/`expectedVh` check
      // (window size ÷ zoom) was tried and reverted: the system's own
      // display scale factor (confirmed live, this sandbox: ~0.834,
      // consistent across every text size, so a real factor, not
      // noise) means `clientWidth` never equals that raw arithmetic at
      // all, by a margin far past any reasonable tolerance - guessing
      // the factor would be worse than not checking, since a wrong
      // guess times out every run instead of flaking on a few. Kept
      // scroll-position-independent and tolerant instead, the same
      // shape as §2.1's own fix: requires *two* consecutive quiet
      // windows before resolving, not one - a value that only looks
      // settled because polling started after a fast transition
      // already finished still passes a single window, but a
      // transition still arriving late enough to beat the first
      // window needs to then also sit still through a second one,
      // which a value that's still conceptually "in flight" rarely
      // does by coincidence.
      let quietWindowsPassed = 0;
      const obs = new MutationObserver(() => {{ last = performance.now(); quietWindowsPassed = 0; }});
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
          last = now; lastWidth = w; lastHeight = h; quietWindowsPassed = 0;
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
        if (ready && now - last > quietMs) {{
          quietWindowsPassed += 1;
          last = now;
          if (quietWindowsPassed >= 2) {{ obs.disconnect(); resolve(true); return; }}
        }}
        if (now > deadline) {{ obs.disconnect(); resolve(false); }}
        else {{ requestAnimationFrame(tick); }}
      }}
      requestAnimationFrame(tick);
    }});
  }}
  // Review 222 §4: a deterministic precondition for `settle` above,
  // not a replacement for it - "wait until it changes" needs no
  // computed target, so it carries no display-scale dependency for
  // `beforeWidth` (read on the Rust side, before the text-size change
  // that applies a new zoom was even written) to get wrong. `null`
  // (this combination's own zoom is unchanged from the previous one,
  // or there's nothing to wait for - `--inject`'s own plan never
  // resizes) resolves immediately, same as a width that already
  // differs by the time this runs - this script's own first tick is
  // never guaranteed to be *before* the render the Rust side's write
  // already scheduled, so if the zoom beat this call here, there's
  // nothing left to wait for either way.
  function waitForZoomChange(beforeWidth, timeoutMs) {{
    return new Promise((resolve) => {{
      if (beforeWidth === null) {{ resolve(); return; }}
      const deadline = performance.now() + timeoutMs;
      function tick() {{
        if (document.documentElement.clientWidth !== beforeWidth) {{ resolve(); return; }}
        if (performance.now() > deadline) {{ resolve(); return; }}
        requestAnimationFrame(tick);
      }}
      requestAnimationFrame(tick);
    }});
  }}
  function isScrollOwner(el) {{
    const cs = getComputedStyle(el);
    return cs.overflowX === 'auto' || cs.overflowX === 'scroll' ||
           cs.overflowY === 'auto' || cs.overflowY === 'scroll';
  }}
  // Found live, first full-matrix run, then found wrong on the very
  // next one: `.tablist`'s own responsive rule (`90-responsive.css`)
  // sets `overflow: visible` under the stacked layout, on purpose -
  // nothing is lost there, it spills out and stays visible. But
  // `overflow: visible` is also the *default* - unset on almost every
  // element in the page - and WebKitGTK's own `scrollWidth` still
  // exceeds `clientWidth` for plenty of those too (confirmed live: a
  // first attempt exempting any `visible` axis silently zeroed out
  // *every* overflow/clipped finding across the whole matrix, not just
  // `.tablist`'s). A structural rule can't tell "deliberately spills
  // out" from "overflow never applied to this element at all" - only a
  // named exception can, the same shape every other allowlist here
  // already uses.
  function doesNotLoseOverflow(el, overflowAllowlist) {{
    return isScrollOwner(el) || matchesAny(el, overflowAllowlist);
  }}
  // Review 220 §2.1: off-screen, hit area and overlap all used to
  // compare `getBoundingClientRect()` boxes with no regard for
  // clipping, so a diff cell or Explorer row scrolled past the bottom
  // of its own pane read as "overlapping" whatever sits underneath
  // that pane in the DOM, like the status bar - it isn't actually
  // visible there, its scroll container's own `overflow` (any value
  // but `visible`, not just the `auto`/`scroll` `isScrollOwner` already
  // tests for - `hidden` clips exactly the same way) cuts it off first.
  // Walks up from `el`'s own parent, intersecting the running rect with
  // every ancestor that clips on either axis; a result with zero or
  // negative width/height means nothing of `el` is visible right now.
  function visibleRect(el) {{
    let rect = el.getBoundingClientRect();
    for (let p = el.parentElement; p; p = p.parentElement) {{
      const pcs = getComputedStyle(p);
      const clipsX = pcs.overflowX !== 'visible';
      const clipsY = pcs.overflowY !== 'visible';
      if (!clipsX && !clipsY) {{ continue; }}
      const pr = p.getBoundingClientRect();
      const left = clipsX ? Math.max(rect.left, pr.left) : rect.left;
      const right = clipsX ? Math.min(rect.right, pr.right) : rect.right;
      const top = clipsY ? Math.max(rect.top, pr.top) : rect.top;
      const bottom = clipsY ? Math.min(rect.bottom, pr.bottom) : rect.bottom;
      rect = {{ left, right, top, bottom, width: right - left, height: bottom - top }};
      if (rect.width <= 0 || rect.height <= 0) {{ return rect; }}
    }}
    return rect;
  }}
  function isClippedAway(rect) {{
    return rect.width <= 0 || rect.height <= 0;
  }}
  // Review 220 §2.2: a `position: sticky`/`fixed`/`absolute` box floats
  // a whole layer above normal flow, and everything painted *inside*
  // it floats too, not just the box's own element - Settings' sticky
  // footer button "overlapping" a `select`/`input`/`p.field-help`
  // beneath it, or *Shortcuts*' absolutely-positioned column
  // overlapping its own `kb-desc`/`h3`, are both this shape. Overlap
  // only compares candidates within the same layer (RFC-098 §2's own
  // "both in normal flow" - something crossing layers on purpose, like
  // that same footer button sitting over an open dialog, is
  // `dialog_occlusion`'s question, not this one's). Returns the
  // nearest ancestor-or-self that establishes a layer, or `null` for
  // the single base, normal-flow layer everything else shares.
  function layerOf(el) {{
    for (let n = el; n; n = n.parentElement) {{
      const p = getComputedStyle(n).position;
      if (p === 'sticky' || p === 'fixed' || p === 'absolute') {{ return n; }}
    }}
    return null;
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
  function matchesAny(el, selectors) {{
    return selectors.some((s) => el.matches(s));
  }}
  // Review 098 §2's own interactive-element check both share: a control
  // nothing can actually reach doesn't need an on-screen position or a
  // minimum size, so neither check should report one. Disabled and
  // aria-hidden are already excluded elsewhere (isVisuallyHidden covers
  // aria-hidden's own clip-rect shape); tabindex="-1" is the one this
  // selector alone would otherwise wrongly include (RFC-087 §1's tab ×).
  function isReachable(el) {{
    if (isVisuallyHidden(el)) {{ return false; }}
    if (el.matches('[tabindex="-1"]')) {{ return false; }}
    if ('disabled' in el && el.disabled) {{ return false; }}
    return true;
  }}
  // RFC-098 §2's own WCAG relative-luminance contrast ratio - the exact
  // formula `xtask::css_shared::contrast_ratio_rgb` already uses for the
  // static gate, reimplemented here because the probe needs it against
  // *rendered*, already-resolved colour (var()/color-mix() included,
  // for free, by getComputedStyle), not CSS source text.
  function srgbChannelToLinear(c) {{
    return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  }}
  function relativeLuminance(r, g, b) {{
    const lr = srgbChannelToLinear(r / 255);
    const lg = srgbChannelToLinear(g / 255);
    const lb = srgbChannelToLinear(b / 255);
    return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
  }}
  function contrastRatio(rgb1, rgb2) {{
    const l1 = relativeLuminance(rgb1[0], rgb1[1], rgb1[2]);
    const l2 = relativeLuminance(rgb2[0], rgb2[1], rgb2[2]);
    const lighter = Math.max(l1, l2);
    const darker = Math.min(l1, l2);
    return (lighter + 0.05) / (darker + 0.05);
  }}
  // Parses `getComputedStyle(...).color`/`.backgroundColor`'s own
  // resolved form - always `rgb(r, g, b)` or `rgba(r, g, b, a)` once the
  // browser has resolved every `var()`/`color-mix()` in it - into
  // [r, g, b, a], a in [0, 1].
  function parseRgba(s) {{
    const m = s.match(/rgba?\(([^)]+)\)/);
    if (!m) {{ return null; }}
    const parts = m[1].split(',').map((p) => parseFloat(p.trim()));
    return [parts[0], parts[1], parts[2], parts.length > 3 ? parts[3] : 1];
  }}
  // Walks up from `el` compositing each ancestor's own
  // `background-color` alpha onto the next, stopping at the first fully
  // opaque one (or the document root, assumed white if even that has
  // none - WebKitGTK's own canvas default). This is the "effective
  // background" RFC-098 §2's own rendered-contrast row asks for; it does
  // not separately walk `opacity` (a different compositing mechanism
  // from a background colour's own alpha channel) - an element under a
  // non-1 `opacity` is a disabled/hover-revealed control already in
  // `RENDERED_CONTRAST_ALLOWLIST` or excluded by `isReachable`-style
  // reasoning in practice, not a gap this check's own scope needs to
  // close itself.
  function effectiveBackground(el) {{
    let r = 255, g = 255, b = 255;
    let node = el;
    while (node) {{
      const bg = parseRgba(getComputedStyle(node).backgroundColor || 'rgba(0,0,0,0)');
      if (bg) {{
        const [br, bgc, bb, ba] = bg;
        if (ba > 0) {{
          r = br * ba + r * (1 - ba);
          g = bgc * ba + g * (1 - ba);
          b = bb * ba + b * (1 - ba);
          if (ba >= 0.999) {{ break; }}
        }}
      }}
      node = node.parentElement;
    }}
    return [r, g, b];
  }}
  return (async () => {{
    // Review 098 stage B: a runtime error anywhere below - in this
    // script's own new checks especially, each walking live computed
    // style rather than a fixed shape - used to leave `dioxus.send`
    // never called at all, and `eval.recv()` has no timeout of its own:
    // a silent hang, not a reported failure. Caught here and reported
    // as `error` instead, the same as the two explicit checks above.
    try {{
    const t0 = performance.now();
    await waitForZoomChange({before_zoom_width_js}, {SETTLE_TIMEOUT_MS});
    const settled = await settle('{ready_selector}', {SETTLE_QUIET_MS}, {SETTLE_TIMEOUT_MS});
    const settle_ms = performance.now() - t0;
    if (!document.querySelector('{ready_selector}')) {{
      dioxus.send(JSON.stringify({{
        settled, settle_ms, overflow: [], clipped: [],
        error: 'view did not open: {ready_selector} not found',
      }}));
      return;
    }}
    const appRoot = document.querySelector('{MEASURE_ROOT_SELECTOR}');
    if (!appRoot) {{
      const out = JSON.stringify({{settled, settle_ms, overflow: [], clipped: [], error: 'measure root not found: {MEASURE_ROOT_SELECTOR}'}});
      dioxus.send(out);
      return;
    }}
    // Review 221 §2.3: with a dialog open, the view behind the scrim
    // is inert and dimmed by design - `button.apply-btn`/`span.diff-
    // mark` findings in the *Settings*/*Shortcuts* rows were really
    // the comparison view sitting underneath, counted a second time
    // under the wrong row, and (rendered contrast especially) measured
    // without the scrim's own dimming ever applied, so the ratio
    // didn't describe anything actually on screen. Every check but
    // `dialog_occlusion` (which already proves nothing covers the
    // dialog, so it needs the full tree to check *against*) now walks
    // only the dialog's own subtree whenever one is open, not
    // `#app-root`'s whole tree.
    const dialogRootEl = document.querySelector('.scrim .modal');
    const root = dialogRootEl || appRoot;
    const overflow = [];
    const clipped = [];
    const off_screen = [];
    const hit_area = [];
    const small_text = [];
    const rendered_contrast = [];
    const labelled = []; // {{el, index, label}} - gathered for label_placement after the loop
    const vw = document.documentElement.clientWidth;
    const vh = document.documentElement.clientHeight;
    const zoom = {zoom};
    const requiredTextContrast = {required_text_contrast};
    const requiredMarkContrast = {required_mark_contrast};
    const overflowAllowlist = {overflow_allowlist};
    const hitAreaAllowlist = {hit_area_allowlist};
    const textSizeAllowlist = {text_size_allowlist};
    const renderedContrastAllowlist = {rendered_contrast_allowlist};
    const renderedContrastMarkSelector = {rendered_contrast_mark_selector};
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
      if ((wOver || hOver) && !doesNotLoseOverflow(el, overflowAllowlist)) {{
        overflow.push({{
          element: describe(el, i), text: '',
          scroll_width: el.scrollWidth, client_width: el.clientWidth,
          scroll_height: el.scrollHeight, client_height: el.clientHeight,
        }});
      }}
      if (wOver && !doesNotLoseOverflow(el, overflowAllowlist)) {{
        const txt = ownText(el);
        if (txt.length > 0) {{
          clipped.push({{
            element: describe(el, i), text: txt.slice(0, 80),
            scroll_width: el.scrollWidth, client_width: el.clientWidth,
            scroll_height: el.scrollHeight, client_height: el.clientHeight,
          }});
        }}
      }}

      // Off-screen controls and hit area (RFC-098 §2): the same
      // interactive candidates, two different questions about the same
      // `getBoundingClientRect()`.
      if (el.matches('{interactive_selector}') && isReachable(el)) {{
        let r = el.getBoundingClientRect();
        // Review 220 §2.3: `input.profile-radio` (and any input like
        // it) is visually hidden by size and opacity, not by the
        // clip-rect technique `isVisuallyHidden` already recognises -
        // `appearance: none; position: absolute; width: 1px; height:
        // 1px; opacity: 0` (03-components.css), kept focusable on
        // purpose, with its own `<label for>` carrying the visible,
        // clickable text. Measuring the input alone reports its own
        // 1x1 box as a hit-area failure; measuring it as the union of
        // the input and that label - which already is what a user
        // can actually see and click, native `<label for>` behaviour
        // included - reports the real, reachable target instead. A
        // visible, reasonably-sized input gains nothing or little
        // from the union (its own box already dominates), so this
        // applies unconditionally rather than gating on size first.
        if (el.tagName === 'INPUT' && el.id) {{
          const labelEl = document.querySelector('label[for="' + el.id + '"]');
          if (labelEl && !isVisuallyHidden(labelEl)) {{
            const lr = labelEl.getBoundingClientRect();
            if (lr.width > 0 && lr.height > 0) {{
              const left = Math.min(r.left, lr.left);
              const top = Math.min(r.top, lr.top);
              const right = Math.max(r.right, lr.right);
              const bottom = Math.max(r.bottom, lr.bottom);
              r = {{ left, top, right, bottom, width: right - left, height: bottom - top }};
            }}
          }}
        }}
        // Off-screen (RFC-098 §2) keeps the clipped box (review 221
        // §2.1's own instruction: "Overlap and Off-screen keep the
        // clipped box"). Found live (`--inject off-screen`, first run:
        // a scrollable `.modal` - F224's own `overflow-y: auto` - sits
        // above every one of its descendants, so a `position: fixed`
        // button inside it was wrongly exempted as "reachable by
        // scrolling" even though fixed positioning is always viewport-
        // relative; no ancestor's own scroll offset ever moves it.
        // Found wrong the other way, review 220 §2.3: `input.profile-
        // radio` is `position: absolute` too, but *not* viewport-
        // relative like the button above - its containing block is its
        // own row, which scrolls with the modal exactly as an ordinary
        // flow element would. "Absolute always escapes clipping" was
        // this sandbox's own one example, not a rule `position:
        // absolute` keeps in general - only `fixed` is unconditionally
        // viewport-relative.
        const vr = cs.position !== 'fixed' ? visibleRect(el) : r;
        if (!isClippedAway(vr) &&
            (r.right <= 0 || r.bottom <= 0 || r.left >= vw || r.top >= vh)) {{
          off_screen.push({{
            element: describe(el, i), text: ownText(el).slice(0, 80),
            detail: 'box at (' + Math.round(r.left) + ',' + Math.round(r.top) + ') ' +
              Math.round(r.width) + 'x' + Math.round(r.height) +
              ', viewport ' + vw + 'x' + vh,
          }});
        }}
        // Hit area (RFC-098 §2): the element's own box, with its
        // label's (above), full stop - review 221 §2.1's own finding.
        // Clipping decided whether it's *considered at all* here too,
        // once (via the same `isClippedAway(vr)` gate off-screen still
        // uses above) - wrong, found live: the only difference across
        // five otherwise-identical CI runs was always the same shape,
        // a Settings checkbox reported in some attempts and not
        // others, two lines at a time. A control's size doesn't change
        // when it's partly scrolled out of view - you scroll it in -
        // but `.modal`'s own scroll position settling a few px
        // differently run to run flipped `isClippedAway` between empty
        // and not, at exactly the checkbox's own edge. `isReachable`
        // (tabindex="-1", disabled, `isVisuallyHidden`'s clip-rect
        // technique) and `w > 0 && h > 0` below already cover "never
        // rendered at all" without depending on a scroll container's
        // own, inherently transient, current offset.
        {{
          // Found live, first run against a real view: `wry`'s own zoom
          // (`set_zoom_level`, what `text_size` drives) does not change
          // what `getBoundingClientRect` reports for an individual
          // element's own absolute-px box - a 36px-tall button measures
          // 36 at every text size, confirmed by comparing the same
          // button at 100% and 200% directly. Only the *viewport's* own
          // `clientWidth`/`clientHeight` shrinks in CSS-px terms (the
          // settle condition's own reason for polling it), which is
          // what lets a `max-width` media query flip the stacked layout
          // - a fixed-px control's own size is unaffected either way.
          // No normalisation, then: the raw (or unioned) box is already
          // the right unit.
          const w = r.width;
          const h = r.height;
          if (w > 0 && h > 0 && (w < 23.5 || h < 23.5) &&
              !matchesAny(el, hitAreaAllowlist)) {{
            hit_area.push({{
              element: describe(el, i), text: ownText(el).slice(0, 80),
              detail: w.toFixed(1) + 'x' + h.toFixed(1) + 'px, needs 24x24',
            }});
          }}
        }}
      }}

      // Text size (RFC-098 §2): the RFC's own reason for scoping this to
      // text size 100% is that every font scales with zoom elsewhere -
      // found NOT true live (hit area, just above): a fixed-px
      // `font-size` reads the same raw value at every text size, same
      // as a fixed-px control's own box. Kept scoped to 100% anyway,
      // exactly as the RFC states, since this is a report-worthy
      // discrepancy in its own reasoning, not licence to re-decide the
      // check's scope here.
      const ownTxt = ownText(el);
      if (Math.abs(zoom - 1) < 0.001 && ownTxt.length > 0 &&
          !matchesAny(el, textSizeAllowlist)) {{
        const fontPx = parseFloat(cs.fontSize);
        if (fontPx < 12) {{
          small_text.push({{
            element: describe(el, i), text: ownTxt.slice(0, 80),
            detail: 'font-size ' + fontPx + 'px, needs >= 12px',
          }});
        }}
      }}

      // Rendered contrast (RFC-098 §2): every element with its own
      // direct text, against its effective (composited) background.
      // Review 221 §3: `.diff-mark` ("−"/"+" on its own hunk-row
      // background) is a mark, not text - `xtask::contrast`'s own,
      // already-reviewed classification for this exact use
      // (`RENDERED_CONTRAST_MARK_SELECTOR`'s own doc comment has the
      // full reason), so it's held to that lower floor here too,
      // instead of the text one every other own-text element gets.
      if (ownTxt.length > 0 && !matchesAny(el, renderedContrastAllowlist)) {{
        const fg = parseRgba(cs.color);
        if (fg && fg[3] > 0.01) {{
          const bg = effectiveBackground(el);
          const ratio = contrastRatio([fg[0], fg[1], fg[2]], bg);
          const isMark = matchesAny(el, renderedContrastMarkSelector);
          const required = isMark ? requiredMarkContrast : requiredTextContrast;
          if (ratio < required) {{
            rendered_contrast.push({{
              element: describe(el, i), text: ownTxt.slice(0, 80),
              detail: 'contrast ' + ratio.toFixed(2) + ':1 against rgb(' +
                bg.map((c) => Math.round(c)).join(',') + '), needs ' +
                required + ':1',
            }});
          }}
        }}
      }}

      // Gathered for label placement, below - every region named by
      // `data-layout-labels` (review 220 §2.4: labelled *regions*, like
      // the diff columns `diff.rs` marks this way - review 220 §4 -
      // not a form field's own `aria-labelledby`, which this used to
      // scan for generically and wrongly ran this geometry check on
      // the ignore-list inputs in Settings instead). Each region names
      // every candidate label it could have, space-separated, one per
      // layout (`diff.rs`'s own doc comment there has the full reason
      // this is a data attribute and not `aria-labelledby`); the
      // candidate with a non-zero box right now is the layout's actual,
      // visible label.
      const layoutLabelsAttr = el.getAttribute('data-layout-labels');
      if (layoutLabelsAttr) {{
        for (const candidateId of layoutLabelsAttr.split(/\s+/).filter(Boolean)) {{
          const candidate = document.getElementById(candidateId);
          if (!candidate) {{ continue; }}
          const cr = candidate.getBoundingClientRect();
          if (cr.width > 0 && cr.height > 0) {{
            labelled.push({{ el, index: i, labelEl: candidate }});
            break;
          }}
        }}
      }}
    }}

    // Label placement (review 216 §3): a labelled region's own visible
    // label must overlap it horizontally, sit at or above its top, and
    // have no other labelled region's box lying between them.
    const label_placement = [];
    for (const {{ el, index, labelEl }} of labelled) {{
      const regionRect = el.getBoundingClientRect();
      const labelRect = labelEl.getBoundingClientRect();
      const horizontalOverlap = labelRect.left < regionRect.right &&
        labelRect.right > regionRect.left;
      const aboveOrAtTop = labelRect.top <= regionRect.top + 1;
      let reason = null;
      if (!horizontalOverlap) {{ reason = 'label has no horizontal overlap with its region'; }}
      else if (!aboveOrAtTop) {{ reason = 'label sits below the region it names'; }}
      else {{
        for (const other of labelled) {{
          if (other.el === el) {{ continue; }}
          const otherRect = other.el.getBoundingClientRect();
          if (otherRect.top >= labelRect.bottom && otherRect.bottom <= regionRect.top) {{
            reason = 'another labelled region (' + describe(other.el, other.index) +
              ') sits between the label and its own region';
            break;
          }}
        }}
      }}
      if (reason) {{
        label_placement.push({{ element: describe(el, index), text: ownText(labelEl).slice(0, 80), detail: reason }});
      }}
    }}

    // Review 221 §2.2: F224's own property, checked directly - with a
    // dialog open, its border box lies inside the viewport - rather
    // than through the generic scroll-vs-client overflow check above,
    // which `.modal`'s own flex-item `min-height: auto` default (a
    // documented CSS quirk: a flex child's `overflow-y: auto` clips
    // its own *rendered* box correctly, confirmed live - `.modal`'s
    // own clientHeight sits right at its max-height - but the flex
    // parent's layout still accounts for the item's full, unclipped
    // intrinsic content size) made an unreliable way to catch this one
    // specific shape: `.scrim`'s own scrollHeight reflected that
    // intrinsic size, not anything actually painted outside either
    // box, and a named allowance for `.scrim` would have exempted the
    // one thing this is for. Named directly, on the dialog's own box
    // against the viewport, it has no such false-positive path, and
    // reuses `Finding`'s own scroll/client shape (the edge position
    // against the viewport's own size on each axis, not a scroll
    // size) rather than adding a new result field for one check.
    const dialogForOverflow = document.querySelector('.scrim .modal');
    if (dialogForOverflow) {{
      const dr = dialogForOverflow.getBoundingClientRect();
      if (dr.top < 0 || dr.bottom > vh || dr.left < 0 || dr.right > vw) {{
        overflow.push({{
          element: describe(dialogForOverflow, -1), text: '',
          scroll_width: dr.right, client_width: vw,
          scroll_height: dr.bottom, client_height: vh,
        }});
      }}
    }}

    // Review 217/218/219: nothing covers an open dialog - a grid of
    // points sampled inside the dialog's own box must each hit the
    // dialog or a descendant, except `.toast` (the app's only error
    // notice, transient and dismissed by a click, review 218 §2).
    const dialog_occlusion = [];
    const dialogEl = document.querySelector('.scrim .modal');
    if (dialogEl) {{
      const r = dialogEl.getBoundingClientRect();
      const steps = 5;
      for (let gx = 1; gx < steps; gx++) {{
        for (let gy = 1; gy < steps; gy++) {{
          const px = r.left + (r.width * gx) / steps;
          const py = r.top + (r.height * gy) / steps;
          if (px < 0 || py < 0 || px >= vw || py >= vh) {{ continue; }}
          const hit = document.elementFromPoint(px, py);
          if (!hit) {{ continue; }}
          const isDialog = hit === dialogEl || dialogEl.contains(hit);
          const isToast = hit.closest && hit.closest('.toast') !== null;
          if (!isDialog && !isToast) {{
            dialog_occlusion.push({{
              element: 'point(' + Math.round(px) + ',' + Math.round(py) + ')',
              text: '', detail: 'hit ' + describe(hit, -1) + ' instead of the dialog',
            }});
          }}
        }}
      }}
    }}

    // Overlap (review 218 §4): two visible boxes, each with their own
    // text or interactive, neither containing the other, in the same
    // layer (review 220 §2.2), intersecting by more than 1px in both
    // axes. Scoped to the same candidates `small_text`/
    // `rendered_contrast` already walk past (own text) plus the
    // interactive set, not every element in the tree - a defect here
    // is about two *contentful* boxes crossing, not any two boxes at
    // all.
    const overlapCandidates = [];
    for (let i = 0; i < all.length; i++) {{
      const el = all[i];
      if (isVisuallyHidden(el)) {{ continue; }}
      const cs = getComputedStyle(el);
      const hasOwnText = ownText(el).length > 0;
      const isInteractive = el.matches('{interactive_selector}') && isReachable(el);
      if (!hasOwnText && !isInteractive) {{ continue; }}
      // Review 220 §2.1: clipped out by its own scroll container (any
      // `overflow` but `visible`, on either axis, not just the
      // `auto`/`scroll` a user can actually scroll - `hidden` clips
      // exactly the same way) doesn't count, the same reasoning
      // off-screen and hit area now share (`visibleRect`, above) -
      // visible only within every clipping ancestor's own box at once,
      // not just the nearest one. Only `fixed` escapes this
      // (unconditionally viewport-relative); `absolute`'s own
      // containing block is often a local, scrollable ancestor (review
      // 220 §2.3's own `input.profile-radio` finding), same reasoning
      // as off-screen/hit area just above.
      const rect = cs.position !== 'fixed' ? visibleRect(el) : el.getBoundingClientRect();
      if (isClippedAway(rect)) {{ continue; }}
      // Found live (first full-matrix run), fixed differently since
      // (review 221 §2.3): with a dialog open, the background view
      // behind its scrim used to still have every element "in the DOM
      // tree with overlapping coordinates" - the Explorer's own tree,
      // say, under the Settings dialog - but none of it was actually
      // *visible*, by design. A per-candidate `.scrim`-containment
      // check here (narrowed, after an earlier, broader form wrongly
      // excluded genuinely overlapping pairs too) is now redundant:
      // `root` itself is the dialog's own subtree whenever one is
      // open, so the background is never in `all` to begin with.
      overlapCandidates.push({{ el, index: i, rect, layer: layerOf(el) }});
    }}
    const overlap = [];
    for (let a = 0; a < overlapCandidates.length; a++) {{
      for (let b = a + 1; b < overlapCandidates.length; b++) {{
        const ca = overlapCandidates[a];
        const cb = overlapCandidates[b];
        // Review 220 §2.2: only within one layer - crossing layers
        // (a sticky/fixed/absolute box's own contents against normal
        // flow, or against a different such box) is by design, not
        // this check's business.
        if (ca.layer !== cb.layer) {{ continue; }}
        if (ca.el.contains(cb.el) || cb.el.contains(ca.el)) {{ continue; }}
        const ra = ca.rect;
        const rb = cb.rect;
        const xOverlap = Math.min(ra.right, rb.right) - Math.max(ra.left, rb.left);
        const yOverlap = Math.min(ra.bottom, rb.bottom) - Math.max(ra.top, rb.top);
        if (xOverlap > 1 && yOverlap > 1) {{
          overlap.push({{
            element: describe(ca.el, ca.index),
            text: ownText(ca.el).slice(0, 80),
            detail: 'overlaps ' + describe(cb.el, cb.index) + ' by ' +
              Math.round(xOverlap) + 'x' + Math.round(yOverlap) + 'px',
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
    dioxus.send(JSON.stringify({{
      settled, settle_ms, overflow, clipped, off_screen, hit_area, small_text,
      rendered_contrast, label_placement, dialog_occlusion, overlap, toast_check,
    }}));
    }} catch (e) {{
      dioxus.send(JSON.stringify({{
        settled: false, settle_ms: 0, overflow: [], clipped: [],
        error: 'measurement script threw: ' + (e && e.stack ? e.stack : String(e)),
      }}));
    }}
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
        // Stage B's own axes, left out of this plan entirely: each
        // defaults to the single value stage A's own matrix always ran
        // at, so an old plan (this one's own shape) still measures
        // exactly what it always did.
        assert_eq!(plan.themes, vec!["light".to_string()]);
        assert_eq!(plan.contrasts, vec!["standard".to_string()]);
        assert_eq!(plan.colour_visions, vec!["standard".to_string()]);
    }

    #[test]
    fn stage_b_view_kinds_deserialize_and_name_themselves() {
        let raw = r#"{
            "results_path": "/tmp/results.jsonl",
            "views": [
                {"kind": "explorer"},
                {"kind": "deep-compare", "left": "/l", "right": "/r"},
                {"kind": "merge-tool", "local": "/l", "remote": "/rm", "merged": "/m"},
                {"kind": "shortcuts"}
            ],
            "text_sizes": [100],
            "languages": ["en"]
        }"#;
        let plan: Plan = serde_json::from_str(raw).expect("plan must parse");
        let names: Vec<&str> = plan.views.iter().map(ViewPlan::name).collect();
        assert_eq!(
            names,
            vec!["explorer", "deep-compare", "mergetool", "shortcuts"]
        );
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
        let script = measurement_script(".diff-wrap", 4.5, 3.0, 1.0, None);
        assert!(script.contains("querySelector('.diff-wrap')"));
    }

    #[test]
    fn parse_theme_defaults_to_light_for_an_unrecognized_value() {
        assert_eq!(parse_theme("dark"), Theme::Dark);
        assert_eq!(parse_theme("light"), Theme::Light);
        assert_eq!(parse_theme("night"), Theme::Night);
        assert_eq!(parse_theme("system"), Theme::System);
        assert_eq!(parse_theme("xx"), Theme::Light);
    }

    #[test]
    fn json_string_array_renders_a_valid_js_array_literal() {
        assert_eq!(json_string_array(&[]), "[]");
        assert_eq!(json_string_array(&["a"]), "[\"a\"]");
        assert_eq!(json_string_array(&["a", "b"]), "[\"a\", \"b\"]");
    }

    /// Review 098 §2's own "Allowed by" column, for overflow/clipped:
    /// "the known scroll owners... named in a list in the probe" - stage
    /// A applied this generically (`isScrollOwner`'s own computed-style
    /// check) and said as much in its own doc comment; `.tablist` is the
    /// first named entry, found live when its own responsive `overflow:
    /// visible` (a deliberate spill, nothing lost) first surfaced as a
    /// false overflow finding under the stacked layout. A `.scrim` entry
    /// (reply 189b) was refused by review 221 §2.2 - it would have
    /// exempted the exact regression F224 is about, not just a false
    /// positive - so it's checked by name as *absent*, not present:
    /// `.tablist` alone, so a future edit that quietly re-adds an
    /// allowance for the dialog backdrop is caught too.
    #[test]
    fn tablist_is_the_overflow_allowlists_only_named_entry() {
        assert_eq!(OVERFLOW_ALLOWLIST, &[".tablist"]);
    }

    /// Review 219's own round found exactly this shape live: an allowlist
    /// entry containing a double quote (`[aria-disabled="true"]`)
    /// produced `"[aria-disabled="true"]"` once wrapped by
    /// `json_string_array` for the measurement script's own JS source —
    /// invalid syntax, which made the *whole* `document::eval` never
    /// call `dioxus.send` at all, hanging `eval.recv()` forever (no
    /// timeout of its own) rather than failing visibly. Falsify by
    /// reverting `RENDERED_CONTRAST_ALLOWLIST`'s own single quotes back
    /// to double: this test catches the shape directly, instead of
    /// waiting to find it again as a silent hang.
    #[test]
    fn no_allowlist_entry_contains_a_double_quote() {
        for list in [
            OVERFLOW_ALLOWLIST,
            HIT_AREA_ALLOWLIST,
            TEXT_SIZE_ALLOWLIST,
            RENDERED_CONTRAST_ALLOWLIST,
        ] {
            for entry in list {
                assert!(
                    !entry.contains('"'),
                    "{entry:?} has a double quote, which collides with \
                     json_string_array's own wrapping quotes once embedded \
                     in the measurement script - use single quotes for an \
                     attribute selector's value instead"
                );
            }
        }
    }

    #[test]
    fn measurement_script_checks_the_toast_with_element_from_point() {
        let script = measurement_script(".diff-wrap", 4.5, 3.0, 1.0, None);
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
