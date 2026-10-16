//! RFC-091: holds [`DataLocation`] for the life of the run, and the three
//! queries every settings/session/manifest read or write site consults.
//!
//! [`resolve_data_location`](forskscope_ui_logic::resolve_data_location) in
//! `forskscope-ui-logic` is the pure decision; `main.rs`'s
//! `startup_resolve` supplies the two facts it needs (the program's real
//! folder, and whether a marker folder sits beside it) and stores the
//! result here, in [`DATA_LOCATION`], set once before any settings, session
//! or web-view path is resolved — the same `OnceLock` pattern
//! `app::STARTUP_REQUEST` already uses. This module (unlike
//! `startup_resolve`) is part of the `forskscope_ui` *library* target too,
//! since `state.rs`/`state/session.rs`/`ui/view/settings.rs` all consult it.

use std::path::PathBuf;

pub use forskscope_ui_logic::DataLocation;

/// Set once by `main()`, read by `config_dir`/`manifest_dir`/`is_private`
/// and by the web-view `Config` builder. Falls back to `Installed` if never
/// set (a test harness that never calls `main`), matching today's behaviour
/// exactly in that case.
pub static DATA_LOCATION: std::sync::OnceLock<DataLocation> = std::sync::OnceLock::new();

/// `true` exactly when this run is `--private`. Checked by every settings
/// and session read/write site *before* touching `config_dir()` or a
/// repository at all — private mode reads nothing, so there is no path to
/// construct, let alone resolve (RFC-091 §3).
pub(crate) fn is_private() -> bool {
    matches!(DATA_LOCATION.get(), Some(DataLocation::Private))
}

/// The folder `settings.json`/`session.json` live in directly — no
/// `forskscope` subfolder in Portable (RFC-091 §2's table: `settings.json`
/// sits directly in the data location), the existing `.../forskscope`
/// subfolder in Installed (shared platform folders need the app-specific
/// name; a portable folder the user chose does not).
///
/// Never called while `is_private()` — every caller checks that first — so
/// the `Private` arm here is unreachable in practice; it still resolves to
/// something rather than panicking, in case a future caller forgets to
/// check first, which is a logic bug but not a reason to crash.
pub(crate) fn config_dir() -> Option<PathBuf> {
    match DATA_LOCATION.get() {
        Some(DataLocation::Portable(dir)) => Some(dir.clone()),
        Some(DataLocation::Private) => None,
        Some(DataLocation::Installed) | None => {
            dirs_next::config_dir().map(|d| d.join("forskscope"))
        }
    }
}

/// The folder batch-copy manifests are written to. `None` in Private mode
/// (and whenever the Installed platform folder cannot be found) —
/// `forskscope_core::dir::batch_copy` already treats `None` as "write no
/// manifest", so Private mode needs no separate gate here: it is the same
/// "nothing kept" RFC-091 §3 asks for, reached through the existing
/// optional-manifest path rather than a new one.
pub(crate) fn manifest_dir() -> Option<PathBuf> {
    match DATA_LOCATION.get() {
        Some(DataLocation::Private) => None,
        Some(DataLocation::Portable(dir)) => Some(dir.join("manifests")),
        Some(DataLocation::Installed) | None => {
            dirs_next::data_dir().map(|d| d.join("forskscope").join("manifests"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_private_is_false_until_data_location_is_set() {
        // DATA_LOCATION is a process-wide OnceLock shared across every test
        // in this binary; only assert the one property that holds
        // regardless of what another test may have already set it to.
        if DATA_LOCATION.get().is_none() {
            assert!(!is_private());
        }
    }
}
