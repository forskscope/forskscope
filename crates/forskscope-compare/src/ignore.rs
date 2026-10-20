//! User-defined ignore rules for directory listing and comparison (RFC-056,
//! F149).
//!
//! Three categories of rule:
//!
//! - **File extension rules** — case-insensitive, stored without a leading dot.
//!   `"o"` and `".O"` both match `file.o`.
//! - **Directory-name patterns** — supports an optional single `*` wildcard.
//!   `"target"` matches exactly; `"*.cache"` matches `build.cache`;
//!   `"tmp*"` matches `tmpfiles`.
//! - **`hide_dotfiles`** — a single switch, independent of the two above: when
//!   set, an entry whose name marks it hidden by platform convention is
//!   skipped, file or directory, whatever its name or extension otherwise is.
//!
//! The rules filter *discovery* (listing, digest, recursive compare).
//! An explicit file open always proceeds regardless of any ignore rule.
//!
//! `hide_dotfiles` is **not** applied by the Explorer tree's own scan
//! (`FilteringExecutor` in `forskscope-ui`): the tree widget already has its
//! own instant, zero-I/O mechanism for this
//! (`dioxus_swdir_tree::DisplayFilter`, switched by `set_filter`), and
//! filtering hidden entries out of the raw scan would break that — the
//! widget re-derives visibility from a cache that has to still contain them.
//! `hide_dotfiles` governs the *walk* (`recursive_diff`, `list_recursive_for_display`
//! and their `_with_rules` forms), used by Deep Compare and RFC-080 tier 1,
//! where there is no separate display layer — the result **is** the answer.
//! The Explorer's own setting drives both: the tree's `DisplayFilter` and
//! this field, from the same boolean, so neither half of the product answers
//! a question the other does not (F149).

/// Compiled ignore rules persisted in user settings.
///
/// Serialized form is two comma-separated strings plus a flag in
/// `AppSettings`. Parsing produces this struct; it is passed into directory
/// operations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IgnoreRules {
    /// Lowercased extension tokens, no leading dot.
    pub file_extensions: Vec<String>,
    /// Directory-name glob patterns (may contain at most one `*`).
    pub dir_patterns: Vec<String>,
    /// Skip entries hidden by platform convention (F149). See the module doc
    /// for why this does *not* reach the Explorer tree's own scan.
    pub hide_dotfiles: bool,
}

impl IgnoreRules {
    /// Parse from comma-separated settings strings. `hide_dotfiles` defaults to
    /// `false` — set the field directly afterward (it is `pub`); the caller
    /// (`AppSettings::ignore_rules`) does.
    ///
    /// Extension input: `".o, class, .TMP"` → `["o", "class", "tmp"]`.
    /// Pattern input:   `"target, *.cache"` → `["target", "*.cache"]`.
    pub fn from_settings(extensions_csv: &str, dirs_csv: &str) -> Self {
        let file_extensions = extensions_csv
            .split(',')
            .map(|s| s.trim().trim_start_matches('.').to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let dir_patterns = dirs_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            file_extensions,
            dir_patterns,
            hide_dotfiles: false,
        }
    }

    /// `true` when no rules are defined — used to skip filtering work.
    pub fn is_empty(&self) -> bool {
        self.file_extensions.is_empty() && self.dir_patterns.is_empty() && !self.hide_dotfiles
    }

    /// Returns `true` if a file with the given `name` should be ignored.
    ///
    /// Matches against the file's extension (case-insensitive).
    pub fn is_file_ignored(&self, name: &str) -> bool {
        if self.file_extensions.is_empty() {
            return false;
        }
        // Extension is everything after the last dot, lower-cased.
        if let Some(ext) = name.rsplit('.').next()
            && name != ext
        {
            // There was a dot — `name != ext` means the dot existed.
            let ext_lc = ext.to_lowercase();
            if self.file_extensions.iter().any(|e| e == &ext_lc) {
                return true;
            }
        }
        false
    }

    /// Returns `true` if a directory with the given `name` should be ignored.
    ///
    /// Patterns are matched case-sensitively (POSIX convention).
    /// A single `*` is treated as "any run of characters"; other glob
    /// meta-characters are treated as literals.
    pub fn is_dir_ignored(&self, name: &str) -> bool {
        self.dir_patterns.iter().any(|p| glob_match(p, name))
    }
}

/// Wildcard matcher supporting up to two `*` characters.
///
/// | Stars | Semantics |
/// |-------|-----------|
/// | 0 | Exact match |
/// | 1 | Prefix + suffix (e.g. `*.cache`, `tmp*`) |
/// | 2 | Prefix + substring + suffix (e.g. `*backup*`) |
/// | 3+ | Falls back to prefix + suffix using the first and last star |
fn glob_match(pattern: &str, name: &str) -> bool {
    let star_positions: Vec<usize> = pattern
        .char_indices()
        .filter_map(|(i, c)| if c == '*' { Some(i) } else { None })
        .collect();

    match star_positions.len() {
        0 => pattern == name,
        1 => {
            let s = star_positions[0];
            let prefix = &pattern[..s];
            let suffix = &pattern[s + 1..];
            name.len() >= prefix.len() + suffix.len()
                && name.starts_with(prefix)
                && name.ends_with(suffix)
        }
        2 => {
            let s1 = star_positions[0];
            let s2 = star_positions[1];
            let prefix = &pattern[..s1];
            let middle = &pattern[s1 + 1..s2];
            let suffix = &pattern[s2 + 1..];
            let min_len = prefix.len() + middle.len() + suffix.len();
            if name.len() < min_len || !name.starts_with(prefix) {
                return false;
            }
            let after_prefix = if suffix.is_empty() {
                name
            } else {
                match name.strip_suffix(suffix) {
                    Some(s) => s,
                    None => return false,
                }
            };
            let inner = &after_prefix[prefix.len()..];
            middle.is_empty() || inner.contains(middle)
        }
        _ => {
            // More than two stars: honour first and last only.
            let s_first = star_positions[0];
            let s_last = *star_positions.last().unwrap();
            let prefix = &pattern[..s_first];
            let suffix = &pattern[s_last + 1..];
            name.len() >= prefix.len() + suffix.len()
                && name.starts_with(prefix)
                && name.ends_with(suffix)
        }
    }
}
