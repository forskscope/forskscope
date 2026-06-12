//! CSS class contract coverage test (RFC-024, RFC-034).
//!
//! Verifies that every CSS class token produced by `forskscope-core` is
//! defined somewhere in the main stylesheet. This prevents silent breakage
//! where a core constant is renamed or added but the CSS is not updated.
//!
//! This test runs without GTK — it reads the CSS file as a static string.

/// The main stylesheet, included at compile time.
///
/// The path is relative to the workspace root; if the file moves this test
/// will fail at compile time rather than silently at runtime.
const MAIN_CSS: &str = include_str!(
    "../../../crates/forskscope-ui/assets/main.css"
);

fn css_contains_class(css: &str, class: &str) -> bool {
    // Strip leading dot to form the selector token, then look for it.
    // This is a simple substring search — sufficient for a flat CSS file.
    let selector = format!(".{class}");
    css.contains(&selector)
}

#[test]
fn line_decoration_css_classes_defined_in_main_css() {
    use forskscope_core::diff_decoration::LineDecorationKind;

    let classes = [
        LineDecorationKind::Unchanged.css_class(),
        LineDecorationKind::Added.css_class(),
        LineDecorationKind::Deleted.css_class(),
        LineDecorationKind::Modified.css_class(),
        LineDecorationKind::EmptyCounterpart.css_class(),
        LineDecorationKind::Conflict.css_class(),
        LineDecorationKind::MergeApplied.css_class(),
    ];

    for class in &classes {
        assert!(
            css_contains_class(MAIN_CSS, class),
            "main.css must define CSS class .{class} (from LineDecorationKind::css_class)"
        );
    }
}

#[test]
fn inline_decoration_css_classes_defined_in_main_css() {
    use forskscope_core::diff_decoration::InlineDecorationKind;

    let classes = [
        InlineDecorationKind::InsertedChars.css_class(),
        InlineDecorationKind::DeletedChars.css_class(),
        InlineDecorationKind::ReplacedChars.css_class(),
        InlineDecorationKind::WhitespaceOnly.css_class(),
    ];

    for class in &classes {
        assert!(
            css_contains_class(MAIN_CSS, class),
            "main.css must define CSS class .{class} (from InlineDecorationKind::css_class)"
        );
    }
}

#[test]
fn conflict_navigator_css_classes_defined_in_main_css() {
    use forskscope_core::conflict_nav::{ConflictFilter, ConflictNavigator};
    use forskscope_core::merge::ThreeWayMergeSession;

    // Build a session with one conflict and inspect the resulting CSS classes.
    let sess = ThreeWayMergeSession::from_texts(
        "base\n",
        "left\n",
        "right\n",
    );
    let nav = ConflictNavigator::build(&sess, None, ConflictFilter::All);

    // The navigator entries carry the css_class for the current status.
    // For a fresh session, all conflicts are Unresolved.
    for entry in &nav.entries {
        let class = entry.css_class();
        assert!(
            css_contains_class(MAIN_CSS, class),
            "main.css must define CSS class .{class} (from ConflictNavigatorEntry::css_class)"
        );
    }

    // Also verify the other statuses that can appear after resolution.
    // We test them by checking the literal class names from the source.
    let static_classes = [
        "fsk-conflict-unresolved",
        "fsk-conflict-left",
        "fsk-conflict-right",
        "fsk-conflict-both",
        "fsk-conflict-manual",
        "fsk-conflict-ignored",
    ];
    for class in &static_classes {
        assert!(
            css_contains_class(MAIN_CSS, class),
            "main.css must define CSS class .{class}"
        );
    }
}

#[test]
fn row_state_gutter_symbols_are_distinct() {
    // Smoke test: RowState::gutter_symbol must be unique across variants.
    use forskscope_core::line_map::RowState;
    let symbols: std::collections::HashSet<char> = [
        RowState::Equal, RowState::Inserted, RowState::Deleted,
        RowState::Modified, RowState::Conflict, RowState::Collapsed,
    ].iter().map(|s| s.gutter_symbol()).collect();
    assert_eq!(symbols.len(), 6, "all RowState gutter symbols must be distinct");
}

#[test]
fn all_css_vars_used_are_defined_in_main_css() {
    // Collect all --name tokens from var(--name) usages.
    // Use a simple scan: look for "var(--" then grab everything up to ")"
    // but only keep the var name (no spaces, letters, digits, hyphens only).
    let mut used = std::collections::HashSet::new();
    let mut rest = MAIN_CSS;
    while let Some(pos) = rest.find("var(--") {
        rest = &rest[pos + 6..]; // skip "var(--"
        // The var name ends at the first ')' or whitespace
        let end = rest.find(|c: char| c == ')' || c.is_whitespace())
            .unwrap_or(rest.len());
        let var_name = &rest[..end];
        // Only accept names matching CSS custom property syntax: [a-z0-9-]+
        if !var_name.is_empty() && var_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            used.insert(var_name);
        }
    }

    for var in &used {
        let definition = format!("--{var}:");
        assert!(
            MAIN_CSS.contains(&definition),
            "CSS variable '--{var}' is used in main.css but never defined"
        );
    }
}
