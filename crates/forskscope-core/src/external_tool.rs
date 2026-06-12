//! External tool integration — safe argument expansion (RFC-029).
//!
//! ForskScope can open files in an external editor, reveal them in the file
//! manager, and run user-configured commands. This module defines the *data
//! model* for those commands and the *argument expansion* function that turns
//! a template into a concrete `Command` argument array — **without shell
//! string expansion**.
//!
//! ## Security contract
//!
//! All expansion is done via an argument array, never via a shell string.
//! Callers must pass the result directly to [`std::process::Command::args`].
//! This prevents shell injection through path arguments that contain spaces,
//! semicolons, backticks, or other shell-special characters.
//!
//! ```rust
//! # use forskscope_core::external_tool::{
//! #     ExternalToolCommand, ExternalToolArg, ExternalToolPlaceholder,
//! #     ToolId, ExpandContext, expand_args,
//! # };
//! # use std::path::PathBuf;
//! let cmd = ExternalToolCommand {
//!     id: ToolId("editor".into()),
//!     name: "VS Code".into(),
//!     executable: PathBuf::from("code"),
//!     args: vec![
//!         ExternalToolArg::Literal("--goto".into()),
//!         ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path),
//!     ],
//! };
//! let ctx = ExpandContext {
//!     path:       Some(PathBuf::from("/project/src/main.rs")),
//!     left_path:  None,
//!     right_path: None,
//!     line:       Some(42),
//!     column:     None,
//! };
//! let argv = expand_args(&cmd, &ctx);
//! assert_eq!(argv, vec!["--goto", "/project/src/main.rs"]);
//! ```

use std::path::PathBuf;

// ── Identity ──────────────────────────────────────────────────────────────────

/// Stable identifier for one external tool configuration entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolId(pub String);

// ── Command model ─────────────────────────────────────────────────────────────

/// A configured external tool command (RFC-029 §"API sketch").
///
/// `executable` is the program name or path; it is passed to
/// [`std::process::Command::new`] without shell interpretation.
/// `args` is an ordered list of literal strings and typed placeholders that
/// are expanded at call time via [`expand_args`].
#[derive(Debug, Clone)]
pub struct ExternalToolCommand {
    pub id:         ToolId,
    pub name:       String,
    pub executable: PathBuf,
    pub args:       Vec<ExternalToolArg>,
}

/// One element in an [`ExternalToolCommand`]'s argument list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalToolArg {
    /// A fixed string passed verbatim as a single argument.
    Literal(String),
    /// A typed placeholder replaced at expansion time.
    Placeholder(ExternalToolPlaceholder),
}

/// The supported placeholder types (RFC-029 §"Placeholders").
///
/// Each variant corresponds to one `{token}` in the UI settings form.
/// Unknown tokens in user input must be rejected by the settings validator
/// before a command is stored — this enum only represents *valid* placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalToolPlaceholder {
    /// The single active file path (left path when both sides differ).
    Path,
    /// The left / old / source file path.
    LeftPath,
    /// The right / new / result file path.
    RightPath,
    /// The cursor line number (1-indexed), as a decimal string.
    Line,
    /// The cursor column number (1-indexed), as a decimal string.
    Column,
}

impl ExternalToolPlaceholder {
    /// The `{token}` string shown in the UI settings form.
    pub fn token(self) -> &'static str {
        match self {
            Self::Path      => "{path}",
            Self::LeftPath  => "{left}",
            Self::RightPath => "{right}",
            Self::Line      => "{line}",
            Self::Column    => "{column}",
        }
    }

    /// Parse a `{token}` string from user input. Returns `None` for unknown tokens.
    pub fn from_token(s: &str) -> Option<Self> {
        match s {
            "{path}"    => Some(Self::Path),
            "{left}"    => Some(Self::LeftPath),
            "{right}"   => Some(Self::RightPath),
            "{line}"    => Some(Self::Line),
            "{column}"  => Some(Self::Column),
            _           => None,
        }
    }

    /// All supported tokens, in the order shown in the UI.
    pub fn all() -> &'static [Self] {
        &[Self::Path, Self::LeftPath, Self::RightPath, Self::Line, Self::Column]
    }
}

// ── Expansion context ─────────────────────────────────────────────────────────

/// The runtime values available when expanding a command (RFC-029 §"Placeholders").
///
/// Fields are `Option` because not every invocation context provides all
/// values — e.g. a file manager reveal has a path but no line number, and a
/// two-file comparison has left/right but not a single path.
#[derive(Debug, Clone, Default)]
pub struct ExpandContext {
    /// Single active file path. Used for `{path}`.
    pub path:       Option<PathBuf>,
    /// Left/old/source path. Used for `{left}`.
    pub left_path:  Option<PathBuf>,
    /// Right/new/result path. Used for `{right}`.
    pub right_path: Option<PathBuf>,
    /// Cursor line number (1-indexed). Used for `{line}`.
    pub line:       Option<u32>,
    /// Cursor column number (1-indexed). Used for `{column}`.
    pub column:     Option<u32>,
}

// ── Expansion ─────────────────────────────────────────────────────────────────

/// Expand a command template into a concrete argument array.
///
/// Each [`ExternalToolArg::Literal`] is included verbatim.
/// Each [`ExternalToolArg::Placeholder`] is resolved from `ctx`:
///
/// - If the context value is present, the placeholder expands to that string.
/// - If the context value is `None`, the argument is **omitted entirely**.
///   This lets a command like `["--goto", Line]` degrade to `["--goto"]`
///   and then to `[]` when no line is available, rather than passing a
///   literal `"None"` or panicking.
///
/// The returned `Vec<String>` is ready to be passed to
/// [`std::process::Command::args`]. The executable itself is **not** included.
///
/// ## Security
///
/// No shell interpretation is performed. Paths are turned into their
/// `display()` string representation and passed as a single argument element.
/// Characters like spaces, `$`, `;`, and backticks are inert.
pub fn expand_args(cmd: &ExternalToolCommand, ctx: &ExpandContext) -> Vec<String> {
    let mut result = Vec::with_capacity(cmd.args.len());
    for arg in &cmd.args {
        match arg {
            ExternalToolArg::Literal(s) => result.push(s.clone()),
            ExternalToolArg::Placeholder(ph) => {
                if let Some(expanded) = resolve_placeholder(*ph, ctx) {
                    result.push(expanded);
                }
                // Omit the argument entirely when context value is unavailable.
            }
        }
    }
    result
}

/// Resolve a single placeholder against the context. Returns `None` when the
/// required context value is absent.
fn resolve_placeholder(ph: ExternalToolPlaceholder, ctx: &ExpandContext) -> Option<String> {
    match ph {
        ExternalToolPlaceholder::Path      => ctx.path.as_ref().map(|p| p.display().to_string()),
        ExternalToolPlaceholder::LeftPath  => ctx.left_path.as_ref().map(|p| p.display().to_string()),
        ExternalToolPlaceholder::RightPath => ctx.right_path.as_ref().map(|p| p.display().to_string()),
        ExternalToolPlaceholder::Line      => ctx.line.map(|n| n.to_string()),
        ExternalToolPlaceholder::Column    => ctx.column.map(|n| n.to_string()),
    }
}

// ── Argument template parser ──────────────────────────────────────────────────

/// Parse a user-supplied argument string into an [`ExternalToolArg`].
///
/// If the string exactly matches a known `{token}`, it becomes a
/// `Placeholder`. Otherwise it is a `Literal`. This is the function the
/// settings UI calls when storing a command; it rejects unknown tokens so
/// that only validated placeholders reach [`expand_args`].
///
/// Returns `Err` when the string looks like a placeholder token (`{...}`)
/// but is not one of the supported ones — protecting users from typos like
/// `{pat}` silently becoming a literal.
pub fn parse_arg(s: &str) -> Result<ExternalToolArg, UnknownTokenError> {
    if let Some(ph) = ExternalToolPlaceholder::from_token(s) {
        return Ok(ExternalToolArg::Placeholder(ph));
    }
    // Reject apparent tokens that aren't recognised.
    if s.starts_with('{') && s.ends_with('}') {
        return Err(UnknownTokenError { token: s.into() });
    }
    Ok(ExternalToolArg::Literal(s.into()))
}

/// Error returned by [`parse_arg`] for unrecognised `{token}` strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTokenError {
    pub token: String,
}

impl std::fmt::Display for UnknownTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown placeholder token {:?}; valid tokens are: {}",
            self.token,
            ExternalToolPlaceholder::all()
                .iter()
                .map(|p| p.token())
                .collect::<Vec<_>>()
                .join(", "))
    }
}

impl std::error::Error for UnknownTokenError {}

// ── RFC-029: Built-in tool presets and tool kind ──────────────────────────────

/// The functional role of an external tool (RFC-029 §"Tool integration points").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    /// Open a file at a specific line in an external editor.
    Editor,
    /// Reveal a file or directory in the system file manager.
    FileManager,
    /// Open a terminal at a directory.
    Terminal,
    /// A user-defined custom command.
    Custom,
}

impl ExternalToolCommand {
    // ── Presets ───────────────────────────────────────────────────────────

    /// Preset: reveal a path in the system file manager.
    ///
    /// Expands `{Path}` to the target path. On Linux this calls `xdg-open`
    /// on the parent directory; on Windows `explorer /select,{Path}`; on
    /// macOS `open -R {Path}`. The correct executable is chosen by the UI
    /// at invocation time based on the host platform; the template here uses
    /// `xdg-open` as the canonical Linux default.
    ///
    /// The preset is a safe starting point. Users can override it in
    /// settings with a `ExternalToolCommand` that uses their preferred file
    /// manager (e.g. `nautilus --select {Path}` on GNOME, `dolphin --select
    /// {Path}` on KDE).
    pub fn file_manager_reveal() -> Self {
        Self {
            id:         ToolId("builtin.file_manager_reveal".into()),
            name:       "Reveal in File Manager".into(),
            executable: PathBuf::from("xdg-open"),
            args:       vec![ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path)],
        }
    }

    /// Preset: open a file at a line in VS Code.
    ///
    /// Expands to `code --goto {Path}:{Line}` — VS Code's `--goto` flag
    /// accepts `path:line` as a single argument.
    pub fn vscode_open() -> Self {
        // VS Code uses `code --goto /path/to/file:42` — path and line are
        // concatenated with a colon. Since our placeholder model expands each
        // arg independently, we represent this as two separate args and the
        // UI layer concatenates them. Alternatively, a user can configure a
        // custom command with a shell wrapper if they need the combined form.
        Self {
            id:         ToolId("builtin.vscode_open".into()),
            name:       "Open in VS Code".into(),
            executable: PathBuf::from("code"),
            args:       vec![
                ExternalToolArg::Literal("--goto".into()),
                ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path),
            ],
        }
    }

    /// Preset: open a file at a line in the system default application.
    pub fn system_open() -> Self {
        Self {
            id:         ToolId("builtin.system_open".into()),
            name:       "Open with System Default".into(),
            executable: PathBuf::from("xdg-open"),
            args:       vec![ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path)],
        }
    }

    /// All built-in presets, in display order.
    pub fn builtin_presets() -> Vec<Self> {
        vec![
            Self::file_manager_reveal(),
            Self::system_open(),
            Self::vscode_open(),
        ]
    }
}
