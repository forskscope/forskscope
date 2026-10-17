//! RFC-091 §1: the impure facts `resolve_data_location` needs, and the
//! web-view `Config` wiring that follows from its answer. Bin-only, the
//! same way `webview2` is (`main.rs` is the only caller of any of this —
//! `startup_mode`, used by the lib target too, holds the result and the
//! shared queries `config_dir`/`manifest_dir`/`is_private`).

use std::path::{Path, PathBuf};

use dioxus_desktop::Config;
use forskscope_ui_logic::Platform;

use crate::startup_mode::DataLocation;

/// The platform fact `resolve_data_location` needs for RFC-091 §2a, read
/// once here (the one impure/environment-reading side) rather than inside
/// the pure resolver itself, so criterion 9's resolver test can exercise
/// macOS from any machine by passing `Platform::MacOs` directly.
pub(crate) fn current_platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Other
    }
}

/// RFC-091 §2: "detected at startup by an actual write probe, not by
/// permission bits" — permission bits can lie (root, ACLs, a read-only
/// mount reported as writable, an overlay filesystem), so this writes a
/// real file into `dir` and removes it again, the same thing any later
/// settings/session/manifest write into this folder will actually attempt.
pub(crate) fn probe_writable(dir: &Path) -> bool {
    let probe = dir.join(".forskscope-write-probe");
    if std::fs::write(&probe, b"").is_ok() {
        let _ = std::fs::remove_file(&probe);
        true
    } else {
        false
    }
}

/// The program's real folder — `std::env::current_exe()`, symbolic links
/// resolved. `None` if either call fails, which `resolve_data_location`
/// treats as "no marker folder to find" (RFC-091 §1 row 3 falls through to
/// row 4, Installed) rather than an error — a program that cannot even
/// locate itself has no portable folder to offer.
pub(crate) fn program_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let real = resolve_real_exe(&exe)?;
    real.parent().map(Path::to_path_buf)
}

/// `std::fs::canonicalize`, split out of [`program_dir`] so a test can
/// exercise the symlink-resolution claim directly (RFC-091 §1: "a link in
/// `~/bin` to a program elsewhere uses the real program's folder") without
/// needing to relaunch the test binary through a symlink.
fn resolve_real_exe(exe_path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(exe_path).ok()
}

/// Whether `forskscope-data` exists directly inside `dir` — RFC-091 §1's
/// marker folder. A plain existence check, not a "looks writable" guess:
/// portable mode is never inferred from a folder merely being writable
/// (§2: "never guessed").
pub(crate) fn marker_exists(dir: &Path) -> bool {
    dir.join("forskscope-data").is_dir()
}

/// Private mode's one temporary data root —
/// `<tmp>/forskscope-private-<pid>` — created lazily on first use and
/// shared by [`redirect_xdg_dirs`] (Linux) and [`configure_webview_data_dir`]
/// so both agree on the same folder instead of each creating, and leaving
/// behind, their own. `None` if the folder could not even be created (e.g.
/// the system temp directory itself is unwritable) — every caller treats
/// that as "fall back to today's default" rather than failing to start.
static PRIVATE_DATA_ROOT: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();

fn private_data_root() -> Option<PathBuf> {
    PRIVATE_DATA_ROOT
        .get_or_init(|| {
            let dir =
                std::env::temp_dir().join(format!("forskscope-private-{}", std::process::id()));
            if std::fs::create_dir_all(&dir).is_ok() {
                // SAFETY: `remove_private_data_root` takes no arguments and
                // returns nothing, matching `atexit`'s required signature
                // exactly; it only reads the `OnceLock` set immediately
                // above (via `private_data_root`, the only caller that
                // reaches this branch), never anything that could be
                // mutated concurrently from elsewhere.
                unsafe { libc::atexit(remove_private_data_root) };
                Some(dir)
            } else {
                None
            }
        })
        .clone()
}

/// **Why `atexit`, not a `Drop` guard on a `main()` local.**
/// `dioxus_desktop::launch::launch` returns `!`: on every platform this
/// project targets, it hands the event loop to the OS toolkit and the
/// process exits from inside that call, never returning control to `main`.
/// A cleanup guard held in a local there would never run its `Drop`.
/// `libc::atexit` is registered instead — every supported platform's C
/// runtime still invokes `atexit` handlers on the same `exit()` the event
/// loop ultimately calls to end the process.
extern "C" fn remove_private_data_root() {
    if let Some(Some(dir)) = PRIVATE_DATA_ROOT.get() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// The `<data>` root RFC-091 §2's table and §3's temp folder both describe:
/// the chosen folder for Portable, the one shared private temp folder
/// (created on first call) for Private, `None` for Installed — today's
/// behaviour, nothing redirected.
pub(crate) fn data_root(location: &DataLocation) -> Option<PathBuf> {
    match location {
        DataLocation::Installed => None,
        DataLocation::Portable(dir) => Some(dir.clone()),
        DataLocation::Private => private_data_root(),
    }
}

/// Points the web-view's own `with_data_directory` storage at `<data>/webview`
/// for Portable/Private, or leaves `cfg` unchanged for `Installed`/a Private
/// folder that could not be created — today's behaviour, where
/// `dioxus_desktop` picks its own platform default.
pub(crate) fn configure_webview_data_dir(cfg: Config, location: &DataLocation) -> Config {
    match data_root(location) {
        Some(dir) => cfg.with_data_directory(dir.join("webview")),
        None => cfg,
    }
}

/// The three XDG Base Directory variables WebKitGTK, GLib and GTK take
/// their default cache/data/state locations from, each pointed at its own
/// subfolder of `data` — pure, so the mapping itself is testable without
/// touching the environment. `redirect_xdg_dirs` below is this function's
/// only caller, and it is Linux-only, so this is too — a non-Linux build
/// would otherwise flag it as dead code.
#[cfg(target_os = "linux")]
fn xdg_override_paths(data: &Path) -> [(&'static str, PathBuf); 3] {
    [
        ("XDG_CACHE_HOME", data.join("xdg").join("cache")),
        ("XDG_DATA_HOME", data.join("xdg").join("data")),
        ("XDG_STATE_HOME", data.join("xdg").join("state")),
    ]
}

/// RFC-091 §2 (review 167): `with_data_directory` alone leaves WebKitGTK's
/// cache and HSTS storage behind — confirmed live (request 156a/157): `wry`
/// 0.53.5's `WebContextImpl::new` only wires `with_data_directory` through
/// to WebKitGTK's own *data* directory, not its cache, and upstream's
/// partial fix (wry #1547) is pinned out of reach by `dioxus-desktop` 0.7.9.
/// WebKitGTK, GLib and GTK all take their **default** cache/data/state
/// locations from `XDG_CACHE_HOME`/`XDG_DATA_HOME`/`XDG_STATE_HOME` instead
/// — the supported, documented way to relocate them, and (per the same
/// measurement) it also catches the toolkit's own `gvfs-metadata`/`glycin`
/// writes that `with_data_directory` was never going to move.
///
/// **Must run before GTK initializes and before any other thread exists**
/// — GLib caches these values on first use, so this has to be called at the
/// very top of `main`, before anything else touches the data-location
/// result. `XDG_CONFIG_HOME` is deliberately left alone: reading the user's
/// GTK theme and font settings is wanted, and nothing is ever written
/// there. A no-op for `Installed` (`data_root` returns `None`) and for a
/// Private folder that could not be created.
///
/// Linux only. macOS's WKWebView does not read these variables at all (a
/// separate RFC-091 amendment to the owner covers that); Windows's
/// WebView2 already keeps its cache inside what `with_data_directory` sets
/// (review 167).
#[cfg(target_os = "linux")]
pub(crate) fn redirect_xdg_dirs(location: &DataLocation) {
    let Some(data) = data_root(location) else {
        return;
    };
    let overrides = xdg_override_paths(&data);
    for (_, path) in &overrides {
        let _ = std::fs::create_dir_all(path);
    }
    forskscope_core::env_passthrough::record_before_override(&[
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
    ]);
    // SAFETY: called at the very start of main(), before dioxus_desktop,
    // tao or GTK initialize anything and before any other thread exists in
    // this process — the precondition std::env::set_var's edition-2024
    // unsafety exists to flag. No other code in this process reads or
    // writes these three variables before this point.
    unsafe {
        for (name, path) in &overrides {
            std::env::set_var(name, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_is_macos_only_under_cfg_target_os_macos() {
        let platform = current_platform();
        if cfg!(target_os = "macos") {
            assert_eq!(platform, Platform::MacOs);
        } else {
            assert_eq!(platform, Platform::Other);
        }
    }

    #[test]
    fn probe_writable_is_true_for_a_real_writable_directory() {
        let base = std::env::temp_dir().join(format!(
            "forskscope-startup-resolve-test-probe-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&base).unwrap();

        assert!(probe_writable(&base));
        // The probe file must not be left behind.
        assert_eq!(std::fs::read_dir(&base).unwrap().count(), 0);

        std::fs::remove_dir_all(&base).ok();
    }

    #[cfg(unix)]
    #[test]
    fn probe_writable_is_false_for_a_read_only_directory() {
        use std::os::unix::fs::PermissionsExt;

        let base = std::env::temp_dir().join(format!(
            "forskscope-startup-resolve-test-probe-ro-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o500)).unwrap();

        let writable = probe_writable(&base);

        // Restore write permission before cleanup, regardless of outcome.
        std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::remove_dir_all(&base).ok();

        assert!(
            !writable,
            "a directory with no write permission for its owner must fail the probe - this test does not apply running as root, which bypasses the permission check entirely"
        );
    }

    /// RFC-091 §1: "a link in `~/bin` to a program elsewhere uses the real
    /// program's folder." Falsify by having `resolve_real_exe` return its
    /// input unchanged (skip `canonicalize`) and this must fail — the
    /// resolved parent would then be the symlink's own folder, not the
    /// real program's.
    #[test]
    fn resolve_real_exe_follows_a_symlink_to_the_real_program_folder() {
        let base = std::env::temp_dir().join(format!(
            "forskscope-startup-resolve-test-{}-{}",
            std::process::id(),
            line!()
        ));
        let real_dir = base.join("real");
        let link_dir = base.join("bin");
        std::fs::create_dir_all(&real_dir).unwrap();
        std::fs::create_dir_all(&link_dir).unwrap();
        let real_exe = real_dir.join("forskscope");
        std::fs::write(&real_exe, b"").unwrap();
        let link = link_dir.join("forskscope");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_exe, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&real_exe, &link).unwrap();

        let resolved = resolve_real_exe(&link).expect("canonicalize should follow the symlink");
        assert_eq!(resolved, real_exe.canonicalize().unwrap());
        assert_eq!(
            resolved.parent().unwrap(),
            real_dir.canonicalize().unwrap(),
            "the resolved folder must be the real program's, not the symlink's"
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn resolve_real_exe_on_a_plain_non_symlink_path_resolves_to_itself() {
        let base = std::env::temp_dir().join(format!(
            "forskscope-startup-resolve-test-plain-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let exe = base.join("forskscope");
        std::fs::write(&exe, b"").unwrap();

        let resolved = resolve_real_exe(&exe).unwrap();
        assert_eq!(resolved, exe.canonicalize().unwrap());

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn marker_exists_is_true_only_for_a_real_directory_named_forskscope_data() {
        let base = std::env::temp_dir().join(format!(
            "forskscope-startup-resolve-test-marker-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&base).unwrap();
        assert!(!marker_exists(&base), "nothing there yet");

        std::fs::write(base.join("forskscope-data"), b"not a directory").unwrap();
        assert!(
            !marker_exists(&base),
            "a file named forskscope-data is not the marker"
        );
        std::fs::remove_file(base.join("forskscope-data")).unwrap();

        std::fs::create_dir(base.join("forskscope-data")).unwrap();
        assert!(marker_exists(&base));

        std::fs::remove_dir_all(&base).ok();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn xdg_override_paths_names_a_distinct_subfolder_per_variable() {
        let data = Path::new("/portable/data");
        let overrides = xdg_override_paths(data);
        assert_eq!(overrides[0], ("XDG_CACHE_HOME", data.join("xdg/cache")));
        assert_eq!(overrides[1], ("XDG_DATA_HOME", data.join("xdg/data")));
        assert_eq!(overrides[2], ("XDG_STATE_HOME", data.join("xdg/state")));
    }

    /// RFC-091 §2/review 167: the three XDG variables must actually end up
    /// pointing inside the portable data folder — this is `redirect_xdg_dirs`
    /// run for real, not just its pure path mapping, confirming the whole
    /// resolver (data_root -> xdg_override_paths -> env::set_var) runs
    /// correctly from one call, the way `main()`'s first line does it.
    /// Falsify by removing the `std::env::set_var` loop and this fails —
    /// `std::env::var` would still read whatever the ambient test process
    /// already had (almost certainly unset).
    #[cfg(target_os = "linux")]
    #[test]
    fn redirect_xdg_dirs_points_all_three_variables_inside_the_data_folder() {
        let base = std::env::temp_dir().join(format!(
            "forskscope-startup-resolve-test-xdg-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&base).unwrap();

        redirect_xdg_dirs(&DataLocation::Portable(base.clone()));

        assert_eq!(
            std::env::var("XDG_CACHE_HOME").unwrap(),
            base.join("xdg/cache").to_str().unwrap()
        );
        assert_eq!(
            std::env::var("XDG_DATA_HOME").unwrap(),
            base.join("xdg/data").to_str().unwrap()
        );
        assert_eq!(
            std::env::var("XDG_STATE_HOME").unwrap(),
            base.join("xdg/state").to_str().unwrap()
        );
        assert!(base.join("xdg/cache").is_dir());
        assert!(base.join("xdg/data").is_dir());
        assert!(base.join("xdg/state").is_dir());

        std::fs::remove_dir_all(&base).ok();
    }
}
