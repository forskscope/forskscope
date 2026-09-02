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

// ── Test support shared across the surfaces converted in §5 ────────────────

#[cfg(test)]
pub(crate) mod test_support {
    use std::rc::Rc;

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
        use forskscope_core::persist::schema::settings::runtime::{
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
        use forskscope_core::persist::schema::session::runtime::{
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

    // ── swallow_when_typing ─────────────────────────────────────────────────

    #[test]
    fn swallow_when_typing_stops_propagation() {
        let e = key_event(Key::Character("s".into()), Modifiers::CONTROL);
        assert!(e.propagates(), "test setup: a fresh event must propagate");
        swallow_when_typing(&e);
        assert!(!e.propagates());
    }
}
