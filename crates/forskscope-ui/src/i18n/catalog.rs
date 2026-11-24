//! RFC-101 §3 (handoff 114): the parser for this project's own catalog
//! format, `crates/forskscope-ui/i18n/<lang>.catalog` — `{ $name }`
//! placeholders, `id`/`id.one`/`id.other` plural forms, chosen over Fluent
//! in review 232 §3.1 on reply 198's measurement (our own parser: 135
//! lines, no runtime dependency; Fluent: +215.7 KiB, nine new crates to
//! audit indefinitely).
//!
//! This file is used two ways from one source, deliberately: `build.rs`
//! `#[path]`-includes it directly (so the real build-time codegen step
//! runs the exact same parser these tests exercise, not a hand-copied
//! twin of it — `build.rs` cannot depend on its own crate's not-yet-built
//! library, so an `include`-by-path is the only way to share it), and
//! `i18n/mod.rs` declares it `#[cfg(test)]`-only (nothing at runtime needs
//! to *parse* a catalog — only to read the table `build.rs` already
//! generated from one, which is [`crate::i18n::StaticForms`], a different
//! type, defined there), so `cargo test` is what exercises `mod tests`
//! below. Pure and I/O-free either way: [`parse`] takes already-read
//! `&str`, never a path.
//!
//! **Deliberately not reused by `cargo xtask i18n`'s own checks** (every
//! language has every message; no unused message; placeholders and
//! plural forms match): `xtask` has its own independent `[workspace]` and
//! never depends on the crate it audits (DEC-005, `xtask/Cargo.toml`) —
//! "a per-push result is a property of the commit" cuts both ways, and
//! `xtask` must keep running even when `forskscope-ui` itself fails to
//! build. Its own catalog scanner is a deliberate, small duplicate.
//!
//! **Strict by design (RFC-101 §3):** a malformed line is a parse error,
//! never a silently-dropped or silently-guessed message — `build.rs`
//! turns that `Err` into a build failure naming the file and line, so a
//! translator's typo fails the build, not a user's screen.

use std::fmt;

/// One catalog message's text, by form. A message with no count names one
/// form (`Single`); a message with a count names one text per plural
/// category (`Plural`), e.g. `one`/`other` for English, `other` only for
/// Japanese (RFC-101 §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessageForms {
    Single(String),
    /// `(form, text)` pairs, in the order they appeared in the file —
    /// never empty (a `.form =` line always inserts one place).
    Plural(Vec<(String, String)>),
}

/// A parsed catalog: every message id this file defines, in file order,
/// each with its own form(s). Ids are unique — a repeated id (even a
/// second `.other` for one already fully defined) is a parse error, not a
/// silent overwrite.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ParsedCatalog {
    pub(crate) messages: Vec<(String, MessageForms)>,
}

impl ParsedCatalog {
    #[cfg(test)]
    pub(crate) fn get(&self, id: &str) -> Option<&MessageForms> {
        self.messages.iter().find(|(i, _)| i == id).map(|(_, f)| f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CatalogError {
    /// A non-blank, non-comment line that isn't `id = text` or
    /// `id.form = text`.
    MalformedLine { line_no: usize, line: String },
    /// The same id (bare, or the same `id.form`) defined twice.
    DuplicateId { line_no: usize, id: String },
    /// One `.form` line for an id that already has a bare (non-plural)
    /// definition, or vice versa — a message is one or the other, never
    /// both.
    MixedSingleAndPlural { line_no: usize, id: String },
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedLine { line_no, line } => write!(
                f,
                "line {line_no}: not a valid catalog line (expected `id = text` or \
                 `id.form = text`): {line:?}"
            ),
            Self::DuplicateId { line_no, id } => {
                write!(f, "line {line_no}: {id:?} is already defined")
            }
            Self::MixedSingleAndPlural { line_no, id } => write!(
                f,
                "line {line_no}: {id:?} has both a plain definition and a `.form` one — \
                 a message is either plural or not, never both"
            ),
        }
    }
}

fn is_id_byte(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
}

fn is_form_byte(b: u8) -> bool {
    b.is_ascii_lowercase()
}

/// Splits `"id"` or `"id.form"` into its two parts, validating the
/// character set (kebab-case id, lowercase-only form). `None` if either
/// part is empty or contains a byte outside its allowed set.
fn split_id(key: &str) -> Option<(&str, Option<&str>)> {
    let (id_part, form_part) = match key.split_once('.') {
        Some((id, form)) => (id, Some(form)),
        None => (key, None),
    };
    if id_part.is_empty() || !id_part.bytes().all(is_id_byte) {
        return None;
    }
    if id_part.starts_with('-') || id_part.ends_with('-') || id_part.contains("--") {
        return None;
    }
    if let Some(form) = form_part
        && (form.is_empty() || !form.bytes().all(is_form_byte))
    {
        return None;
    }
    Some((id_part, form_part))
}

/// Parses one catalog file's already-read contents. Blank lines and
/// `#`-prefixed comment lines are skipped; every other line must be
/// exactly `id = text` or `id.form = text` (one literal `" = "`
/// separator; everything before it is the key, everything after it, to
/// end of line, is the text verbatim — no quoting, no escaping, so a
/// message is always exactly one physical line).
pub(crate) fn parse(source: &str) -> Result<ParsedCatalog, CatalogError> {
    let mut catalog = ParsedCatalog::default();
    // Tracks which ids are plural (have at least one `.form`) vs. single
    // (a bare `id =`), so a line mixing the two styles for the same id is
    // caught instead of silently splitting that id across both shapes.
    let mut is_plural: std::collections::BTreeMap<String, bool> = std::collections::BTreeMap::new();

    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((key, text)) = line.split_once(" = ") else {
            return Err(CatalogError::MalformedLine {
                line_no,
                line: line.to_string(),
            });
        };
        let Some((id, form)) = split_id(key) else {
            return Err(CatalogError::MalformedLine {
                line_no,
                line: line.to_string(),
            });
        };

        let this_is_plural = form.is_some();
        match is_plural.get(id) {
            Some(&existing) if existing != this_is_plural => {
                return Err(CatalogError::MixedSingleAndPlural {
                    line_no,
                    id: id.to_string(),
                });
            }
            _ => {
                is_plural.insert(id.to_string(), this_is_plural);
            }
        }

        match form {
            None => {
                if catalog.messages.iter().any(|(i, _)| i == id) {
                    return Err(CatalogError::DuplicateId {
                        line_no,
                        id: id.to_string(),
                    });
                }
                catalog
                    .messages
                    .push((id.to_string(), MessageForms::Single(text.to_string())));
            }
            Some(form) => {
                match catalog.messages.iter_mut().find(|(i, _)| i == id) {
                    Some((_, MessageForms::Plural(forms))) => {
                        if forms.iter().any(|(f, _)| f == form) {
                            return Err(CatalogError::DuplicateId {
                                line_no,
                                id: format!("{id}.{form}"),
                            });
                        }
                        forms.push((form.to_string(), text.to_string()));
                    }
                    Some((_, MessageForms::Single(_))) => {
                        // `is_plural` above already catches a bare id
                        // later gaining a `.form` line or vice versa;
                        // this arm is unreachable in practice but kept
                        // explicit rather than `unreachable!()`, since a
                        // parser's own invariants are exactly where a
                        // silent wrong-branch bug would hide otherwise.
                        return Err(CatalogError::MixedSingleAndPlural {
                            line_no,
                            id: id.to_string(),
                        });
                    }
                    None => {
                        catalog.messages.push((
                            id.to_string(),
                            MessageForms::Plural(vec![(form.to_string(), text.to_string())]),
                        ));
                    }
                }
            }
        }
    }

    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_message_parses_as_single() {
        let catalog =
            parse("could-not-open = Could not open \"{ $name }\" — { $error }.\n").unwrap();
        assert_eq!(
            catalog.get("could-not-open"),
            Some(&MessageForms::Single(
                "Could not open \"{ $name }\" — { $error }.".to_string()
            ))
        );
    }

    #[test]
    fn a_plural_message_parses_as_plural_in_file_order() {
        let catalog = parse(
            "search-matches.one = { $count } match\n\
             search-matches.other = { $count } matches\n",
        )
        .unwrap();
        assert_eq!(
            catalog.get("search-matches"),
            Some(&MessageForms::Plural(vec![
                ("one".to_string(), "{ $count } match".to_string()),
                ("other".to_string(), "{ $count } matches".to_string()),
            ]))
        );
    }

    #[test]
    fn blank_lines_and_comments_are_skipped() {
        let catalog = parse(
            "# a translator's note\n\
             \n\
             remove-entry = Remove { $entry }\n\
             \n\
             # another note\n",
        )
        .unwrap();
        assert_eq!(catalog.messages.len(), 1);
    }

    /// Falsifies the strict-parsing requirement: a line with no `" = "`
    /// separator at all must fail, not be silently dropped (RFC-101 §3:
    /// "a malformed line fails the build").
    #[test]
    fn a_line_with_no_separator_is_a_parse_error() {
        let err = parse("this is not a catalog line\n").unwrap_err();
        assert!(matches!(
            err,
            CatalogError::MalformedLine { line_no: 1, .. }
        ));
    }

    #[test]
    fn an_id_with_uppercase_is_a_parse_error() {
        let err = parse("Could-Not-Open = x\n").unwrap_err();
        assert!(matches!(
            err,
            CatalogError::MalformedLine { line_no: 1, .. }
        ));
    }

    #[test]
    fn an_id_with_a_double_hyphen_is_a_parse_error() {
        let err = parse("could--not-open = x\n").unwrap_err();
        assert!(matches!(err, CatalogError::MalformedLine { .. }));
    }

    #[test]
    fn a_form_with_uppercase_is_a_parse_error() {
        let err = parse("search-matches.One = x\n").unwrap_err();
        assert!(matches!(err, CatalogError::MalformedLine { .. }));
    }

    #[test]
    fn a_duplicate_bare_id_is_a_parse_error() {
        let err =
            parse("remove-entry = Remove { $entry }\nremove-entry = Remove { $entry } again\n")
                .unwrap_err();
        assert!(matches!(err, CatalogError::DuplicateId { line_no: 2, .. }));
    }

    #[test]
    fn a_duplicate_plural_form_is_a_parse_error() {
        let err = parse(
            "search-matches.one = { $count } match\n\
             search-matches.one = { $count } match again\n",
        )
        .unwrap_err();
        assert!(matches!(err, CatalogError::DuplicateId { line_no: 2, .. }));
    }

    #[test]
    fn mixing_a_bare_id_and_a_form_for_the_same_id_is_a_parse_error() {
        let err = parse("could-not-open = x\ncould-not-open.other = y\n").unwrap_err();
        assert!(matches!(
            err,
            CatalogError::MixedSingleAndPlural { line_no: 2, .. }
        ));

        let err2 = parse("search-matches.other = x\nsearch-matches = y\n").unwrap_err();
        assert!(matches!(
            err2,
            CatalogError::MixedSingleAndPlural { line_no: 2, .. }
        ));
    }
}
