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
) -> Element {
    rsx! {
        select {
            title,
            id,
            aria_label,
            value: "{value}",
            onchange: move |e| onchange.call(e.value()),
            for opt in options {
                option {
                    value: "{opt.value}",
                    selected: opt.value == value,
                    "{opt.label}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

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
