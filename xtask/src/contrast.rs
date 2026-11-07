//! `cargo xtask contrast` (F214/F215/F216, handoff 094): the readability
//! floor. Three checks, all against the real CSS sources under
//! `crates/forskscope-ui/assets/css/`, not a second copy of their values:
//!
//! 1. A declared table of foreground/background token pairs, each with its
//!    WCAG 2.x minimum (4.5 for text, 3 for a control's identifying edge),
//!    resolved from `01-tokens.css` for every theme.
//! 2. No `opacity` below 1 on a selector outside a reasoned allowlist.
//! 3. No `font-size` below 12px outside a reasoned allowlist of non-text
//!    glyphs.
//!
//! **Why not every theme × colour-vision combination (6, not 3), the way
//! `colour-vision.rs` covers its own six del/ins/rep tokens:** none of the
//! tokens the pair table names (`--text`, `--muted`, `--accent`, `--warn`,
//! `--ok`, `--err`, `--control-border`, `--surface`/`-2`/`-3`) has a
//! `.theme-X.cv-rgsafe` override in `01-tokens.css` — `cargo xtask
//! colour-vision` already owns the six tokens that do (`--del-bg`,
//! `--ins-bg`, `--del-mark`, `--ins-mark`, `--rep-bg`, and `--text` against
//! each), including their own 4.5/3.0 contrast floors, across every theme
//! and both palettes. Re-checking them here would just repeat that gate's
//! own numbers, not add anything a colour-vision switch could change.
//!
//! **How the pair table was found (F214's own question):** every
//! `color: var(--x)` in `assets/css/*.css` whose element's own background
//! (or its nearest ancestor's, read from the component tree and the CSS
//! around it, not inferred mechanically) is one of `--surface`/`-2`/`-3`,
//! paired with that background; every control's identifying edge
//! (`border: ... var(--control-border)` after this handoff's own fix)
//! against the same three. Pairs against `--del-bg`/`--ins-bg`/`--rep-bg`
//! are colour-vision's, not repeated here. No pair in the table could not
//! be determined this way.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use crate::css_shared::{contrast_ratio, find_decl, parse_rules, strip_comments};

// ── 1. Declared foreground/background pairs ─────────────────────────────

struct Pair {
    label: &'static str,
    fg: &'static str,
    bg: &'static str,
    min: f64,
}

const TEXT_MIN: f64 = 4.5;
const CONTROL_MIN: f64 = 3.0;

/// Every text-bearing token against every surface it is actually painted
/// on (`--text`/`--muted`: the app body, tree rows, status lines, badges;
/// `--accent`: links, the Compare button, pressed/active toggles;
/// `--warn`/`--ok`/`--err`: status glyphs, toasts, the statusbar), plus
/// `--control-border`, the one edge token this handoff adds (F215).
#[rustfmt::skip]
const PAIRS: &[Pair] = &[
    Pair { label: "--text on --surface",            fg: "--text",           bg: "--surface",   min: TEXT_MIN },
    Pair { label: "--text on --surface-2",          fg: "--text",           bg: "--surface-2", min: TEXT_MIN },
    Pair { label: "--text on --surface-3",          fg: "--text",           bg: "--surface-3", min: TEXT_MIN },
    Pair { label: "--muted on --surface",           fg: "--muted",          bg: "--surface",   min: TEXT_MIN },
    Pair { label: "--muted on --surface-2",         fg: "--muted",          bg: "--surface-2", min: TEXT_MIN },
    Pair { label: "--muted on --surface-3",         fg: "--muted",          bg: "--surface-3", min: TEXT_MIN },
    Pair { label: "--accent on --surface",          fg: "--accent",         bg: "--surface",   min: TEXT_MIN },
    Pair { label: "--accent on --surface-2",        fg: "--accent",         bg: "--surface-2", min: TEXT_MIN },
    Pair { label: "--accent on --surface-3",        fg: "--accent",         bg: "--surface-3", min: TEXT_MIN },
    Pair { label: "--warn on --surface",            fg: "--warn",           bg: "--surface",   min: TEXT_MIN },
    Pair { label: "--warn on --surface-2",          fg: "--warn",           bg: "--surface-2", min: TEXT_MIN },
    Pair { label: "--warn on --surface-3",          fg: "--warn",           bg: "--surface-3", min: TEXT_MIN },
    Pair { label: "--ok on --surface",              fg: "--ok",             bg: "--surface",   min: TEXT_MIN },
    Pair { label: "--ok on --surface-2",            fg: "--ok",             bg: "--surface-2", min: TEXT_MIN },
    Pair { label: "--ok on --surface-3",             fg: "--ok",             bg: "--surface-3", min: TEXT_MIN },
    Pair { label: "--err on --surface",             fg: "--err",            bg: "--surface",   min: TEXT_MIN },
    Pair { label: "--err on --surface-2",           fg: "--err",            bg: "--surface-2", min: TEXT_MIN },
    Pair { label: "--err on --surface-3",           fg: "--err",            bg: "--surface-3", min: TEXT_MIN },
    Pair { label: "--control-border on --surface",   fg: "--control-border", bg: "--surface",   min: CONTROL_MIN },
    Pair { label: "--control-border on --surface-2", fg: "--control-border", bg: "--surface-2", min: CONTROL_MIN },
    Pair { label: "--control-border on --surface-3", fg: "--control-border", bg: "--surface-3", min: CONTROL_MIN },
];

const THEMES: [&str; 3] = ["dark", "light", "night"];

fn check_pairs(root: &Path, problems: &mut Vec<String>) {
    let css_path = root.join("crates/forskscope-ui/assets/css/01-tokens.css");
    let css = fs::read_to_string(&css_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", css_path.display()));
    let rules = parse_rules(&css);

    for &theme in &THEMES {
        let sel = format!(".theme-{theme}");
        for pair in PAIRS {
            let fg = find_decl(&rules, &sel, pair.fg)
                .unwrap_or_else(|| panic!("{sel} has no {} declaration in 01-tokens.css", pair.fg));
            let bg = find_decl(&rules, &sel, pair.bg)
                .unwrap_or_else(|| panic!("{sel} has no {} declaration in 01-tokens.css", pair.bg));
            let ratio = contrast_ratio(fg, bg);
            if ratio < pair.min {
                problems.push(format!(
                    "{theme}: {} ({fg} on {bg}) has contrast {ratio:.2}, below the required {:.1}",
                    pair.label, pair.min
                ));
            }
        }
    }
}

// ── 2 & 3. Opacity and font-size, scanned across every assets/css/*.css ──

/// A CSS file's `selector { decl: value; ... }` rules, keeping every
/// selector (not just `.`-prefixed ones — `button:disabled`, `input,
/// select` and the rest are exactly what these two checks need to see)
/// and every declaration (not just `--`-prefixed custom properties, the
/// filter `css_shared::parse_rules` applies for the token file it reads).
/// A second, differently-filtered parser rather than parameterising the
/// shared one: the two have different jobs on different files, and a
/// shared filter closure would need to thread two unrelated predicates
/// through every call site that uses either.
struct CssRule {
    selector: String,
    decls: Vec<(String, String)>,
}

/// The index just past this body's *matching* closing `}`, counting
/// nested `{`/`}` pairs rather than stopping at the first `}` - an
/// `@keyframes` block's own `to { ... }` (or `from`/percentage) step
/// nests one level deeper, and stopping early there corrupts everything
/// parsed after it (caught by `parse_css_rules_skips_at_rules` failing
/// without this, not by inspection: the stray trailing `}` from the
/// inner block became the start of the next "selector").
fn matching_close(body: &str) -> usize {
    let mut depth = 1usize;
    for (i, ch) in body.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    panic!("unterminated CSS block");
}

fn parse_css_rules(css: &str) -> Vec<CssRule> {
    let css = strip_comments(css);
    let mut rules = Vec::new();
    let mut rest = css.as_str();
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_string();
        let after = &rest[open + 1..];
        let close = matching_close(after);
        let body = &after[..close];
        if !selector.is_empty() && !selector.starts_with('@') {
            let decls = body
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .filter_map(|decl| decl.split_once(':'))
                .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
                .collect();
            rules.push(CssRule { selector, decls });
        }
        rest = &after[close + 1..];
    }
    rules
}

fn css_files(root: &Path) -> Vec<PathBuf> {
    let css_dir = root.join("crates/forskscope-ui/assets/css");
    let mut entries: Vec<PathBuf> = fs::read_dir(&css_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", css_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("css"))
        .collect();
    entries.sort();
    entries
}

/// One allowlist entry: `selector` is the rule's full selector text
/// exactly as it appears in the source (a comma-separated group matches
/// as one entry, the same shape the rule itself has), `reason` is why it
/// is allowed, printed as part of this gate's own output so the allowlist
/// doubles as its own documentation.
struct Allowed {
    selector: &'static str,
    reason: &'static str,
}

const OPACITY_ALLOWLIST: &[Allowed] = &[
    Allowed {
        selector: ".profile-radio",
        reason: "visually hidden: a native radio input (appearance:none), not removed from \
                 the accessibility tree or keyboard - the visible, clickable indicator is the \
                 `▸`/space mark already in .profile-name's own text",
    },
    Allowed {
        selector: "button:disabled",
        reason: "disabled: WCAG 1.4.3/1.4.11 exempt disabled controls from contrast \
                 requirements",
    },
    Allowed {
        selector: ".search-nav:disabled",
        reason: "disabled",
    },
    Allowed {
        selector: ".tab-close",
        reason: "hover-revealed control: invisible until hover/active, RFC-097 stage 3 \
                 removes this interaction pattern rather than fixing its resting contrast",
    },
    Allowed {
        selector: ".tab-slot:hover .tab-close, .tab-slot:has(.tab.active) .tab-close",
        reason: "hover-revealed control, RFC-097 stage 3",
    },
    Allowed {
        selector: ".tab-slot:hover .tab-close:hover",
        reason: "hover-revealed control, RFC-097 stage 3",
    },
    Allowed {
        selector: ".deep-compare-btn",
        reason: "hover-revealed control: dimmed at rest, full at hover, RFC-097 stage 3 \
                 removes this interaction pattern rather than fixing its resting contrast",
    },
    Allowed {
        selector: ".deep-row:hover .deep-compare-btn",
        reason: "hover-revealed control, RFC-097 stage 3",
    },
];

const FONT_SIZE_ALLOWLIST: &[Allowed] = &[
    Allowed {
        selector: ".dirty-dot",
        reason: "non-text glyph (a status dot, \"●\"), aria-hidden - the tab's own accessible \
                 name carries the state in words (\", unsaved\", RFC-087 §1)",
    },
    Allowed {
        selector: ".tab-loading-spinner",
        reason: "non-text glyph (a spinner, \"⟳\"), aria-hidden - the tab's own accessible \
                 name carries the state in words (\", loading\", RFC-087 §1)",
    },
];

fn check_opacity(root: &Path, problems: &mut Vec<String>) {
    for path in css_files(root) {
        let css = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        for rule in parse_css_rules(&css) {
            let Some((_, value)) = rule.decls.iter().find(|(name, _)| name == "opacity") else {
                continue;
            };
            let Ok(opacity) = value.parse::<f64>() else {
                continue;
            };
            if opacity >= 1.0 {
                continue;
            }
            if OPACITY_ALLOWLIST
                .iter()
                .any(|a| a.selector == rule.selector)
            {
                continue;
            }
            problems.push(format!(
                "{}: `{}` has `opacity: {value}` with no allowlist entry - either the \
                 element is exempt (disabled, or a hover-revealed control RFC-097 stage 3 \
                 removes) and belongs in OPACITY_ALLOWLIST with its reason, or the opacity \
                 itself is the defect",
                path.display(),
                rule.selector
            ));
        }
    }
}

fn check_font_size(root: &Path, problems: &mut Vec<String>) {
    for path in css_files(root) {
        let css = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        for rule in parse_css_rules(&css) {
            let Some((_, value)) = rule.decls.iter().find(|(name, _)| name == "font-size") else {
                continue;
            };
            let Some(px) = value.strip_suffix("px") else {
                continue;
            };
            let Ok(px) = px.parse::<f64>() else {
                continue;
            };
            if px >= 12.0 {
                continue;
            }
            if FONT_SIZE_ALLOWLIST
                .iter()
                .any(|a| a.selector == rule.selector)
            {
                continue;
            }
            problems.push(format!(
                "{}: `{}` has `font-size: {value}` with no allowlist entry - either it is a \
                 non-text glyph (aria-hidden, with its meaning given elsewhere) and belongs \
                 in FONT_SIZE_ALLOWLIST with its reason, or it must be raised to at least \
                 12px",
                path.display(),
                rule.selector
            ));
        }
    }
}

// ── The gate itself ──────────────────────────────────────────────────────

pub fn run(root: &Path) {
    let mut problems = Vec::new();
    check_pairs(root, &mut problems);
    check_opacity(root, &mut problems);
    check_font_size(root, &mut problems);

    if !problems.is_empty() {
        eprintln!("contrast check failed:");
        for p in &problems {
            eprintln!("  - {p}");
        }
        process::exit(1);
    }

    println!(
        "contrast check passed: every declared foreground/background pair meets its WCAG \
         minimum in every theme ({} pairs x {} themes); no `opacity` below 1 and no \
         `font-size` below 12px outside their reasoned allowlists ({} opacity, {} font-size):",
        PAIRS.len(),
        THEMES.len(),
        OPACITY_ALLOWLIST.len(),
        FONT_SIZE_ALLOWLIST.len()
    );
    for a in OPACITY_ALLOWLIST {
        println!("  opacity allowed: `{}` - {}", a.selector, a.reason);
    }
    for a in FONT_SIZE_ALLOWLIST {
        println!("  font-size allowed: `{}` - {}", a.selector, a.reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_css_rules_keeps_pseudo_class_and_compound_selectors() {
        let css = "button:disabled { opacity: .45; }\n\
                    input, select { border: 1px solid var(--control-border); }";
        let rules = parse_css_rules(css);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selector, "button:disabled");
        assert_eq!(rules[1].selector, "input, select");
        assert_eq!(
            rules[0].decls,
            vec![("opacity".to_string(), ".45".to_string())]
        );
    }

    #[test]
    fn parse_css_rules_skips_at_rules_and_their_nested_blocks() {
        let css = "@keyframes spin { to { transform: rotate(360deg); } }\n.x { opacity: .5; }";
        let rules = parse_css_rules(css);
        // The whole `@keyframes` block, nested `to { ... }` step included,
        // is skipped as one balanced unit - falsified by reverting
        // `matching_close` to a plain `.find('}')`: the inner step's own
        // closing brace then ends the "rule" early, and the stray
        // trailing `}` corrupts the selector this test looks for next.
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].selector, ".x");
    }

    #[test]
    fn check_pairs_fails_a_pair_below_its_minimum() {
        let mut problems = Vec::new();
        // A tiny, self-contained token file: `--accent` on `--surface-3`
        // is deliberately near-identical (a real, measured failure shape),
        // every other pair trivially passes (black text on white, etc.).
        let root = fake_workspace(
            line!(),
            "01-tokens.css",
            ".theme-dark { --surface: #000000; --surface-2: #000000; --surface-3: #101010; \
             --text: #ffffff; --muted: #ffffff; --accent: #111111; --warn: #ffffff; \
             --ok: #ffffff; --err: #ffffff; --control-border: #ffffff; }\n\
             .theme-light { --surface: #000000; --surface-2: #000000; --surface-3: #000000; \
             --text: #ffffff; --muted: #ffffff; --accent: #ffffff; --warn: #ffffff; \
             --ok: #ffffff; --err: #ffffff; --control-border: #ffffff; }\n\
             .theme-night { --surface: #000000; --surface-2: #000000; --surface-3: #000000; \
             --text: #ffffff; --muted: #ffffff; --accent: #ffffff; --warn: #ffffff; \
             --ok: #ffffff; --err: #ffffff; --control-border: #ffffff; }\n",
        );
        check_pairs(&root, &mut problems);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("dark") && p.contains("--accent on --surface-3")),
            "expected a dark/--accent-on-surface-3 failure among {problems:?}"
        );
    }

    #[test]
    fn check_opacity_flags_an_unlisted_opacity_below_one() {
        let mut problems = Vec::new();
        let root = fake_workspace(line!(), "99-fixture.css", ".not-allowed { opacity: .5; }\n");
        check_opacity(&root, &mut problems);
        assert!(
            problems.iter().any(|p| p.contains(".not-allowed")),
            "expected a problem naming .not-allowed among {problems:?}"
        );
    }

    #[test]
    fn check_opacity_accepts_an_allowlisted_selector() {
        let mut problems = Vec::new();
        let root = fake_workspace(
            line!(),
            "99-fixture.css",
            "button:disabled { opacity: .45; }\n",
        );
        check_opacity(&root, &mut problems);
        assert!(
            problems.is_empty(),
            "expected no problems, got {problems:?}"
        );
    }

    #[test]
    fn check_font_size_flags_an_unlisted_small_size() {
        let mut problems = Vec::new();
        let root = fake_workspace(
            line!(),
            "99-fixture.css",
            ".not-allowed { font-size: 11px; }\n",
        );
        check_font_size(&root, &mut problems);
        assert!(
            problems.iter().any(|p| p.contains(".not-allowed")),
            "expected a problem naming .not-allowed among {problems:?}"
        );
    }

    #[test]
    fn check_font_size_accepts_an_allowlisted_selector() {
        let mut problems = Vec::new();
        let root = fake_workspace(
            line!(),
            "99-fixture.css",
            ".dirty-dot { font-size: 10px; }\n",
        );
        check_font_size(&root, &mut problems);
        assert!(
            problems.is_empty(),
            "expected no problems, got {problems:?}"
        );
    }

    #[test]
    fn check_font_size_accepts_twelve_px_and_above() {
        let mut problems = Vec::new();
        let root = fake_workspace(
            line!(),
            "99-fixture.css",
            ".fine { font-size: 12px; }\n.also-fine { font-size: 14px; }\n",
        );
        check_font_size(&root, &mut problems);
        assert!(
            problems.is_empty(),
            "expected no problems, got {problems:?}"
        );
    }

    // ── test fixture: a throwaway crates/forskscope-ui/assets/css/ tree,
    //    the same std::env::temp_dir() + pid + line!() convention
    //    crate_boundaries.rs's own tests already use - no tempfile
    //    dependency for a handful of single-file fixtures. ──

    fn fake_workspace(unique: u32, filename: &str, contents: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "fsk-xtask-contrast-{}-{unique}",
            std::process::id()
        ));
        let css_dir = root.join("crates/forskscope-ui/assets/css");
        fs::create_dir_all(&css_dir).expect("create css dir");
        fs::write(css_dir.join(filename), contents).expect("write fixture css");
        root
    }
}
