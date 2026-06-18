//! Session persistence (RFC-035): save and restore open tabs across launches.

use std::path::PathBuf;

use app_json_settings::ConfigManager;
use dioxus::prelude::*;

use crate::state::Store;
use crate::state::compare::open_compare;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    /// Pairs of `(left_path, right_path)` for each open comparison tab.
    pub tabs: Vec<(String, String)>,
}

fn session_manager() -> ConfigManager<SessionState> {
    ConfigManager::new().with_filename("session.json")
}

/// Persist the current open tabs for restoration on next launch.
pub fn save_session(store: &Store) {
    let tabs = store.tabs.read();
    let saved: Vec<(String, String)> = tabs.iter()
        .filter_map(|tab| {
            let l = tab.left_path.as_ref()?.display().to_string();
            let r = tab.right_path.as_ref()?.display().to_string();
            Some((l, r))
        })
        .collect();
    let _ = session_manager().save(&SessionState { tabs: saved });
}

/// Load the last-saved session, opening each tab whose paths still exist.
/// Silently skips pairs where both sides are gone.
pub fn restore_session(store: &mut Store) {
    let state = session_manager().load_or_default().unwrap_or_default();
    for (left, right) in state.tabs {
        let lp = PathBuf::from(&left);
        let rp = PathBuf::from(&right);
        if lp.exists() || rp.exists() {
            open_compare(store, lp, rp);
        }
    }
}

/// Close the tab at `index`, adjusting the active index so another tab
/// (or the Explorer) remains visible.
pub fn close_tab(store: &mut Store, index: usize) {
    store.tabs.write().remove(index);
    let len        = store.tabs.read().len();
    let new_active = if len == 0 { None } else { Some(index.min(len - 1)) };
    store.active.set(new_active);
    save_session(store);
}



#[cfg(test)]
mod tests;
