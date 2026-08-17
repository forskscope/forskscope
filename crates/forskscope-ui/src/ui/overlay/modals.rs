//! Overlay modals: all safety and action dialogs dispatched from `ModalLayer`.
//!
//! Submodules by category:
//! - `file`     — `OverwriteModal`, `SaveAsModal`, `ReloadModal`, `SwapModal`, `ConfirmDiffOptionChangeModal`
//! - `tab`      — `CloseTabModal`
//! - `copy`     — `ConfirmDirOpModal`, `BatchCopyModal`, `BatchResultModal`
//! - `about`    — `AboutModal`
//! - `recovery` — `SettingsRecoveryModal`, `SessionRecoveryModal` (RFC-076)

pub mod about;
pub mod copy;
pub mod file;
pub mod recovery;
pub mod tab;

pub use about::AboutModal;
pub use copy::{BatchCopyModal, BatchResultModal, ConfirmDirOpModal};
pub use file::{
    ConfirmDiffOptionChangeModal, ConfirmSaveAsOverwriteModal, OverwriteModal, ReloadModal,
    SaveAsModal, SwapModal,
};
pub use recovery::{SessionRecoveryModal, SettingsRecoveryModal};
pub use tab::CloseTabModal;

use dioxus::prelude::*;

/// F69: every destructive-confirmation modal marks its Cancel-equivalent
/// button `autofocus: true`, which reliably moves DOM focus into it on
/// WebKitGTK (confirmed on a real desktop, review 067 §2) but not on
/// WebView2 - there, focus stays on whatever control was clicked to open
/// the modal, so a screen-reader user hitting Enter/Space right after the
/// modal announces itself lands on the *background* control, not even the
/// destructive action. The `autofocus` attribute is not wrong to keep
/// (WebKitGTK needs nothing else); this re-asserts it explicitly once the
/// modal has mounted, for the engine that does not honor it on its own.
///
/// Attach to each modal's outer `.scrim` div's `onmounted`, not the button
/// itself - by the time an element's `onmounted` fires, its own subtree
/// (the button included) is already in the DOM, so this only needs to run
/// once per modal, on the div every one of them already renders first.
pub(crate) fn focus_autofocus_button(_event: Event<MountedData>) {
    spawn(async move {
        let _ =
            dioxus::document::eval("document.querySelector('.scrim [autofocus]')?.focus();").await;
    });
}
