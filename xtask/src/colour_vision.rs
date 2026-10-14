//! `cargo xtask colour-vision` (RFC-088 §3): measures the diff view's
//! removed/added colour pairs — the line backgrounds and the character
//! marks, for every theme and for both colour-vision palettes — for
//! visibility under colour vision deficiency. Parses `01-tokens.css`
//! directly rather than duplicating the token values in Rust, so the gate
//! checks what actually ships.
//!
//! **Colour-blindness simulation.** Machado, G. M., Oliveira, M. M., &
//! Fernandes, L. A. F. (2009). "A Physiologically-based Model for
//! Simulation of Color Vision Deficiency." IEEE Transactions on
//! Visualization and Computer Graphics, 15(6), 1291–1298. The severity-1.0
//! RGB transform matrices below are the ones commonly derived from that
//! paper's model (as used by, for example, many browser-based simulators)
//! and are applied directly in **linear** RGB — never gamma-encoded sRGB —
//! since that is the model's own working space.
//!
//! **Colour-difference and contrast maths.** CIE76 ΔE, computed in CIELAB
//! against the D65 standard illuminant (plain Euclidean distance in L*a*b*
//! — simpler and more conservative than CIEDE2000, and sufficient at the
//! "is this pair even visually distinguishable" granularity this gate
//! checks). Contrast is the WCAG 2.x relative-luminance ratio
//! (<https://www.w3.org/TR/WCAG21/#contrast-minimum>).
//!
//! **Thresholds (RFC-088 §3).** Standard palette: ΔE ≥ 20 under normal
//! vision only — its known failure under CVD is the reason this RFC
//! exists, not something this gate asserts against. Red–green-safe
//! palette: ΔE ≥ 20 under normal vision, deuteranopia and protanopia.
//! Every pair, regardless of palette: ≥ 4.5 WCAG contrast for `--text` on
//! each background, ≥ 3 for each mark on its own background.

use std::fs;
use std::path::Path;
use std::process;

// ── sRGB / linear / XYZ / Lab ───────────────────────────────────────────────

type Rgb = (f64, f64, f64);

fn hex_to_srgb(hex: &str) -> Rgb {
    let hex = hex.trim_start_matches('#');
    assert_eq!(hex.len(), 6, "expected a 6-digit hex colour, got {hex:?}");
    let chan = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16)
            .unwrap_or_else(|e| panic!("bad hex colour {hex:?}: {e}")) as f64
            / 255.0
    };
    (chan(0..2), chan(2..4), chan(4..6))
}

fn srgb_channel_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_to_linear((r, g, b): Rgb) -> Rgb {
    (
        srgb_channel_to_linear(r),
        srgb_channel_to_linear(g),
        srgb_channel_to_linear(b),
    )
}

/// sRGB primaries to CIEXYZ, D65 white point (the standard sRGB→XYZ matrix).
fn linear_to_xyz((r, g, b): Rgb) -> Rgb {
    (
        0.4124564 * r + 0.3575761 * g + 0.1804375 * b,
        0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        0.0193339 * r + 0.1191920 * g + 0.9503041 * b,
    )
}

/// CIEXYZ to CIELAB, D65 reference white (Xn, Yn, Zn below).
fn xyz_to_lab((x, y, z): Rgb) -> Rgb {
    const XN: f64 = 0.95047;
    const YN: f64 = 1.0;
    const ZN: f64 = 1.08883;
    const DELTA: f64 = 6.0 / 29.0;
    let f = |t: f64| {
        if t > DELTA.powi(3) {
            t.cbrt()
        } else {
            t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x / XN), f(y / YN), f(z / ZN));
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

fn delta_e76(lab1: Rgb, lab2: Rgb) -> f64 {
    let d0 = lab1.0 - lab2.0;
    let d1 = lab1.1 - lab2.1;
    let d2 = lab1.2 - lab2.2;
    (d0 * d0 + d1 * d1 + d2 * d2).sqrt()
}

// ── Colour vision deficiency simulation (Machado et al. 2009) ──────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Simulation {
    Normal,
    Deuteranopia,
    Protanopia,
}

impl Simulation {
    fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Deuteranopia => "deuteranopia",
            Self::Protanopia => "protanopia",
        }
    }
}

/// Severity-1.0 simulation, applied in *linear* RGB. `Normal` is the
/// identity — simulating "normal vision" is simply not altering the colour.
fn simulate((r, g, b): Rgb, sim: Simulation) -> Rgb {
    let raw = match sim {
        Simulation::Normal => return (r, g, b),
        Simulation::Protanopia => (
            0.152286 * r + 1.052583 * g + -0.204868 * b,
            0.114503 * r + 0.786281 * g + 0.099216 * b,
            -0.003882 * r + -0.048116 * g + 1.051998 * b,
        ),
        Simulation::Deuteranopia => (
            0.367322 * r + 0.860646 * g + -0.227968 * b,
            0.280085 * r + 0.672501 * g + 0.047413 * b,
            -0.011820 * r + 0.042940 * g + 0.968881 * b,
        ),
    };
    // The matrices can produce components slightly outside [0, 1] near the
    // gamut edge; clamped before XYZ/Lab conversion, the standard practice
    // for this simulation (an unclamped negative drives `xyz_to_lab`'s cube
    // root into complex-number territory for no physical colour).
    (
        raw.0.clamp(0.0, 1.0),
        raw.1.clamp(0.0, 1.0),
        raw.2.clamp(0.0, 1.0),
    )
}

/// ΔE (CIE76) between two sRGB hex colours, each run through `sim` first.
fn simulated_delta_e(hex1: &str, hex2: &str, sim: Simulation) -> f64 {
    let lab = |hex: &str| {
        xyz_to_lab(linear_to_xyz(simulate(
            srgb_to_linear(hex_to_srgb(hex)),
            sim,
        )))
    };
    delta_e76(lab(hex1), lab(hex2))
}

// ── WCAG contrast ────────────────────────────────────────────────────────

fn relative_luminance(hex: &str) -> f64 {
    let (r, g, b) = srgb_to_linear(hex_to_srgb(hex));
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn contrast_ratio(hex1: &str, hex2: &str) -> f64 {
    let (l1, l2) = (relative_luminance(hex1), relative_luminance(hex2));
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

// ── Parsing 01-tokens.css ────────────────────────────────────────────────

/// One `.selector { --name: value; ... }` rule, in source order.
struct Rule {
    selector: String,
    decls: Vec<(String, String)>,
}

/// Strips every `/* ... */` comment. Comment text ahead of a selector (for
/// example this file's own section banners) would otherwise become part of
/// the "selector" the naive scan below extracts, failing the `starts_with
/// ('.')` check and silently dropping the whole rule — caught by running
/// this gate against the real file, not by inspection, when the
/// `.cv-rgsafe` rules right after a banner comment resolved to nothing.
fn strip_comments(css: &str) -> String {
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
fn parse_rules(css: &str) -> Vec<Rule> {
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

fn find_decl<'a>(rules: &'a [Rule], selector: &str, var: &str) -> Option<&'a str> {
    rules
        .iter()
        .find(|r| r.selector == selector)?
        .decls
        .iter()
        .find(|(name, _)| name == var)
        .map(|(_, value)| value.as_str())
}

const TOKENS: [&str; 5] = ["--text", "--del-bg", "--ins-bg", "--del-mark", "--ins-mark"];

/// The five tokens this gate needs, resolved for one theme × palette
/// combination: the palette's own override where `01-tokens.css` declares
/// one (`.theme-X.cv-rgsafe`), else the base theme's value — exactly CSS's
/// own cascade, since `.theme-X.cv-rgsafe` has higher specificity and the
/// browser resolves it the same way.
fn resolve(rules: &[Rule], theme: &str, rgsafe: bool) -> [String; 5] {
    let base_sel = format!(".theme-{theme}");
    let rgsafe_sel = format!(".theme-{theme}.cv-rgsafe");
    TOKENS.map(|var| {
        let base = find_decl(rules, &base_sel, var)
            .unwrap_or_else(|| panic!("{base_sel} has no {var} declaration in 01-tokens.css"));
        if rgsafe {
            find_decl(rules, &rgsafe_sel, var).unwrap_or(base)
        } else {
            base
        }
        .to_string()
    })
}

// ── The gate itself ──────────────────────────────────────────────────────

const THEMES: [&str; 3] = ["dark", "light", "night"];
const DELTA_E_MIN: f64 = 20.0;
const TEXT_CONTRAST_MIN: f64 = 4.5;
const MARK_CONTRAST_MIN: f64 = 3.0;

pub fn run(root: &Path) {
    let css_path = root.join("crates/forskscope-ui/assets/css/01-tokens.css");
    let css = fs::read_to_string(&css_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", css_path.display()));
    let rules = parse_rules(&css);

    let mut problems = Vec::new();

    for &theme in &THEMES {
        for &rgsafe in &[false, true] {
            let [text, del_bg, ins_bg, del_mark, ins_mark] = resolve(&rules, theme, rgsafe);
            let palette = if rgsafe { "red-green-safe" } else { "standard" };

            let sims: &[Simulation] = if rgsafe {
                &[
                    Simulation::Normal,
                    Simulation::Deuteranopia,
                    Simulation::Protanopia,
                ]
            } else {
                // The standard palette's known failure under CVD is this
                // RFC's own motivation, not something this gate asserts
                // against — only its normal-vision ΔE is gated.
                &[Simulation::Normal]
            };

            for &sim in sims {
                let bg_de = simulated_delta_e(&del_bg, &ins_bg, sim);
                if bg_de < DELTA_E_MIN {
                    problems.push(format!(
                        "{theme}/{palette}: background pair ({del_bg} vs {ins_bg}) under {} \
                         has ΔE {bg_de:.1}, below the required {DELTA_E_MIN:.1}",
                        sim.label(),
                    ));
                }
                let mark_de = simulated_delta_e(&del_mark, &ins_mark, sim);
                if mark_de < DELTA_E_MIN {
                    problems.push(format!(
                        "{theme}/{palette}: mark pair ({del_mark} vs {ins_mark}) under {} \
                         has ΔE {mark_de:.1}, below the required {DELTA_E_MIN:.1}",
                        sim.label(),
                    ));
                }
            }

            let text_on_del = contrast_ratio(&text, &del_bg);
            if text_on_del < TEXT_CONTRAST_MIN {
                problems.push(format!(
                    "{theme}/{palette}: text {text} on del-bg {del_bg} has contrast \
                     {text_on_del:.1}, below the required {TEXT_CONTRAST_MIN:.1}"
                ));
            }
            let text_on_ins = contrast_ratio(&text, &ins_bg);
            if text_on_ins < TEXT_CONTRAST_MIN {
                problems.push(format!(
                    "{theme}/{palette}: text {text} on ins-bg {ins_bg} has contrast \
                     {text_on_ins:.1}, below the required {TEXT_CONTRAST_MIN:.1}"
                ));
            }
            let del_mark_contrast = contrast_ratio(&del_mark, &del_bg);
            if del_mark_contrast < MARK_CONTRAST_MIN {
                problems.push(format!(
                    "{theme}/{palette}: del-mark {del_mark} on del-bg {del_bg} has contrast \
                     {del_mark_contrast:.1}, below the required {MARK_CONTRAST_MIN:.1}"
                ));
            }
            let ins_mark_contrast = contrast_ratio(&ins_mark, &ins_bg);
            if ins_mark_contrast < MARK_CONTRAST_MIN {
                problems.push(format!(
                    "{theme}/{palette}: ins-mark {ins_mark} on ins-bg {ins_bg} has contrast \
                     {ins_mark_contrast:.1}, below the required {MARK_CONTRAST_MIN:.1}"
                ));
            }
        }
    }

    if !problems.is_empty() {
        eprintln!("colour-vision check failed:");
        for p in &problems {
            eprintln!("  - {p}");
        }
        process::exit(1);
    }
    println!(
        "colour-vision check passed: every removed/added pair in every theme and palette stays \
         distinguishable (ΔE ≥ {DELTA_E_MIN:.0}) under normal vision, and red-green-safe stays \
         distinguishable under deuteranopia and protanopia too; text and mark contrast meet WCAG."
    );
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
    fn delta_e_of_a_colour_against_itself_is_zero() {
        assert_eq!(
            simulated_delta_e("#4d2e14", "#4d2e14", Simulation::Normal),
            0.0
        );
    }

    #[test]
    fn contrast_ratio_of_black_on_white_is_the_wcag_maximum() {
        let ratio = contrast_ratio("#000000", "#ffffff");
        assert!((ratio - 21.0).abs() < 0.01, "got {ratio}");
    }

    #[test]
    fn contrast_ratio_is_symmetric_in_its_two_arguments() {
        assert_eq!(
            contrast_ratio("#123456", "#abcdef"),
            contrast_ratio("#abcdef", "#123456")
        );
    }

    /// Falsify by changing a `.theme-dark.cv-rgsafe` token to its
    /// `.theme-dark` value in `01-tokens.css`: this parses it right back
    /// out as equal to the base, and the gate's own ΔE check against that
    /// pair then fails (proving the gate reads the override, not a cached
    /// or hardcoded value).
    #[test]
    fn resolve_falls_back_to_the_base_theme_when_the_palette_has_no_override() {
        let css = ".theme-dark { --text: #e6e6e6; --del-bg: #111111; --ins-bg: #222222; \
                    --del-mark: #333333; --ins-mark: #444444; }\n\
                    .theme-dark.cv-rgsafe { --del-bg: #999999; }";
        let rules = parse_rules(css);
        let standard = resolve(&rules, "dark", false);
        let rgsafe = resolve(&rules, "dark", true);
        assert_eq!(standard[1], "#111111");
        assert_eq!(rgsafe[1], "#999999", "the override must win");
        assert_eq!(
            rgsafe[2], "#222222",
            "a token with no override falls back to the base"
        );
    }
}
