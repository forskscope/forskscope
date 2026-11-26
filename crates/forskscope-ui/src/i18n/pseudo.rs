//! RFC-101 stage C (handoff 118): the pseudo-language's own text
//! generator — accent every Latin letter, stretch the message by 40%
//! (padding inside the brackets), and wrap it in `⟦…⟧`, so an
//! untranslated or cut-off string is visible at a glance in the layout
//! matrix. `layout-probe`-only (this whole module), generated from the
//! English catalog's own text at call time — never stored, never a
//! `pseudo.catalog` file: `build.rs`'s own id-parity assert and `cargo
//! xtask i18n`'s `required_forms` both glob every `*.catalog` file in
//! `i18n/` unconditionally (dev tooling over source files, not gated by
//! any Cargo feature at all), so a stored pseudo catalog would need
//! hand-maintaining in lockstep with `en.catalog` forever, exactly the
//! mechanical-translation-file burden RFC-101 exists to get away from.

/// One Latin letter's own accented form, case-preserved. Every letter
/// a-z/A-Z gets one; `q`/`Q` is the one exception with no common
/// single-codepoint Latin accent, so it's `q`/`Q` plus a combining
/// diaeresis (U+0308) instead — the only place this function's own
/// output is two chars for one input letter, which [`pseudo_template`]'s
/// own length accounting (counted *after* accenting, not before)
/// already allows for.
fn accent_char(c: char) -> String {
    let accented = match c.to_ascii_lowercase() {
        'a' => "á",
        'b' => "ḃ",
        'c' => "ĉ",
        'd' => "ď",
        'e' => "é",
        'f' => "ḟ",
        'g' => "ĝ",
        'h' => "ĥ",
        'i' => "î",
        'j' => "ĵ",
        'k' => "ķ",
        'l' => "ĺ",
        'm' => "ḿ",
        'n' => "ñ",
        'o' => "ô",
        'p' => "ṕ",
        'q' => "q\u{0308}",
        'r' => "ř",
        's' => "š",
        't' => "ť",
        'u' => "û",
        'v' => "ṽ",
        'w' => "ŵ",
        'x' => "ẋ",
        'y' => "ý",
        'z' => "ž",
        // Not a Latin letter (digit, punctuation, an already-non-Latin
        // glyph such as "←"/"→" that a few catalog messages carry) -
        // left exactly as it is.
        _ => return c.to_string(),
    };
    if c.is_ascii_uppercase() {
        accented.to_uppercase()
    } else {
        accented.to_string()
    }
}

/// A single glyph repeated to fill the 40% stretch - distinct from any
/// accented letter [`accent_char`] produces, so a capture or a test can
/// always tell padding from the message's own (accented) text.
const FILLER: char = 'ẇ';

/// No run of [`FILLER`] is ever longer than this many characters
/// (review 245 §1.1): the first version glued the whole side's padding
/// into one solid, unbroken run - for a 60-character help text, about
/// 12 `ẇ`s fused straight onto the first and last real word, with
/// nowhere for a line to break. No real language makes a 20-character
/// word out of an ordinary sentence, so several of the first triage's
/// findings may have been this generator's doing, not the app's.
const MAX_FILLER_RUN: usize = 6;

/// Renders `len` filler characters as separate, space-joined "words" of
/// at most [`MAX_FILLER_RUN`] each, rather than one solid run - review
/// 245 §1.1's own fix. The interior spaces are themselves counted
/// against `len` (a space cost the same one unit of budget a filler
/// glyph would), so the caller's own 40%-stretch accounting is
/// unaffected by this function - it always renders exactly `len`
/// characters.
fn spread_padding(len: usize) -> String {
    if len == 0 {
        return String::new();
    }
    // `len.div_ceil(MAX_FILLER_RUN)` groups, each sized so no single one
    // exceeds `MAX_FILLER_RUN` even after `num_groups - 1` characters of
    // budget are spent on the separating spaces between them.
    let num_groups = len.div_ceil(MAX_FILLER_RUN);
    let filler_total = len - (num_groups - 1);
    let base = filler_total / num_groups;
    let extra = filler_total % num_groups;
    let mut out = String::with_capacity(len);
    for i in 0..num_groups {
        if i > 0 {
            out.push(' ');
        }
        let run = base + usize::from(i < extra);
        for _ in 0..run {
            out.push(FILLER);
        }
    }
    out
}

/// One side's full padding, with a single boundary space against the
/// message itself, spent out of the same `len` budget
/// [`spread_padding`] renders into filler words - review 247 §2.2:
/// `pseudo_template` used to join its padding straight onto the
/// message with nothing between them, so the padding's own nearest
/// filler character fused onto the message's own first or last
/// letter, turning a real word into one unbreakable run with whatever
/// filler ran up against it (how `.kb-desc`'s own word came to be too
/// wide for its column). `left_padding`/`right_padding` (below) each
/// carve one character off their own `len` for that boundary space,
/// rather than adding it on top - `pseudo_template`'s own 40%-stretch
/// accounting is unaffected.
fn left_padding(len: usize) -> String {
    if len == 0 {
        return String::new();
    }
    let mut out = spread_padding(len - 1);
    out.push(' ');
    out
}

/// The right-hand mirror of [`left_padding`]: the boundary space comes
/// first here, immediately against the message's own last character,
/// with [`spread_padding`]'s own filler words after it.
fn right_padding(len: usize) -> String {
    if len == 0 {
        return String::new();
    }
    let mut out = String::from(' ');
    out.push_str(&spread_padding(len - 1));
    out
}

/// Accents every Latin letter in `english` outside a `{ ... }`
/// placeholder span (copied through verbatim, so `{ $count }` still
/// substitutes correctly), stretches the result by 40% with
/// [`left_padding`]/[`right_padding`] split across both sides, and
/// wraps the whole thing in `⟦…⟧`.
pub(crate) fn pseudo_template(english: &str) -> String {
    let mut accented = String::new();
    let mut chars = english.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            accented.push(c);
            for c2 in chars.by_ref() {
                accented.push(c2);
                if c2 == '}' {
                    break;
                }
            }
        } else {
            accented.push_str(&accent_char(c));
        }
    }

    let base_len = accented.chars().count();
    let target_len = ((base_len as f64) * 1.4).round() as usize;
    let pad_needed = target_len.saturating_sub(base_len);
    let left_pad_len = pad_needed / 2;
    let right_pad_len = pad_needed - left_pad_len;

    let mut out = String::with_capacity(accented.len() + pad_needed + 2);
    out.push('⟦');
    out.push_str(&left_padding(left_pad_len));
    out.push_str(&accented);
    out.push_str(&right_padding(right_pad_len));
    out.push('⟧');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ascii_letter_is_accented_and_case_is_preserved() {
        for c in 'a'..='z' {
            let accented = accent_char(c);
            assert_ne!(accented, c.to_string(), "{c:?} was not accented at all");
            assert!(
                accented.chars().next().unwrap().is_lowercase(),
                "{c:?} -> {accented:?} lost its lowercase"
            );
        }
        for c in 'A'..='Z' {
            let accented = accent_char(c);
            assert_ne!(accented, c.to_string(), "{c:?} was not accented at all");
            assert!(
                accented.chars().next().unwrap().is_uppercase(),
                "{c:?} -> {accented:?} lost its uppercase"
            );
        }
    }

    #[test]
    fn a_non_latin_character_passes_through_unaccented() {
        assert_eq!(accent_char('←'), "←");
        assert_eq!(accent_char('5'), "5");
        assert_eq!(accent_char(' '), " ");
    }

    #[test]
    fn brackets_wrap_everything() {
        let out = pseudo_template("Settings");
        assert!(out.starts_with('⟦'), "{out:?} does not start with ⟦");
        assert!(out.ends_with('⟧'), "{out:?} does not end with ⟧");
    }

    #[test]
    fn placeholders_are_preserved_verbatim() {
        let out = pseudo_template("Could not save { $error }");
        assert!(
            out.contains("{ $error }"),
            "{out:?} lost its own placeholder"
        );
    }

    #[test]
    fn multiple_placeholders_are_all_preserved() {
        let out = pseudo_template("{ $count } of { $total } done");
        assert!(out.contains("{ $count }"), "{out:?}");
        assert!(out.contains("{ $total }"), "{out:?}");
    }

    /// The stretch is measured on the accented text, between the
    /// brackets - not on the raw English input, and not counting the
    /// bracket glyphs themselves.
    #[test]
    fn the_stretch_is_40_percent_within_one_character() {
        for english in [
            "Settings",
            "Could not save { $error }",
            "a",
            "This is a considerably longer sentence, to see the stretch hold at this length too.",
        ] {
            let out = pseudo_template(english);
            let inner: String = out
                .strip_prefix('⟦')
                .and_then(|s| s.strip_suffix('⟧'))
                .expect("wrapped in brackets")
                .to_string();
            let base_len = english.chars().count();
            // The accented text can be 1 char longer than the raw
            // English per occurrence of 'q'/'Q' (accent_char's own one
            // two-codepoint case) - accounted for by measuring the
            // *actual* accented length, same as pseudo_template itself
            // does, rather than assuming accenting never changes length.
            let q_count = english
                .chars()
                .filter(|c| c.eq_ignore_ascii_case(&'q'))
                .count();
            let accented_len = base_len + q_count;
            let expected = ((accented_len as f64) * 1.4).round() as i64;
            let actual = inner.chars().count() as i64;
            assert!(
                (actual - expected).abs() <= 1,
                "{english:?}: expected stretch to {expected} chars (±1), got {actual} ({inner:?})"
            );
        }
    }

    #[test]
    fn padding_is_the_filler_glyph_not_accented_text() {
        let out = pseudo_template("x");
        // "x" alone (len 1) stretched by 40% rounds to 1 - no padding
        // to check there; use a longer, still-trivial string instead.
        let out2 = pseudo_template("xy");
        for c in out.chars().chain(out2.chars()) {
            assert!(
                c == '⟦'
                    || c == '⟧'
                    || c == FILLER
                    || c == ' '
                    || c == accent_char('x').chars().next().unwrap()
                    || c == accent_char('y').chars().next().unwrap(),
                "unexpected character {c:?} in {out:?}/{out2:?}"
            );
        }
    }

    /// Review 245 §1.1: no run of the filler glyph may be longer than
    /// [`MAX_FILLER_RUN`] - checked directly against [`spread_padding`]
    /// across a wide range of lengths (0 through 200, past any message
    /// in the real catalog), not a hand-picked sample.
    #[test]
    fn no_filler_run_is_longer_than_the_max() {
        for len in 0..=200usize {
            let padding = spread_padding(len);
            let longest_run = padding
                .split(' ')
                .map(|group| group.chars().count())
                .max()
                .unwrap_or(0);
            assert!(
                longest_run <= MAX_FILLER_RUN,
                "len {len}: a run of {longest_run} filler characters in {padding:?}"
            );
        }
    }

    /// [`spread_padding`]'s own budget accounting: the interior spaces
    /// are spent out of `len`, never added on top of it, so it always
    /// renders exactly `len` characters - the invariant
    /// [`pseudo_template`]'s own 40%-stretch test above depends on.
    #[test]
    fn spread_padding_always_renders_exactly_len_characters() {
        for len in 0..=200usize {
            let padding = spread_padding(len);
            assert_eq!(
                padding.chars().count(),
                len,
                "spread_padding({len}) rendered {:?} ({} chars)",
                padding,
                padding.chars().count()
            );
        }
    }

    /// Spread padding never starts or ends with a space (there would be
    /// nothing on the other side of it to separate) and never contains
    /// two spaces back to back (an empty "word" between them).
    #[test]
    fn spread_padding_has_no_stray_or_doubled_spaces() {
        for len in 0..=200usize {
            let padding = spread_padding(len);
            if padding.is_empty() {
                continue;
            }
            assert!(!padding.starts_with(' '), "len {len}: {padding:?}");
            assert!(!padding.ends_with(' '), "len {len}: {padding:?}");
            assert!(!padding.contains("  "), "len {len}: {padding:?}");
        }
    }

    /// Review 247 §2.2: the padding never touches a letter of the
    /// message - `left_padding`'s own output, whenever there's any
    /// padding at all, ends in the boundary space (never a filler
    /// character), and `right_padding`'s own output starts with it.
    /// Checked directly against both functions across a wide range of
    /// lengths, not just `pseudo_template`'s own end-to-end shape.
    #[test]
    fn left_and_right_padding_always_end_in_the_boundary_space() {
        for len in 1..=200usize {
            let left = left_padding(len);
            assert!(
                left.ends_with(' '),
                "left_padding({len}) = {left:?} doesn't end in a space"
            );
            let right = right_padding(len);
            assert!(
                right.starts_with(' '),
                "right_padding({len}) = {right:?} doesn't start with a space"
            );
        }
    }

    /// The same invariant, still carrying the exact-`len` budget
    /// [`spread_padding`] itself already guarantees - `left_padding`/
    /// `right_padding` only redistribute one character of it as the
    /// boundary space, never add to it.
    #[test]
    fn left_and_right_padding_still_render_exactly_len_characters() {
        for len in 0..=200usize {
            assert_eq!(
                left_padding(len).chars().count(),
                len,
                "left_padding({len})"
            );
            assert_eq!(
                right_padding(len).chars().count(),
                len,
                "right_padding({len})"
            );
        }
    }

    /// End-to-end, on real messages: `pseudo_template`'s own output
    /// never has a filler character sitting directly against the
    /// message's own first or last (accented) letter - the actual
    /// defect this round fixes (a real word plus a directly-glued
    /// filler run, too wide for `.kb-desc`'s own column). Built from
    /// the same accenting `pseudo_template` itself does, not a
    /// hand-picked expected string, so this holds for any message.
    #[test]
    fn the_padding_never_fuses_onto_the_message_in_a_real_template() {
        for english in [
            "Settings",
            "This is a considerably longer sentence, to see the boundary hold at this length too.",
            "Explorer layout",
        ] {
            let accented: String = english.chars().map(accent_char).collect();
            let out = pseudo_template(english);
            let inner = out
                .strip_prefix('⟦')
                .and_then(|s| s.strip_suffix('⟧'))
                .expect("wrapped in brackets");
            assert!(
                inner.contains(&accented),
                "{english:?}: {inner:?} doesn't contain the accented message {accented:?} intact"
            );
            let before = &inner[..inner.find(&accented).unwrap()];
            let after = &inner[inner.find(&accented).unwrap() + accented.len()..];
            assert!(
                before.is_empty() || before.ends_with(' '),
                "{english:?}: {inner:?} - a filler character sits directly before the message"
            );
            assert!(
                after.is_empty() || after.starts_with(' '),
                "{english:?}: {inner:?} - a filler character sits directly after the message"
            );
        }
    }
}
