//! Overlay modals: all safety and action dialogs dispatched from `ModalLayer`.
//!
//! Submodules by category:
//! - `file`     — `OverwriteModal`, `SaveAsModal`, `ReloadModal`, `SwapModal`
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
pub use file::{OverwriteModal, ReloadModal, SaveAsModal, SwapModal};
pub use recovery::{SessionRecoveryModal, SettingsRecoveryModal};
pub use tab::CloseTabModal;
