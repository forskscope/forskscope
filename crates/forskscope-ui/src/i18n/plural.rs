//! RFC-101 §2 (handoff 114): CLDR cardinal plural rules, one small
//! function per supported language, tested against CLDR's own examples
//! (the "Verification" section's own requirement) — not against this
//! file's own prose restating them.
//!
//! Every count this app shows is a non-negative integer (file counts,
//! match counts, byte progress), never a fraction or a negative number,
//! so each rule below only needs to classify `i64` — CLDR's own rules are
//! stated over a richer operand set (fractional part, visible decimals)
//! that integers always satisfy trivially.

use crate::state::Lang;

/// The plural category `count` selects for `lang`, as a CLDR form name
/// (`"one"`, `"other"`, …) — a [`crate::i18n::catalog::MessageForms`]'s
/// own form key, so a lookup is a direct string match, not a second
/// translation step.
pub(crate) fn category(lang: Lang, count: i64) -> &'static str {
    match lang {
        // CLDR `en` cardinal rule: one -> i = 1 and v = 0 (exactly the
        // integer 1, positive or negative by CLDR's own rule text, though
        // this app never passes a negative count); other -> everything
        // else, including 0.
        Lang::En => {
            if count == 1 {
                "one"
            } else {
                "other"
            }
        }
        // CLDR `ja` cardinal rule: other -> everything. Japanese has no
        // "one" category at all - RFC-101 §2's own stated motivation
        // ("1 件一致" / "3 件一致" are identical today, correctly, because
        // Japanese genuinely has only one plural form).
        Lang::Ja => "other",
        // Handoff 118 §1: "the pseudo-language uses English's rule" -
        // literally English's own, not a stand-in.
        #[cfg(feature = "layout-probe")]
        Lang::Pseudo => {
            if count == 1 {
                "one"
            } else {
                "other"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CLDR's own published examples for the `en` cardinal rule
    // (cldr/plurals.xml, cardinal "en"): one -> 1; other -> 0, 2~.
    #[test]
    fn english_one_is_exactly_1() {
        assert_eq!(category(Lang::En, 1), "one");
    }

    #[test]
    fn english_other_covers_zero_and_everything_above_one() {
        for n in [0, 2, 3, 4, 5, 10, 11, 21, 100, 1_000_000] {
            assert_eq!(category(Lang::En, n), "other", "n = {n}");
        }
    }

    // CLDR's own published examples for the `ja` cardinal rule: other ->
    // 0~ (no "one" category exists for Japanese at all).
    #[test]
    fn japanese_is_always_other() {
        for n in [0, 1, 2, 3, 5, 10, 11, 100, 1_000_000] {
            assert_eq!(category(Lang::Ja, n), "other", "n = {n}");
        }
    }

    // Handoff 118 §1: "the pseudo-language uses English's rule" -
    // same shape as `english_one_is_exactly_1`/`english_other_...`
    // above, against `Lang::Pseudo` instead.
    #[test]
    #[cfg(feature = "layout-probe")]
    fn pseudo_uses_englishs_own_rule() {
        assert_eq!(category(Lang::Pseudo, 1), "one");
        for n in [0, 2, 3, 4, 5, 10, 11, 21, 100, 1_000_000] {
            assert_eq!(category(Lang::Pseudo, n), "other", "n = {n}");
        }
    }
}
