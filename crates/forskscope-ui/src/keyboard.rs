//! Global keyboard ownership (RFC-060), made testable (handoff 020).
//!
//! RFC-060's rule shipped as a decision buried inside `app.rs`'s `onkeydown`
//! closure, which cannot be called from a test as written — so the RFC's own
//! stated purpose ("so the class of bug cannot reappear as new shortcuts or
//! new input surfaces are added") had nothing enforcing it. This module
//! lifts that decision into [`global_key_action`], a pure function `app.rs`
//! calls into as a thin dispatcher, and gives the complementary obligation —
//! a text input must swallow a keydown before it can reach the global
//! handler — one shared, testable helper: [`swallow_when_typing`].

use std::rc::Rc;

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;

// ── Part A: the global-key decision ─────────────────────────────────────────

/// A coarse summary of [`crate::state::Modal`]'s open/closed/recovery state
/// — narrow enough to construct directly in a test, unlike `Modal` itself
/// (its recovery variants carry a full `SettingsRuntimeResolution` /
/// `SessionRuntimeResolution`, not needed just to exercise a keyboard-guard
/// branch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModalState {
    None,
    /// RFC-076 patch 6: a recovery choice must never be made by an
    /// accidental keypress — every one of its actions must be explicit.
    Recovery,
    Other,
}

impl ModalState {
    pub(crate) fn from_modal(modal: &crate::state::Modal) -> Self {
        match modal {
            crate::state::Modal::None => Self::None,
            crate::state::Modal::SettingsRecovery(_) | crate::state::Modal::SessionRecovery(_) => {
                Self::Recovery
            }
            _ => Self::Other,
        }
    }

    fn is_open(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// What a single global keypress should do, decided from state alone — no
/// store mutation, no `spawn`, no `document::eval`. `app.rs`'s `onkeydown`
/// reads the current state, calls this, then performs whatever the returned
/// action says; the mapping from action to effect lives there, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GlobalKeyAction {
    Ignore,
    CloseModal,
    MoveFocus(i32),
    SearchNext,
    SearchPrev,
    ApplyFocusedHunk,
    Save,
    Undo,
    Redo,
    RequestCloseTab,
    OpenKeyboardRef,
    OpenSearch,
    /// RFC-087 §2: select the next / previous tab in the flat Explorer +
    /// file + directory list, wrapping. The one pair of actions this
    /// function returns with no active comparison tab — see
    /// [`global_key_action`]'s own doc for why the gate moved.
    NextTab,
    PrevTab,
}

/// The decision RFC-060 guards: Escape closes an ordinary modal but never a
/// recovery one; every other global shortcut is swallowed while any modal is
/// open (P0-1: e.g. Ctrl+S must not write the file behind an overwrite
/// dialog); with no modal open and no active tab, only Escape-shaped
/// no-ops make sense.
pub(crate) fn global_key_action(
    key: &Key,
    mods: Modifiers,
    modal: ModalState,
    has_active_tab: bool,
) -> GlobalKeyAction {
    use GlobalKeyAction as A;

    if *key == Key::Escape {
        return if modal.is_open() && modal != ModalState::Recovery {
            A::CloseModal
        } else {
            A::Ignore
        };
    }

    if modal.is_open() {
        return A::Ignore;
    }

    // RFC-087 §2: tab switching is the one command that needs no active
    // comparison tab — checked before the gate below, which every other
    // command still passes through unchanged. `swallow_when_typing` keeps
    // this from a focused text field the same way it keeps everything else
    // global out: `app.rs` only ever calls this with events a text input
    // did not already stop.
    match key {
        Key::Tab if mods.contains(Modifiers::CONTROL) => {
            return if mods.contains(Modifiers::SHIFT) {
                A::PrevTab
            } else {
                A::NextTab
            };
        }
        Key::PageDown if mods.contains(Modifiers::CONTROL) => return A::NextTab,
        Key::PageUp if mods.contains(Modifiers::CONTROL) => return A::PrevTab,
        _ => {}
    }

    if !has_active_tab {
        return A::Ignore;
    }

    match key {
        Key::F7 => A::MoveFocus(-1),
        Key::F8 => A::MoveFocus(1),
        // F3 / Shift+F3: next / previous search match.
        Key::F3 => {
            if mods.contains(Modifiers::SHIFT) {
                A::SearchPrev
            } else {
                A::SearchNext
            }
        }
        Key::Enter => A::ApplyFocusedHunk,
        Key::Character(s) if mods.contains(Modifiers::CONTROL) => {
            match s.to_ascii_lowercase().as_str() {
                "s" => A::Save,
                "z" => A::Undo,
                "y" => A::Redo,
                "w" => A::RequestCloseTab,
                "/" => A::OpenKeyboardRef,
                "f" => A::OpenSearch,
                _ => A::Ignore,
            }
        }
        _ => A::Ignore,
    }
}

// ── Part B: the input obligation, made structural ───────────────────────────

/// Stops a keydown from bubbling past this element to the global handler —
/// the obligation RFC-060's purpose statement depends on ("the global
/// onkeydown yields ... to text inputs"). Call this first, unconditionally,
/// in every text-input `onkeydown`: the obligation is then discharged by
/// construction, not by remembering to add it — a new input either calls
/// this, or its omission is visibly not there in review.
pub(crate) fn swallow_when_typing(e: &Event<KeyboardData>) {
    e.stop_propagation();
}

// ── Part C: a focused control keeps its own Enter (F125) ───────────────────

/// Stops **Enter** from reaching the global handler; every other key
/// continues to it, so F7/F8, Ctrl+S and the rest still work with a control
/// focused.
///
/// Enter activates a focused button, and it is *also* the global "apply the
/// focused change" key (RFC-060). Both fired: pressing Enter on *More ▼*,
/// *Wrap*, *Settings* or a hunk's *Use* button applied the focused hunk as
/// well (F125, reproduced in the real app; Space, which activates a button
/// the same way but is not a global key, was clean). A focused control is the
/// same argument `swallow_when_typing` makes for a text field: it takes
/// priority. The Enter binding itself is untouched — with nothing focused
/// but the app root, Enter still applies the focused hunk.
pub(crate) fn controls_keep_enter(e: &Event<KeyboardData>) {
    if e.key() == Key::Enter {
        e.stop_propagation();
    }
}

/// A region whose descendants are the app's focusable controls (the header,
/// the tab bar, and the body holding the diff toolbar, the hunk *Use*
/// buttons and the search bar). Wrapping them here, rather than wiring each
/// button, discharges [`controls_keep_enter`] for a control added later.
///
/// The app root (`#app-root`, which holds focus when nothing else does) is
/// deliberately **not** inside one, so Enter pressed with focus on the root
/// still reaches the global handler.
#[component]
pub(crate) fn ControlZone(
    class: String,
    #[props(default)] role: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "{class}",
            role,
            aria_labelledby,
            onkeydown: move |e| controls_keep_enter(&e),
            {children}
        }
    }
}

// ── Part D: focusing an already-mounted target (F185) ───────────────────────

/// `.set_focus(true)` on an already-mounted target's handle, via
/// `spawn_forever` so the caller's own `onkeydown` returns immediately. A
/// no-op if the handle has not been captured yet — a silent no-op is the
/// right failure mode for a focus call, which is advisory, not a panic.
/// Shared by every keyboard surface that moves focus programmatically
/// between its own already-mounted elements (the path bar's ⋯ menu, F185;
/// the tab list's roving tabindex, RFC-087) — never for a target that
/// might not exist in the DOM yet, which needs to focus itself from its
/// own `onmounted` instead (see each caller's own account of why).
///
/// `spawn_forever`, not `spawn`: live AT-SPI testing of RFC-087's
/// close-then-refocus requirement found that a plain `spawn` ties the task
/// to whatever component scope is current when it's called — and every
/// real caller here (the tab list's own `Delete` handling, the dirty-tab
/// confirm modal's "Discard and close") calls this from the very
/// component that is itself about to unmount as a result of the same
/// click. Dioxus drops a `spawn`-ed task when its owning scope tears down,
/// so `set_focus` silently never ran; `spawn_forever` runs detached from
/// any one scope's lifecycle (the same reason `state::compare` already
/// uses it for a reload that must survive its own view unmounting).
pub(crate) fn focus_handle(handle: Option<Rc<MountedData>>) {
    if let Some(h) = handle {
        dioxus_core::spawn_forever(async move {
            let _ = h.set_focus(true).await;
        });
    }
}

// ── Test support shared across the surfaces converted in §5 ────────────────

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    struct FakeKeyboardEvent {
        key: Key,
        modifiers: Modifiers,
    }

    impl ModifiersInteraction for FakeKeyboardEvent {
        fn modifiers(&self) -> Modifiers {
            self.modifiers
        }
    }

    impl HasKeyboardData for FakeKeyboardEvent {
        fn key(&self) -> Key {
            self.key.clone()
        }
        fn code(&self) -> Code {
            Code::Unidentified
        }
        fn location(&self) -> Location {
            Location::Standard
        }
        fn is_auto_repeating(&self) -> bool {
            false
        }
        fn is_composing(&self) -> bool {
            false
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// A manufactured `Event<KeyboardData>`, `propagates() == true` until a
    /// handler under test calls `stop_propagation()` on it — the same
    /// object identity a real Dioxus dispatch would hand a handler.
    pub(crate) fn key_event(key: Key, modifiers: Modifiers) -> Event<KeyboardData> {
        Event::new(
            Rc::new(KeyboardData::new(FakeKeyboardEvent { key, modifiers })),
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::key_event;
    use super::*;
    use crate::state::Modal;

    // ── ModalState::from_modal ──────────────────────────────────────────────

    #[test]
    fn from_modal_classifies_none() {
        assert_eq!(ModalState::from_modal(&Modal::None), ModalState::None);
    }

    #[test]
    fn from_modal_classifies_an_ordinary_modal_as_other() {
        assert_eq!(ModalState::from_modal(&Modal::Settings), ModalState::Other);
    }

    #[test]
    fn from_modal_classifies_settings_recovery_as_recovery() {
        use forskscope_desktop::persist::schema::settings::runtime::{
            SettingsRuntimeOutcome, SettingsRuntimeResolution,
        };
        let resolution = SettingsRuntimeResolution {
            value: Default::default(),
            write_disabled: true,
            outcome: SettingsRuntimeOutcome::Fresh,
            raw_bytes: None,
        };
        assert_eq!(
            ModalState::from_modal(&Modal::SettingsRecovery(resolution)),
            ModalState::Recovery
        );
    }

    #[test]
    fn from_modal_classifies_session_recovery_as_recovery() {
        use forskscope_desktop::persist::schema::session::runtime::{
            SessionRuntimeOutcome, SessionRuntimeResolution,
        };
        let resolution = SessionRuntimeResolution {
            value: Default::default(),
            write_disabled: true,
            outcome: SessionRuntimeOutcome::Fresh,
            raw_bytes: None,
        };
        assert_eq!(
            ModalState::from_modal(&Modal::SessionRecovery(resolution)),
            ModalState::Recovery
        );
    }

    // ── global_key_action: the falsifiable guard (handoff 020 §6 tests 1/2) ─

    /// Test 1: falsify by removing this crate's `modal_open` early return
    /// from `global_key_action` (temporarily changing `if modal.is_open()`
    /// to `if false`) — Ctrl+S must then no longer be ignored while a modal
    /// is open, and this test must fail.
    #[test]
    fn ctrl_s_is_ignored_while_a_modal_is_open() {
        assert_eq!(
            global_key_action(
                &Key::Character("s".into()),
                Modifiers::CONTROL,
                ModalState::Other,
                true,
            ),
            GlobalKeyAction::Ignore,
        );
    }

    #[test]
    fn ctrl_s_saves_with_no_modal_open_and_an_active_tab() {
        assert_eq!(
            global_key_action(
                &Key::Character("s".into()),
                Modifiers::CONTROL,
                ModalState::None,
                true,
            ),
            GlobalKeyAction::Save,
        );
    }

    #[test]
    fn escape_closes_an_ordinary_modal() {
        assert_eq!(
            global_key_action(&Key::Escape, Modifiers::empty(), ModalState::Other, true),
            GlobalKeyAction::CloseModal,
        );
    }

    /// Test 2: falsify by removing the `&& modal != ModalState::Recovery`
    /// condition from `global_key_action`'s Escape branch — Escape must then
    /// no longer be ignored for a recovery modal, and this test must fail.
    #[test]
    fn escape_does_not_dismiss_a_recovery_modal() {
        assert_eq!(
            global_key_action(&Key::Escape, Modifiers::empty(), ModalState::Recovery, true),
            GlobalKeyAction::Ignore,
        );
    }

    #[test]
    fn escape_does_nothing_with_no_modal_open() {
        assert_eq!(
            global_key_action(&Key::Escape, Modifiers::empty(), ModalState::None, true),
            GlobalKeyAction::Ignore,
        );
    }

    #[test]
    fn global_shortcuts_are_ignored_with_no_active_tab() {
        assert_eq!(
            global_key_action(
                &Key::Character("s".into()),
                Modifiers::CONTROL,
                ModalState::None,
                false,
            ),
            GlobalKeyAction::Ignore,
        );
        assert_eq!(
            global_key_action(&Key::F7, Modifiers::empty(), ModalState::None, false),
            GlobalKeyAction::Ignore,
        );
        assert_eq!(
            global_key_action(&Key::Enter, Modifiers::empty(), ModalState::None, false),
            GlobalKeyAction::Ignore,
        );
    }

    #[test]
    fn plain_character_without_control_is_ignored() {
        assert_eq!(
            global_key_action(
                &Key::Character("s".into()),
                Modifiers::empty(),
                ModalState::None,
                true,
            ),
            GlobalKeyAction::Ignore,
        );
    }

    #[test]
    fn f7_and_f8_move_focus() {
        assert_eq!(
            global_key_action(&Key::F7, Modifiers::empty(), ModalState::None, true),
            GlobalKeyAction::MoveFocus(-1),
        );
        assert_eq!(
            global_key_action(&Key::F8, Modifiers::empty(), ModalState::None, true),
            GlobalKeyAction::MoveFocus(1),
        );
    }

    #[test]
    fn f3_moves_to_the_next_match_and_shift_f3_to_the_previous() {
        assert_eq!(
            global_key_action(&Key::F3, Modifiers::empty(), ModalState::None, true),
            GlobalKeyAction::SearchNext,
        );
        assert_eq!(
            global_key_action(&Key::F3, Modifiers::SHIFT, ModalState::None, true),
            GlobalKeyAction::SearchPrev,
        );
    }

    #[test]
    fn enter_applies_the_focused_hunk() {
        assert_eq!(
            global_key_action(&Key::Enter, Modifiers::empty(), ModalState::None, true),
            GlobalKeyAction::ApplyFocusedHunk,
        );
    }

    #[test]
    fn ctrl_shortcuts_map_to_their_actions() {
        let cases = [
            ("z", GlobalKeyAction::Undo),
            ("y", GlobalKeyAction::Redo),
            ("w", GlobalKeyAction::RequestCloseTab),
            ("/", GlobalKeyAction::OpenKeyboardRef),
            ("f", GlobalKeyAction::OpenSearch),
        ];
        for (ch, expected) in cases {
            assert_eq!(
                global_key_action(
                    &Key::Character(ch.into()),
                    Modifiers::CONTROL,
                    ModalState::None,
                    true,
                ),
                expected,
                "Ctrl+{ch} must map to {expected:?}",
            );
        }
    }

    #[test]
    fn an_unbound_ctrl_character_is_ignored() {
        assert_eq!(
            global_key_action(
                &Key::Character("q".into()),
                Modifiers::CONTROL,
                ModalState::None,
                true,
            ),
            GlobalKeyAction::Ignore,
        );
    }

    // ── RFC-087 §2: tab switching from anywhere ─────────────────────────────

    /// The one pair of actions `global_key_action` returns with no active
    /// comparison tab — the premise of "works from the Explorer". Falsify
    /// by leaving the `!has_active_tab` gate where it was, ahead of the tab
    /// switch match: this then fails with `Ignore`.
    #[test]
    fn ctrl_tab_switches_tabs_with_no_active_tab() {
        assert_eq!(
            global_key_action(&Key::Tab, Modifiers::CONTROL, ModalState::None, false),
            GlobalKeyAction::NextTab,
        );
        assert_eq!(
            global_key_action(
                &Key::Tab,
                Modifiers::CONTROL | Modifiers::SHIFT,
                ModalState::None,
                false,
            ),
            GlobalKeyAction::PrevTab,
        );
    }

    #[test]
    fn ctrl_page_down_and_up_also_switch_tabs_with_no_active_tab() {
        assert_eq!(
            global_key_action(&Key::PageDown, Modifiers::CONTROL, ModalState::None, false),
            GlobalKeyAction::NextTab,
        );
        assert_eq!(
            global_key_action(&Key::PageUp, Modifiers::CONTROL, ModalState::None, false),
            GlobalKeyAction::PrevTab,
        );
    }

    #[test]
    fn tab_switching_also_works_with_an_active_tab() {
        assert_eq!(
            global_key_action(&Key::Tab, Modifiers::CONTROL, ModalState::None, true),
            GlobalKeyAction::NextTab,
        );
    }

    /// Acceptance criterion 3: while a modal is open, the tab-switch keys
    /// do nothing either — the one exception to "needs no active tab" is
    /// not an exception to "a modal owns every key".
    #[test]
    fn tab_switching_is_ignored_while_a_modal_is_open() {
        assert_eq!(
            global_key_action(&Key::Tab, Modifiers::CONTROL, ModalState::Other, false),
            GlobalKeyAction::Ignore,
        );
        assert_eq!(
            global_key_action(&Key::Tab, Modifiers::CONTROL, ModalState::Recovery, false),
            GlobalKeyAction::Ignore,
        );
    }

    #[test]
    fn plain_tab_with_no_control_is_not_a_tab_switch() {
        assert_eq!(
            global_key_action(&Key::Tab, Modifiers::empty(), ModalState::None, true),
            GlobalKeyAction::Ignore,
        );
    }

    // ── swallow_when_typing ─────────────────────────────────────────────────

    #[test]
    fn swallow_when_typing_stops_propagation() {
        let e = key_event(Key::Character("s".into()), Modifiers::CONTROL);
        assert!(e.propagates(), "test setup: a fresh event must propagate");
        swallow_when_typing(&e);
        assert!(!e.propagates());
    }

    // ── F125: a focused control keeps its own Enter ─────────────────────────

    /// Falsify by making `controls_keep_enter` a no-op: Enter then propagates
    /// to the global handler, where it is `ApplyFocusedHunk` — the bug.
    #[test]
    fn enter_on_a_control_does_not_reach_the_global_handler() {
        let e = key_event(Key::Enter, Modifiers::empty());
        assert!(e.propagates(), "test setup");
        controls_keep_enter(&e);
        assert!(!e.propagates());
    }

    /// Precedence, not removal: every other key still reaches the global
    /// handler with a control focused, so F7/F8, Ctrl+S and Ctrl+Z work from
    /// the toolbar. Space activates a button too but is not a global key.
    #[test]
    fn other_keys_still_reach_the_global_handler_from_a_control() {
        for (key, mods) in [
            (Key::F8, Modifiers::empty()),
            (Key::F3, Modifiers::empty()),
            (Key::Escape, Modifiers::empty()),
            (Key::Character(" ".into()), Modifiers::empty()),
            (Key::Character("s".into()), Modifiers::CONTROL),
            (Key::Character("z".into()), Modifiers::CONTROL),
        ] {
            let e = key_event(key.clone(), mods);
            controls_keep_enter(&e);
            assert!(e.propagates(), "{key:?} must still propagate");
        }
    }

    /// The designed route survives: Enter with nothing but the app root
    /// focused is still `ApplyFocusedHunk` (RFC-060; the keyboard reference
    /// documents it).
    #[test]
    fn enter_still_applies_the_focused_hunk_when_no_control_owns_it() {
        assert_eq!(
            global_key_action(&Key::Enter, Modifiers::empty(), ModalState::None, true),
            GlobalKeyAction::ApplyFocusedHunk,
        );
    }

    fn keydown_listeners(root: fn() -> Element) -> usize {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_to_vec()
            .edits
            .iter()
            .filter(|m| {
                matches!(
                    m,
                    dioxus_core::Mutation::NewEventListener { name, .. } if *name == "keydown"
                )
            })
            .count()
    }

    fn with_store<F: FnOnce() -> Element>(f: F) -> Element {
        use_context_provider(|| {
            crate::state::Store::new(
                crate::state::AppSettings::default(),
                Default::default(),
                false,
            )
        });
        f()
    }

    /// The wiring, not just the helper: a bare `ControlZone` renders
    /// exactly one `keydown` listener, its own. Falsify by deleting
    /// `onkeydown` from `ControlZone`: both counts drop to zero.
    ///
    /// The tab bar renders **two** with nothing but the permanent Explorer
    /// tab (no file or directory tabs): `ControlZone`'s, and the Explorer
    /// tab's own (RFC-087 — every `role="tab"` needs its own keydown for
    /// ←/→/Home/End/Delete, independent of `ControlZone`). Falsifying
    /// `ControlZone` alone only brings the tab bar's count to one, not
    /// zero — proven directly below, rather than folded into the same
    /// zero-or-not assertion the other uses. RFC-089: the tab bar now also
    /// carries the *Settings* and *?* buttons the removed `Header` used to
    /// — plain buttons, no `onkeydown` of their own, so this count is
    /// unaffected by their addition.
    #[test]
    fn the_tab_bar_and_a_bare_control_zone_carry_the_enter_guard() {
        fn zone() -> Element {
            rsx! { ControlZone { class: "z", button { "x" } } }
        }
        fn tabbar() -> Element {
            with_store(|| rsx! { crate::ui::layout::tabs::TabBar {} })
        }
        assert_eq!(keydown_listeners(zone), 1, "ControlZone");
        assert_eq!(
            keydown_listeners(tabbar),
            2,
            "TabBar: ControlZone + the Explorer tab"
        );
    }
}
