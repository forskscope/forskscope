//! RFC-091 §1: where ForskScope keeps its data, decided once at startup,
//! before any settings, session or web-view path is resolved.
//!
//! Two pure functions carry the whole decision: [`extract_data_location_flags`]
//! pulls `--private` and `--data-dir <path>` out of argv (ahead of the
//! positional file arguments, the same way `--version` already is), and
//! [`resolve_data_location`] turns those flags, [`Platform`], and two facts
//! about the running program — its real folder, and whether a
//! `forskscope-data` folder sits beside it — into one [`DataLocation`].
//! Neither touches the filesystem or the environment; the caller
//! (`forskscope-ui::startup_mode`/`startup_resolve`) supplies all of it,
//! which is where `std::env::current_exe`, `std::fs` and `cfg!(target_os)`
//! actually run.

use std::fmt;
use std::path::{Path, PathBuf};

/// Where ForskScope keeps everything it writes, decided once and never
/// reconsidered for the life of the run (RFC-091 §1: "The decision is made
/// once... so all four kinds of data always agree on where they are").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataLocation {
    /// `--private`: nothing kept (RFC-091 §3).
    Private,
    /// `--data-dir <path>`, or a `forskscope-data` folder beside the
    /// program — everything goes inside this folder (RFC-091 §2).
    Portable(PathBuf),
    /// Today's behaviour, unchanged: the platform config/data folders.
    Installed,
}

/// The platform fact [`resolve_data_location`] needs for RFC-091 §2a, passed
/// in rather than read internally (`cfg!(target_os = ...)`) so the decision
/// stays a pure function of its arguments and acceptance criterion 9's
/// resolver test can exercise macOS from any machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// `dioxus-desktop` neither applies the data directory to WKWebView nor
    /// exposes a non-persistent store there, so neither mode's promise can
    /// be kept (RFC-091 §2a).
    MacOs,
    /// Every other platform this project ships on. Windows gets this same
    /// refusal too, the RFC says, if its own WebView2 cache is ever shown
    /// to escape `with_data_directory` — not true today (measured).
    Other,
}

/// Which flag [`DataLocationError::NotAvailableOnPlatform`] refused, so its
/// `Display` can name the one the caller actually typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupFlag {
    Private,
    DataDir,
}

/// Why [`resolve_data_location`] refused to decide, before any window
/// opens (RFC-091 §1's contradiction, and §2a's platform refusal) — the
/// caller exits 2 on either, printing the message below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataLocationError {
    /// `--private` and `--data-dir` name opposite destinations for the same
    /// decision — RFC-091 §1 calls this a contradiction, not a priority to
    /// break by picking one.
    PrivateAndDataDir,
    /// RFC-091 §2a: this platform's system web view cannot keep this mode's
    /// promise, so the mode does not start rather than run with a promise
    /// that silently does not hold.
    NotAvailableOnPlatform(StartupFlag),
}

impl fmt::Display for DataLocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataLocationError::PrivateAndDataDir => write!(
                f,
                "--private and --data-dir cannot be combined: --private starts \
                 from nothing and keeps nothing, --data-dir names a folder to \
                 keep everything in. Choose one."
            ),
            DataLocationError::NotAvailableOnPlatform(StartupFlag::Private) => write!(
                f,
                "--private is not available on macOS: the system web view keeps \
                 its own data in your home folder, so ForskScope cannot promise \
                 to save nothing."
            ),
            DataLocationError::NotAvailableOnPlatform(StartupFlag::DataDir) => write!(
                f,
                "--data-dir is not available on macOS: the system web view keeps \
                 its own data in your home folder, so ForskScope cannot keep \
                 everything in the chosen folder."
            ),
        }
    }
}

impl std::error::Error for DataLocationError {}

/// RFC-091 §1's resolution table, row by row, the first match winning,
/// with §2a's platform refusal checked for either flag before its row would
/// otherwise succeed:
///
/// | # | Condition | Mode |
/// |---|---|---|
/// | 1 | `private` | Private (refused on macOS) |
/// | 2 | `data_dir` given | Portable(that path) (refused on macOS) |
/// | 3 | `marker_exists`, not macOS | Portable(`program_dir`/`forskscope-data`) |
/// | 4 | otherwise | Installed |
///
/// Row 3 is simply **ignored** on macOS, not refused — no flag was typed,
/// so there is nothing to print an error about; it falls through to
/// Installed the same as a marker that is absent (RFC-091 §2: "The marker
/// folder is ignored there").
///
/// `program_dir` and `marker_exists` are facts the caller establishes
/// (`std::env::current_exe`, symlinks resolved, and an existence check) —
/// this function only arranges them into a decision, so the arrangement
/// itself is testable without touching a real filesystem.
pub fn resolve_data_location(
    platform: Platform,
    private: bool,
    data_dir: Option<PathBuf>,
    program_dir: Option<&Path>,
    marker_exists: bool,
) -> Result<DataLocation, DataLocationError> {
    if private && data_dir.is_some() {
        return Err(DataLocationError::PrivateAndDataDir);
    }
    if private {
        if platform == Platform::MacOs {
            return Err(DataLocationError::NotAvailableOnPlatform(
                StartupFlag::Private,
            ));
        }
        return Ok(DataLocation::Private);
    }
    if let Some(dir) = data_dir {
        if platform == Platform::MacOs {
            return Err(DataLocationError::NotAvailableOnPlatform(
                StartupFlag::DataDir,
            ));
        }
        return Ok(DataLocation::Portable(dir));
    }
    if platform != Platform::MacOs
        && marker_exists
        && let Some(dir) = program_dir
    {
        return Ok(DataLocation::Portable(dir.join("forskscope-data")));
    }
    Ok(DataLocation::Installed)
}

/// `--private` and `--data-dir <path>`, parsed out of argv. Everything else
/// about this run's arguments — the file paths, `--version`, `--help` — is
/// someone else's job; this type carries only RFC-091's own two flags.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataLocationFlags {
    pub private: bool,
    pub data_dir: Option<PathBuf>,
}

/// Why [`extract_data_location_flags`] refused an argument list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagParseError {
    /// `--data-dir` was the last argument, with no path after it.
    DataDirMissingValue,
    /// A `--`-prefixed token that is none of this function's own flags.
    /// RFC-091 §5: "An unknown `--` flag is an error, not a file name" — the
    /// alternative, letting it fall through to `parse_startup_args` as a
    /// literal path, would fail later with a confusing "file not found"
    /// instead of naming the actual mistake.
    Unknown(String),
}

impl fmt::Display for FlagParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlagParseError::DataDirMissingValue => {
                write!(f, "--data-dir requires a path argument")
            }
            FlagParseError::Unknown(flag) => write!(f, "unknown flag {flag:?}"),
        }
    }
}

impl std::error::Error for FlagParseError {}

/// Pulls `--private` and `--data-dir <path>` out of `args`, wherever they
/// appear, and returns them alongside the remaining arguments in their
/// original relative order — what `parse_startup_args` sees next. Parsed
/// first, the same way `--version`/`--help`/`--diagnostics` already are
/// (RFC-091 §5), so `forskscope --private a.txt b.txt` still opens a
/// comparison, privately.
pub fn extract_data_location_flags(
    args: &[String],
) -> Result<(DataLocationFlags, Vec<String>), FlagParseError> {
    let mut flags = DataLocationFlags::default();
    let mut rest = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--private" => {
                flags.private = true;
                i += 1;
            }
            "--data-dir" => {
                let value = args.get(i + 1).ok_or(FlagParseError::DataDirMissingValue)?;
                flags.data_dir = Some(PathBuf::from(value));
                i += 2;
            }
            s if s.starts_with("--") => {
                return Err(FlagParseError::Unknown(s.to_string()));
            }
            _ => {
                rest.push(args[i].clone());
                i += 1;
            }
        }
    }
    Ok((flags, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── resolve_data_location: RFC-091 §1's table, row by row ──────────────

    #[test]
    fn row_1_private_wins_regardless_of_the_other_rows() {
        let result = resolve_data_location(
            Platform::Other,
            true,
            None,
            Some(Path::new("/opt/forskscope")),
            true, // even with a marker present, private still wins
        );
        assert_eq!(result, Ok(DataLocation::Private));
    }

    #[test]
    fn row_2_data_dir_wins_over_a_marker() {
        let result = resolve_data_location(
            Platform::Other,
            false,
            Some(PathBuf::from("/mnt/usb/data")),
            Some(Path::new("/opt/forskscope")),
            true,
        );
        assert_eq!(
            result,
            Ok(DataLocation::Portable(PathBuf::from("/mnt/usb/data")))
        );
    }

    #[test]
    fn row_3_the_marker_names_a_forskscope_data_folder_beside_the_program() {
        let result = resolve_data_location(
            Platform::Other,
            false,
            None,
            Some(Path::new("/opt/forskscope")),
            true,
        );
        assert_eq!(
            result,
            Ok(DataLocation::Portable(PathBuf::from(
                "/opt/forskscope/forskscope-data"
            )))
        );
    }

    #[test]
    fn row_3_no_marker_and_no_program_dir_both_fall_through_to_installed() {
        // A marker check that could not even be attempted (no program
        // folder) must not be treated as "found" — it must fall to row 4,
        // the same as a marker that was checked and is absent.
        assert_eq!(
            resolve_data_location(Platform::Other, false, None, None, false),
            Ok(DataLocation::Installed)
        );
        assert_eq!(
            resolve_data_location(Platform::Other, false, None, None, true),
            Ok(DataLocation::Installed)
        );
    }

    #[test]
    fn row_4_otherwise_installed() {
        let result = resolve_data_location(
            Platform::Other,
            false,
            None,
            Some(Path::new("/opt/forskscope")),
            false,
        );
        assert_eq!(result, Ok(DataLocation::Installed));
    }

    #[test]
    fn private_with_data_dir_is_a_conflict_not_a_priority_order() {
        let result = resolve_data_location(
            Platform::Other,
            true,
            Some(PathBuf::from("/mnt/usb/data")),
            Some(Path::new("/opt/forskscope")),
            false,
        );
        assert_eq!(result, Err(DataLocationError::PrivateAndDataDir));
    }

    #[test]
    fn the_conflict_message_names_both_flags() {
        let message = DataLocationError::PrivateAndDataDir.to_string();
        assert!(message.contains("--private"));
        assert!(message.contains("--data-dir"));
    }

    // ── §2a: macOS refuses either flag, falsified with the platform as a
    //    parameter (acceptance criterion 9) ────────────────────────────────

    #[test]
    fn macos_refuses_private_naming_the_flag_and_the_platform() {
        let result = resolve_data_location(Platform::MacOs, true, None, None, false);
        assert_eq!(
            result,
            Err(DataLocationError::NotAvailableOnPlatform(
                StartupFlag::Private
            ))
        );
        let message = result.unwrap_err().to_string();
        assert!(message.contains("--private"));
        assert!(message.contains("macOS"));
    }

    #[test]
    fn macos_refuses_data_dir_naming_the_flag_and_the_platform() {
        let result = resolve_data_location(
            Platform::MacOs,
            false,
            Some(PathBuf::from("/Volumes/usb/data")),
            None,
            false,
        );
        assert_eq!(
            result,
            Err(DataLocationError::NotAvailableOnPlatform(
                StartupFlag::DataDir
            ))
        );
        let message = result.unwrap_err().to_string();
        assert!(message.contains("--data-dir"));
        assert!(message.contains("macOS"));
    }

    #[test]
    fn macos_ignores_the_marker_folder_rather_than_refusing_it() {
        // No flag was typed here, so there is nothing to refuse — row 3 is
        // simply skipped on macOS and this falls through to Installed, the
        // same outcome as a marker that was checked and found absent.
        let result = resolve_data_location(
            Platform::MacOs,
            false,
            None,
            Some(Path::new("/Applications/ForskScope.app/Contents/MacOS")),
            true,
        );
        assert_eq!(result, Ok(DataLocation::Installed));
    }

    #[test]
    fn macos_private_and_data_dir_together_is_still_the_conflict_not_the_platform_refusal() {
        // The contradiction is checked first, and does not depend on the
        // platform — refusing it must not be read as a macOS-specific
        // message when the real problem is the two flags together.
        let result = resolve_data_location(
            Platform::MacOs,
            true,
            Some(PathBuf::from("/Volumes/usb/data")),
            None,
            false,
        );
        assert_eq!(result, Err(DataLocationError::PrivateAndDataDir));
    }

    // ── extract_data_location_flags ─────────────────────────────────────

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_flags_leaves_every_argument_untouched_and_in_order() {
        let (flags, rest) = extract_data_location_flags(&args(&["left.txt", "right.txt"])).unwrap();
        assert_eq!(flags, DataLocationFlags::default());
        assert_eq!(rest, args(&["left.txt", "right.txt"]));
    }

    #[test]
    fn private_is_extracted_and_the_files_survive_in_order() {
        let (flags, rest) =
            extract_data_location_flags(&args(&["--private", "a.txt", "b.txt"])).unwrap();
        assert!(flags.private);
        assert_eq!(flags.data_dir, None);
        assert_eq!(rest, args(&["a.txt", "b.txt"]));
    }

    #[test]
    fn data_dir_consumes_its_own_value_and_leaves_the_files_in_order() {
        let (flags, rest) =
            extract_data_location_flags(&args(&["a.txt", "--data-dir", "/mnt/usb", "b.txt"]))
                .unwrap();
        assert_eq!(flags.data_dir, Some(PathBuf::from("/mnt/usb")));
        assert!(!flags.private);
        assert_eq!(rest, args(&["a.txt", "b.txt"]));
    }

    #[test]
    fn data_dir_with_no_following_argument_is_an_error() {
        let result = extract_data_location_flags(&args(&["a.txt", "--data-dir"]));
        assert_eq!(result, Err(FlagParseError::DataDirMissingValue));
    }

    #[test]
    fn an_unrecognized_double_dash_flag_is_rejected_not_treated_as_a_file() {
        let result = extract_data_location_flags(&args(&["--typo", "a.txt"]));
        assert_eq!(result, Err(FlagParseError::Unknown("--typo".to_string())));
    }

    #[test]
    fn a_single_dash_argument_is_left_alone_not_rejected() {
        // Short flags (-V, -h) are handled by a different, earlier check in
        // main() — this function only owns `--`-prefixed tokens.
        let (_, rest) = extract_data_location_flags(&args(&["-V"])).unwrap();
        assert_eq!(rest, args(&["-V"]));
    }

    #[test]
    fn both_flags_extract_together_leaving_resolve_data_location_to_judge_the_combination() {
        // --private and --data-dir together is a conflict, but that's
        // resolve_data_location's call to make, not this function's —
        // extraction succeeds and hands both flags onward unexamined.
        let (flags, rest) = extract_data_location_flags(&args(&[
            "--private",
            "--data-dir",
            "/x",
            "left.txt",
            "right.txt",
        ]))
        .unwrap();
        assert!(flags.private);
        assert_eq!(flags.data_dir, Some(PathBuf::from("/x")));
        assert_eq!(rest, args(&["left.txt", "right.txt"]));
    }
}
