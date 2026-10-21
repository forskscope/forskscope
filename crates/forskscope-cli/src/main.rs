//! `forskscope-cli` — headless ForskScope for scripts and AI agents
//! (RFC-094). Depends only on `forskscope-compare` and `forskscope-report`,
//! enforced by `cargo xtask crate-boundaries` (RFC-093 §3).
//!
//! ```text
//! forskscope-cli compare <left> <right> [options]
//! forskscope-cli mcp --root <dir> [--root <dir> …] [--allow-show]
//! forskscope-cli --version | --help
//! ```
//!
//! Hand-written argument parsing, like the GUI's — no parser framework
//! (RFC-094 §1). Reads no settings, writes no session: every run starts
//! from nothing and ends with nothing kept (RFC-094's "stateless" property).

mod compare;
mod mcp;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("forskscope-cli {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    let exit_code = match args.first().map(String::as_str) {
        Some("compare") => compare::run(&args[1..]),
        Some("mcp") => mcp::run(&args[1..]),
        Some(other) => {
            eprintln!("forskscope-cli: unrecognized command `{other}` (see --help)");
            2
        }
        None => {
            eprintln!("forskscope-cli: a command is required (see --help)");
            2
        }
    };
    std::process::exit(exit_code);
}

fn print_help() {
    print!(
        "Usage:\n  \
         forskscope-cli compare <left> <right> [options]   Compare two files or two folders\n  \
         forskscope-cli mcp --root <dir> [--root <dir> …] [--allow-show]\n  \
         \x20                                                   Serve MCP over stdio for an AI agent\n  \
         forskscope-cli --version, -V                       Print the version and exit\n  \
         forskscope-cli --help, -h                           Print this message and exit\n\n\
         compare options:\n  \
         \x20 --format text|json|patch|html   Output format (default: text)\n  \
         \x20 --algorithm myers|patience|lcs|histogram   Diff algorithm (default: myers)\n  \
         \x20 --ignore-whitespace             Ignore whitespace differences\n  \
         \x20 --ignore-case                   Ignore case differences\n  \
         \x20 --ignore-newlines               Ignore newline-style (CRLF/LF) differences\n  \
         \x20 --context <n>                   Unified-diff context lines (default: 3)\n  \
         \x20 --ignore-ext <a,b,...>          Folder comparison: skip these extensions\n  \
         \x20 --ignore-dir <a,b,...>          Folder comparison: skip these directory names\n  \
         \x20 --hide-dotfiles                 Folder comparison: skip hidden entries\n  \
         \x20 --paths names|relative|full     How paths are shown (default: names)\n  \
         \x20 --max-files <n>                 Folder JSON: cap on files with full change content (default: 50)\n  \
         \x20 --max-change-bytes <n>          Folder JSON: cap on change text bytes (default: 5242880)\n\n\
         Exit codes: 0 same, 1 different, 2 error or refusal.\n"
    );
}
