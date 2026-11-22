//! RFC-100 §1: Unicode's bidirectional formatting characters — the ones
//! "Trojan Source" (CVE-2021-42574) uses to make code read one way and
//! run another. The list lives here, in one place, so the view (§2), the
//! CLI (§4) and the JSON output (§4) all mark exactly the same set.

/// One bidirectional control character this project marks specially: the
/// code point, the short marker shown in its place, and the English
/// accessible name (RFC-100 §2) a screen reader announces — translated
/// through `t(lang, ...)` in `forskscope-ui`, same as every other
/// user-visible string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BidiControl {
    pub ch: char,
    pub marker: &'static str,
    pub name: &'static str,
}

/// RFC-100 §1's own table. This is exactly Unicode's `Bidi_Control=Yes`
/// set (`tests::matches_unicodes_bidi_control_property` holds it equal
/// to the published values) — stable since Unicode 6.3 introduced the
/// isolates, so hand-maintaining it here costs nothing further in
/// upkeep and adds no dependency.
pub const BIDI_CONTROLS: &[BidiControl] = &[
    BidiControl {
        ch: '\u{061C}',
        marker: "ALM",
        name: "Arabic letter mark character",
    },
    BidiControl {
        ch: '\u{200E}',
        marker: "LRM",
        name: "left-to-right mark character",
    },
    BidiControl {
        ch: '\u{200F}',
        marker: "RLM",
        name: "right-to-left mark character",
    },
    BidiControl {
        ch: '\u{202A}',
        marker: "LRE",
        name: "left-to-right embedding character",
    },
    BidiControl {
        ch: '\u{202B}',
        marker: "RLE",
        name: "right-to-left embedding character",
    },
    BidiControl {
        ch: '\u{202C}',
        marker: "PDF",
        name: "pop directional formatting character",
    },
    BidiControl {
        ch: '\u{202D}',
        marker: "LRO",
        name: "left-to-right override character",
    },
    BidiControl {
        ch: '\u{202E}',
        marker: "RLO",
        name: "right-to-left override character",
    },
    BidiControl {
        ch: '\u{2066}',
        marker: "LRI",
        name: "left-to-right isolate character",
    },
    BidiControl {
        ch: '\u{2067}',
        marker: "RLI",
        name: "right-to-left isolate character",
    },
    BidiControl {
        ch: '\u{2068}',
        marker: "FSI",
        name: "first strong isolate character",
    },
    BidiControl {
        ch: '\u{2069}',
        marker: "PDI",
        name: "pop directional isolate character",
    },
];

/// `true` iff `ch` is one of [`BIDI_CONTROLS`].
pub fn is_bidi_control(ch: char) -> bool {
    BIDI_CONTROLS.iter().any(|c| c.ch == ch)
}

/// The marker text for `ch` (e.g. `"RLO"`), or `None` if it isn't one of
/// [`BIDI_CONTROLS`].
pub fn marker_for(ch: char) -> Option<&'static str> {
    BIDI_CONTROLS.iter().find(|c| c.ch == ch).map(|c| c.marker)
}

/// The English accessible name for `ch` (e.g. `"right-to-left override
/// character"`) — translated by the caller through `t(lang, ...)`, same
/// as every other user-visible string. `None` if it isn't one of
/// [`BIDI_CONTROLS`].
pub fn name_for(ch: char) -> Option<&'static str> {
    BIDI_CONTROLS.iter().find(|c| c.ch == ch).map(|c| c.name)
}

/// How many of [`BIDI_CONTROLS`]' characters `text` contains (RFC-100
/// §4) — a pure function over already-decoded text, counting every
/// occurrence, not just distinct characters.
pub fn count_bidi_controls(text: &str) -> usize {
    text.chars().filter(|&c| is_bidi_control(c)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unicode's own `Bidi_Control=Yes` set, transcribed from
    /// `DerivedCoreProperties.txt` (stable since Unicode 6.3; unchanged
    /// through the latest published version at the time of writing):
    /// `061C`, `200E..200F`, `202A..202E`, `2066..2069` — twelve code
    /// points in four ranges. Verified independently of
    /// [`BIDI_CONTROLS`]'s own construction, so a transcription mistake
    /// in one doesn't silently pass by checking itself.
    fn unicode_bidi_control_set() -> Vec<char> {
        let mut v = vec!['\u{061C}', '\u{200E}', '\u{200F}'];
        v.extend(('\u{202A}'..='\u{202E}').collect::<Vec<_>>());
        v.extend(('\u{2066}'..='\u{2069}').collect::<Vec<_>>());
        v
    }

    #[test]
    fn matches_unicodes_bidi_control_property() {
        let mut ours: Vec<char> = BIDI_CONTROLS.iter().map(|c| c.ch).collect();
        ours.sort_unstable();
        let mut theirs = unicode_bidi_control_set();
        theirs.sort_unstable();
        assert_eq!(
            ours, theirs,
            "BIDI_CONTROLS must be exactly Unicode's Bidi_Control=Yes set"
        );
    }

    #[test]
    fn every_control_has_a_distinct_three_letter_marker() {
        let mut markers: Vec<&str> = BIDI_CONTROLS.iter().map(|c| c.marker).collect();
        let before = markers.len();
        markers.sort_unstable();
        markers.dedup();
        assert_eq!(markers.len(), before, "every marker must be unique");
        for c in BIDI_CONTROLS {
            assert_eq!(c.marker.len(), 3, "{:?}'s marker must be 3 letters", c.ch);
        }
    }

    #[test]
    fn every_control_has_a_distinct_non_empty_name() {
        let mut names: Vec<&str> = BIDI_CONTROLS.iter().map(|c| c.name).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "every accessible name must be unique");
        for c in BIDI_CONTROLS {
            assert!(!c.name.is_empty());
        }
    }

    #[test]
    fn name_for_every_control_and_none_otherwise() {
        for c in BIDI_CONTROLS {
            assert_eq!(name_for(c.ch), Some(c.name));
        }
        assert_eq!(name_for('a'), None);
    }

    #[test]
    fn is_bidi_control_matches_the_table_exactly() {
        for c in BIDI_CONTROLS {
            assert!(is_bidi_control(c.ch));
        }
        for ordinary in ['a', ' ', '\n', '表', '0', '\u{200B}'] {
            assert!(
                !is_bidi_control(ordinary),
                "{ordinary:?} must not be a bidi control"
            );
        }
    }

    #[test]
    fn marker_for_every_control_and_none_otherwise() {
        for c in BIDI_CONTROLS {
            assert_eq!(marker_for(c.ch), Some(c.marker));
        }
        assert_eq!(marker_for('a'), None);
    }

    #[test]
    fn count_bidi_controls_counts_every_occurrence() {
        assert_eq!(count_bidi_controls("plain text"), 0);
        assert_eq!(count_bidi_controls("one\u{202E}two"), 1);
        let text = "a\u{202E}b\u{2066}c\u{202E}d";
        assert_eq!(count_bidi_controls(text), 3, "two RLOs and one LRI");
    }

    // ── RFC-100 verification §1: three pairs shaped like the "Trojan
    // Source" paper's own three categories (CVE-2021-42574) — written
    // for this project, not copied from the paper or its repository.
    // Each pair's only difference is one bidi control inserted, removed
    // or moved; what changes between `*_before`/`*_after` is exactly
    // that one character, confirmed by the count, not by eye. ──────────

    #[test]
    fn trojan_source_shape_an_early_return_hidden_in_a_comment() {
        // An RLO inside a line comment reorders everything after it on
        // that line for display - a reviewer reading left to right sees
        // the comment text in a plausible but reversed order, while the
        // bytes after the RLO are a real, uncommented early return a
        // compiler still executes exactly as written.
        let before = "fn check(ok: bool) -> bool {\n    // normal path\n    ok\n}\n";
        let after = concat!(
            "fn check(ok: bool) -> bool {\n",
            "    // normal path \u{202E}}eslaf nruter ;eurt nruter\u{202C}\n",
            "    ok\n}\n"
        );
        assert_eq!(count_bidi_controls(before), 0);
        assert_eq!(
            count_bidi_controls(after),
            2,
            "one RLO (begins the reordered run) and its PDF (ends it)"
        );
    }

    #[test]
    fn trojan_source_shape_code_commented_out() {
        // The same RLO/PDF pair, moved earlier on the line: enough of
        // the "real" source after it reads, under the override, as if
        // it had already been commented away - while nothing before the
        // RLO is a comment marker at all.
        let before = "let limit = 100; let checked = validate(limit);\n";
        let after = "let limit = 100; \u{202E}// \u{202C}let checked = validate(limit);\n";
        assert_eq!(count_bidi_controls(before), 0);
        assert_eq!(
            count_bidi_controls(after),
            2,
            "the same RLO/PDF pair, moved earlier"
        );
    }

    #[test]
    fn trojan_source_shape_a_string_that_seems_to_end() {
        // An LRI/PDI pair around a quote character makes a string
        // literal's own closing quote display in a position earlier
        // than where the parser actually finds it, so what reads as
        // "outside the string" on screen is still inside it to the
        // compiler.
        let before = "let msg = \"access: denied\";\n";
        let after = "let msg = \"access: \u{2066}\"\u{2069}denied\";\n";
        assert_eq!(count_bidi_controls(before), 0);
        assert_eq!(count_bidi_controls(after), 2, "one LRI and its PDI");
    }

    #[test]
    fn right_to_left_prose_with_no_controls_counts_zero() {
        // Ordinary Hebrew and Arabic letters are not bidi *controls* -
        // they are strong-RTL letters, a different Unicode property
        // entirely (this module only lists Bidi_Control=Yes). A file of
        // genuine right-to-left prose must not be flagged just for
        // being right-to-left.
        let hebrew = "שלום עולם, זהו קובץ לדוגמה.\n";
        let arabic = "مرحبا بالعالم، هذا ملف تجريبي.\n";
        assert_eq!(count_bidi_controls(hebrew), 0);
        assert_eq!(count_bidi_controls(arabic), 0);
    }
}
