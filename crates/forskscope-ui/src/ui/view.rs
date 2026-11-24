//! Main user-facing workspace views.

use dioxus::prelude::*;

use crate::i18n::{TemplateSegment, template_segments};
use crate::state::Lang;

/// RFC-101 stage B, review 232 §3.4: renders a `tf(lang, id, args,
/// count)`-shaped message as RSX, with each placeholder named in
/// `isolate` wrapped in its own `<bdi>` — a placeholder carrying user
/// data (a name, a path, error text) is isolated in the interface,
/// **never** by inserting isolate characters, which RFC-100 flags as
/// suspicious. Every other placeholder substitutes as plain text, same
/// as `tf()`.
///
/// Only for rendered body text: `tf()` itself is still the right call
/// for an attribute (`aria-label`, `title`, a window title), where no
/// markup is possible at all (review 232 §3.4's own "where markup isn't
/// possible, it goes in as it is" rule, with F235's control-character
/// escaping).
#[component]
pub(crate) fn TranslatedText(
    lang: Lang,
    id: &'static str,
    #[props(default)] args: Vec<(&'static str, String)>,
    #[props(default)] isolate: Vec<&'static str>,
    #[props(default)] count: Option<i64>,
) -> Element {
    rsx! {
        for segment in template_segments(lang, id, count) {
            match segment {
                TemplateSegment::Text(text) => rsx! { "{text}" },
                TemplateSegment::Placeholder(name) => {
                    let value = if name == "count" {
                        count.map(|n| n.to_string()).unwrap_or_default()
                    } else {
                        args.iter()
                            .find(|(n, _)| *n == name)
                            .map(|(_, v)| v.clone())
                            .unwrap_or_default()
                    };
                    if isolate.contains(&name.as_str()) {
                        rsx! { bdi { "{value}" } }
                    } else {
                        rsx! { "{value}" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_nodes(mutations: &dioxus_core::Mutations) -> Vec<String> {
        mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect()
    }

    /// Review 232 §3.4: `TranslatedText` is what every call site that
    /// isolates a user-data placeholder (a name, a path, error text)
    /// renders through, in place of a flat `tf()` string. This can't
    /// confirm the `<bdi>` wrapping itself reaches a renderer (this
    /// workspace has no `dioxus-ssr`, and `VirtualDom::rebuild_to_vec`'s
    /// own `Mutations` writer discards each `Template` it loads, tag
    /// names included - `i18n::mod::tests`' own
    /// `a_users_file_name_is_its_own_segment_not_folded_into_surrounding_text`
    /// is what actually proves the Hebrew name reaches its own
    /// `Placeholder` segment, the thing this component's `isolate` list
    /// wraps) - this is the render-level half: a Hebrew file name
    /// reaches a real text node, in both languages, with nothing lost
    /// or panicking, exactly where `could-not-open-error` is used.
    #[test]
    fn a_hebrew_file_name_renders_intact_in_both_languages() {
        let hebrew_name = "קובץ.txt";

        #[component]
        fn Root(lang: crate::state::Lang, name: String) -> Element {
            rsx! {
                p {
                    TranslatedText {
                        lang,
                        id: "could-not-open-error",
                        args: vec![("name", name), ("error", "x".to_string())],
                        isolate: vec!["name"],
                    }
                }
            }
        }

        for lang in [crate::state::Lang::En, crate::state::Lang::Ja] {
            let mut vdom = VirtualDom::new_with_props(
                Root,
                RootProps {
                    lang,
                    name: hebrew_name.to_string(),
                },
            );
            let mutations = vdom.rebuild_to_vec();
            let texts = text_nodes(&mutations);
            assert!(
                texts.iter().any(|t| t.contains(hebrew_name)),
                "{lang:?}: expected the Hebrew file name among the rendered text nodes: {texts:?}"
            );
        }
    }
}

pub mod deep_compare;
pub mod diff;
pub mod diff_actions;
pub mod digest_epoch;
pub mod dir_pane;
pub mod explorer;
pub mod export_dir_report;
pub mod hunk;
/// F235: every other place a file/dir *name* is shown reuses RFC-100's
/// own `BidiText` (previously private to `hunk.rs`'s diff rows) instead
/// of a second implementation.
pub(crate) use hunk::BidiText;
pub mod search;
pub mod settings;
