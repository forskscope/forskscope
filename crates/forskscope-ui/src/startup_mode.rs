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

/// Set once by `main()`, immediately after the Portable data folder is
/// created — the outcome of `startup_resolve::probe_writable` (RFC-091 §2:
/// "detected... by an actual write probe, not by permission bits"). Left
/// unset for Private/Installed, where it is never consulted.
pub static PORTABLE_WRITABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// `true` exactly when this run is Portable and its data location's write
/// probe failed. Every settings/session/manifest write site checks this
/// alongside `is_private()` — reads still happen normally; RFC-091 §2's
/// "Read-only data location" is "the app reads what it can... saves
/// nothing", not "starts from defaults" the way Private does.
pub(crate) fn is_portable_read_only() -> bool {
    matches!(DATA_LOCATION.get(), Some(DataLocation::Portable(_)))
        && matches!(PORTABLE_WRITABLE.get(), Some(false))
}

/// What the status bar and About dialog show (RFC-091 §2/§3, acceptance
/// criterion 7) — one value folding together [`DATA_LOCATION`] and
/// [`PORTABLE_WRITABLE`] so neither caller needs to know the write-probe
/// exists, only this one derived fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeDisplay {
    /// Today's behaviour; the RFC's texts are shown in every *other* case,
    /// so this one maps to showing nothing.
    Installed,
    /// Portable, writable — the folder is this `PathBuf` (tooltip).
    Portable(PathBuf),
    /// Portable, but the write probe failed — the folder is this `PathBuf`
    /// (tooltip), and nothing is being saved into it.
    PortableReadOnly(PathBuf),
    Private,
}

pub(crate) fn mode_display() -> ModeDisplay {
    mode_display_for(DATA_LOCATION.get(), PORTABLE_WRITABLE.get().copied())
}

/// The pure half of [`mode_display`], split out so a test can exercise
/// every combination directly — `DATA_LOCATION`/`PORTABLE_WRITABLE` are
/// process-wide `OnceLock`s shared across every test in this binary, so
/// nothing can set them to a specific value for one test alone.
fn mode_display_for(
    location: Option<&DataLocation>,
    portable_writable: Option<bool>,
) -> ModeDisplay {
    match location {
        Some(DataLocation::Private) => ModeDisplay::Private,
        Some(DataLocation::Portable(dir)) => {
            if portable_writable == Some(false) {
                ModeDisplay::PortableReadOnly(dir.clone())
            } else {
                ModeDisplay::Portable(dir.clone())
            }
        }
        Some(DataLocation::Installed) | None => ModeDisplay::Installed,
    }
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

/// The folder batch-copy manifests are written to. `None` in Private mode,
/// in a read-only Portable location (RFC-091 §2: "writes nothing
/// anywhere"), and whenever the Installed platform folder cannot be found —
/// `forskscope_core::dir::batch_copy` already treats `None` as "write no
/// manifest", so neither case needs a separate gate here: both reach the
/// same "nothing kept" outcome through the existing optional-manifest path
/// rather than a new one.
pub(crate) fn manifest_dir() -> Option<PathBuf> {
    match DATA_LOCATION.get() {
        Some(DataLocation::Private) => None,
        Some(DataLocation::Portable(dir)) => {
            if is_portable_read_only() {
                None
            } else {
                Some(dir.join("manifests"))
            }
        }
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

    #[test]
    fn mode_display_for_installed_shows_nothing() {
        assert_eq!(
            mode_display_for(Some(&DataLocation::Installed), None),
            ModeDisplay::Installed
        );
        assert_eq!(mode_display_for(None, None), ModeDisplay::Installed);
    }

    #[test]
    fn mode_display_for_private_is_private_regardless_of_the_writable_fact() {
        assert_eq!(
            mode_display_for(Some(&DataLocation::Private), None),
            ModeDisplay::Private
        );
        assert_eq!(
            mode_display_for(Some(&DataLocation::Private), Some(false)),
            ModeDisplay::Private
        );
    }

    #[test]
    fn mode_display_for_portable_writable_or_unprobed_is_plain_portable() {
        let dir = PathBuf::from("/mnt/usb/data");
        assert_eq!(
            mode_display_for(Some(&DataLocation::Portable(dir.clone())), Some(true)),
            ModeDisplay::Portable(dir.clone())
        );
        // No probe result yet (e.g. a test harness that never ran main) must
        // not be read as "read-only" - only an actual failed probe does.
        assert_eq!(
            mode_display_for(Some(&DataLocation::Portable(dir.clone())), None),
            ModeDisplay::Portable(dir)
        );
    }

    #[test]
    fn mode_display_for_portable_with_a_failed_probe_is_read_only() {
        let dir = PathBuf::from("/mnt/usb/data");
        assert_eq!(
            mode_display_for(Some(&DataLocation::Portable(dir.clone())), Some(false)),
            ModeDisplay::PortableReadOnly(dir)
        );
    }
}
