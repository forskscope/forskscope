//! Runtime platform diagnostic information (RFC-026 §"Diagnostics panel").
//!
//! [`PlatformInfo`] collects environment facts that help users and maintainers
//! diagnose build and runtime issues. It is purely read-only — it inspects
//! compile-time constants and environment variables; it never writes anything.
//!
//! The About / Diagnostics panel copies this to the clipboard so users can
//! paste it into bug reports without needing a terminal.
//!
//! [`flatpak`] (RFC-095 §4.1) is the one place that knows whether this
//! process is running inside a Flatpak sandbox, and whether the home
//! folder is granted. Every Flatpak-specific behaviour elsewhere in the
//! app calls this function - none checks `/.flatpak-info` itself.

/// What [`flatpak`] (RFC-095 §4.1) found. Outside a Flatpak, `in_flatpak`
/// is `false` and the other two fields hold their defaults - callers
/// should branch on `in_flatpak`, not on `app_id`/`home_granted` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatpakInfo {
    /// Whether `/.flatpak-info` exists at all - the sandbox's own signal
    /// that this process is running inside a Flatpak.
    pub in_flatpak: bool,
    /// `[Application] name=` from `/.flatpak-info` - the Flatpak app ID.
    /// `None` outside a Flatpak, or if the file exists but that key is
    /// missing (malformed, not expected in practice).
    pub app_id: Option<String>,
    /// Whether `[Context] filesystems=` contains `home` or `host` (with
    /// or without a `:rw`/`:ro` suffix) - the home-folder grant actually
    /// in force, including a run-time change
    /// (`flatpak override --filesystem=home`/`--nofilesystem=home`).
    /// Always `false` outside a Flatpak.
    pub home_granted: bool,
}

const FLATPAK_INFO_PATH: &str = "/.flatpak-info";

/// Reads `/.flatpak-info` once and reports what RFC-095 §4.1 needs. See
/// [`FlatpakInfo`] for what each field means.
pub fn flatpak() -> FlatpakInfo {
    flatpak_at(FLATPAK_INFO_PATH)
}

/// `flatpak`'s own body, taking the path as a parameter - `pub(crate)` so
/// `tests/platform_tests.rs` can exercise the real read-then-parse path
/// end to end (a real missing path for "no file," a real temp file for
/// every other planned input) rather than only the string-level parser.
pub(crate) fn flatpak_at(path: &str) -> FlatpakInfo {
    match std::fs::read_to_string(path) {
        Ok(contents) => parse_flatpak_info(&contents),
        Err(_) => FlatpakInfo {
            in_flatpak: false,
            app_id: None,
            home_granted: false,
        },
    }
}

/// The parser `flatpak` (above) calls on the real file's contents -
/// `pub(crate)` so `tests/platform_tests.rs` can drive it directly on
/// planned inputs, without writing a real `/.flatpak-info` on disk.
/// `/.flatpak-info` is a GLib key file: `[Section]` headers, `key=value`
/// lines; a line's section is whichever `[...]` header most recently
/// preceded it in the file, so a `filesystems=` line outside `[Context]`
/// is read as belonging to that other section and ignored here, exactly
/// as a key file parser would.
pub(crate) fn parse_flatpak_info(contents: &str) -> FlatpakInfo {
    let mut app_id = None;
    let mut home_granted = false;
    let mut section = "";
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line;
            continue;
        }
        match section {
            "[Application]" => {
                if let Some(value) = line.strip_prefix("name=") {
                    app_id = Some(value.trim().to_string());
                }
            }
            "[Context]" => {
                if let Some(value) = line.strip_prefix("filesystems=") {
                    home_granted = value.split(';').any(|entry| {
                        let name = entry.split(':').next().unwrap_or("").trim();
                        name == "home" || name == "host"
                    });
                }
            }
            _ => {}
        }
    }
    FlatpakInfo {
        in_flatpak: true,
        app_id,
        home_granted,
    }
}

/// Runtime and compile-time platform information.
///
/// All fields are `String` so the struct is straightforwardly serialisable
/// and displayable without additional formatting helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInfo {
    /// ForskScope version (`CARGO_PKG_VERSION`).
    pub app_version: String,
    /// Rust compiler version used to build this binary (`VERGEN_RUSTC_SEMVER`
    /// if available; falls back to `"unknown"`).
    pub rustc_version: String,
    /// Target triple this binary was compiled for (e.g. `x86_64-unknown-linux-gnu`).
    pub target_triple: String,
    /// OS family as reported by `std::env::consts::OS` (`"linux"`, `"windows"`, `"macos"`, …).
    pub os: String,
    /// CPU architecture (`std::env::consts::ARCH`).
    pub arch: String,
    /// Number of logical CPUs (`std::thread::available_parallelism`, best-effort).
    pub logical_cpus: String,
    /// Value of `$HOME` or `%USERPROFILE%`, redacted after the last path separator
    /// to avoid leaking the username in bug reports.
    pub home_redacted: String,
    /// Value of `$XDG_DATA_HOME` or platform default config dir prefix (Linux only).
    pub config_dir_hint: String,
}

impl PlatformInfo {
    /// Collect current platform information.
    pub fn collect() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION").into(),
            rustc_version: option_env!("VERGEN_RUSTC_SEMVER")
                .unwrap_or("unknown")
                .into(),
            target_triple: std::env::consts::FAMILY.into(), // approximation; see note
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            logical_cpus: std::thread::available_parallelism()
                .map(|n| n.get().to_string())
                .unwrap_or_else(|_| "unknown".into()),
            home_redacted: redact_home(),
            config_dir_hint: config_dir_hint(),
        }
    }

    /// Format as a multi-line diagnostic string suitable for clipboard copy.
    ///
    /// ```
    /// use forskscope_desktop::platform::PlatformInfo;
    /// let info = PlatformInfo::collect();
    /// let s = info.to_report();
    /// assert!(s.contains("ForskScope"));
    /// assert!(s.contains("OS:"));
    /// ```
    pub fn to_report(&self) -> String {
        format!(
            "ForskScope {}\nRust: {}\nOS: {} ({})\nArch: {}\nCPUs: {}\nHome: {}\nConfig: {}",
            self.app_version,
            self.rustc_version,
            self.os,
            self.target_triple,
            self.arch,
            self.logical_cpus,
            self.home_redacted,
            self.config_dir_hint,
        )
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn redact_home() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "unknown".into());

    if home == "unknown" {
        return home;
    }

    // Keep only the last component (e.g. `/home/<user>` → `/home/***`)
    let sep = if cfg!(windows) { '\\' } else { '/' };
    if let Some(idx) = home.rfind(sep) {
        format!("{}{}***", &home[..idx], sep)
    } else {
        "***".into()
    }
}

fn config_dir_hint() -> String {
    // Prefer XDG on Linux; fall back to ~/.config
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| "~/.config (default)".into())
    }
    #[cfg(target_os = "macos")]
    {
        "~/Library/Application Support".into()
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").unwrap_or_else(|_| "%APPDATA%".into())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        "unknown".into()
    }
}
