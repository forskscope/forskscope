//! Comparison tab model: `CompareTab`, `TabState`, and tab-level mutations.

use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_core::diff::DiffDocument;
use forskscope_core::document::LoadedDocument;
use forskscope_core::{DiffOptions, MergeSession, compute_diff};

use crate::state::settings::Lang;

/// Lifecycle state of a comparison tab (RFC-065).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TabState {
    /// Background load is in progress. Tab shows a spinner.
    Loading,
    /// Load and diff complete. Tab shows the diff view.
    Ready,
    /// Load or diff failed. Tab shows a recoverable error message.
    Error(String),
}

#[derive(Clone)]
pub struct CompareTab {
    pub title:      String,
    pub left_path:  Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    /// Lifecycle state — `Loading` until background task completes (RFC-065).
    pub state:      TabState,
    pub left_doc:   LoadedDocument,
    pub right_doc:  LoadedDocument,
    pub diff:       DiffDocument,
    pub merge:      MergeSession,
    pub diff_options: DiffOptions,
    pub can_save:   bool,
    pub char_mode:  bool,
    pub word_wrap:  bool,
    pub focused_change: usize,
}

impl CompareTab {
    pub fn right_label(&self) -> String {
        self.right_doc.text.as_ref()
            .map(|t| t.encoding.label.clone())
            .unwrap_or_else(|| "—".into())
    }
}

pub fn recompute_diff(tab: &mut CompareTab) {
    let diff = compute_diff(
        tab.left_doc.diff_text(), tab.right_doc.diff_text(), tab.diff_options,
    );
    tab.merge          = MergeSession::from_diff(&diff);
    tab.diff           = diff;
    tab.focused_change = 0;
    tab.char_mode      = false;
}

pub fn swap_sides(store: &mut crate::state::Store, index: usize) {
    let mut tabs = store.tabs.write();
    let Some(tab) = tabs.get_mut(index) else { return };
    std::mem::swap(&mut tab.left_doc,  &mut tab.right_doc);
    std::mem::swap(&mut tab.left_path, &mut tab.right_path);
    tab.can_save = tab.left_doc.kind.is_mergeable_text()
        && tab.right_doc.kind.is_mergeable_text();
    recompute_diff(tab);
}

/// Derive a human-readable tab title from the two file paths.
pub(crate) fn tab_title(l: &std::path::Path, r: &std::path::Path, lang: Lang) -> String {
    use crate::i18n::t;
    let ln = l.file_name().map(|n| n.to_string_lossy().into_owned());
    let rn = r.file_name().map(|n| n.to_string_lossy().into_owned());
    match (ln, rn) {
        (Some(a), Some(b)) if a == b => a,
        (Some(a), Some(b))           => format!("{a} ↔ {b}"),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => t(lang, "comparison"),
    }
}



#[cfg(test)]
mod tests;
