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

/// Accents every Latin letter in `english` outside a `{ ... }`
/// placeholder span (copied through verbatim, so `{ $count }` still
/// substitutes correctly), stretches the result by 40% with
/// [`FILLER`] padding split across both sides, and wraps the whole
/// thing in `⟦…⟧`.
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
    for _ in 0..left_pad_len {
        out.push(FILLER);
    }
    out.push_str(&accented);
    for _ in 0..right_pad_len {
        out.push(FILLER);
    }
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
                    || c == accent_char('x').chars().next().unwrap()
                    || c == accent_char('y').chars().next().unwrap(),
                "unexpected character {c:?} in {out:?}/{out2:?}"
            );
        }
    }
}
