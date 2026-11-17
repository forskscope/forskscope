//! RFC-099 §3: parsing and editing the two ignore-list settings fields
//! (*Ignore file extensions*, *Ignore directory names*) as badges rather
//! than one free-text field.
//!
//! The stored format is unchanged — two plain, comma-separated `String`
//! fields on `AppSettings`, read by
//! [`forskscope_compare::IgnoreRules::from_settings`]. Only the
//! *interaction* changes here: a typed or pasted batch is split on
//! commas (so typing one entry followed by a comma, and pasting several
//! at once, both go through the same path), normalized per entry with
//! the identical function the engine itself calls
//! (`normalize_extension_token`/`normalize_dir_token`), checked against
//! what is already there, and only the genuinely new entries are
//! appended. Removing a badge rewrites the stored text from the
//! remaining normalized entries, so the stored text and the displayed
//! badges never drift apart once either has changed — a pre-badges
//! file's own un-normalized or duplicate-after-normalization text loads
//! and displays correctly ([`current_entries`] collapses it), and keeps
//! its exact original text until the first add or remove actually
//! changes something.
//!
//! This replaces the field's own former per-keystroke
//! `FieldDebounce`-based commit (F136): nothing here ever writes on a
//! keystroke that isn't itself an add or a remove, so the defect
//! `FieldDebounce` existed to fix cannot recur.

use forskscope_compare::ignore::{normalize_dir_token, normalize_extension_token};

/// Which of the two ignore-list fields a call is about — the only thing
/// that decides which normalization applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreListKind {
    Extension,
    DirPattern,
}

impl IgnoreListKind {
    fn normalize(self, raw: &str) -> Option<String> {
        match self {
            Self::Extension => normalize_extension_token(raw),
            Self::DirPattern => normalize_dir_token(raw),
        }
    }
}

/// The current badges for one field: normalized, in the stored text's
/// own order, with any duplicate-after-normalization collapsed to its
/// first occurrence (a file edited by hand, or saved before this
/// feature existed, might have one).
pub fn current_entries(csv: &str, kind: IgnoreListKind) -> Vec<String> {
    let mut out = Vec::new();
    for raw in csv.split(',') {
        if let Some(n) = kind.normalize(raw)
            && !out.contains(&n)
        {
            out.push(n);
        }
    }
    out
}

/// One attempt to add a typed or pasted batch to a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddOutcome {
    /// The field's new stored text. Exactly the input `csv`, unchanged,
    /// when nothing was actually added (every entry typed was already
    /// present, or normalized to nothing at all).
    pub csv: String,
    /// Entries actually added, normalized, in the order typed.
    pub added: Vec<String>,
    /// Entries typed that were already present (by normalized value) —
    /// the field's own "Already in the list" message, or silence if
    /// this is empty.
    pub duplicates: Vec<String>,
}

/// Splits `typed` on commas, normalizes each piece, and appends
/// whichever aren't already in `csv`'s own current entries (checked
/// against each other too, so pasting `"tmp, tmp"` adds `tmp` once).
pub fn add_entries(csv: &str, typed: &str, kind: IgnoreListKind) -> AddOutcome {
    let mut entries = current_entries(csv, kind);
    let mut added = Vec::new();
    let mut duplicates = Vec::new();
    for raw in typed.split(',') {
        let Some(n) = kind.normalize(raw) else {
            continue;
        };
        if entries.contains(&n) {
            duplicates.push(n);
        } else {
            entries.push(n.clone());
            added.push(n);
        }
    }
    AddOutcome {
        csv: if added.is_empty() {
            csv.to_string()
        } else {
            entries.join(", ")
        },
        added,
        duplicates,
    }
}

/// Removes one badge, identified by its own normalized, displayed form,
/// and rewrites the stored text from whatever remains.
pub fn remove_entry(csv: &str, target: &str, kind: IgnoreListKind) -> String {
    current_entries(csv, kind)
        .into_iter()
        .filter(|e| e != target)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(entries: &[&str]) -> Vec<String> {
        entries.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn current_entries_normalizes_and_dedupes() {
        assert_eq!(
            current_entries(" .TMP , log, .tmp ", IgnoreListKind::Extension),
            v(&["tmp", "log"])
        );
    }

    #[test]
    fn current_entries_of_an_empty_field_is_empty() {
        assert_eq!(
            current_entries("", IgnoreListKind::Extension),
            Vec::<String>::new()
        );
    }

    #[test]
    fn adding_one_typed_entry() {
        let out = add_entries("tmp", "log", IgnoreListKind::Extension);
        assert_eq!(out.csv, "tmp, log");
        assert_eq!(out.added, v(&["log"]));
        assert!(out.duplicates.is_empty());
    }

    #[test]
    fn adding_normalizes_the_typed_entry_the_same_way_as_the_engine() {
        let out = add_entries("tmp", ".LOG", IgnoreListKind::Extension);
        assert_eq!(out.added, v(&["log"]));
    }

    #[test]
    fn pasting_several_at_once_adds_each() {
        let out = add_entries("", "o, class, tmp", IgnoreListKind::Extension);
        assert_eq!(out.csv, "o, class, tmp");
        assert_eq!(out.added, v(&["o", "class", "tmp"]));
    }

    #[test]
    fn a_duplicate_by_normalized_value_is_not_added_twice() {
        let out = add_entries("tmp", ".TMP", IgnoreListKind::Extension);
        assert_eq!(out.csv, "tmp", "unchanged - nothing was actually added");
        assert!(out.added.is_empty());
        assert_eq!(out.duplicates, v(&["tmp"]));
    }

    #[test]
    fn a_duplicate_within_the_same_pasted_batch_is_only_added_once() {
        let out = add_entries("", "tmp, tmp", IgnoreListKind::Extension);
        assert_eq!(out.added, v(&["tmp"]));
        assert_eq!(out.duplicates, v(&["tmp"]));
    }

    #[test]
    fn dir_patterns_keep_case_unlike_extensions() {
        let out = add_entries("", "Node_Modules", IgnoreListKind::DirPattern);
        assert_eq!(out.added, v(&["Node_Modules"]));
    }

    #[test]
    fn an_empty_or_whitespace_only_typed_value_adds_nothing() {
        let out = add_entries("tmp", "   ", IgnoreListKind::Extension);
        assert_eq!(out.csv, "tmp");
        assert!(out.added.is_empty());
        assert!(out.duplicates.is_empty());
    }

    #[test]
    fn removing_a_badge_drops_only_that_entry() {
        assert_eq!(
            remove_entry(
                "target, node_modules, dist",
                "node_modules",
                IgnoreListKind::DirPattern
            ),
            "target, dist"
        );
    }

    #[test]
    fn removing_the_only_entry_leaves_an_empty_field() {
        assert_eq!(remove_entry("tmp", "tmp", IgnoreListKind::Extension), "");
    }

    #[test]
    fn removing_rewrites_pre_existing_un_normalized_text() {
        // A file saved before this feature existed, or edited by hand,
        // might hold "..O, .TMP" - removing one badge is also the
        // moment the other's own stored text catches up to its display.
        assert_eq!(
            remove_entry("..O, .TMP", "o", IgnoreListKind::Extension),
            "tmp"
        );
    }
}
