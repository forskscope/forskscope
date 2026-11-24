//! Localization (RFC-009 §locale; RFC-101 stage B, handoff 114).
//!
//! Translations live in `crates/forskscope-ui/i18n/<lang>.catalog`, one
//! file per language, parsed **at build time** by `build.rs` (via
//! [`catalog`], shared with this module's own tests) into the
//! `CATALOG_EN`/`CATALOG_JA` tables [`include!`]d below — never a Rust
//! `match`, so a translator never edits Rust to contribute (RFC-101's own
//! motivation). Message ids are stable slugs, not English text (the
//! previous design, replaced here) — the one deliberately named exception
//! is a short, fixed set of ids whose text is looked up **dynamically**
//! at a handful of call sites (`BidiText`, the theme/contrast `<select>`
//! labels) rather than through a literal `t(lang, "id")` the audit's
//! static scan can see; those ids are still plain kebab-case slugs, just
//! read from a `&'static str` field in another crate instead of written
//! as a literal here — covered by their own tests below, against the
//! real tables, not a copy of them (reviews 174/212's own rule).
//!
//! Unknown language or unknown id both degrade gracefully rather than
//! panicking in a release build: English is tried as a fallback language,
//! and the id itself is the final fallback — the same "missing
//! translations degrade gracefully" policy this module has always had,
//! now also covering a message present in one language's catalog but
//! (briefly, before `cargo xtask i18n` catches it) not the other's.

// `build.rs` reaches the parser by `#[path]`-including `catalog.rs`
// directly into its own, separate compilation (it cannot depend on this
// not-yet-built library) — so nothing at runtime needs this module *to
// parse* a catalog, only to read the table `build.rs` already generated
// from one ([`StaticForms`], below). `cfg(test)` so `cargo test` still
// exercises the parser's own unit tests, without `parse`/`MessageForms`
// being unreachable (and therefore dead code) in a normal build.
#[cfg(test)]
pub(crate) mod catalog;
mod plural;

use crate::state::Lang;

/// The `'static`-data shape of a catalog message, as `build.rs` writes
/// it into `CATALOG_EN`/`CATALOG_JA` below — `&'static str` throughout,
/// so the whole table is itself a `static`, with no parsing or
/// allocation at startup.
#[derive(Debug, Clone, Copy)]
pub(crate) enum StaticForms {
    Single(&'static str),
    Plural(&'static [(&'static str, &'static str)]),
}

impl StaticForms {
    /// One representative text, for the two dynamic-lookup tests below
    /// that only need to compare "is this language's text different from
    /// English's", not render a real message — the `other` form for a
    /// plural message, since every supported language defines one.
    #[cfg(test)]
    fn text_for_test(&self) -> &'static str {
        match self {
            Self::Single(text) => text,
            Self::Plural(forms) => forms
                .iter()
                .find(|(f, _)| *f == "other")
                .map(|(_, text)| *text)
                .unwrap_or(""),
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/i18n_tables.rs"));

fn lookup(lang: Lang, id: &str) -> Option<&'static StaticForms> {
    let table: &[(&str, StaticForms)] = match lang {
        Lang::En => CATALOG_EN,
        Lang::Ja => CATALOG_JA,
    };
    table
        .binary_search_by_key(&id, |(i, _)| *i)
        .ok()
        .map(|idx| &table[idx].1)
}

/// Translate a plain message with no placeholders and no count.
pub fn t(lang: Lang, id: &str) -> String {
    tf(lang, id, &[], None)
}

/// Translate a message with named placeholders (`{ $name }`) and, for a
/// message with plural forms, the count that selects one (also available
/// to the template itself as `{ $count }`, so a caller never passes it
/// twice).
///
/// Panics if `id` names a plural message and `count` is `None` — there is
/// no sensible form to pick, and showing the raw template to a user is
/// worse than failing loudly at the one call site that forgot it. Passing
/// a `count` for a message that *isn't* plural is not a caller mistake:
/// plenty of messages show a `{ $count }` with no plural variation at all
/// (the same wording for one or many, by design, not yet upgraded to a
/// plural form, or deliberately not needing one) — `count` only ever
/// drives the plural *selection*, and is always separately available as
/// an ordinary `{ $count }` substitution either way.
pub fn tf(lang: Lang, id: &str, args: &[(&str, &str)], count: Option<i64>) -> String {
    let forms = lookup(lang, id).or_else(|| lookup(Lang::En, id));
    let Some(forms) = forms else {
        return substitute(id, args, count);
    };
    let template = match forms {
        StaticForms::Single(text) => *text,
        StaticForms::Plural(forms) => {
            let Some(n) = count else {
                panic!("tf({id:?}): this message is plural but no count was given");
            };
            let category = plural::category(lang, n);
            forms
                .iter()
                .find(|(f, _)| *f == category)
                .or_else(|| forms.iter().find(|(f, _)| *f == "other"))
                .map(|(_, t)| *t)
                .unwrap_or(id)
        }
    };
    substitute(template, args, count)
}

fn substitute(template: &str, args: &[(&str, &str)], count: Option<i64>) -> String {
    let mut out = template.to_string();
    if let Some(n) = count {
        out = out.replace("{ $count }", &n.to_string());
    }
    for (name, value) in args {
        out = out.replace(&format!("{{ ${name} }}"), value);
    }
    out
}

/// One piece of a template, in order — either literal text to show as
/// written, or the name of a `{ $name }` placeholder to substitute.
/// [`substitute`] above is the flat-string case of walking these same
/// segments; [`crate::ui::view::TranslatedText`] is the one that keeps
/// them apart, so a placeholder carrying user data can render inside its
/// own isolating element (review 232 §3.4) instead of being spliced into
/// one opaque string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TemplateSegment {
    Text(String),
    Placeholder(String),
}

/// The resolved template for `id` in `lang` (plural form chosen by
/// `count`, same rule [`tf`] itself uses), split into [`TemplateSegment`]s
/// rather than substituted into one string.
pub(crate) fn template_segments(lang: Lang, id: &str, count: Option<i64>) -> Vec<TemplateSegment> {
    let forms = lookup(lang, id).or_else(|| lookup(Lang::En, id));
    let template = match forms {
        None => return vec![TemplateSegment::Text(id.to_string())],
        Some(StaticForms::Single(text)) => *text,
        Some(StaticForms::Plural(forms)) => {
            let Some(n) = count else {
                panic!("template_segments({id:?}): this message is plural but no count was given");
            };
            let category = plural::category(lang, n);
            forms
                .iter()
                .find(|(f, _)| *f == category)
                .or_else(|| forms.iter().find(|(f, _)| *f == "other"))
                .map(|(_, t)| *t)
                .unwrap_or(id)
        }
    };
    split_template(template)
}

fn split_template(template: &str) -> Vec<TemplateSegment> {
    let mut out = Vec::new();
    let mut rest = template;
    loop {
        let Some(start) = rest.find("{ $") else {
            if !rest.is_empty() {
                out.push(TemplateSegment::Text(rest.to_string()));
            }
            break;
        };
        if start > 0 {
            out.push(TemplateSegment::Text(rest[..start].to_string()));
        }
        let name_start = start + 3;
        let Some(name_len) = rest[name_start..].find(" }") else {
            // No closing " }" - the rest is literal text, braces and all
            // (matches `catalog::placeholders`' own "near-miss" rule:
            // this isn't a real placeholder).
            out.push(TemplateSegment::Text(rest[start..].to_string()));
            break;
        };
        out.push(TemplateSegment::Placeholder(
            rest[name_start..name_start + name_len].to_string(),
        ));
        rest = &rest[name_start + name_len + 2..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::PathBuf;

    use forskscope_compare::diff::{DiffOptions, compute_diff};
    use forskscope_compare::dir::{RecEntry, RecStatus};
    use forskscope_report::report::{
        ChangeHunk, DirComparisonReport, FileChecksum, FileComparisonReport, ReportOptions,
        change_hunks_from_diff, render_dir_report, render_file_report,
    };

    // ── RFC-101 stage B carries forward review 174 §5's own rule, ported
    // from the old i18n.rs (deleted by this handoff) rather than lost
    // with it: "Japanese through a stand-in translator is not evidence.
    // Render in a UI-crate test with the real t(Lang::Ja, …). Assert
    // that no string the renderer requests comes back untranslated." ──

    /// A renderer call, boxed so its concrete closure type (and the
    /// translator type it takes) doesn't need spelling out at every call
    /// site.
    type RenderCall<'a> = Box<dyn FnOnce(&dyn Fn(&str) -> String) -> String + 'a>;

    /// Records every id `render_file_report`/`render_dir_report`
    /// actually requests through `tr`, driven by the real `t(Lang::Ja,
    /// …)` — not a copy of the catalog.
    fn requested_ja_ids(render: RenderCall<'_>) -> Vec<String> {
        let requested: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let tr = |s: &str| -> String {
            requested.borrow_mut().push(s.to_string());
            t(Lang::Ja, s)
        };
        let _ = render(&tr);
        requested.into_inner()
    }

    /// Checking `lookup(Lang::Ja, id).is_some()` rather than comparing
    /// translated text to the id is deliberate: a deliberately-identical
    /// entry (`report-sha-256 = "SHA-256"`, a universal technical term)
    /// must count as translated, not as a gap that merely produces the
    /// same text as a coincidence.
    fn assert_every_requested_id_has_a_real_ja_entry(ids: &[String]) {
        let mut missing: Vec<&str> = ids
            .iter()
            .map(String::as_str)
            .filter(|id| lookup(Lang::Ja, id).is_none())
            .collect();
        missing.sort_unstable();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "these ids the renderer requested have no catalog entry for Japanese at all - \
             they fall through to the raw id/English text, not a real translation: {missing:?}"
        );
    }

    fn comprehensive_file_report() -> FileComparisonReport {
        let diff = compute_diff(
            "one\ntwo\nthree\n",
            "one\nTWO\nthree\nfour\n",
            DiffOptions {
                ignore_case: true,
                ..DiffOptions::default()
            },
        );
        let mut report = FileComparisonReport::from_diff(
            &diff,
            Some(&PathBuf::from("/left/report.txt")),
            Some(&PathBuf::from("/right/report.txt")),
            ReportOptions::default(),
        )
        .with_checksums(
            Some(FileChecksum::of_bytes(b"left")),
            Some(FileChecksum::of_bytes(b"right")),
        );
        // A real, pre-existing warning string (not caller-pushed free
        // text like an encoding label, which is inherently untranslatable
        // dynamic content, not something this test is about) - rendered
        // as-is, never through `tr`, so it doesn't appear among the
        // requested ids either way.
        report.push_note("Large file — inline diff disabled.");
        report
    }

    fn comprehensive_dir_report() -> DirComparisonReport {
        let entries = vec![
            RecEntry {
                rel_path: PathBuf::from("equal.txt"),
                status: RecStatus::Equal,
                left_size: Some(1),
                right_size: Some(1),
            },
            RecEntry {
                rel_path: PathBuf::from("changed.txt"),
                status: RecStatus::Changed,
                left_size: Some(2),
                right_size: Some(3),
            },
            RecEntry {
                rel_path: PathBuf::from("left_only.txt"),
                status: RecStatus::LeftOnly,
                left_size: Some(1),
                right_size: None,
            },
            RecEntry {
                rel_path: PathBuf::from("right_only.txt"),
                status: RecStatus::RightOnly,
                left_size: None,
                right_size: Some(1),
            },
            RecEntry {
                rel_path: PathBuf::from("a_link"),
                status: RecStatus::Symlink,
                left_size: None,
                right_size: None,
            },
            RecEntry {
                rel_path: PathBuf::from("unreadable.txt"),
                status: RecStatus::Unreadable,
                left_size: None,
                right_size: None,
            },
            RecEntry {
                rel_path: PathBuf::from("cancelled.txt"),
                status: RecStatus::Computing,
                left_size: Some(1),
                right_size: Some(1),
            },
        ];
        let diff_options = DiffOptions {
            ignore_case: true,
            ..DiffOptions::default()
        };
        let mut report = DirComparisonReport::from_entries(
            &entries,
            Some(&PathBuf::from("/left")),
            Some(&PathBuf::from("/right")),
            Some(&diff_options),
            ReportOptions::default(),
        );
        let changed_row = report
            .file_rows
            .iter_mut()
            .find(|r| r.path == "changed.txt")
            .unwrap();
        changed_row.left_checksum = Some(FileChecksum::of_bytes(b"old"));
        changed_row.right_checksum = Some(FileChecksum::of_bytes(b"new"));
        let d = compute_diff("old\n", "new\n", DiffOptions::default());
        let hunks: Vec<ChangeHunk> = change_hunks_from_diff(&d, 3);
        changed_row.change_hunks = Some(hunks);
        // Exercise the cap-omission message too, for real.
        report.change_hunks_omitted.push("huge.txt".into());
        report
    }

    #[test]
    fn every_id_a_file_report_requests_in_japanese_has_a_real_translation() {
        let report = comprehensive_file_report();
        let ids = requested_ja_ids(Box::new(|tr| {
            render_file_report(&report, "0.200.0", "now", false, tr)
        }));
        assert!(
            !ids.is_empty(),
            "the renderer requested nothing - fixture is too thin"
        );
        assert_every_requested_id_has_a_real_ja_entry(&ids);
    }

    #[test]
    fn every_id_a_dir_report_requests_in_japanese_has_a_real_translation() {
        let report = comprehensive_dir_report();
        let ids = requested_ja_ids(Box::new(|tr| {
            render_dir_report(&report, "0.200.0", "now", false, tr)
        }));
        assert!(
            !ids.is_empty(),
            "the renderer requested nothing - fixture is too thin"
        );
        assert_every_requested_id_has_a_real_ja_entry(&ids);
    }

    /// Falsification: an id this test does *not* cover (because nothing
    /// requests it) must not silently pass - confirms
    /// `lookup(Lang::Ja, ..).is_none()` really does detect a gap, not
    /// just that the fixture is comprehensive by accident.
    #[test]
    fn a_genuinely_untranslated_id_is_detected_as_missing() {
        assert!(lookup(Lang::Ja, "an-id-that-is-definitely-not-in-the-catalog").is_none());
    }

    /// Review 185 §1: `report::html::cap_omission_template_key`'s four
    /// shapes (`n`/`max_files` each singular or plural) - checked against
    /// the real catalog above, not a test-local copy of the
    /// translations (the defect review 185 found in the previous version
    /// of this test, which lived in `forskscope-report` and could never
    /// fail against a real missing entry since its own translator closure
    /// held its own copy of all four strings).
    #[test]
    fn every_cap_omission_shape_in_japanese_has_a_real_translation() {
        for omitted_count in [1usize, 2] {
            for max_files in [1usize, 50] {
                let entries: Vec<RecEntry> = (0..omitted_count)
                    .map(|i| RecEntry {
                        rel_path: PathBuf::from(format!("huge{i}.txt")),
                        status: RecStatus::Changed,
                        left_size: Some(1),
                        right_size: Some(1),
                    })
                    .collect();
                let options = ReportOptions {
                    max_files_with_content: max_files,
                    ..ReportOptions::default()
                };
                let mut report =
                    DirComparisonReport::from_entries(&entries, None, None, None, options);
                for i in 0..omitted_count {
                    report.change_hunks_omitted.push(format!("huge{i}.txt"));
                }
                let ids = requested_ja_ids(Box::new(|tr| {
                    render_dir_report(&report, "0.200.0", "now", false, tr)
                }));
                assert!(
                    !ids.is_empty(),
                    "n={omitted_count}, max_files={max_files}: the renderer requested nothing"
                );
                assert_every_requested_id_has_a_real_ja_entry(&ids);
            }
        }
    }

    /// Review 175 §2: the checksum caveat sentence, rendered through the
    /// real `t(Lang::Ja, …)`, must carry `<code>sha256sum</code>` and no
    /// literal backtick in either language - not just in a mock-translator
    /// unit test of `checksum_caveat` itself, but in what the real app
    /// would actually produce.
    #[test]
    fn the_checksum_caveat_uses_code_markup_and_no_backtick_in_both_languages() {
        let report = comprehensive_file_report();
        for lang in [Lang::En, Lang::Ja] {
            let html = render_file_report(&report, "0.200.0", "now", false, &|s| t(lang, s));
            assert!(
                html.contains("<code>sha256sum</code>"),
                "{lang:?}: missing <code>sha256sum</code> markup"
            );
            assert!(
                !html.contains('`'),
                "{lang:?}: a literal backtick leaked into the rendered page"
            );
        }
    }

    #[test]
    fn t_translates_a_plain_message() {
        assert_eq!(
            t(Lang::Ja, "test-fixture-plain"),
            "プレーンなテストメッセージ"
        );
        assert_eq!(t(Lang::En, "test-fixture-plain"), "Plain test message");
    }

    #[test]
    fn t_falls_back_to_the_id_for_an_unknown_message() {
        assert_eq!(t(Lang::En, "no-such-message-id"), "no-such-message-id");
        assert_eq!(t(Lang::Ja, "no-such-message-id"), "no-such-message-id");
    }

    #[test]
    fn tf_substitutes_a_named_placeholder() {
        let out = tf(
            Lang::En,
            "test-fixture-placeholder",
            &[("name", "World")],
            None,
        );
        assert_eq!(out, "Hello, World!");
    }

    #[test]
    fn tf_picks_the_plural_form_from_count_and_substitutes_it_too() {
        assert_eq!(
            tf(Lang::En, "test-fixture-plural", &[], Some(1)),
            "1 test item"
        );
        assert_eq!(
            tf(Lang::En, "test-fixture-plural", &[], Some(3)),
            "3 test items"
        );
        // Japanese has only "other" - both counts render the same template.
        assert_eq!(
            tf(Lang::Ja, "test-fixture-plural", &[], Some(1)),
            "テスト項目 1 件"
        );
        assert_eq!(
            tf(Lang::Ja, "test-fixture-plural", &[], Some(3)),
            "テスト項目 3 件"
        );
    }

    /// Falsifies `tf`'s own fail-closed check: a plural message reached
    /// with no count must panic, not silently show `{ $count }` to a user
    /// (the same reasoning F237 applied to a missing config-root guard).
    #[test]
    #[should_panic(expected = "this message is plural but no count was given")]
    fn tf_panics_for_a_plural_message_with_no_count() {
        tf(Lang::En, "test-fixture-plural", &[], None);
    }

    /// RFC-100 §2: `BidiText` calls `t(lang, name)` with `name` read from
    /// `forskscope_compare::BIDI_CONTROLS` at runtime, not a literal
    /// `t(lang, "...")` the i18n audit's static scan can see - checked
    /// against the real table, not a copy of it (reviews 174/212's rule).
    #[test]
    fn every_bidi_control_name_has_a_real_distinct_ja_translation() {
        let mut missing = Vec::new();
        let mut untranslated = Vec::new();
        for c in forskscope_compare::BIDI_CONTROLS {
            match lookup(Lang::Ja, c.name) {
                None => missing.push(c.name),
                Some(forms) => {
                    let ja_text = forms.text_for_test();
                    let en_text = lookup(Lang::En, c.name).map(|f| f.text_for_test());
                    if Some(ja_text) == en_text {
                        untranslated.push(c.name);
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "these bidi control names have no catalog entry at all: {missing:?}"
        );
        assert!(
            untranslated.is_empty(),
            "these bidi control names' Japanese entry is identical to English, \
             not a real translation: {untranslated:?}"
        );
    }

    /// Review 232 §3.4: a placeholder carrying user data (a name, a
    /// path, error text) must come back as its **own** segment, never
    /// folded into surrounding `Text` — that's what lets
    /// `ui::view::TranslatedText` wrap exactly that one value in its own
    /// `<bdi>`, in both an English and a Japanese sentence, instead of
    /// splicing it into one opaque string. A Hebrew (right-to-left)
    /// file name is RFC-100's own worked example of what isolation is
    /// for. Falsify by reverting `split_template` to return one `Text`
    /// segment per template (no placeholder splitting): this fails
    /// immediately, since `name` would then appear nowhere as its own
    /// segment.
    #[test]
    fn a_users_file_name_is_its_own_segment_not_folded_into_surrounding_text() {
        let hebrew_name = "קובץ.txt";
        for (lang, id) in [
            (Lang::En, "could-not-open-error"),
            (Lang::Ja, "could-not-open-error"),
        ] {
            let segments = template_segments(lang, id, None);
            // Build the rendered string the old flat `tf()` would have
            // produced, to confirm the segments really do reconstruct
            // the same message (nothing lost in the split) ...
            let rebuilt: String = segments
                .iter()
                .map(|s| match s {
                    TemplateSegment::Text(t) => t.clone(),
                    TemplateSegment::Placeholder(name) if name == "name" => hebrew_name.to_string(),
                    TemplateSegment::Placeholder(name) if name == "error" => "x".to_string(),
                    TemplateSegment::Placeholder(_) => String::new(),
                })
                .collect();
            assert_eq!(
                rebuilt,
                tf(lang, id, &[("name", hebrew_name), ("error", "x")], None),
                "{lang:?}: splitting into segments and substituting must reproduce tf()'s own output"
            );
            // ... and that the name is reachable as a standalone
            // Placeholder("name") segment, the thing `TranslatedText`
            // isolates in its own `<bdi>`.
            assert!(
                segments
                    .iter()
                    .any(|s| matches!(s, TemplateSegment::Placeholder(n) if n == "name")),
                "{lang:?}: expected a standalone Placeholder(\"name\") segment, got {segments:?}"
            );
        }
    }

    /// Same reasoning as the bidi test above, for the theme/contrast
    /// `<select>` labels (`ui::view::settings::modal`'s own
    /// `t(lang, c.label)` call sites).
    #[test]
    fn every_theme_and_contrast_choice_label_has_a_real_distinct_ja_translation() {
        use forskscope_ui_logic::{contrast_choices, theme_choices};

        let mut labels: Vec<&str> = Vec::new();
        for system_offered in [false, true] {
            for saved_is_unavailable_system in [false, true] {
                for c in theme_choices(system_offered, saved_is_unavailable_system) {
                    labels.push(c.label);
                }
            }
        }
        for c in contrast_choices() {
            labels.push(c.label);
        }
        labels.sort_unstable();
        labels.dedup();

        let mut missing = Vec::new();
        let mut untranslated = Vec::new();
        for label in labels {
            match lookup(Lang::Ja, label) {
                None => missing.push(label),
                Some(forms) => {
                    let ja_text = forms.text_for_test();
                    let en_text = lookup(Lang::En, label).map(|f| f.text_for_test());
                    if Some(ja_text) == en_text {
                        untranslated.push(label);
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "these labels have no catalog entry at all: {missing:?}"
        );
        assert!(
            untranslated.is_empty(),
            "these labels' Japanese entry is identical to English, not a real \
             translation: {untranslated:?}"
        );
    }
}
