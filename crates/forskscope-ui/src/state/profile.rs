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

/// Make profile `index` the active one and persist it — what the Settings
/// → Advanced profile radio group's `onchange` calls (RFC-087 §3), the same
/// two statements the earlier clickable name used before it became a radio.
pub fn select_profile(store: &mut Store, index: usize) {
    store.settings.write().active_profile = index;
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

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;
    use forskscope_core::persist::schema::PersistenceLoad;
    use forskscope_core::persist::schema::settings::SettingsRepository;

    use super::select_profile;
    use crate::state::{ConfigRootOverrideGuard, with_test_store};

    /// RFC-087 §3 / acceptance criterion 4: selecting a profile through the
    /// radio group persists it to `settings.json`, exactly like the earlier
    /// clickable name did. Falsify by dropping the `persist` call from
    /// `select_profile`: this fails.
    #[test]
    fn select_profile_persists_the_new_active_profile() {
        let dir =
            std::env::temp_dir().join(format!("fsk-ui-select-profile-{}", std::process::id()));
        let config_home = dir.join("config");
        std::fs::create_dir_all(&config_home).unwrap();
        let _config_root = ConfigRootOverrideGuard::set(config_home.clone());

        with_test_store(|store| {
            let profile_count = store.settings.read().profiles.len();
            assert!(
                profile_count >= 2,
                "test premise: at least two built-in profiles to select between"
            );
            assert_eq!(store.settings.read().active_profile, 0, "test premise");

            select_profile(store, 1);
            assert_eq!(store.settings.read().active_profile, 1);
        });

        let settings_path = config_home.join("forskscope").join("settings.json");
        let repo = SettingsRepository::new(settings_path);
        match repo.load() {
            PersistenceLoad::Current { value } => {
                assert_eq!(value.active_profile, 1);
            }
            other => panic!("expected Current, got {other:?}"),
        }
    }
}
