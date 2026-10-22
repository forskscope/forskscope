//! `forskscope mcp --root <dir> [--root <dir> …] [--allow-show]`
//! (RFC-094 §4, RFC-096): an MCP server over standard input/output, for
//! AI agents. Hand-written, with no new dependency (review 182 §1's
//! decision, after measuring `rmcp` — see
//! `.git-exclude/review-request/162a-mcp-library.md`), at protocol
//! revision `2025-11-25` (review 182 §1: the newest revision that still
//! has an `initialize` handshake; `rmcp`'s own doc comment names
//! `2026-07-28` as the one that replaced it with per-request `_meta`
//! (SEP-2567), which this implementation does not speak).
//!
//! Stateless and read-only, like `compare`: no settings read, no session
//! written, and `--root` is the agent's whole world (`mcp::roots`) —
//! every path it supplies is resolved and checked against the roots
//! before anything is read.

mod jsonrpc;
mod roots;
mod tools;

use std::path::PathBuf;

pub fn run(args: &[String]) -> i32 {
    // RFC-096 §1: `forskscope mcp --help` shows this subcommand's own
    // options, not the app's.
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return 0;
    }

    let (roots, allow_show) = match parse_args(args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("forskscope mcp: {msg}");
            return 2;
        }
    };
    jsonrpc::serve(roots, allow_show);
    0
}

fn print_help() {
    print!(
        "Usage:\n  \
         forskscope mcp --root <dir> [--root <dir> …] [--allow-show]\n\n\
         \x20 --root <dir>    A directory the agent may compare within or\n  \
         \x20                 below; repeatable, at least one is required\n  \
         \x20 --allow-show    Also offer the show_to_person tool, which\n  \
         \x20                 opens the app on a pair for a person to see\n"
    );
}

fn parse_args(args: &[String]) -> Result<(Vec<PathBuf>, bool), String> {
    let mut roots = Vec::new();
    let mut allow_show = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--root" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--root requires a value".to_string())?;
                let canonical =
                    std::fs::canonicalize(value).map_err(|e| format!("--root {value}: {e}"))?;
                if !canonical.is_dir() {
                    return Err(format!("--root {value}: not a directory"));
                }
                roots.push(canonical);
            }
            "--allow-show" => allow_show = true,
            other => return Err(format!("unrecognized option `{other}`")),
        }
        i += 1;
    }
    if roots.is_empty() {
        return Err("at least one --root is required".to_string());
    }
    Ok((roots, allow_show))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_least_one_root_is_required() {
        let err = parse_args(&[]).unwrap_err();
        assert!(err.contains("--root"));
    }

    #[test]
    fn a_nonexistent_root_is_refused_before_the_server_ever_starts() {
        let args = vec!["--root".to_string(), "/does/not/exist/at/all".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn a_root_that_is_a_file_not_a_directory_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("f.txt");
        std::fs::write(&file, "x").unwrap();
        let args = vec!["--root".to_string(), file.to_str().unwrap().to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn multiple_roots_and_allow_show_are_both_parsed() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let args = vec![
            "--root".to_string(),
            a.path().to_str().unwrap().to_string(),
            "--root".to_string(),
            b.path().to_str().unwrap().to_string(),
            "--allow-show".to_string(),
        ];
        let (roots, allow_show) = parse_args(&args).unwrap();
        assert_eq!(roots.len(), 2);
        assert!(allow_show);
    }

    #[test]
    fn allow_show_defaults_to_false() {
        let tmp = tempfile::tempdir().unwrap();
        let args = vec![
            "--root".to_string(),
            tmp.path().to_str().unwrap().to_string(),
        ];
        let (_, allow_show) = parse_args(&args).unwrap();
        assert!(!allow_show);
    }
}
