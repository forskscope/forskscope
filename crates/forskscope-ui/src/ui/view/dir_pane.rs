//! Dir-pane building blocks used by the Explorer aligned view (RFC-054).
//!
//! This module provides the types, helper functions, and leaf components
//! (`PathBar`, `TreeRow`) used by `explorer.rs`.  The `DirPane` monolith that
//! managed its own tree state is gone; tree ownership now lives in Explorer so
//! both panes can be rendered in an aligned structure.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

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
fn status_kind_label(kind: RowStatusKind, lang: Lang) -> String {
    match kind {
        RowStatusKind::Equal => t(lang, "Identical"),
        RowStatusKind::Different => t(lang, "Different"),
        RowStatusKind::LeftOnly => t(lang, "Only on the left"),
        RowStatusKind::RightOnly => t(lang, "Only on the right"),
        RowStatusKind::Computing => t(lang, "Comparing…"),
        RowStatusKind::Error => t(lang, "Comparison failed"),
        RowStatusKind::NotCompared => t(lang, "Directory contents not compared — use Deep Compare"),
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

pub struct FilteringExecutor {
    pub rules: IgnoreRules,
}
// IgnoreRules is plain Vec<String>; Send + Sync derive automatically.

impl ScanExecutor for FilteringExecutor {
    fn spawn_blocking(&self, job: ScanJob) -> ScanFuture {
        let rules = self.rules.clone();
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
/// `← → ⌂ 📁 │ /path/segments/current ✎`
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
) -> Element {
    // Pre-compute everything before closures consume values.
    let segs = path_segs(&path);
    let n = segs.len();
    let path_str = path.display().to_string();
    let path_str_reset = path_str.clone();
    let path_str_blur = path_str.clone();

    let mut edit_mode: Signal<bool> = use_signal(|| false);
    let mut input_val: Signal<String> = use_signal(|| path_str.clone());
    let mut input_err: Signal<bool> = use_signal(|| false);

    use_effect(move || {
        if !*edit_mode.read() {
            input_val.set(path_str.clone());
        }
    });

    rsx! {
        div { class: "path-bar",
            button { class: "path-btn", title: t(lang, "Back"),    disabled: !can_back,    onclick: move |_| on_back.call(()),    "←" }
            button { class: "path-btn", title: t(lang, "Forward"), disabled: !can_forward, onclick: move |_| on_forward.call(()), "→" }
            button { class: "path-btn", title: t(lang, "Go up one directory"),
                onclick: move |_| {
                    let p = path.parent().map(|p| p.to_path_buf());
                    if let Some(p) = p { on_navigate.call(p); }
                }, "↑" }
            button { class: "path-btn", title: t(lang, "Home directory"),
                onclick: move |_| on_navigate.call(home_dir()), "⌂" }
            button { class: "path-btn", title: t(lang, "Open folder…"),
                onclick: move |_| {
                    let nav = on_navigate;
                    spawn(async move {
                        let r = tokio::task::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
                            .await.ok().flatten();
                        if let Some(p) = r { nav.call(p); }
                    });
                }, "📁" }

            div { class: "path-segments",
                if *edit_mode.read() {
                    input {
                        class: if *input_err.read() { "path-input error" } else { "path-input" },
                        r#type: "text", value: "{input_val}", autofocus: true,
                        oninput:  move |e| { input_val.set(e.value()); input_err.set(false); },
                        onkeydown: move |e| {
                            // RFC-060/handoff 020 §5: swallow every key first, not just
                            // Enter/Escape — a Ctrl+S typed while editing this path must
                            // not save the active tab behind it either.
                            crate::keyboard::swallow_when_typing(&e);
                            if e.key() == Key::Enter {
                                let v = PathBuf::from(input_val.read().cloned());
                                if v.is_dir() { edit_mode.set(false); on_navigate.call(v); }
                                else { input_err.set(true); }
                            }
                            if e.key() == Key::Escape {
                                input_val.set(path_str_reset.clone());
                                edit_mode.set(false); input_err.set(false);
                            }
                        },
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
                    button { class: "path-btn path-edit-btn", title: t(lang, "Edit path"),
                        onclick: move |_| edit_mode.set(true), "✎" }
                }
            }
        }
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
                    let label = status_kind_label(kind, lang);
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
            "var t = document.getElementById('aligned-tree'); if(t) t.scrollTop = 0;",
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
        ];
        for kind in kinds {
            for lang in [Lang::En, Lang::Ja] {
                let label = status_kind_label(kind, lang);
                assert!(
                    !label.is_empty(),
                    "{kind:?} has an empty accessible label for {lang:?}"
                );
            }
        }
    }

    // F74: `NotCompared` must use its own glyph/class - not silently fall
    // back to `Equal`'s (which would recreate the false-equal claim this
    // state exists to replace) or to `Computing`'s (a spinner that never
    // resolves, handoff 007 §4's first defect). The glyph/class contract
    // itself is `status.rs`'s (tested there); this confirms `dir_pane.rs`'s
    // label layer keeps `NotCompared` distinct from `Equal` too.
    #[test]
    fn not_compared_has_a_distinct_label_from_equal() {
        let label = status_kind_label(RowStatusKind::NotCompared, Lang::En);
        assert_ne!(label, status_kind_label(RowStatusKind::Equal, Lang::En));
    }

    // Review 077 §4b: `LeftOnly`/`RightOnly` render distinct glyphs
    // (`←`/`→`) and must announce distinct labels too - RFC-009 §7
    // requires symbol and screen-reader text to carry the same meaning,
    // and a shared "only on this side" label for both left this direction
    // sighted-only.
    #[test]
    fn left_only_and_right_only_have_distinct_labels() {
        for lang in [Lang::En, Lang::Ja] {
            let left = status_kind_label(RowStatusKind::LeftOnly, lang);
            let right = status_kind_label(RowStatusKind::RightOnly, lang);
            assert_ne!(
                left, right,
                "LeftOnly and RightOnly must have distinct labels for {lang:?}"
            );
        }
    }
}
