//! `cargo xtask contrast` (F214/F215/F216, handoff 094; pair table and
//! coverage check rebuilt per review 196): the readability floor. Four
//! checks, all against the real CSS sources under
//! `crates/forskscope-ui/assets/css/`, not a second copy of their values:
//!
//! 1. A declared table of foreground/background pairs, each with its WCAG
//!    2.x minimum (4.5 for text, 3 for a control's identifying edge or a
//!    mark), resolved from `01-tokens.css` for every theme × colour-vision
//!    combination (3 × 2 = 6) — `.cv-rgsafe` overrides `--del-bg`/`--ins-bg`/
//!    `--del-mark`/`--ins-mark`/`--rep-bg`/`--rep-mark` in `.theme-light`
//!    only, and all six now appear in this table (review 196 found the
//!    first version's claim that they didn't was the bug, not a fact).
//! 2. No `opacity` below 1 on a selector outside a reasoned allowlist.
//! 3. No `font-size` below 12px outside a reasoned allowlist of non-text
//!    glyphs.
//! 4. Every foreground token named in a `color:` declaration, and every
//!    background expression named in a `background`/`background-color`
//!    declaration, across every file in `assets/css/`, appears in at least
//!    one table entry — so the next token this app paints text or a
//!    background with can't quietly miss the table the way review 196
//!    found several already had.
//!
//! **How the pair table was found, this time (review 196's own question).**
//! Every `color:` declaration's value, and every `background`/
//! `background-color` declaration's value, across `assets/css/*.css` (check
//! 4 now enforces this is the *complete* list, not a sample of it). A value
//! that is a plain `var(--x)` pairs with whatever sits behind that element;
//! a `color-mix(in srgb, …)` value is resolved by compositing its two
//! colours at the stated percentage, in sRGB, matching the `in srgb`
//! keyword's own browser behaviour. Several mixes were written against
//! `transparent` with the real backdrop left to be inferred from the
//! element tree; where that real backdrop is *fixed* (nothing else is ever
//! painted behind that element), the CSS itself now names it directly —
//! `.tree-row.selected`, `.profile-row.active`, `.diff-row.match` and
//! `.diff-warning-banner` — so the table's `bg` text matches the CSS
//! verbatim and check 4 can confirm it without a side channel. Where the
//! real backdrop genuinely varies at render time (`.in-del`/`.in-ins`, an
//! inline span that can sit over a replaced line's own background *or* a
//! search match's, depending on what the row around it is doing), the CSS
//! keeps `transparent` and the table's entry says which backdrop it
//! assumed and why (`ASSUMED_OVER`, below) — exactly the "state the
//! assumption" review 196 asked for in that case.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use crate::css_shared::{Rgb, contrast_ratio_rgb, hex_to_srgb, resolve_token};

// ── 1. Declared foreground/background pairs ─────────────────────────────

struct Pair {
    label: &'static str,
    /// A `color:` declaration's literal value, e.g. `"var(--accent)"`.
    fg: &'static str,
    /// A `background`/`background-color` declaration's literal value,
    /// exactly as it appears (or would appear, for `.in-del`/`.in-ins`'s
    /// case) in `assets/css/*.css` — this is also what check 4 matches
    /// against to confirm the pair table covers it.
    bg: &'static str,
    /// Set only when `bg` contains `transparent` and the real backdrop
    /// can't be read off the CSS alone (an inline span whose true backdrop
    /// depends on the row around it at render time) — the stated
    /// assumption review 196 asked every such entry to carry.
    assumed_over: Option<&'static str>,
    min: f64,
    /// RFC-097 stage 2 §2.2: the tighter floor *High* contrast raises this
    /// pair to, checked instead of `min` under the `contrast-high` axis.
    /// `None` for every pair *High* doesn't touch at all (most of them —
    /// nothing about those tokens changes under `.contrast-high`, so the
    /// same `min` applies in both contrast levels).
    high_min: Option<f64>,
}

const TEXT_MIN: f64 = 4.5;
const CONTROL_MIN: f64 = 3.0;
/// RFC-097 stage 2 §2.2: "secondary text (--muted) to at least 7:1 on
/// every surface" (handoff 098 §2.2).
const HIGH_TEXT_MIN: f64 = 7.0;
/// "control edges and the diff's marks to at least 4.5:1" (handoff 098
/// §2.2) - the same floor `TEXT_MIN` already uses for ordinary text, but
/// named separately so the two can diverge later without one dragging
/// the other (the same reasoning F215's own `--control-border` comment,
/// 01-tokens.css, gives for not aliasing a value that happens to equal
/// another one today).
const HIGH_MARK_MIN: f64 = 4.5;

/// Every foreground token this app actually paints with `color:`, against
/// every background expression it is actually painted on with `background`/
/// `background-color` — check 4 enforces that this is the complete list,
/// not a sample. `--control-border` pairs are the one exception: a
/// `border:` colour, not a `color:`/`background:` value, so check 4 never
/// requires them, but F215 still needs them checked and this is where
/// `cargo xtask contrast` has always put them.
#[rustfmt::skip]
const PAIRS: &[Pair] = &[
    // ── Plain text/mark on a plain surface ──
    Pair { label: "--text on --surface",            fg: "var(--text)",   bg: "var(--surface)",   assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--text on --surface-2",          fg: "var(--text)",   bg: "var(--surface-2)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--text on --surface-3",          fg: "var(--text)",   bg: "var(--surface-3)", assumed_over: None, min: TEXT_MIN, high_min: None },
    // RFC-097 stage 2 §2.2: High raises secondary text to "the main text's
    // colour or close to it, at least 7:1" on every surface it appears on
    // - these three, the diff gutter's two below, and a selected tree
    // row's own --muted entry further down all gain high_min: Some(HIGH_TEXT_MIN).
    Pair { label: "--muted on --surface",           fg: "var(--muted)",  bg: "var(--surface)",   assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    Pair { label: "--muted on --surface-2",         fg: "var(--muted)",  bg: "var(--surface-2)", assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    Pair { label: "--muted on --surface-3",         fg: "var(--muted)",  bg: "var(--surface-3)", assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    Pair { label: "--accent on --surface",          fg: "var(--accent)", bg: "var(--surface)",   assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--accent on --surface-2",        fg: "var(--accent)", bg: "var(--surface-2)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--accent on --surface-3",        fg: "var(--accent)", bg: "var(--surface-3)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--warn on --surface",            fg: "var(--warn)",   bg: "var(--surface)",   assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--warn on --surface-2",          fg: "var(--warn)",   bg: "var(--surface-2)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--warn on --surface-3",          fg: "var(--warn)",   bg: "var(--surface-3)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--ok on --surface",              fg: "var(--ok)",     bg: "var(--surface)",   assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--ok on --surface-2",            fg: "var(--ok)",     bg: "var(--surface-2)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--ok on --surface-3",             fg: "var(--ok)",     bg: "var(--surface-3)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--err on --surface",             fg: "var(--err)",    bg: "var(--surface)",   assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--err on --surface-2",           fg: "var(--err)",    bg: "var(--surface-2)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--err on --surface-3",           fg: "var(--err)",    bg: "var(--surface-3)", assumed_over: None, min: TEXT_MIN, high_min: None },
    // --ins-mark doubles as the Explorer's "equal" status glyph
    // (`.tree-status.status-equal`) - a *text* use, F216 already treats
    // Explorer status symbols as text, so it needs the same three pairs
    // every other status colour gets. (review 196 found light's
    // standard-palette value missed --surface-3 here: 4.21, fixed to
    // #187533, now 4.80 - the mark-on-its-own-background use below is
    // unaffected, still comfortably over 3:1.)
    Pair { label: "--ins-mark on --surface (status text)",   fg: "var(--ins-mark)", bg: "var(--surface)",   assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--ins-mark on --surface-2 (status text)", fg: "var(--ins-mark)", bg: "var(--surface-2)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--ins-mark on --surface-3 (status text)", fg: "var(--ins-mark)", bg: "var(--surface-3)", assumed_over: None, min: TEXT_MIN, high_min: None },
    // ── Text on a diff hunk's own row background ──
    Pair { label: "--text on --del-bg",   fg: "var(--text)",  bg: "var(--del-bg)", assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    Pair { label: "--text on --ins-bg",   fg: "var(--text)",  bg: "var(--ins-bg)", assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    Pair { label: "--text on --rep-bg",   fg: "var(--text)",  bg: "var(--rep-bg)", assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    // `.pane-gutter`'s line numbers: --muted, overridden to --del-bg/--ins-bg
    // on a del/ins row (no rep-bg variant - a replace hunk's gutter keeps
    // the del/ins split per side, never --rep-bg itself).
    Pair { label: "--muted on --del-bg (gutter)", fg: "var(--muted)", bg: "var(--del-bg)", assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    Pair { label: "--muted on --ins-bg (gutter)", fg: "var(--muted)", bg: "var(--ins-bg)", assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    // ── Marks on their own row background (3:1; colour-vision's ΔE floor
    // also binds these, but review 196 asked that they be in THIS table
    // too, so nothing drops out if colour-vision's own list ever changes) ──
    // RFC-097 stage 2 §2.2: High raises "control edges and the diff's
    // marks to at least 4.5:1" (handoff 098 §2.2) - these three marks
    // gain high_min: Some(HIGH_MARK_MIN), alongside --control-border below.
    Pair { label: "--del-mark on --del-bg", fg: "var(--del-mark)", bg: "var(--del-bg)", assumed_over: None, min: CONTROL_MIN, high_min: Some(HIGH_MARK_MIN) },
    Pair { label: "--ins-mark on --ins-bg", fg: "var(--ins-mark)", bg: "var(--ins-bg)", assumed_over: None, min: CONTROL_MIN, high_min: Some(HIGH_MARK_MIN) },
    // --rep-mark (review 196 §3): its own token now, not --warn, which had
    // no red-green-safe override and measured 2.76 here in Light.
    Pair { label: "--rep-mark on --rep-bg", fg: "var(--rep-mark)", bg: "var(--rep-bg)", assumed_over: None, min: CONTROL_MIN, high_min: Some(HIGH_MARK_MIN) },
    // ── Inline character-level spans (`.in-del`/`.in-ins`): only ever
    // rendered inside a Replace hunk's row (`pair_inline`, hunk.rs, gated
    // on `HunkKind::Replace`) - ASSUMED_OVER states that, rather than the
    // search-match override (`.diff-row.match .cell`) that could in
    // principle also apply to the same cell at the same time; review 196
    // accepted this as the primary case to table. ──
    Pair { label: "--text on .in-del",  fg: "var(--text)", bg: "color-mix(in srgb, var(--del-mark) 25%, transparent)", assumed_over: Some("var(--rep-bg)"), min: TEXT_MIN, high_min: None },
    Pair { label: "--text on .in-ins",  fg: "var(--text)", bg: "color-mix(in srgb, var(--ins-mark) 25%, transparent)", assumed_over: Some("var(--rep-bg)"), min: TEXT_MIN, high_min: None },
    // ── Tinted backgrounds whose real backdrop is fixed, so the CSS now
    // names it directly instead of `transparent` (see this module's own
    // doc comment) ──
    Pair { label: "--text on a search match (.diff-row.match)",    fg: "var(--text)",   bg: "color-mix(in srgb, var(--accent) 12%, var(--surface))",   assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--text on the warning banner (.diff-warning-banner)", fg: "var(--text)", bg: "color-mix(in srgb, var(--warn) 14%, var(--surface))", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--text on .profile-row.active",  fg: "var(--text)",   bg: "color-mix(in srgb, var(--accent) 12%, var(--surface))",   assumed_over: None, min: TEXT_MIN, high_min: None },
    // `.tree-row.selected`: 18% put warn/ins-mark/err below 4.5:1 in Light
    // (and err in Night) - 10% (review 196 §3) clears all three. Review
    // 197 §1 asked that --text/--muted be tabled too (a real pair on a
    // background that was about to change again, not just "checked"),
    // and that the row's own selection indicator - the leading-edge
    // --accent bar (03-components.css), not the tint - get its own entry.
    Pair { label: "--warn on a selected tree row",     fg: "var(--warn)",     bg: "color-mix(in srgb, var(--accent) 10%, var(--surface))", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--ins-mark on a selected tree row", fg: "var(--ins-mark)", bg: "color-mix(in srgb, var(--accent) 10%, var(--surface))", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--err on a selected tree row",      fg: "var(--err)",      bg: "color-mix(in srgb, var(--accent) 10%, var(--surface))", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--text on a selected tree row",     fg: "var(--text)",     bg: "color-mix(in srgb, var(--accent) 10%, var(--surface))", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--muted on a selected tree row",    fg: "var(--muted)",    bg: "color-mix(in srgb, var(--accent) 10%, var(--surface))", assumed_over: None, min: TEXT_MIN, high_min: Some(HIGH_TEXT_MIN) },
    // The selection indicator itself (`box-shadow: inset 3px 0 0 0
    // var(--accent)`) - not a `color:`/`background:` value, so check 4
    // never requires this entry, the same way --control-border's own
    // entries aren't check-4-required; hand-added because WCAG 1.4.11
    // still applies to it.
    Pair { label: "the selected-row indicator (box-shadow)", fg: "var(--accent)", bg: "color-mix(in srgb, var(--accent) 10%, var(--surface))", assumed_over: None, min: CONTROL_MIN, high_min: None },
    // RFC-097 stage 3 §3: Deep Compare's selected row carries the same
    // indicator, over the same background - same reasoning as the
    // Explorer's own entry just above, not re-derived.
    Pair { label: "the Deep Compare selected-row indicator (box-shadow)", fg: "var(--accent)", bg: "color-mix(in srgb, var(--accent) 10%, var(--surface))", assumed_over: None, min: CONTROL_MIN, high_min: None },
    // `.filter-toggle[aria-pressed="true"]`/`.path-bar.pane-focused`: 12%/
    // 10% put --accent's own text below 4.5:1 (4.41/4.32 dark/light;
    // 4.46 light) - both now 8% (review 196 §3).
    Pair { label: "--accent on *Filter* pressed",       fg: "var(--accent)", bg: "color-mix(in srgb, var(--accent) 8%, var(--surface-2))", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--accent on the focused pane's path bar", fg: "var(--accent)", bg: "color-mix(in srgb, var(--accent) 8%, var(--surface-2))", assumed_over: None, min: TEXT_MIN, high_min: None },
    // ── Solid token-on-token fills elsewhere in the app ──
    Pair { label: "--text on --danger-bg (tab close, hover)", fg: "var(--text)",    bg: "var(--danger-bg)", assumed_over: None, min: TEXT_MIN, high_min: None },
    Pair { label: "--surface on --accent (*Compare*, hover)",  fg: "var(--surface)", bg: "var(--accent)",    assumed_over: None, min: TEXT_MIN, high_min: None },
    // ── Control edges (F215; not a `color:`/`background:` pairing, so
    // check 4 never requires these, but they still need their own 3:1 -
    // 4.5:1 under High, RFC-097 stage 2 §2.2) ──
    Pair { label: "--control-border on --surface",   fg: "var(--control-border)", bg: "var(--surface)",   assumed_over: None, min: CONTROL_MIN, high_min: Some(HIGH_MARK_MIN) },
    Pair { label: "--control-border on --surface-2", fg: "var(--control-border)", bg: "var(--surface-2)", assumed_over: None, min: CONTROL_MIN, high_min: Some(HIGH_MARK_MIN) },
    Pair { label: "--control-border on --surface-3", fg: "var(--control-border)", bg: "var(--surface-3)", assumed_over: None, min: CONTROL_MIN, high_min: Some(HIGH_MARK_MIN) },
    // Review 211 §4: disabled-control text (`button:disabled`/`.search-nav:
    // disabled`/`.compare-btn:disabled`/`.path-btn:disabled`) is pinned to
    // `--muted-standard`, never raised by `.contrast-high` - the RFC's own
    // floor for a disabled control is 3:1, so that is `high_min` here too
    // (not 7:1, `--muted`'s own ordinary-text floor): the point of this
    // token is that it does *not* move between contrast levels, so both
    // columns check the same, already-comfortably-clearing value.
    Pair { label: "disabled text (--muted-standard) on --surface",   fg: "var(--muted-standard)", bg: "var(--surface)",   assumed_over: None, min: CONTROL_MIN, high_min: Some(CONTROL_MIN) },
    Pair { label: "disabled text (--muted-standard) on --surface-2", fg: "var(--muted-standard)", bg: "var(--surface-2)", assumed_over: None, min: CONTROL_MIN, high_min: Some(CONTROL_MIN) },
    Pair { label: "disabled text (--muted-standard) on --surface-3", fg: "var(--muted-standard)", bg: "var(--surface-3)", assumed_over: None, min: CONTROL_MIN, high_min: Some(CONTROL_MIN) },
];

const THEMES: [&str; 3] = ["dark", "light", "night"];
/// Every theme × colour-vision combination (review 196 §1): `.cv-rgsafe`
/// overrides six tokens in `.theme-light` only, several of which now appear
/// in `PAIRS`, so both passes of every theme are checked, not just one.
const PALETTES: [(&str, bool); 2] = [("standard", false), ("red-green-safe", true)];
/// RFC-097 stage 2 §2.2: the third, independent axis — every theme ×
/// colour-vision combination is now also checked at both contrast levels
/// (3 × 2 × 2 = 12), `.contrast-high`'s own tokens resolved from
/// `01-tokens.css` the same way `.cv-rgsafe`'s always have been.
const CONTRASTS: [(&str, bool); 2] = [("standard", false), ("high", true)];

/// Splits `s` on the first top-level comma - one not inside a `(...)` - so
/// a nested `color-mix(...)` in `c1`'s own position doesn't fool a naive
/// `.find(',')` into splitting inside it. Not needed by any pair in this
/// table today (no pair nests one mix inside another), but resolving a
/// `color-mix()` generically has to get this right regardless, the same
/// way `contrast.rs`'s own `matching_close` has to get nested `{}` right
/// for `@keyframes` even though no *pair* needs it.
fn split_top_level_comma(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => return Some((&s[..i], &s[i + 1..])),
            _ => {}
        }
    }
    None
}

/// Resolves one CSS colour *value* - `var(--x)`, a `#hex` literal,
/// `transparent` (only when `assumed_over` supplies the real backdrop), or
/// `color-mix(in srgb, C1 P%, C2)` (recursively, blending in sRGB, matching
/// the `in srgb` keyword's own browser behaviour) - to an sRGB triple,
/// resolving any `var()` through `theme`/`rgsafe`'s own cascade.
fn resolve_css_color(
    expr: &str,
    rules: &[crate::css_shared::Rule],
    theme: &str,
    rgsafe: bool,
    contrast_high: bool,
    assumed_over: Option<&str>,
) -> Rgb {
    let expr = expr.trim();
    if expr == "transparent" {
        let base = assumed_over
            .unwrap_or_else(|| panic!("`transparent` in {expr:?} has no stated assumed backdrop"));
        return resolve_css_color(base, rules, theme, rgsafe, contrast_high, assumed_over);
    }
    if let Some(inner) = expr.strip_prefix("var(").and_then(|s| s.strip_suffix(')')) {
        let value = resolve_token(rules, theme, rgsafe, contrast_high, inner.trim());
        return resolve_css_color(&value, rules, theme, rgsafe, contrast_high, assumed_over);
    }
    if expr.starts_with('#') {
        return hex_to_srgb(expr);
    }
    if let Some(inner) = expr
        .strip_prefix("color-mix(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let inner = inner.strip_prefix("in srgb,").unwrap_or(inner).trim();
        let (part1, part2) = split_top_level_comma(inner)
            .unwrap_or_else(|| panic!("malformed color-mix() in {expr:?}: no comma"));
        let (color1, pct) = part1
            .trim()
            .rsplit_once(' ')
            .unwrap_or_else(|| panic!("malformed color-mix() percentage in {expr:?}"));
        let pct: f64 = pct
            .trim()
            .strip_suffix('%')
            .unwrap_or_else(|| panic!("expected a percentage in {expr:?}, got {pct:?}"))
            .parse()
            .unwrap_or_else(|e| panic!("bad percentage in {expr:?}: {e}"));
        let c1 = resolve_css_color(color1, rules, theme, rgsafe, contrast_high, assumed_over);
        let c2 = resolve_css_color(part2, rules, theme, rgsafe, contrast_high, assumed_over);
        let p = pct / 100.0;
        return (
            p * c1.0 + (1.0 - p) * c2.0,
            p * c1.1 + (1.0 - p) * c2.1,
            p * c1.2 + (1.0 - p) * c2.2,
        );
    }
    panic!("don't know how to resolve colour expression {expr:?}");
}

fn check_pairs(root: &Path, problems: &mut Vec<String>) {
    let css_path = root.join("crates/forskscope-ui/assets/css/01-tokens.css");
    let css = fs::read_to_string(&css_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", css_path.display()));
    let rules = crate::css_shared::parse_rules(&css);

    for &theme in &THEMES {
        for &(palette, rgsafe) in &PALETTES {
            for &(contrast_label, contrast_high) in &CONTRASTS {
                for pair in PAIRS {
                    let min = if contrast_high {
                        pair.high_min.unwrap_or(pair.min)
                    } else {
                        pair.min
                    };
                    let fg = resolve_css_color(
                        pair.fg,
                        &rules,
                        theme,
                        rgsafe,
                        contrast_high,
                        pair.assumed_over,
                    );
                    let bg = resolve_css_color(
                        pair.bg,
                        &rules,
                        theme,
                        rgsafe,
                        contrast_high,
                        pair.assumed_over,
                    );
                    let ratio = contrast_ratio_rgb(fg, bg);
                    if ratio < min {
                        problems.push(format!(
                            "{theme}/{palette}/{contrast_label}: {} ({} on {}) has contrast \
                             {ratio:.2}, below the required {min:.1}",
                            pair.label, pair.fg, pair.bg
                        ));
                    }
                }
            }
        }
    }
}

// ── 2, 3 & 4. Opacity, font-size and colour/background coverage, scanned
//    across every assets/css/*.css ──────────────────────────────────────

/// A CSS file's `selector { decl: value; ... }` rules, keeping every
/// selector (not just `.`-prefixed ones — `button:disabled`, `input,
/// select` and the rest are exactly what these checks need to see) and
/// every declaration (not just `--`-prefixed custom properties, the filter
/// `css_shared::parse_rules` applies for the token file it reads). A
/// second, differently-filtered parser rather than parameterising the
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
    let css = crate::css_shared::strip_comments(css);
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

/// One literal `background`/`background-color` *value* exempt from check 4
/// (review 196 §2): no text is ever drawn against it, so no contrast pair
/// could describe it meaningfully. `none` and `transparent`, as a whole
/// top-level value (not nested inside a `color-mix()`), mean "paint
/// nothing" rather than naming an undetermined colour, so both are exempt
/// outright, with no entry needed here - the pairing concern in that case
/// belongs to whatever *is* painted further back, already covered by its
/// own entry.
struct AllowedValue {
    value: &'static str,
    reason: &'static str,
}

const BACKGROUND_VALUE_ALLOWLIST: &[AllowedValue] = &[
    AllowedValue {
        value: "#1e1f22",
        reason: "html/body/#main's pre-paint fallback (00-reset.css) - prevents a white flash \
                 before .app paints its own themed surface; nothing is ever drawn on top of it",
    },
    AllowedValue {
        value: "rgba(0,0,0,.45)",
        reason: "the modal scrim: a full-page dimming overlay behind the modal; only the \
                 modal itself (opaque, var(--surface)) is drawn on top of it, not text \
                 directly",
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

/// Check 4 (review 196 §2): every `color:` value and every
/// `background`/`background-color` value actually used anywhere in
/// `assets/css/*.css` must appear as a `fg`/`bg` (respectively) in
/// `PAIRS`, or be covered by `BACKGROUND_VALUE_ALLOWLIST` for a background
/// with no text drawn on it. A vocabulary check, not a per-rule one: it
/// confirms every token/expression this app ever paints with has *some*
/// accounted-for contrast pair, the same gap review 196 found in the first
/// version of this table (`--ins-mark` as the Explorer's own status text,
/// never checked against a surface at all).
/// `var(--name, fallback)` normalises to `var(--name)` for coverage
/// matching: the fallback is dead code as long as `01-tokens.css` always
/// defines `--name` (every token this table names does), so a pair
/// covering the bare `var(--name)` form covers this one too.
fn normalize_var_fallback(value: &str) -> std::borrow::Cow<'_, str> {
    if let Some(inner) = value.strip_prefix("var(").and_then(|s| s.strip_suffix(')'))
        && let Some((name, _fallback)) = inner.split_once(',')
    {
        return std::borrow::Cow::Owned(format!("var({})", name.trim()));
    }
    std::borrow::Cow::Borrowed(value)
}

fn check_coverage(root: &Path, problems: &mut Vec<String>) {
    let fg_covered: Vec<&str> = PAIRS.iter().map(|p| p.fg).collect();
    let bg_covered: Vec<&str> = PAIRS.iter().map(|p| p.bg).collect();

    for path in css_files(root) {
        let css = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        for rule in parse_css_rules(&css) {
            for (name, value) in &rule.decls {
                let normalized = normalize_var_fallback(value);
                if name == "color" {
                    if !fg_covered.contains(&normalized.as_ref()) {
                        problems.push(format!(
                            "{}: `{}` has `color: {value}`, not covered by any PAIRS entry's \
                             `fg` - add a pair for it, or explain why it needs none",
                            path.display(),
                            rule.selector
                        ));
                    }
                } else if name == "background" || name == "background-color" {
                    if normalized == "none" || normalized == "transparent" {
                        continue;
                    }
                    if BACKGROUND_VALUE_ALLOWLIST
                        .iter()
                        .any(|a| a.value == normalized.as_ref())
                    {
                        continue;
                    }
                    if !bg_covered.contains(&normalized.as_ref()) {
                        problems.push(format!(
                            "{}: `{}` has `{name}: {value}`, not covered by any PAIRS entry's \
                             `bg` and not in BACKGROUND_VALUE_ALLOWLIST - add a pair for it, \
                             allowlist it with a reason, or explain why it needs neither",
                            path.display(),
                            rule.selector
                        ));
                    }
                }
            }
        }
    }
}

// ── The gate itself ──────────────────────────────────────────────────────

pub fn run(root: &Path) {
    let mut problems = Vec::new();
    check_pairs(root, &mut problems);
    check_opacity(root, &mut problems);
    check_font_size(root, &mut problems);
    check_coverage(root, &mut problems);

    if !problems.is_empty() {
        eprintln!("contrast check failed:");
        for p in &problems {
            eprintln!("  - {p}");
        }
        process::exit(1);
    }

    println!(
        "contrast check passed: every declared pair meets its WCAG minimum (its own High floor, \
         under contrast-high) in every theme × colour-vision × contrast combination ({} pairs x \
         {} themes x {} palettes x {} contrast levels); no `opacity` below 1 and no `font-size` \
         below 12px outside their reasoned allowlists ({} opacity, {} font-size); every \
         `color:`/`background:` value in assets/css/*.css is covered by a pair or \
         BACKGROUND_VALUE_ALLOWLIST ({} entries):",
        PAIRS.len(),
        THEMES.len(),
        PALETTES.len(),
        CONTRASTS.len(),
        OPACITY_ALLOWLIST.len(),
        FONT_SIZE_ALLOWLIST.len(),
        BACKGROUND_VALUE_ALLOWLIST.len(),
    );
    for a in OPACITY_ALLOWLIST {
        println!("  opacity allowed: `{}` - {}", a.selector, a.reason);
    }
    for a in FONT_SIZE_ALLOWLIST {
        println!("  font-size allowed: `{}` - {}", a.selector, a.reason);
    }
    for a in BACKGROUND_VALUE_ALLOWLIST {
        println!("  background value allowed: `{}` - {}", a.value, a.reason);
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

    /// A self-contained token fixture naming every custom property any
    /// `PAIRS` entry's `fg`/`bg` can resolve to, for one theme - trivially
    /// compliant everywhere except the one pair the test deliberately
    /// breaks, the same shape the fixture had before review 196 grew the
    /// table.
    fn compliant_theme(selector: &str) -> String {
        format!(
            "{selector} {{ --surface: #000000; --surface-2: #000000; --surface-3: #101010; \
             --text: #ffffff; --muted: #ffffff; --muted-standard: #ffffff; --accent: #111111; \
             --warn: #ffffff; --ok: #ffffff; --err: #ffffff; --control-border: #ffffff; \
             --del-bg: #000000; --ins-bg: #000000; --rep-bg: #000000; --danger-bg: #000000; \
             --del-mark: #ffffff; --ins-mark: #ffffff; --rep-mark: #ffffff; }}\n"
        )
    }

    #[test]
    fn check_pairs_fails_a_pair_below_its_minimum() {
        let mut problems = Vec::new();
        // `--accent` on `--surface-3` is deliberately near-identical in
        // `.theme-dark` (a real, measured failure shape) - the assertion
        // below only checks that this one, expected failure is among the
        // problems found, the same as every other pair this fixture also
        // happens to fail (--accent is now dark-on-dark against every
        // surface, not just `--surface-3`) is not a concern for this test.
        let mut css = compliant_theme(".theme-dark")
            .replace("--surface-3: #101010;", "--surface-3: #121212;");
        css = css.replace("--accent: #111111;", "--accent: #131313;");
        css.push_str(&compliant_theme(".theme-light"));
        css.push_str(&compliant_theme(".theme-night"));
        let root = fake_workspace(line!(), "01-tokens.css", &css);
        check_pairs(&root, &mut problems);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("dark/standard") && p.contains("--accent on --surface-3")),
            "expected a dark/--accent-on-surface-3 failure among {problems:?}"
        );
    }

    #[test]
    fn resolve_css_color_blends_a_color_mix_in_srgb() {
        let rules = Vec::new();
        let rgb = resolve_css_color(
            "color-mix(in srgb, #ffffff 50%, #000000)",
            &rules,
            "dark",
            false,
            false,
            None,
        );
        assert!((rgb.0 - 0.5).abs() < 1e-9);
        assert!((rgb.1 - 0.5).abs() < 1e-9);
        assert!((rgb.2 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn resolve_css_color_resolves_transparent_against_its_stated_backdrop() {
        let rules = Vec::new();
        let rgb = resolve_css_color("transparent", &rules, "dark", false, false, Some("#336699"));
        assert_eq!(rgb, hex_to_srgb("#336699"));
    }

    #[test]
    #[should_panic(expected = "no stated assumed backdrop")]
    fn resolve_css_color_panics_on_transparent_with_no_assumed_backdrop() {
        let rules = Vec::new();
        resolve_css_color("transparent", &rules, "dark", false, false, None);
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

    #[test]
    fn check_coverage_flags_an_uncovered_color_value() {
        let mut problems = Vec::new();
        let root = fake_workspace(
            line!(),
            "99-fixture.css",
            ".mystery { color: var(--does-not-exist-in-any-pair); }\n",
        );
        check_coverage(&root, &mut problems);
        assert!(
            problems.iter().any(|p| p.contains(".mystery")),
            "expected a problem naming .mystery among {problems:?}"
        );
    }

    #[test]
    fn check_coverage_flags_an_uncovered_background_value() {
        let mut problems = Vec::new();
        let root = fake_workspace(
            line!(),
            "99-fixture.css",
            ".mystery { background: color-mix(in srgb, var(--accent) 99%, var(--surface)); }\n",
        );
        check_coverage(&root, &mut problems);
        assert!(
            problems.iter().any(|p| p.contains(".mystery")),
            "expected a problem naming .mystery among {problems:?}"
        );
    }

    #[test]
    fn check_coverage_accepts_a_covered_value() {
        let mut problems = Vec::new();
        let root = fake_workspace(line!(), "99-fixture.css", ".fine { color: var(--text); }\n");
        check_coverage(&root, &mut problems);
        assert!(
            problems.is_empty(),
            "expected no problems, got {problems:?}"
        );
    }

    #[test]
    fn check_coverage_normalizes_a_var_fallback_to_its_bare_form() {
        let mut problems = Vec::new();
        // `var(--warn)` is covered; the fallback form of the same token
        // must be recognised as the same value, not a new, uncovered one
        // (`.tree-status.status-different`'s real shape, 10-view-explorer.css).
        let root = fake_workspace(
            line!(),
            "99-fixture.css",
            ".fine { color: var(--warn, #c90); }\n",
        );
        check_coverage(&root, &mut problems);
        assert!(
            problems.is_empty(),
            "expected the fallback form to be covered, got {problems:?}"
        );
    }

    #[test]
    fn check_coverage_ignores_none_transparent_and_the_value_allowlist() {
        let mut problems = Vec::new();
        let root = fake_workspace(
            line!(),
            "99-fixture.css",
            ".a { background: none; }\n\
             .b { background: transparent; }\n\
             .c { background: rgba(0,0,0,.45); }\n",
        );
        check_coverage(&root, &mut problems);
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
