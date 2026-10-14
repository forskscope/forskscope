//! Compare profile management: add and remove user-defined profiles (RFC-009).

use dioxus::prelude::*;

use forskscope_core::diff::InlineMode;
use forskscope_core::{NewlineCompareMode, WhitespaceMode};

use crate::state::{DiffAlgorithmSetting, Store};

#[allow(clippy::too_many_arguments)]
pub fn add_profile(
    store: &mut Store,
    name: String,
    whitespace: WhitespaceMode,
    ignore_case: bool,
    algorithm: DiffAlgorithmSetting,
    newlines: NewlineCompareMode,
    inline_mode: InlineMode,
) {
    store
        .settings
        .write()
        .profiles
        .push(crate::state::settings::DiffProfile {
            name,
            whitespace,
            ignore_case,
            algorithm,
            newlines,
            inline_mode,
            built_in: false,
        });
    crate::ui::view::settings::persist(*store);
}

pub fn remove_profile(store: &mut Store, index: usize) {
    let is_builtin = store
        .settings
        .read()
        .profiles
        .get(index)
        .map(|p| p.built_in)
        .unwrap_or(true);
    if is_builtin {
        return;
    }
    let mut s = store.settings.write();
    s.profiles.remove(index);
    if s.active_profile >= s.profiles.len() {
        s.active_profile = s.profiles.len().saturating_sub(1);
    }
    drop(s);
    crate::ui::view::settings::persist(*store);
}
