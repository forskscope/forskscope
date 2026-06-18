//! Overlay modals: all safety and action dialogs dispatched from `ModalLayer`.
//!
//! Submodules by category:
//! - `file`  — `OverwriteModal`, `SaveAsModal`, `ReloadModal`, `SwapModal`
//! - `tab`   — `CloseTabModal`
//! - `copy`  — `ConfirmDirOpModal`, `BatchCopyModal`, `BatchResultModal`
//! - `about` — `AboutModal`

pub mod about;
pub mod copy;
pub mod file;
pub mod tab;

pub use about::AboutModal;
pub use copy::{BatchCopyModal, BatchResultModal, ConfirmDirOpModal};
pub use file::{OverwriteModal, ReloadModal, SaveAsModal, SwapModal};
pub use tab::CloseTabModal;

use crate::state::Store;

/// Force-save (skip mtime check). Used by `OverwriteModal`.
pub(super) fn save_tab_force(store: &mut Store, index: usize) {
    crate::ui::view::diff::save_tab(store, index, true);
}
