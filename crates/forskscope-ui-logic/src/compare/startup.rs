//! CLI startup request parsing and mergetool-to-compare conversion
//! (RFC-077, audit finding B3).
//!
//! Replaces the previous loosely-coupled `STARTUP_PAIR`/`STARTUP_MERGED`
//! `OnceLock` pair with one typed request, parsed once and validated —
//! an unsupported argument count is now a startup error, not a silent
//! fallback to the Explorer workspace.
//!
//! [`StartupRequest::into_compare_request`] is where the RFC-077 model
//! boundary actually lives: normal compare saves to its right input;
//! mergetool compares local/remote but saves to the distinct merged output.
//! Nothing downstream of this conversion ever needs to know it came from a
//! three-argument launch — it sees one [`CompareRequest`] either way.

use std::fmt;
use std::path::PathBuf;

/// What ForskScope should do at startup, parsed from CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupRequest {
    /// No arguments: open the Explorer workspace.
    Explorer,
    /// Two arguments: normal two-file diff (`git difftool` compatible).
    Compare { left: PathBuf, right: PathBuf },
    /// Three arguments: Git mergetool (`local remote merged`) — diff local
    /// vs remote, save the result to `merged`.
    MergeTool {
        local: PathBuf,
        remote: PathBuf,
        merged: PathBuf,
    },
}

/// An unsupported argument count. `--diagnostics` is handled separately by
/// the caller, before this parser ever sees the argument list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupArgError {
    pub arg_count: usize,
}

impl fmt::Display for StartupArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected 0 arguments (Explorer), 2 (two-file compare: <left> <right>), \
             or 3 (git mergetool: <local> <remote> <merged>) — got {}",
            self.arg_count
        )
    }
}

impl std::error::Error for StartupArgError {}

/// Parses positional CLI arguments (already stripped of `argv[0]` and any
/// `--diagnostics` flag) into a [`StartupRequest`]. Any arity other than
/// 0/2/3 is rejected rather than silently opening the Explorer workspace —
/// RFC-077: "Argument parsing must reject unsupported arity with
/// diagnostics and a non-zero exit."
pub fn parse_startup_args(args: &[String]) -> Result<StartupRequest, StartupArgError> {
    match args {
        [] => Ok(StartupRequest::Explorer),
        [left, right] => Ok(StartupRequest::Compare {
            left: PathBuf::from(left),
            right: PathBuf::from(right),
        }),
        [local, remote, merged] => Ok(StartupRequest::MergeTool {
            local: PathBuf::from(local),
            remote: PathBuf::from(remote),
            merged: PathBuf::from(merged),
        }),
        other => Err(StartupArgError {
            arg_count: other.len(),
        }),
    }
}

/// What is compared, and where a save goes — the single field that used to
/// be split ambiguously across `right_path` and a post-hoc `STARTUP_MERGED`
/// mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareRequest {
    pub left_input: PathBuf,
    pub right_input: PathBuf,
    pub save_destination: SaveDestination,
}

/// Where a successful save writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveDestination {
    /// Normal two-file mode: save overwrites the compared right input.
    RightInput,
    /// Git mergetool mode: save writes to a path distinct from anything
    /// being compared.
    Explicit(PathBuf),
}

impl StartupRequest {
    /// Converts a startup request into what will actually be compared and
    /// saved. `Explorer` has nothing to compare, so this returns `None` for
    /// it — the caller's branch on `Some`/`None` is the same shape as
    /// today's `STARTUP_PAIR.get()` check, just typed.
    pub fn into_compare_request(self) -> Option<CompareRequest> {
        match self {
            StartupRequest::Explorer => None,
            StartupRequest::Compare { left, right } => Some(CompareRequest {
                left_input: left,
                right_input: right,
                save_destination: SaveDestination::RightInput,
            }),
            StartupRequest::MergeTool {
                local,
                remote,
                merged,
            } => Some(CompareRequest {
                left_input: local,
                right_input: remote,
                save_destination: SaveDestination::Explicit(merged),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    // ── parse_startup_args ───────────────────────────────────────────────

    #[test]
    fn no_arguments_is_explorer() {
        assert_eq!(parse_startup_args(&args(&[])), Ok(StartupRequest::Explorer));
    }

    #[test]
    fn two_arguments_is_compare() {
        let result = parse_startup_args(&args(&["left.txt", "right.txt"]));
        assert_eq!(
            result,
            Ok(StartupRequest::Compare {
                left: PathBuf::from("left.txt"),
                right: PathBuf::from("right.txt"),
            })
        );
    }

    #[test]
    fn three_arguments_is_mergetool() {
        let result = parse_startup_args(&args(&["local.txt", "remote.txt", "merged.txt"]));
        assert_eq!(
            result,
            Ok(StartupRequest::MergeTool {
                local: PathBuf::from("local.txt"),
                remote: PathBuf::from("remote.txt"),
                merged: PathBuf::from("merged.txt"),
            })
        );
    }

    #[test]
    fn one_argument_is_rejected_not_silently_explorer() {
        let result = parse_startup_args(&args(&["only-one.txt"]));
        assert_eq!(result, Err(StartupArgError { arg_count: 1 }));
    }

    #[test]
    fn four_arguments_is_rejected_not_silently_explorer() {
        let result = parse_startup_args(&args(&["a", "b", "c", "d"]));
        assert_eq!(result, Err(StartupArgError { arg_count: 4 }));
    }

    #[test]
    fn startup_arg_error_message_is_non_empty_and_names_the_count() {
        let err = StartupArgError { arg_count: 5 };
        let message = err.to_string();
        assert!(!message.is_empty());
        assert!(message.contains('5'));
    }

    // ── into_compare_request ─────────────────────────────────────────────

    #[test]
    fn explorer_has_nothing_to_compare() {
        assert_eq!(StartupRequest::Explorer.into_compare_request(), None);
    }

    #[test]
    fn compare_saves_to_the_right_input() {
        let request = StartupRequest::Compare {
            left: PathBuf::from("left.txt"),
            right: PathBuf::from("right.txt"),
        };
        assert_eq!(
            request.into_compare_request(),
            Some(CompareRequest {
                left_input: PathBuf::from("left.txt"),
                right_input: PathBuf::from("right.txt"),
                save_destination: SaveDestination::RightInput,
            })
        );
    }

    #[test]
    fn mergetool_compares_local_vs_remote_but_saves_to_merged() {
        let request = StartupRequest::MergeTool {
            local: PathBuf::from("local.txt"),
            remote: PathBuf::from("remote.txt"),
            merged: PathBuf::from("merged.txt"),
        };
        assert_eq!(
            request.into_compare_request(),
            Some(CompareRequest {
                left_input: PathBuf::from("local.txt"),
                right_input: PathBuf::from("remote.txt"),
                save_destination: SaveDestination::Explicit(PathBuf::from("merged.txt")),
            })
        );
    }

    #[test]
    fn mergetool_save_destination_is_never_the_compared_right_input() {
        // The exact defect RFC-077 closes: the save target must be a
        // genuinely distinct identity, not aliased to right_input.
        let request = StartupRequest::MergeTool {
            local: PathBuf::from("local.txt"),
            remote: PathBuf::from("remote.txt"),
            merged: PathBuf::from("merged.txt"),
        };
        let compare = request.into_compare_request().unwrap();
        match compare.save_destination {
            SaveDestination::Explicit(path) => {
                assert_ne!(path, compare.right_input);
            }
            SaveDestination::RightInput => panic!("mergetool must never save to the right input"),
        }
    }
}
