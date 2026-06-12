//! Runtime platform diagnostic information (RFC-026 §"Diagnostics panel").
//!
//! [`PlatformInfo`] collects environment facts that help users and maintainers
//! diagnose build and runtime issues. It is purely read-only — it inspects
//! compile-time constants and environment variables; it never writes anything.
//!
//! The About / Diagnostics panel copies this to the clipboard so users can
//! paste it into bug reports without needing a terminal.

/// Runtime and compile-time platform information.
///
/// All fields are `String` so the struct is straightforwardly serialisable
/// and displayable without additional formatting helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInfo {
    /// ForskScope version (`CARGO_PKG_VERSION`).
    pub app_version:      String,
    /// Rust compiler version used to build this binary (`VERGEN_RUSTC_SEMVER`
    /// if available; falls back to `"unknown"`).
    pub rustc_version:    String,
    /// Target triple this binary was compiled for (e.g. `x86_64-unknown-linux-gnu`).
    pub target_triple:    String,
    /// OS family as reported by `std::env::consts::OS` (`"linux"`, `"windows"`, `"macos"`, …).
    pub os:               String,
    /// CPU architecture (`std::env::consts::ARCH`).
    pub arch:             String,
    /// Number of logical CPUs (`std::thread::available_parallelism`, best-effort).
    pub logical_cpus:     String,
    /// Value of `$HOME` or `%USERPROFILE%`, redacted after the last path separator
    /// to avoid leaking the username in bug reports.
    pub home_redacted:    String,
    /// Value of `$XDG_DATA_HOME` or platform default config dir prefix (Linux only).
    pub config_dir_hint:  String,
}

impl PlatformInfo {
    /// Collect current platform information.
    pub fn collect() -> Self {
        Self {
            app_version:   env!("CARGO_PKG_VERSION").into(),
            rustc_version: option_env!("VERGEN_RUSTC_SEMVER")
                .unwrap_or("unknown")
                .into(),
            target_triple: std::env::consts::FAMILY.into(), // approximation; see note
            os:            std::env::consts::OS.into(),
            arch:          std::env::consts::ARCH.into(),
            logical_cpus:  std::thread::available_parallelism()
                .map(|n| n.get().to_string())
                .unwrap_or_else(|_| "unknown".into()),
            home_redacted: redact_home(),
            config_dir_hint: config_dir_hint(),
        }
    }

    /// Format as a multi-line diagnostic string suitable for clipboard copy.
    ///
    /// ```
    /// use forskscope_core::platform::PlatformInfo;
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

    if home == "unknown" { return home; }

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
        std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| "~/.config (default)".into())
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
