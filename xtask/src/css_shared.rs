//! Shared by `colour_vision.rs` and `contrast.rs`: both gates read
//! `01-tokens.css`'s `.selector { --name: value; }` rules directly rather
//! than duplicating the token values in Rust (so each checks what actually
//! ships), and both need WCAG 2.x relative-luminance contrast. This is the
//! one place either owns, so the two gates can't drift against each other
//! on what a hex colour or a CSS rule even means.

// ── sRGB / linear ────────────────────────────────────────────────────────

pub type Rgb = (f64, f64, f64);

pub fn hex_to_srgb(hex: &str) -> Rgb {
    let hex = hex.trim_start_matches('#');
    assert_eq!(hex.len(), 6, "expected a 6-digit hex colour, got {hex:?}");
    let chan = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16)
            .unwrap_or_else(|e| panic!("bad hex colour {hex:?}: {e}")) as f64
            / 255.0
    };
    (chan(0..2), chan(2..4), chan(4..6))
}

pub fn srgb_channel_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn srgb_to_linear((r, g, b): Rgb) -> Rgb {
    (
        srgb_channel_to_linear(r),
        srgb_channel_to_linear(g),
        srgb_channel_to_linear(b),
    )
}

// ── WCAG contrast ────────────────────────────────────────────────────────

pub fn relative_luminance_rgb((r, g, b): Rgb) -> f64 {
    let (r, g, b) = srgb_to_linear((r, g, b));
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub fn contrast_ratio_rgb(rgb1: Rgb, rgb2: Rgb) -> f64 {
    let (l1, l2) = (relative_luminance_rgb(rgb1), relative_luminance_rgb(rgb2));
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

pub fn contrast_ratio(hex1: &str, hex2: &str) -> f64 {
    contrast_ratio_rgb(hex_to_srgb(hex1), hex_to_srgb(hex2))
}

// ── Parsing a token CSS file ────────────────────────────────────────────

/// One `.selector { --name: value; ... }` rule, in source order.
pub struct Rule {
    pub selector: String,
    pub decls: Vec<(String, String)>,
}

/// Strips every `/* ... */` comment. Comment text ahead of a selector
/// would otherwise become part of the "selector" the naive scan below
/// extracts, failing the `starts_with('.')` check and silently dropping
/// the whole rule — caught by running this against the real file, not by
/// inspection, when a `.cv-rgsafe` rule right after a banner comment
/// resolved to nothing (colour-vision's own history).
pub fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = match rest[start..].find("*/") {
            Some(end) => &rest[start + end + 2..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// A line-oriented scan, not a general CSS parser: good enough for a file
/// that is hand-written and has no nested rules.
pub fn parse_rules(css: &str) -> Vec<Rule> {
    let css = strip_comments(css);
    let mut rules = Vec::new();
    let mut rest = css.as_str();
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_string();
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .unwrap_or_else(|| panic!("unterminated CSS rule for selector {selector:?}"));
        let body = &after[..close];
        if selector.starts_with('.') {
            let decls = body
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .filter_map(|decl| decl.split_once(':'))
                .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
                .filter(|(name, _)| name.starts_with("--"))
                .collect();
            rules.push(Rule { selector, decls });
        }
        rest = &after[close + 1..];
    }
    rules
}

pub fn find_decl<'a>(rules: &'a [Rule], selector: &str, var: &str) -> Option<&'a str> {
    rules
        .iter()
        .find(|r| r.selector == selector)?
        .decls
        .iter()
        .find(|(name, _)| name == var)
        .map(|(_, value)| value.as_str())
}

/// One custom property, resolved for one theme × colour-vision × contrast
/// combination: `01-tokens.css`'s own cascade, generalised to any `--name`
/// so `contrast.rs` can resolve whichever tokens its pair table's CSS
/// values happen to name (RFC-097 stage 2 §2.2 added `contrast_high` to
/// what RFC-088 left as a two-axis cascade).
///
/// Applied least to most specific, exactly matching real CSS specificity
/// (`.theme-X` < `.theme-X.contrast-high` = `.theme-X.cv-rgsafe` (tied,
/// broken by *declaration order* in `01-tokens.css`: `.cv-rgsafe` blocks
/// come first, `.contrast-high` blocks after, so `.contrast-high` wins a
/// genuine tie) < `.theme-X.cv-rgsafe.contrast-high`) — a value only
/// overrides when the rule that would declare it actually exists; a
/// selector with no declaration for `var` never masks a less specific
/// one that does. `01-tokens.css`'s own `.theme-light.cv-rgsafe.
/// contrast-high` comment explains why a same-specificity tie was closed
/// by restating the correct value explicitly, rather than relied on here
/// (F220's own lesson: never let two same-specificity rules race for one
/// property on one element) — this function's ordering below exists to
/// match that restated value, not to be the thing that decides it.
pub fn resolve_token(
    rules: &[Rule],
    theme: &str,
    rgsafe: bool,
    contrast_high: bool,
    var: &str,
) -> String {
    let base_sel = format!(".theme-{theme}");
    let mut resolved = find_decl(rules, &base_sel, var)
        .unwrap_or_else(|| panic!("{base_sel} has no {var} declaration in 01-tokens.css"));

    if contrast_high {
        let sel = format!(".theme-{theme}.contrast-high");
        if let Some(v) = find_decl(rules, &sel, var) {
            resolved = v;
        }
    }
    if rgsafe {
        let sel = format!(".theme-{theme}.cv-rgsafe");
        if let Some(v) = find_decl(rules, &sel, var) {
            resolved = v;
        }
    }
    if rgsafe && contrast_high {
        let sel = format!(".theme-{theme}.cv-rgsafe.contrast-high");
        if let Some(v) = find_decl(rules, &sel, var) {
            resolved = v;
        }
    }
    resolved.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_srgb_parses_pure_white_and_black() {
        assert_eq!(hex_to_srgb("#ffffff"), (1.0, 1.0, 1.0));
        assert_eq!(hex_to_srgb("#000000"), (0.0, 0.0, 0.0));
    }

    #[test]
    fn contrast_ratio_of_black_on_white_is_the_wcag_maximum() {
        assert!((contrast_ratio("#000000", "#ffffff") - 21.0).abs() < 0.01);
    }

    #[test]
    fn contrast_ratio_is_symmetric_in_its_two_arguments() {
        let a = contrast_ratio("#336699", "#ffffff");
        let b = contrast_ratio("#ffffff", "#336699");
        assert!((a - b).abs() < 1e-9);
    }

    #[test]
    fn contrast_ratio_rgb_agrees_with_the_hex_form() {
        let hex = contrast_ratio("#336699", "#ffffff");
        let rgb = contrast_ratio_rgb(hex_to_srgb("#336699"), hex_to_srgb("#ffffff"));
        assert!((hex - rgb).abs() < 1e-9);
    }

    #[test]
    fn resolve_token_falls_back_to_the_base_theme_when_the_palette_has_no_override() {
        let css = ".theme-dark { --x: #111111; }\n.theme-dark.cv-rgsafe { --x: #999999; }";
        let rules = parse_rules(css);
        assert_eq!(
            resolve_token(&rules, "dark", false, false, "--x"),
            "#111111"
        );
        assert_eq!(
            resolve_token(&rules, "dark", true, false, "--x"),
            "#999999",
            "the override must win"
        );
    }

    #[test]
    fn resolve_token_falls_back_when_rgsafe_is_true_but_declares_no_override() {
        let css = ".theme-dark { --y: #222222; }";
        let rules = parse_rules(css);
        assert_eq!(
            resolve_token(&rules, "dark", true, false, "--y"),
            "#222222",
            "a token with no override falls back to the base"
        );
    }

    // ── RFC-097 stage 2 §2.2: the third (contrast) axis ─────────────────────

    #[test]
    fn resolve_token_applies_the_contrast_high_override() {
        let css = ".theme-dark { --x: #111111; }\n.theme-dark.contrast-high { --x: #eeeeee; }";
        let rules = parse_rules(css);
        assert_eq!(
            resolve_token(&rules, "dark", false, false, "--x"),
            "#111111"
        );
        assert_eq!(
            resolve_token(&rules, "dark", false, true, "--x"),
            "#eeeeee",
            "the contrast-high override must win"
        );
    }

    #[test]
    fn resolve_token_prefers_the_triple_override_over_either_pair_alone() {
        let css = ".theme-dark { --x: #111111; }\n\
                   .theme-dark.cv-rgsafe { --x: #222222; }\n\
                   .theme-dark.contrast-high { --x: #333333; }\n\
                   .theme-dark.cv-rgsafe.contrast-high { --x: #444444; }";
        let rules = parse_rules(css);
        assert_eq!(
            resolve_token(&rules, "dark", true, true, "--x"),
            "#444444",
            "both axes active must resolve to the selector naming both, not either alone"
        );
    }

    #[test]
    fn resolve_token_contrast_high_alone_does_not_leak_into_rgsafe_only() {
        let css = ".theme-dark { --x: #111111; }\n.theme-dark.contrast-high { --x: #eeeeee; }";
        let rules = parse_rules(css);
        assert_eq!(
            resolve_token(&rules, "dark", true, false, "--x"),
            "#111111",
            "rgsafe alone, with no cv-rgsafe override declared, must still fall back to the \
             base - not to a contrast-high value that doesn't apply here"
        );
    }

    #[test]
    fn parse_rules_skips_a_comment_directly_ahead_of_a_selector() {
        let css = "/* banner */\n.theme-dark { --x: #111111; }";
        let rules = parse_rules(css);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].selector, ".theme-dark");
        assert_eq!(find_decl(&rules, ".theme-dark", "--x"), Some("#111111"));
    }

    #[test]
    fn find_decl_is_none_for_a_missing_selector_or_variable() {
        let rules = parse_rules(".a { --x: #fff; }");
        assert_eq!(find_decl(&rules, ".b", "--x"), None);
        assert_eq!(find_decl(&rules, ".a", "--y"), None);
    }
}
