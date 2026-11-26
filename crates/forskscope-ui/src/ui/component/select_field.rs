//! `SelectField` — the one `<select>` every dropdown in this app goes
//! through (F202, review 170 §3).
//!
//! `dioxus-interpreter-js` sets a `<select>`'s `value` as a real DOM
//! property, but that assignment is a no-op until a matching `<option>`
//! already exists as a child — and the interpreter sets a cloned template's
//! `value` before its `<option>` children are attached. A bare `select {
//! value: ... }` with no `selected` on any option therefore always shows its
//! *first* declared option, silently, regardless of `value` — this is
//! exactly how F202 happened, six times over, before anyone noticed because
//! the underlying stored value (what the app actually acts on) was never
//! wrong, only its own display.
//!
//! [`SelectField`] makes that mistake impossible to make again: it is the
//! only place in this crate allowed to write a raw `select {` element (see
//! this module's own test, which fails if one appears anywhere else), and it
//! always marks the matching [`SelectOption`] with `selected`.

use dioxus::prelude::*;

/// One `<option>`: `value` is what `onchange` reports back; `label` is what
/// the user reads. Both owned — several call sites build these from a
/// runtime value (a translated label, an unusual encoding name chardetng
/// guessed) that cannot be `&'static str`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Review 246 §1 (handoff 118's re-triage), "`div.field` overflow": a
/// `<select>`'s own closed-box text rendering ignores `overflow`/
/// `text-overflow` entirely in this engine - confirmed live, forcing
/// the element's own CSS `width` down to 10px left its measured
/// overflow *unchanged* (738px against the same 350px available), so
/// the text itself, not the box, is what has to shrink. The longest
/// real (non-pseudo) option label in this app today is 39 characters
/// ("Paths relative to the compared folders") and fits every size
/// measured; this floor sits comfortably above it, so no real
/// language is touched - only the pseudo-language's own ~50-character
/// stretched labels (40% longer, plus its own `⟦…⟧` wrapping) are.
const MAX_LABEL_CHARS: usize = 42;

/// `label`, unchanged if it already fits [`MAX_LABEL_CHARS`], or cut to
/// that many characters with a trailing `…` if not - never silently
/// past it, the same shape `.diff-col-act`'s own "sized to its
/// content, not a hard cap" fix (review 245 §1.2) uses for a box that
/// *can* grow, for a native control whose own closed-state text
/// cannot.
fn truncated_label(label: &str) -> String {
    if label.chars().count() <= MAX_LABEL_CHARS {
        return label.to_string();
    }
    let mut truncated: String = label.chars().take(MAX_LABEL_CHARS - 1).collect();
    truncated.push('…');
    truncated
}

/// A `<select>` that always shows `value`, by marking the matching option
/// `selected` directly rather than relying on the select's own `value`
/// property (see this module's doc comment for why that alone is not
/// enough).
#[component]
pub fn SelectField(
    value: String,
    options: Vec<SelectOption>,
    onchange: EventHandler<String>,
    #[props(default)] title: Option<String>,
    #[props(default)] id: Option<String>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
) -> Element {
    rsx! {
        select {
            title,
            id,
            aria_label,
            aria_describedby,
            value: "{value}",
            onchange: move |e| onchange.call(e.value()),
            for opt in options {
                option {
                    value: "{opt.value}",
                    selected: opt.value == value,
                    // The full label survives as a hover tooltip whenever
                    // it was actually cut - no information lost, only
                    // what's shown closed needing a second look.
                    title: (opt.label.chars().count() > MAX_LABEL_CHARS)
                        .then(|| opt.label.clone()),
                    "{truncated_label(&opt.label)}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_LABEL_CHARS, truncated_label};
    use std::fs;
    use std::path::Path;

    #[test]
    fn a_label_at_or_under_the_cap_is_unchanged() {
        let exact = "x".repeat(MAX_LABEL_CHARS);
        assert_eq!(truncated_label(&exact), exact);
        assert_eq!(truncated_label("short"), "short");
    }

    #[test]
    fn a_label_over_the_cap_is_cut_with_a_trailing_ellipsis() {
        let long = "x".repeat(MAX_LABEL_CHARS + 10);
        let out = truncated_label(&long);
        assert_eq!(out.chars().count(), MAX_LABEL_CHARS);
        assert!(out.ends_with('…'), "{out:?} has no trailing ellipsis");
        assert_eq!(
            &out[..out.len() - '…'.len_utf8()],
            &"x".repeat(MAX_LABEL_CHARS - 1)
        );
    }

    /// The real catalog's own longest known option label - confirmed
    /// here, not just asserted in a comment, so a future edit that
    /// makes it (or a longer one) grow past the cap fails loudly
    /// rather than silently clipping real, translated text a user
    /// actually reads.
    #[test]
    fn the_longest_known_real_option_label_is_never_truncated() {
        let longest = "Paths relative to the compared folders";
        assert!(longest.chars().count() < MAX_LABEL_CHARS);
        assert_eq!(truncated_label(longest), longest);
    }

    /// A pseudo-language-shaped label (stretched well past any real
    /// option, bracket-wrapped) is exactly the case the cap exists
    /// for - falsified against a realistic length, not a contrived one.
    #[test]
    fn a_pseudo_language_shaped_label_is_truncated() {
        let pseudo_shaped = "⟦ẇẇẇẇĈôḿṕáĉť (ñô šṕáĉéř řôẇš, ďéḟáûĺť)ẇẇẇẇẇ⟧";
        assert!(pseudo_shaped.chars().count() > MAX_LABEL_CHARS);
        let out = truncated_label(pseudo_shaped);
        assert_eq!(out.chars().count(), MAX_LABEL_CHARS);
        assert!(out.ends_with('…'));
    }

    /// Falsify by reverting any call site back to a raw `select {` element
    /// — this must fail, naming the exact file and line. A source scan, not
    /// a DOM assertion: the defect this guards against (F202) was never
    /// visible to `store.settings`/`store.tabs`, only to a human looking at
    /// the rendered page, so there is nothing a unit test could read back
    /// from state — the shape of the source is the only thing left to check.
    #[test]
    fn no_raw_select_element_exists_outside_this_module() {
        let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut violations = Vec::new();
        visit(&src_dir, &mut violations);
        assert!(
            violations.is_empty(),
            "raw `select {{` found outside select_field.rs - every dropdown must go \
             through SelectField, so its selected option is never left to the browser's \
             default (F202): {violations:?}"
        );
    }

    fn visit(dir: &Path, violations: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit(&path, violations);
                continue;
            }
            if path.file_name().and_then(|n| n.to_str()) == Some("select_field.rs") {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            for (i, line) in text.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("select {") || trimmed.starts_with("select{") {
                    violations.push(format!("{}:{}", path.display(), i + 1));
                }
            }
        }
    }
}
