//! Dir-pane building blocks used by the Explorer aligned view (RFC-054).
//!
//! This module provides the types, helper functions, and leaf components
//! (`PathBar`, `TreeRow`) used by `explorer.rs`.  The `DirPane` monolith that
//! managed its own tree state is gone; tree ownership now lives in Explorer so
//! both panes can be rendered in an aligned structure.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;
use dioxus_core::spawn_forever;

use crate::i18n::t;
use crate::state::Lang;
use dioxus_swdir_tree::ThreadExecutor;
use dioxus_swdir_tree::{LoadPayload, ScanExecutor, ScanFuture, ScanJob};

use forskscope_core::IgnoreRules;
use forskscope_core::dir::EqualityEvidence;
use forskscope_ui_logic::RowStatusKind;

// ── Public types ──────────────────────────────────────────────────────────────

/// Handoff 007/RFC-009 §7: the accessible label for `kind`, in `lang` -
/// every variant must yield a non-empty string, since a bare glyph with no
/// `title` gives a screen reader nothing to announce. `RowStatusKind`'s own
/// `aria_label()` is fixed English (`ui-logic` has no i18n system); this is
/// the `Lang`-aware layer above it, replacing `dir_pane.rs`'s old
/// `status_label` over the now-deleted `DigestState`.
///
/// Review 077 §4b: `LeftOnly`/`RightOnly` render distinct glyphs (`←`/`→`)
/// but used to share one label ("Only on this side") - not a regression
/// (the old `·` glyph never distinguished direction either), but a newly
/// opened gap between what a sighted user sees and what a screen reader
/// announces, which RFC-009 §7 exists to close. Distinct labels now.
///
/// RFC-080 §4: the same state carries **different words on the two row kinds**,
/// because the evidence differs — a file pair is already matched by name, so only
/// its size was checked; a directory pair had names and sizes compared. Hence
/// `is_dir`. Only `MetadataMatch` reads it.
pub(crate) fn status_kind_label(kind: RowStatusKind, lang: Lang, is_dir: bool) -> String {
    match kind {
        // RFC-080 §3/§4 (handoff 060, settled, not to be re-opened): the same
        // wording on both row kinds — a file reaches this only via a real
        // digest read (`DigestEqual`), a directory only via tier 2
        // (`TreeIdentical`), so both have equal claim to the longer form.
        RowStatusKind::Equal => t(lang, "Identical — contents compared"),
        RowStatusKind::Different => t(lang, "Different"),
        RowStatusKind::LeftOnly => t(lang, "Only on the left"),
        RowStatusKind::RightOnly => t(lang, "Only on the right"),
        RowStatusKind::Computing => t(lang, "Comparing…"),
        RowStatusKind::Error => t(lang, "Comparison failed"),
        RowStatusKind::NotCompared => t(lang, "Directory contents not compared — use Deep Compare"),
        RowStatusKind::MetadataMatch if is_dir => {
            t(lang, "Names and sizes match; contents not compared")
        }
        RowStatusKind::MetadataMatch => t(lang, "Size matches; contents not compared"),
    }
}

/// Per-pane navigation history (back/forward).
#[derive(Clone, Default)]
pub struct NavHistory {
    pub entries: Vec<PathBuf>,
    pub idx: usize,
}

impl NavHistory {
    pub fn push(&mut self, path: PathBuf) {
        if self.entries.last().map(|p| p == &path).unwrap_or(false) {
            return;
        }
        self.entries.truncate(self.idx + 1);
        self.entries.push(path);
        self.idx = self.entries.len() - 1;
    }
    pub fn can_back(&self) -> bool {
        self.idx > 0
    }
    pub fn can_forward(&self) -> bool {
        self.idx + 1 < self.entries.len()
    }
    pub fn back(&mut self) -> Option<PathBuf> {
        if self.can_back() {
            self.idx -= 1;
            Some(self.entries[self.idx].clone())
        } else {
            None
        }
    }
    pub fn forward(&mut self) -> Option<PathBuf> {
        if self.can_forward() {
            self.idx += 1;
            Some(self.entries[self.idx].clone())
        } else {
            None
        }
    }
}

// ── Filtering executor ────────────────────────────────────────────────────────

/// The ignore rules a running Explorer scans with (F112).
///
/// A handle, not a snapshot. `use_scan_driver` keeps the executor it is given
/// at mount and drops every later one, so an executor that *owned* the rules
/// filtered with whatever they were when the tab opened, whatever the settings
/// said afterwards. The Explorer updates this handle when the settings change,
/// and the executor reads it each time a scan is requested. Cloning shares the
/// same rules; the lock is held only to copy them.
#[derive(Clone, Default)]
pub struct SharedIgnoreRules(Arc<RwLock<IgnoreRules>>);

impl SharedIgnoreRules {
    pub fn new(rules: IgnoreRules) -> Self {
        Self(Arc::new(RwLock::new(rules)))
    }

    /// The rules in force now.
    pub fn get(&self) -> IgnoreRules {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Replace the rules; the next scan request sees them.
    pub fn set(&self, rules: IgnoreRules) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = rules;
    }
}

pub struct FilteringExecutor {
    pub rules: SharedIgnoreRules,
}

impl ScanExecutor for FilteringExecutor {
    fn spawn_blocking(&self, job: ScanJob) -> ScanFuture {
        // Read at the moment the scan is requested, not when the executor was
        // built: this is what makes a settings change reach a running tree.
        let rules = self.rules.get();
        let f: ScanJob = Box::new(move || {
            let mut p: LoadPayload = job();
            if !rules.is_empty()
                && let Ok(ref mut entries) = p.result
            {
                entries.retain(|e| {
                    let name = e.file_name().to_str().unwrap_or("");
                    if e.is_dir {
                        !rules.is_dir_ignored(name)
                    } else {
                        !rules.is_file_ignored(name)
                    }
                });
            }
            p
        });
        ThreadExecutor.spawn_blocking(f)
    }
}

// ── PathBar component ─────────────────────────────────────────────────────────

/// Full-featured path navigation bar.
///
/// Layout (single row, never wraps):
/// `← → ↑ │ /path/segments/current │ ⋯`
///
/// F185 (handoff 074): ⌂, 📁 and ✎ no longer sit in the bar as their own
/// buttons — they live in the **⋯** menu (`Home folder`, `Open folder…`,
/// `Edit path`), a small WAI-ARIA menu button local to this component (not
/// a reusable framework — #111's context menus are a separate decision).
/// Clicking the current folder name still opens the path editor directly,
/// unchanged.
///
/// The breadcrumb uses `direction: rtl` via CSS so that when the path is too
/// long for the available space, the LEADING segments (ancestors) overflow
/// invisibly to the left while the CURRENT directory stays visible on the right.
#[component]
pub fn PathBar(
    path: PathBuf,
    can_back: bool,
    can_forward: bool,
    on_back: EventHandler<()>,
    on_forward: EventHandler<()>,
    on_navigate: EventHandler<PathBuf>,
    lang: Lang,
    /// Renders the accent underline/tint (RFC-061 amendment, F185): this
    /// bar replaces the removed pane-root cell as the focus indicator.
    is_focused: bool,
    /// Clicking anywhere on this bar — background, buttons, or a path
    /// segment — focuses this pane. F6 still switches (unchanged).
    on_focus: EventHandler<()>,
    /// This pane's side name ("Left pane" / "Right pane"), already
    /// translated by the caller. Combined with the current folder's short
    /// name, this is the bar's own accessible group name — the job the
    /// removed `role="heading"` cell used to do, kept current as the pane
    /// navigates because it is computed from `path` on every render.
    side_label: String,
) -> Element {
    // Pre-compute everything before closures consume values.
    let segs = path_segs(&path);
    let n = segs.len();
    let path_str = path.display().to_string();
    let path_str_reset = path_str.clone();
    let path_str_blur = path_str.clone();
    let group_label = format!("{side_label} — {}", short_name(&path));

    let mut edit_mode: Signal<bool> = use_signal(|| false);
    let mut input_val: Signal<String> = use_signal(|| path_str.clone());
    let mut input_err: Signal<bool> = use_signal(|| false);
    let mut menu_open: Signal<bool> = use_signal(|| false);

    // F185 §2.1's menu: each focusable target's own `MountedData` handle,
    // captured via its `onmounted` (the same mechanism
    // `ui/overlay/modals.rs`'s `focus_autofocus_button` uses, there via a
    // raw `document::eval` lookup because it only ever needs *today's*
    // autofocused element; here every target is reused across opens, so
    // each keeps its own handle). `set_focus` on a handle works
    // immediately for an already-mounted target — which covers ↑/↓ moving
    // onto any of the three items, and Esc returning to ⋯ — *except* the
    // one case where the target is being created in the very same render
    // pass that is opening the menu (item 0, the moment the menu first
    // opens): calling `set_focus` on a handle captured before that render
    // cannot win a race against the mount it is racing, and silently does
    // nothing when it loses (confirmed live, AT-SPI, while building this).
    // Item 0's `onmounted` therefore also focuses *itself*, which has no
    // such race — by the time an element's own `onmounted` fires, it is
    // already in the DOM.
    let mut more_btn_handle: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut item_handles: Signal<[Option<Rc<MountedData>>; 3]> = use_signal(|| [None, None, None]);

    use_effect(move || {
        if !*edit_mode.read() {
            input_val.set(path_str.clone());
        }
    });

    let home_item = move |_| {
        on_focus.call(());
        on_navigate.call(home_dir());
        menu_open.set(false);
        focus_handle(more_btn_handle.read().clone());
    };
    let open_folder_item = move |_| {
        on_focus.call(());
        menu_open.set(false);
        focus_handle(more_btn_handle.read().clone());
        let nav = on_navigate;
        spawn(async move {
            let r = tokio::task::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
                .await
                .ok()
                .flatten();
            if let Some(p) = r {
                nav.call(p);
            }
        });
    };
    let edit_path_item = move |_| {
        on_focus.call(());
        menu_open.set(false);
        edit_mode.set(true);
    };

    rsx! {
        div { class: "pathbar-wrapper",
            div {
                class: if is_focused { "path-bar pane-focused" } else { "path-bar" },
                role: "group",
                aria_label: "{group_label}",
                onclick: move |_| on_focus.call(()),
                button { class: "path-btn", title: t(lang, "Back"), aria_label: t(lang, "Back"), disabled: !can_back,    onclick: move |_| on_back.call(()),    "←" }
                button { class: "path-btn", title: t(lang, "Forward"), aria_label: t(lang, "Forward"), disabled: !can_forward, onclick: move |_| on_forward.call(()), "→" }
                button { class: "path-btn", title: t(lang, "Go up one directory"), aria_label: t(lang, "Go up one directory"),
                    onclick: move |_| {
                        let p = path.parent().map(|p| p.to_path_buf());
                        if let Some(p) = p { on_navigate.call(p); }
                    }, "↑" }

                div { class: "path-segments",
                    if *edit_mode.read() {
                        input {
                            class: if *input_err.read() { "path-input error" } else { "path-input" },
                            r#type: "text", value: "{input_val}", autofocus: true,
                            oninput:  move |e| { input_val.set(e.value()); input_err.set(false); },
                            onkeydown: move |e| path_input_keydown(
                                &e, edit_mode, input_val, input_err, on_navigate, &path_str_reset,
                            ),
                            onblur: move |_| {
                                let v = PathBuf::from(input_val.read().cloned());
                                if v.is_dir() { edit_mode.set(false); on_navigate.call(v); }
                                else { input_val.set(path_str_blur.clone()); edit_mode.set(false); input_err.set(false); }
                            },
                        }
                    } else {
                        // Root prefix icon — navigates to filesystem root on click.
                        if path.has_root() {
                            { let root = PathBuf::from("/");
                              rsx! { button { class: "bc-root", title: "/",
                                  onclick: move |_| on_navigate.call(root.clone()),
                                  "/" } } }
                            if n > 0 { span { class: "bc-sep", " " } }
                        }
                        for (idx, (seg_path, label)) in segs.iter().enumerate() {
                            if idx > 0 { span { class: "bc-sep", " / " } }
                            if idx == n - 1 {
                                span { class: "bc-current", title: "{label}",
                                    onclick: move |_| edit_mode.set(true),
                                    {trunc_label(label, 20)} }
                            } else {
                                { let t = seg_path.clone(); let full = label.clone();
                                  rsx! { button { class: "bc-seg", title: "{full}",
                                      onclick: move |_| on_navigate.call(t.clone()),
                                      {trunc_label(&full, 20)} } } }
                            }
                        }
                    }
                }

                button {
                    class: "path-btn path-more-btn",
                    title: t(lang, "More"), aria_label: t(lang, "More"),
                    aria_haspopup: "menu",
                    aria_expanded: if *menu_open.read() { "true" } else { "false" },
                    onmounted: move |e| more_btn_handle.set(Some(e.data())),
                    onclick: move |_| {
                        if *menu_open.read() {
                            menu_open.set(false);
                        } else {
                            open_pathbar_menu(menu_open);
                        }
                    },
                    onkeydown: move |e| more_btn_keydown(&e, menu_open),
                    "⋯"
                }
            }
            if *menu_open.read() {
                div { class: "pathbar-menu-catcher", onclick: move |_| menu_open.set(false) }
                div { class: "pathbar-menu", role: "menu", aria_label: t(lang, "More"),
                    button {
                        role: "menuitem", class: "pathbar-menu-item",
                        onmounted: move |e| {
                            let h = e.data();
                            item_handles.write()[0] = Some(h.clone());
                            // The only item that must also focus itself the
                            // instant it exists — see the handle-field doc
                            // comment above.
                            focus_handle(Some(h));
                        },
                        onkeydown: move |e| { menu_item_keydown(&e, 0, menu_open, item_handles, more_btn_handle); },
                        onclick: home_item,
                        span { class: "pathbar-menu-label", {t(lang, "Home folder")} }
                        span { class: "pathbar-menu-shortcut", "Alt+Home" }
                    }
                    button {
                        role: "menuitem", class: "pathbar-menu-item",
                        onmounted: move |e| item_handles.write()[1] = Some(e.data()),
                        onkeydown: move |e| { menu_item_keydown(&e, 1, menu_open, item_handles, more_btn_handle); },
                        onclick: open_folder_item,
                        span { class: "pathbar-menu-label", {t(lang, "Open folder…")} }
                        span { class: "pathbar-menu-shortcut", "Ctrl+O" }
                    }
                    button {
                        role: "menuitem", class: "pathbar-menu-item",
                        onmounted: move |e| item_handles.write()[2] = Some(e.data()),
                        onkeydown: move |e| { menu_item_keydown(&e, 2, menu_open, item_handles, more_btn_handle); },
                        onclick: edit_path_item,
                        span { class: "pathbar-menu-label", {t(lang, "Edit path")} }
                        span { class: "pathbar-menu-shortcut", {t(lang, "click the name")} }
                    }
                }
            }
        }
    }
}

// ── ⋯ menu: keyboard model (F185, handoff 074 §2.1) ────────────────────────────
//
// A small WAI-ARIA menu button, local to `PathBar` — not a framework for
// #111's future context menus. Enter/Space on ⋯ open it via the native
// button click the browser already fires; only ↓ needs its own keydown
// case, since arrow keys are not native button-activation keys.

/// `.set_focus(true)` on an already-mounted target's handle. A no-op if the
/// handle has not been captured yet (should not happen for any of the
/// three targets this is called with, each captured well before any key
/// that reaches this function can fire — but a silent no-op is still the
/// right failure mode over a panic for a focus call, which is advisory).
fn focus_handle(handle: Option<Rc<MountedData>>) {
    if let Some(h) = handle {
        spawn(async move {
            let _ = h.set_focus(true).await;
        });
    }
}

/// Opens the menu. Item 0 focuses itself on mount (see the handle-field
/// doc comment on `PathBar`); this function only ever needs to flip the
/// one signal that makes that mount happen — the ⋯ button's onclick
/// (mouse, or Enter/Space via native button activation) and its onkeydown
/// (↓) both call this one function, so "opens with focus on the first
/// item" is asserted once, against what both input paths actually run.
fn open_pathbar_menu(mut menu_open: Signal<bool>) {
    menu_open.set(true);
}

/// The ⋯ button's own keydown: ↓ opens the menu (Enter/Space already reach
/// `onclick` natively via the browser's own button-activation behaviour, so
/// they need no case here — and `ControlZone`'s `controls_keep_enter`
/// already keeps Enter from reaching the global handler besides). Only ↓ is
/// stopped: this is a plain button, not a text field, so every other key
/// (Tab included) keeps its ordinary behaviour.
fn more_btn_keydown(e: &Event<KeyboardData>, menu_open: Signal<bool>) {
    if e.key() == Key::ArrowDown {
        e.stop_propagation();
        open_pathbar_menu(menu_open);
    }
}

/// A menu item's keydown. Swallows every key first (RFC-060/`swallow_when_typing`'s
/// own rule, restated by the handoff as "keys in the open menu reach
/// nothing behind it") — arrows must not move the tree's row focus, and
/// Enter must not open a row underneath. `own_idx` is this item's own
/// position in `item_handles` (all three items' handles, captured by all
/// three — every item can be wrapped onto from any other, so each needs
/// every handle, not just its immediate neighbours'). `Esc` closes the
/// menu and returns focus to ⋯. Enter/Space activate via the item's own
/// native `onclick`, which closes the menu itself (see each item's closure
/// in `PathBar`).
///
/// Returns the index arrow navigation moved to, so a test can assert the
/// wrap-around math directly against this function's own return value —
/// no signal needs to exist purely to make that observable.
fn menu_item_keydown(
    e: &Event<KeyboardData>,
    own_idx: usize,
    mut menu_open: Signal<bool>,
    item_handles: Signal<[Option<Rc<MountedData>>; 3]>,
    more_btn_handle: Signal<Option<Rc<MountedData>>>,
) -> Option<usize> {
    crate::keyboard::swallow_when_typing(e);
    const LEN: usize = 3;
    match e.key() {
        Key::ArrowDown => {
            let next = (own_idx + 1) % LEN;
            focus_handle(item_handles.read()[next].clone());
            Some(next)
        }
        Key::ArrowUp => {
            let next = (own_idx + LEN - 1) % LEN;
            focus_handle(item_handles.read()[next].clone());
            Some(next)
        }
        Key::Escape => {
            menu_open.set(false);
            focus_handle(more_btn_handle.read().clone());
            None
        }
        // Review 156: Tab (and Shift+Tab, the same key value) must close the
        // menu and let focus move on natively, as the WAI-ARIA menu pattern
        // does — never `prevent_default`, or the browser's own Tab-order
        // traversal never runs and the `.pathbar-menu-catcher` is left
        // covering the window, silently absorbing the user's next click.
        Key::Tab => {
            menu_open.set(false);
            None
        }
        _ => None,
    }
}

/// Review 092 §2: `PathBar`'s `onkeydown` calls this exact function, so a
/// falsification against it (deleting the `swallow_when_typing` call) tests
/// the shipped path, not a copy. RFC-060/handoff 020 §5: swallow every key
/// first, not just Enter/Escape — a Ctrl+S typed while editing this path
/// must not save the active tab behind it either.
fn path_input_keydown(
    e: &Event<KeyboardData>,
    mut edit_mode: Signal<bool>,
    mut input_val: Signal<String>,
    mut input_err: Signal<bool>,
    on_navigate: EventHandler<PathBuf>,
    path_str_reset: &str,
) {
    crate::keyboard::swallow_when_typing(e);
    if e.key() == Key::Enter {
        let v = PathBuf::from(input_val.read().cloned());
        if v.is_dir() {
            edit_mode.set(false);
            on_navigate.call(v);
        } else {
            input_err.set(true);
        }
    }
    if e.key() == Key::Escape {
        input_val.set(path_str_reset.to_string());
        edit_mode.set(false);
        input_err.set(false);
    }
}

// ── TreeRow component ─────────────────────────────────────────────────────────

#[component]
pub fn TreeRow(
    lang: Lang,
    path: PathBuf,
    is_dir: bool,
    is_expanded: bool,
    is_selected: bool,
    depth: u32,
    status: Option<EqualityEvidence>,
    /// `true` when this file was sniffed as binary (RFC-066).
    #[props(default = false)]
    is_binary: bool,
    /// `true` when binary comparison is enabled in Settings (RFC-066).
    #[props(default = true)]
    binary_enabled: bool,
    on_toggle: EventHandler<()>,
    on_select: EventHandler<()>,
    on_dblclick: EventHandler<()>,
    /// RFC-080 tier 2 (handoff 060): starts a tier-2 verify for this row.
    /// Only called from the "Verify" button, which renders only at the
    /// tier-1 match state (`RowStatusKind::MetadataMatch`) — see §3's
    /// placement rule in `start_tier2_verify`'s caller.
    on_verify: EventHandler<()>,
) -> Element {
    let indent = depth * 16;
    let caret = if !is_dir {
        "\u{00A0}"
    } else if is_expanded {
        "▾"
    } else {
        "▸"
    };
    let icon = if is_dir { "📁" } else { "📄" };
    let name = path
        .file_name()
        .unwrap_or(OsStr::new(""))
        .to_string_lossy()
        .into_owned();

    // Binary badge / disabled treatment (RFC-066).
    let binary_blocked = is_binary && !is_dir && !binary_enabled;
    let rc = if binary_blocked {
        if is_selected {
            "tree-row selected binary-blocked"
        } else {
            "tree-row binary-blocked"
        }
    } else if is_selected {
        "tree-row selected"
    } else {
        "tree-row"
    };

    // F74/handoff 007: every status carries an accessible label, routed
    // through `t(lang, …)` like the `bin` badge three lines below -
    // RFC-009 §7 forbids status by styling/glyph alone, and a bare glyph
    // with no `title` gave a screen reader nothing to announce. Rendered
    // through `RowStatusKind` (core's `EqualityEvidence`, mapped by
    // `ui-logic`) rather than the deleted ad-hoc `DigestState` - its
    // `glyph()`/`css_class()` are directly unit-tested in `status.rs`;
    // `status_kind_label` above is the `Lang`-aware layer this file still
    // owns.
    let status_kind = status.as_ref().map(RowStatusKind::from_evidence);
    rsx! {
        div {
            class: "{rc}", role: "row", style: "padding-left: {indent}px",
            onclick:       move |_| { if !binary_blocked { on_select.call(()); } },
            ondoubleclick: move |_| { if !binary_blocked { on_dblclick.call(()); } },
            span { class: "tree-caret",
                onclick: move |e| { e.stop_propagation(); on_toggle.call(()); }, "{caret}" }
            span { class: "tree-icon",  "{icon}" }
            span { class: "tree-label", "{name}" }
            if binary_blocked {
                // F74 review 072 §3: `title` on a bare `span` (role
                // `generic`) is not reliably surfaced as accessible text -
                // a screen reader announces the text content ("bin")
                // instead. `role: "img"` exposes the span as a named node
                // whose `aria_label` replaces that content, which is what
                // actually substitutes the label. `title` kept for the
                // mouse tooltip.
                span {
                    class: "tree-status st-binary",
                    role: "img",
                    aria_label: t(lang, "Binary file. Binary comparison is off — enable it in Settings → Advanced."),
                    title: t(lang, "Binary file. Binary comparison is off — enable it in Settings → Advanced."),
                    "bin"
                }
            } else if let Some(kind) = status_kind {
                {
                    let label = status_kind_label(kind, lang, is_dir);
                    let cls = kind.css_class();
                    let glyph = kind.glyph();
                    rsx! {
                        span {
                            class: "tree-status {cls}",
                            role: "img",
                            aria_label: "{label}",
                            title: "{label}",
                            "{glyph}"
                        }
                    }
                }
            }
            // RFC-080 tier 2 (handoff 060 §3): offered only on a row already
            // at the tier-1 match state — a row proven `Different` needs no
            // verification, and a row at `NotCompared` has no tier-1 result
            // to escalate from. `stop_propagation` so the click starts a
            // verify rather than also firing the row's own `on_select`.
            if status_kind == Some(RowStatusKind::MetadataMatch) {
                button {
                    class: "tree-verify-btn",
                    title: t(lang, "Read every byte to find out for certain."),
                    onclick: move |e| { e.stop_propagation(); on_verify.call(()); },
                    {t(lang, "Verify")}
                }
            }
        }
    }
}

// ── Helper functions (pub for explorer.rs) ────────────────────────────────────

pub fn path_segs(path: &Path) -> Vec<(PathBuf, String)> {
    let mut acc = PathBuf::new();
    path.components()
        .filter_map(|c| {
            acc.push(c);
            let lbl = match &c {
                Component::RootDir => return None, // handled by root icon prefix
                Component::Prefix(p) => p.as_os_str().to_string_lossy().into_owned(),
                Component::Normal(name) => name.to_string_lossy().into_owned(),
                Component::CurDir => ".".into(),
                Component::ParentDir => "..".into(),
            };
            Some((acc.clone(), lbl))
        })
        .collect()
}

pub fn trunc_label(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.into()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn short_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}

/// Persist the new root in settings and update current_dir - the part of a
/// navigation that is the same whether the path is a fresh destination or a
/// history replay. Does **not** touch `history`: pushing is only correct for
/// a fresh destination (F72 - see `navigate_to`/`navigate_to_from_history`).
fn apply_navigation(
    path: PathBuf,
    is_left: bool,
    mut store: crate::state::Store,
    mut current_dir: Signal<PathBuf>,
) {
    {
        let mut s = store.settings.write();
        // Only remember the location when the user opted in (RFC-009 explorer
        // setting). When off, the panes always start at home next launch.
        if s.remember_explorer_dirs {
            if is_left {
                s.last_left_dir = Some(path.clone());
            } else {
                s.last_right_dir = Some(path.clone());
            }
        }
    }
    // Persist so the remembered directory survives a restart. No-op effect on
    // disk content when remember_explorer_dirs is off (last_* left unchanged).
    if store.settings.read().remember_explorer_dirs {
        crate::ui::view::settings::persist(store);
    }
    current_dir.set(path);
    // Scroll the aligned tree to top after navigation. `spawn_forever`, not
    // `spawn`: this one-shot eval has no dependency on the calling scope's
    // lifetime, and `spawn` requires a "current scope" only real event
    // dispatch provides (same constraint `with_test_store`'s Store signals
    // sidestep via `ScopeId::ROOT` - F36/F61) - calling `apply_navigation`
    // outside of one, as F72's regression test does, would otherwise panic
    // on a concern this function's actual behavior does not depend on.
    // Review 070 §4.2: `spawn_forever` means this task is NOT cancelled if
    // the calling component unmounts mid-flight - harmless *here* only
    // because this eval is one-shot and completes immediately, a property
    // of this particular eval, not a guarantee `spawn_forever` gives in
    // general. Do not copy this call site's choice for a task that does
    // real, possibly-long-running work without re-checking that it stays
    // safe to outlive whatever triggered it.
    spawn_forever(async move {
        let _ = dioxus::document::eval(
            "var t = document.getElementById('explorer-tree'); if(t) t.scrollTop = 0;",
        )
        .await;
    });
}

/// Persist the new root in settings, push history, and update current_dir -
/// for a **fresh** destination (a click on a folder, a typed path, Home,
/// Up). Every navigation site except Back/Forward wants this.
pub fn navigate_to(
    path: PathBuf,
    is_left: bool,
    store: crate::state::Store,
    mut history: Signal<NavHistory>,
    current_dir: Signal<PathBuf>,
) {
    history.write().push(path.clone());
    apply_navigation(path, is_left, store, current_dir);
}

/// Re-visits a path Back/Forward already found in `history` - everything
/// `navigate_to` does except the push. F72: `path` here *is* a `history`
/// entry, so pushing it back in is not a no-op the way it looks - `push`'s
/// duplicate guard compares against `entries.last()`, which after
/// `NavHistory::back()` is still the not-yet-truncated forward entry, not
/// `path`. `push` truncates from the current index and re-appends `path`,
/// destroying whatever was ahead of it. Back/Forward navigating through
/// history must never call `push` at all, not just avoid duplicating the
/// current entry.
pub fn navigate_to_from_history(
    path: PathBuf,
    is_left: bool,
    store: crate::state::Store,
    current_dir: Signal<PathBuf>,
) {
    apply_navigation(path, is_left, store, current_dir);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::with_test_store;

    // F72: navigate A → B, press Back (expect a return to A with Forward to
    // B still available), then Forward (expect a return to B). Exercises
    // the real `navigate_to`/`navigate_to_from_history` functions, not just
    // `NavHistory` in isolation - `NavHistory` itself was never the bug;
    // `navigate_to`'s unconditional `history.write().push(...)` on every
    // call, including a Back/Forward replay, was.
    #[test]
    fn back_then_forward_returns_to_the_page_that_was_left() {
        with_test_store(|store| {
            let mut history = Signal::new_in_scope(NavHistory::default(), ScopeId::ROOT);
            let current_dir = Signal::new_in_scope(PathBuf::from("/a"), ScopeId::ROOT);
            history.write().push(PathBuf::from("/a"));

            navigate_to(PathBuf::from("/b"), true, *store, history, current_dir);
            assert_eq!(*current_dir.read(), PathBuf::from("/b"));
            assert!(!history.read().can_forward());

            let back_target = history.write().back();
            assert_eq!(back_target, Some(PathBuf::from("/a")));
            navigate_to_from_history(back_target.unwrap(), true, *store, current_dir);
            assert_eq!(*current_dir.read(), PathBuf::from("/a"));

            assert!(
                history.read().can_forward(),
                "F72: Back must not destroy the Forward entry"
            );
            let fwd_target = history.write().forward();
            assert_eq!(
                fwd_target,
                Some(PathBuf::from("/b")),
                "Forward must return to the page Back just left"
            );
        });
    }

    // F74/RFC-009 §7/handoff 007: every `RowStatusKind` variant must yield
    // a non-empty accessible label, in every supported language - a bare
    // glyph with no `title` gives a screen reader nothing to announce.
    // `RowStatusKind` itself has no `Lang` concept (`ui-logic` has no i18n
    // system); this exercises `dir_pane.rs`'s own `Lang`-aware layer over
    // it.
    #[test]
    fn every_row_status_kind_has_a_non_empty_label_in_every_language() {
        let kinds = [
            RowStatusKind::Computing,
            RowStatusKind::Equal,
            RowStatusKind::Different,
            RowStatusKind::LeftOnly,
            RowStatusKind::RightOnly,
            RowStatusKind::Error,
            RowStatusKind::NotCompared,
            RowStatusKind::MetadataMatch,
        ];
        for kind in kinds {
            for lang in [Lang::En, Lang::Ja] {
                for is_dir in [true, false] {
                    let label = status_kind_label(kind, lang, is_dir);
                    assert!(
                        !label.is_empty(),
                        "{kind:?} (dir: {is_dir}) has an empty accessible label for {lang:?}"
                    );
                }
                let label = status_kind_label(kind, lang, true);
                assert!(
                    !label.is_empty(),
                    "{kind:?} has an empty accessible label for {lang:?}"
                );
            }
        }
    }

    // RFC-080 criterion 6: the tier-1 state's wording differs by row kind (the
    // evidence differs), each is non-empty, and the Japanese differs from the
    // English so the translation is actually present. Falsify by dropping a
    // Japanese entry in `i18n.rs`: the `assert_ne!` for it fails.
    #[test]
    fn the_tier_1_match_has_kind_specific_localised_labels() {
        let en_dir = status_kind_label(RowStatusKind::MetadataMatch, Lang::En, true);
        let en_file = status_kind_label(RowStatusKind::MetadataMatch, Lang::En, false);
        let ja_dir = status_kind_label(RowStatusKind::MetadataMatch, Lang::Ja, true);
        let ja_file = status_kind_label(RowStatusKind::MetadataMatch, Lang::Ja, false);
        assert_eq!(en_dir, "Names and sizes match; contents not compared");
        assert_eq!(en_file, "Size matches; contents not compared");
        assert_ne!(en_dir, en_file);
        assert_ne!(ja_dir, ja_file);
        assert_ne!(ja_dir, en_dir);
        assert_ne!(ja_file, en_file);
        // ...and never the wording of equality.
        assert_ne!(
            en_dir,
            status_kind_label(RowStatusKind::Equal, Lang::En, true)
        );
    }

    // F74: `NotCompared` must use its own glyph/class - not silently fall
    // back to `Equal`'s (which would recreate the false-equal claim this
    // state exists to replace) or to `Computing`'s (a spinner that never
    // resolves, handoff 007 §4's first defect). The glyph/class contract
    // itself is `status.rs`'s (tested there); this confirms `dir_pane.rs`'s
    // label layer keeps `NotCompared` distinct from `Equal` too.
    #[test]
    fn not_compared_has_a_distinct_label_from_equal() {
        let label = status_kind_label(RowStatusKind::NotCompared, Lang::En, true);
        assert_ne!(
            label,
            status_kind_label(RowStatusKind::Equal, Lang::En, true)
        );
    }

    // Review 077 §4b: `LeftOnly`/`RightOnly` render distinct glyphs
    // (`←`/`→`) and must announce distinct labels too - RFC-009 §7
    // requires symbol and screen-reader text to carry the same meaning,
    // and a shared "only on this side" label for both left this direction
    // sighted-only.
    #[test]
    fn left_only_and_right_only_have_distinct_labels() {
        for lang in [Lang::En, Lang::Ja] {
            let left = status_kind_label(RowStatusKind::LeftOnly, lang, false);
            let right = status_kind_label(RowStatusKind::RightOnly, lang, false);
            assert_ne!(
                left, right,
                "LeftOnly and RightOnly must have distinct labels for {lang:?}"
            );
        }
    }

    // ── path_input_keydown (review 092 §2) ──────────────────────────────────

    /// `PathBar`'s `onkeydown` calls this exact function, so a falsification
    /// against it (deleting the `swallow_when_typing` call) tests the
    /// shipped path, not a copy.
    #[test]
    fn path_input_keydown_swallows_every_key() {
        with_test_store(|_store| {
            let edit_mode = Signal::new_in_scope(true, ScopeId::ROOT);
            let input_val = Signal::new_in_scope(String::from("/some/path"), ScopeId::ROOT);
            let input_err = Signal::new_in_scope(false, ScopeId::ROOT);
            // `Callback::new` needs an active scope on the stack, which
            // `with_test_store`'s `in_runtime` alone does not push (unlike
            // `Signal::new_in_scope`, which takes the scope explicitly) —
            // `Runtime::in_scope` supplies one just for this construction.
            let on_navigate: EventHandler<PathBuf> = dioxus_core::Runtime::current()
                .in_scope(ScopeId::ROOT, || EventHandler::new(|_: PathBuf| {}));
            let e = crate::keyboard::test_support::key_event(
                Key::Character("s".into()),
                dioxus::html::input_data::keyboard_types::Modifiers::CONTROL,
            );
            assert!(e.propagates(), "test setup: a fresh event must propagate");
            path_input_keydown(&e, edit_mode, input_val, input_err, on_navigate, "/reset");
            assert!(
                !e.propagates(),
                "typing in the path input must not let Ctrl+S (or any \
                 other key) reach the global keyboard handler behind it"
            );
        });
    }

    #[test]
    fn escape_resets_the_path_input_to_the_value_it_had_before_editing() {
        with_test_store(|_store| {
            let edit_mode = Signal::new_in_scope(true, ScopeId::ROOT);
            let input_val = Signal::new_in_scope(String::from("/typed/garbage"), ScopeId::ROOT);
            let input_err = Signal::new_in_scope(true, ScopeId::ROOT);
            // `Callback::new` needs an active scope on the stack, which
            // `with_test_store`'s `in_runtime` alone does not push (unlike
            // `Signal::new_in_scope`, which takes the scope explicitly) —
            // `Runtime::in_scope` supplies one just for this construction.
            let on_navigate: EventHandler<PathBuf> = dioxus_core::Runtime::current()
                .in_scope(ScopeId::ROOT, || EventHandler::new(|_: PathBuf| {}));
            let e = crate::keyboard::test_support::key_event(
                Key::Escape,
                dioxus::html::input_data::keyboard_types::Modifiers::empty(),
            );
            path_input_keydown(
                &e,
                edit_mode,
                input_val,
                input_err,
                on_navigate,
                "/original",
            );
            assert_eq!(*input_val.read(), "/original");
            assert!(!*edit_mode.read());
            assert!(!*input_err.read());
        });
    }

    // F99/C3: PathBar's Back/Forward/Up/Home/Open-folder buttons carried a
    // `title` tooltip but their content was a bare glyph — a screen reader
    // announces the character, not the action (F74/F80's defect class, in
    // the one view that hadn't been touched). `aria_label` now reuses the
    // same translated string `title` already used, so no new i18n key was
    // needed. Whether the attribute actually reaches the platform
    // accessibility tree is an AT-SPI/UIA assertion, not a unit test —
    // F74 recorded that same limit, and it still holds (the same shape as
    // review 093's disclosed-but-untestable toolbar `<select>`). What this
    // test can and does assert: the rendered element carries an
    // `aria-label` attribute with the expected translated text — falsify
    // by removing any one `aria_label:` from `PathBar` and the
    // corresponding assertion below fails.
    fn path_bar_aria_labels() -> Vec<String> {
        fn root() -> Element {
            rsx! {
                PathBar {
                    path: PathBuf::from("/tmp"),
                    can_back: true,
                    can_forward: true,
                    on_back: move |_| {},
                    on_forward: move |_| {},
                    on_navigate: move |_: PathBuf| {},
                    lang: Lang::En,
                    is_focused: false,
                    on_focus: move |_| {},
                    side_label: "Left pane".to_string(),
                }
            }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();

        mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute {
                    name: "aria-label",
                    value: dioxus_core::AttributeValue::Text(s),
                    ..
                } => Some(s.clone()),
                _ => None,
            })
            .collect()
    }

    /// F185: the path bar's always-visible buttons — ⌂/📁/✎ moved into the
    /// ⋯ menu, which is not in this list because it only renders while
    /// `menu_open` is true, and this just mounts the component fresh.
    #[test]
    fn path_bar_buttons_carry_the_expected_aria_label() {
        let aria_labels = path_bar_aria_labels();
        for expected in ["Back", "Forward", "Go up one directory", "More"] {
            assert!(
                aria_labels.iter().any(|l| l == expected),
                "expected an aria-label {expected:?} among the rendered \
                 attributes {aria_labels:?}"
            );
        }
    }

    /// F185/RFC-061 amendment: the path bar itself is now the labelled
    /// group the removed `role="heading"` pane-root cell used to be —
    /// "{side} — {current folder name}", kept current because it is
    /// computed from `path` on every render, not cached at mount.
    #[test]
    fn path_bar_group_label_names_the_side_and_the_current_folder() {
        let aria_labels = path_bar_aria_labels();
        assert!(
            aria_labels.iter().any(|l| l == "Left pane — tmp"),
            "expected the group aria-label \"Left pane — tmp\" among \
             {aria_labels:?}"
        );
    }

    // ── ⋯ menu keyboard model (F185) ─────────────────────────────────────────
    //
    // `item_handles`/`more_btn_handle` are left `None` throughout: these
    // tests exercise the index math and the key-swallowing contract, not
    // `set_focus` itself (a real `MountedData` handle needs a live
    // renderer — `focus_handle`'s own doc comment covers why a `None`
    // handle is a safe no-op).

    #[test]
    fn more_btn_arrow_down_opens_the_menu() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(false, ScopeId::ROOT);
            let e = crate::keyboard::test_support::key_event(
                Key::ArrowDown,
                dioxus::html::input_data::keyboard_types::Modifiers::empty(),
            );
            assert!(e.propagates(), "test setup");
            more_btn_keydown(&e, menu_open);
            assert!(*menu_open.read(), "↓ on ⋯ must open the menu");
            assert!(!e.propagates(), "↓ must not reach anything behind ⋯ either");
        });
    }

    #[test]
    fn more_btn_other_keys_do_not_open_the_menu_or_get_swallowed() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(false, ScopeId::ROOT);
            let e = crate::keyboard::test_support::key_event(
                Key::Tab,
                dioxus::html::input_data::keyboard_types::Modifiers::empty(),
            );
            assert!(e.propagates(), "test setup");
            more_btn_keydown(&e, menu_open);
            assert!(!*menu_open.read(), "Tab must not open the menu");
            assert!(
                e.propagates(),
                "a plain button must keep its ordinary Tab behaviour"
            );
        });
    }

    #[test]
    fn open_pathbar_menu_opens_it() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(false, ScopeId::ROOT);
            open_pathbar_menu(menu_open);
            assert!(*menu_open.read());
        });
    }

    /// Wrap-around math, asserted against `menu_item_keydown`'s own return
    /// value — no signal needs to exist purely to make this observable.
    #[test]
    fn menu_item_arrow_down_wraps_from_the_last_item_to_the_first() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(true, ScopeId::ROOT);
            let item_handles = Signal::new_in_scope([None, None, None], ScopeId::ROOT);
            let more_btn_handle = Signal::new_in_scope(None, ScopeId::ROOT);
            let e = crate::keyboard::test_support::key_event(
                Key::ArrowDown,
                dioxus::html::input_data::keyboard_types::Modifiers::empty(),
            );
            let next = menu_item_keydown(&e, 2, menu_open, item_handles, more_btn_handle);
            assert_eq!(next, Some(0), "must wrap from the last item to the first");
            assert!(
                *menu_open.read(),
                "arrow navigation must not close the menu"
            );
        });
    }

    #[test]
    fn menu_item_arrow_up_wraps_from_the_first_item_to_the_last() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(true, ScopeId::ROOT);
            let item_handles = Signal::new_in_scope([None, None, None], ScopeId::ROOT);
            let more_btn_handle = Signal::new_in_scope(None, ScopeId::ROOT);
            let e = crate::keyboard::test_support::key_event(
                Key::ArrowUp,
                dioxus::html::input_data::keyboard_types::Modifiers::empty(),
            );
            let next = menu_item_keydown(&e, 0, menu_open, item_handles, more_btn_handle);
            assert_eq!(next, Some(2), "must wrap from the first item to the last");
        });
    }

    /// Handoff 074 §2.1: "Esc closes the menu and returns focus to ⋯." The
    /// focus-return itself needs a real `MountedData` handle (a live
    /// renderer) to observe — this is the half a unit test can: the menu
    /// closes.
    #[test]
    fn menu_item_escape_closes_the_menu() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(true, ScopeId::ROOT);
            let item_handles = Signal::new_in_scope([None, None, None], ScopeId::ROOT);
            let more_btn_handle = Signal::new_in_scope(None, ScopeId::ROOT);
            let e = crate::keyboard::test_support::key_event(
                Key::Escape,
                dioxus::html::input_data::keyboard_types::Modifiers::empty(),
            );
            let next = menu_item_keydown(&e, 1, menu_open, item_handles, more_btn_handle);
            assert!(!*menu_open.read());
            assert_eq!(next, None);
        });
    }

    /// Review 156: Tab must close the menu (so its full-viewport
    /// `.pathbar-menu-catcher` does not linger and absorb the user's next
    /// click) but must leave the browser's own Tab-order traversal alone —
    /// the WAI-ARIA menu pattern, not a focus trap. Falsify by calling
    /// `e.prevent_default()` in the `Key::Tab` arm: this fails on the second
    /// assertion even though the menu still closes correctly.
    #[test]
    fn menu_item_tab_closes_the_menu_without_preventing_the_default_action() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(true, ScopeId::ROOT);
            let item_handles = Signal::new_in_scope([None, None, None], ScopeId::ROOT);
            let more_btn_handle = Signal::new_in_scope(None, ScopeId::ROOT);
            let e = crate::keyboard::test_support::key_event(
                Key::Tab,
                dioxus::html::input_data::keyboard_types::Modifiers::empty(),
            );
            assert!(e.default_action_enabled(), "test setup");
            let next = menu_item_keydown(&e, 1, menu_open, item_handles, more_btn_handle);
            assert!(!*menu_open.read(), "Tab must close the menu");
            assert!(
                e.default_action_enabled(),
                "Tab's own focus-order traversal must not be prevented"
            );
            assert_eq!(next, None);
        });
    }

    /// Shift+Tab is the same key value with a modifier; the modifier must
    /// not change which arm handles it.
    #[test]
    fn menu_item_shift_tab_also_closes_the_menu_without_preventing_the_default_action() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(true, ScopeId::ROOT);
            let item_handles = Signal::new_in_scope([None, None, None], ScopeId::ROOT);
            let more_btn_handle = Signal::new_in_scope(None, ScopeId::ROOT);
            let e = crate::keyboard::test_support::key_event(
                Key::Tab,
                dioxus::html::input_data::keyboard_types::Modifiers::SHIFT,
            );
            let next = menu_item_keydown(&e, 1, menu_open, item_handles, more_btn_handle);
            assert!(!*menu_open.read(), "Shift+Tab must close the menu");
            assert!(e.default_action_enabled());
            assert_eq!(next, None);
        });
    }

    /// "Keys in the open menu reach nothing behind it" (handoff 074 §2.1):
    /// every key on a menu item is swallowed, not only the ones this
    /// function gives a specific meaning to — the same
    /// `swallow_when_typing` discipline `path_input_keydown` and
    /// `filter_input_keydown` already follow. Falsify by removing the
    /// `swallow_when_typing` call: this fails on a plain character key that
    /// none of the `match` arms handle.
    #[test]
    fn menu_item_swallows_every_key_not_just_the_ones_it_acts_on() {
        with_test_store(|_store| {
            let menu_open = Signal::new_in_scope(true, ScopeId::ROOT);
            let item_handles = Signal::new_in_scope([None, None, None], ScopeId::ROOT);
            let more_btn_handle = Signal::new_in_scope(None, ScopeId::ROOT);
            let e = crate::keyboard::test_support::key_event(
                Key::Character("x".into()),
                dioxus::html::input_data::keyboard_types::Modifiers::empty(),
            );
            assert!(e.propagates(), "test setup");
            menu_item_keydown(&e, 0, menu_open, item_handles, more_btn_handle);
            assert!(!e.propagates());
        });
    }
}

#[cfg(test)]
mod filtering_executor_tests {
    use super::*;
    use dioxus_swdir_tree::LoadedEntry;

    fn payload() -> LoadPayload {
        let entry = |name: &str, is_dir: bool| LoadedEntry {
            path: PathBuf::from("/root").join(name),
            is_dir,
            is_hidden: false,
        };
        LoadPayload {
            path: PathBuf::from("/root"),
            generation: 1,
            depth: 0,
            result: Ok(vec![
                entry(".git", true),
                entry("build.log", false),
                entry("keep.txt", false),
            ]),
        }
    }

    /// Run one scan through the executor and return the names it listed.
    fn scan(exec: &FilteringExecutor) -> Vec<String> {
        let fut = exec.spawn_blocking(Box::new(payload));
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let payload = rt.block_on(fut);
        payload
            .result
            .unwrap()
            .iter()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect()
    }

    /// F112: an executor built once — which is what `use_scan_driver` keeps —
    /// filters by the rules in force *when each scan is requested*, not the ones
    /// it was built with. Falsify by making `spawn_blocking` read the rules once
    /// at construction (a snapshot, the old shape): the second and third
    /// assertions fail.
    #[test]
    fn a_settings_change_reaches_the_next_scan_of_an_executor_built_earlier() {
        let shared = SharedIgnoreRules::new(IgnoreRules::default());
        let exec = FilteringExecutor {
            rules: shared.clone(),
        };
        assert_eq!(scan(&exec), vec![".git", "build.log", "keep.txt"]);

        shared.set(IgnoreRules::from_settings("", ".git"));
        assert_eq!(scan(&exec), vec!["build.log", "keep.txt"]);

        shared.set(IgnoreRules::from_settings("log", ".git"));
        assert_eq!(scan(&exec), vec!["keep.txt"]);

        shared.set(IgnoreRules::default());
        assert_eq!(scan(&exec), vec![".git", "build.log", "keep.txt"]);
    }

    /// Both panes' executors share one handle, so one settings change covers both.
    #[test]
    fn two_executors_sharing_a_handle_change_together() {
        let shared = SharedIgnoreRules::new(IgnoreRules::default());
        let left = FilteringExecutor {
            rules: shared.clone(),
        };
        let right = FilteringExecutor {
            rules: shared.clone(),
        };
        shared.set(IgnoreRules::from_settings("", ".git"));
        assert!(!scan(&left).contains(&".git".to_string()));
        assert!(!scan(&right).contains(&".git".to_string()));
    }
}
