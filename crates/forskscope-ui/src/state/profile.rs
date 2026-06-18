//! Compare profile management: add and remove user-defined profiles (RFC-009).

use dioxus::prelude::*;

use crate::state::{Store, DiffAlgorithmSetting};

pub fn add_profile(
    store: &mut Store,
    name: String,
    ignore_whitespace: bool,
    ignore_case: bool,
    algorithm: DiffAlgorithmSetting,
) {
    store.settings.write().profiles.push(crate::state::settings::DiffProfile {
        name, ignore_whitespace, ignore_case, algorithm, built_in: false,
    });
    crate::ui::view::settings::persist(&store.settings.read());
}

pub fn remove_profile(store: &mut Store, index: usize) {
    let is_builtin = store.settings.read()
        .profiles.get(index).map(|p| p.built_in).unwrap_or(true);
    if is_builtin { return; }
    let mut s = store.settings.write();
    s.profiles.remove(index);
    if s.active_profile >= s.profiles.len() {
        s.active_profile = s.profiles.len().saturating_sub(1);
    }
    drop(s);
    crate::ui::view::settings::persist(&store.settings.read());
}
