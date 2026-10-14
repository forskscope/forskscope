//! Handoff 075: two checks for the published site (`docs/site/` + the
//! mdbook guide, assembled into `_site/` by `pages.yml`).
//!
//! `cargo xtask pages-origin-check <index.html>` — the landing page's own
//! footer promises *"This page loads nothing from other sites and collects
//! nothing."* That is a gate, not a hope: every `src=`, `srcset=`, `poster=`,
//! `<link href=`, `<object data=`, `@import` and `url(...)` the page would
//! have the browser fetch must stay same-origin (a relative path, or a
//! fragment). `<a href>` is deliberately not scanned — a link a person
//! clicks is not something the page *loads*.
//!
//! F190 (review 158): the attribute/tag scan is case-insensitive and
//! tolerates whitespace around `=` (`SRC = "…"` is still an `src`
//! attribute). It does not handle an unquoted attribute value
//! (`src=foo.png`, no quotes at all) — noted by the review as a gap, but
//! explicitly left for a later handoff, since nothing this project writes
//! produces one.
//!
//! `cargo xtask pages-link-check <site-dir>` — every relative link or image
//! in the built `_site/`'s own HTML files resolves to a real file in it. A
//! link to a directory resolves to that directory's `index.html`. A page
//! that declares its own `<base href>` (only mdbook's generated 404 page
//! does, so it can be served for a request at any depth) is skipped — its
//! links resolve against an absolute site path this checker does not model,
//! and that page's live behaviour is verified on the deployed site instead
//! (handoff 075 §6.2), not here.

use std::fs;
use std::path::Path;
use std::process;

pub fn run_origin_check(index_html: &Path) {
    let html = fs::read_to_string(index_html)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", index_html.display()));
    let problems = foreign_origin_problems(&html);
    if !problems.is_empty() {
        eprintln!("pages origin check failed ({}):", index_html.display());
        for p in &problems {
            eprintln!("  - {p}");
        }
        process::exit(1);
    }
    println!(
        "pages origin check passed: {} loads nothing from another origin.",
        index_html.display()
    );
}

pub fn run_link_check(site_root: &Path) {
    let mut problems = Vec::new();
    check_dir(site_root, site_root, &mut problems);
    if !problems.is_empty() {
        eprintln!("pages link check failed:");
        for p in &problems {
            eprintln!("  - {p}");
        }
        process::exit(1);
    }
    println!(
        "pages link check passed: every relative link and image in {} resolves.",
        site_root.display()
    );
}

// ── Origin check ─────────────────────────────────────────────────────────

fn foreign_origin_problems(html: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for value in find_attr_values(html, "src") {
        if is_foreign(&value) {
            problems.push(format!("src=\"{value}\" loads from another origin"));
        }
    }
    for value in find_attr_values(html, "poster") {
        if is_foreign(&value) {
            problems.push(format!("poster=\"{value}\" loads from another origin"));
        }
    }
    for value in find_srcset_urls(html) {
        if is_foreign(&value) {
            problems.push(format!(
                "srcset candidate \"{value}\" loads from another origin"
            ));
        }
    }
    for value in find_tag_attr_values(html, "<link", "href") {
        if is_foreign(&value) {
            problems.push(format!("<link href=\"{value}\"> loads from another origin"));
        }
    }
    for value in find_tag_attr_values(html, "<object", "data") {
        if is_foreign(&value) {
            problems.push(format!(
                "<object data=\"{value}\"> loads from another origin"
            ));
        }
    }
    for value in find_css_urls(html) {
        if is_foreign(&value) {
            problems.push(format!("url({value}) loads from another origin"));
        }
    }
    for value in find_css_imports(html) {
        if is_foreign(&value) {
            problems.push(format!("@import \"{value}\" loads from another origin"));
        }
    }
    problems
}

/// Every candidate URL in a `srcset` value — a comma-separated list of
/// `url [descriptor]` pairs (`"small.jpg 480w, large.jpg 800w"`). Only the
/// URL, the leading token of each candidate, matters here.
fn find_srcset_urls(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for value in find_attr_values(html, "srcset") {
        for candidate in value.split(',') {
            if let Some(url) = candidate.split_whitespace().next() {
                out.push(url.to_string());
            }
        }
    }
    out
}

fn is_foreign(value: &str) -> bool {
    let v = value.trim().to_ascii_lowercase();
    v.starts_with("http://") || v.starts_with("https://") || v.starts_with("//")
}

// ── Link check ───────────────────────────────────────────────────────────

fn check_dir(site_root: &Path, dir: &Path, problems: &mut Vec<String>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            check_dir(site_root, &path, problems);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) == Some("html") {
            check_html_file(site_root, &path, problems);
        }
    }
}

fn check_html_file(site_root: &Path, file: &Path, problems: &mut Vec<String>) {
    let html =
        fs::read_to_string(file).unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
    if has_base_href(&html) {
        return;
    }
    let file_dir = file
        .parent()
        .expect("an html file under site_root has a parent");
    let mut targets = find_attr_values(&html, "href");
    targets.extend(find_attr_values(&html, "src"));
    for target in targets {
        if let Some(problem) = unresolved_link(site_root, file_dir, file, &target) {
            problems.push(problem);
        }
    }
}

fn has_base_href(html: &str) -> bool {
    find_tag_attr_values(html, "<base", "href")
        .into_iter()
        .next()
        .is_some()
}

fn unresolved_link(site_root: &Path, file_dir: &Path, file: &Path, target: &str) -> Option<String> {
    let target = target.trim();
    if target.is_empty() || target.starts_with('#') {
        return None;
    }
    if is_foreign(target) || target.starts_with("mailto:") || target.starts_with("data:") {
        return None;
    }
    let without_fragment = target.split('#').next().unwrap_or("");
    let path_part = without_fragment.split('?').next().unwrap_or("");
    if path_part.is_empty() {
        return None; // a same-page "?query" or "#fragment" link
    }

    let resolved = if let Some(rooted) = path_part.strip_prefix('/') {
        site_root.join(rooted)
    } else {
        file_dir.join(path_part)
    };
    let final_path = if resolved.is_dir() {
        resolved.join("index.html")
    } else {
        resolved
    };

    if final_path.is_file() {
        None
    } else {
        Some(format!(
            "{}: \"{target}\" does not resolve (looked for {})",
            file.strip_prefix(site_root).unwrap_or(file).display(),
            final_path
                .strip_prefix(site_root)
                .unwrap_or(&final_path)
                .display()
        ))
    }
}

// ── Small, dependency-free HTML/CSS scanners ────────────────────────────

/// Every `attr="value"` or `attr='value'` in `text`, wherever it appears —
/// good enough for a hand-written page and mdbook's own output, neither of
/// which nest quotes inside attribute values. The attribute name is matched
/// case-insensitively (`SRC=` is still `src=`), and whitespace around `=`
/// is tolerated (`src = "…"`). Guards against matching inside a longer name
/// (`data-src=` must not match `src=`): the character immediately before
/// the match, if any, must not be alphanumeric, `-` or `_`. An unquoted
/// value (`src=foo.png`, no quotes at all) is not matched — F190, left for
/// later, since nothing this project writes produces one.
fn find_attr_values(text: &str, attr: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let attr_lower = attr.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut start = 0usize;
    while let Some(pos) = lower[start..].find(&attr_lower) {
        let abs = start + pos;
        let boundary_ok = abs == 0
            || !matches!(lower.as_bytes()[abs - 1], b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_');
        let mut idx = abs + attr_lower.len();
        idx += whitespace_len(&lower[idx..]);
        if boundary_ok && lower.as_bytes().get(idx) == Some(&b'=') {
            idx += 1;
            idx += whitespace_len(&lower[idx..]);
            if let Some((value, consumed)) = extract_quoted(&text[idx..]) {
                out.push(value);
                start = idx + consumed;
                continue;
            }
        }
        start = abs + attr_lower.len();
    }
    out
}

/// How many leading bytes of `s` are ASCII whitespace.
fn whitespace_len(s: &str) -> usize {
    s.bytes().take_while(|b| b.is_ascii_whitespace()).count()
}

/// Like [`find_attr_values`], but only within occurrences of `tag` (e.g.
/// `"<link"`, matched case-insensitively), bounded by that tag's own
/// closing `>` — so a `href` on an `<a>` is never mistaken for one on a
/// `<link>`.
fn find_tag_attr_values(html: &str, tag: &str, attr: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let tag_lower = tag.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut rest_start = 0usize;
    while let Some(pos) = lower[rest_start..].find(&tag_lower) {
        let abs = rest_start + pos;
        let end = html[abs..]
            .find('>')
            .map(|i| abs + i + 1)
            .unwrap_or(html.len());
        out.extend(find_attr_values(&html[abs..end], attr));
        rest_start = end.max(abs + tag_lower.len());
    }
    out
}

/// `(value, bytes consumed up to and including the closing quote)` for a
/// leading `"..."` or `'...'` in `s`, or `None` if `s` does not start with a
/// quote.
fn extract_quoted(s: &str) -> Option<(String, usize)> {
    let quote = s.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = s[1..].find(quote)?;
    Some((s[1..1 + end].to_string(), 1 + end + 1))
}

/// Every CSS `url(...)` argument in `text`, quotes stripped.
fn find_css_urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find("url(") {
        let after = &rest[pos + 4..];
        let Some(end) = after.find(')') else { break };
        out.push(
            after[..end]
                .trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_string(),
        );
        rest = &after[end + 1..];
    }
    out
}

/// Every `@import "quoted string"` target in `text`. The other form,
/// `@import url(...)`, is left to [`find_css_urls`] — it already finds
/// every `url(...)`, `@import`'s included, so handling it again here would
/// just report the same foreign URL twice.
fn find_css_imports(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find("@import") {
        let after = &rest[pos + "@import".len()..];
        let stmt_end = after.find(';').unwrap_or(after.len());
        let stmt = after[..stmt_end].trim_start();
        if let Some((value, _)) = extract_quoted(stmt) {
            out.push(value);
        }
        rest = &after[stmt_end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    // ── Origin check ─────────────────────────────────────────────────────

    #[test]
    fn a_page_with_only_relative_and_anchor_links_has_no_problems() {
        let html = r#"
            <a class="brand" href="./"><img src="guide/assets/logo.png"></a>
            <a href="https://github.com/forskscope/forskscope">GitHub</a>
            <link rel="icon" href="guide/assets/logo.png">
            <style>body { background: url(guide/assets/bg.png); }</style>
        "#;
        assert!(foreign_origin_problems(html).is_empty());
    }

    /// Falsifies the check: a font link from another host must be caught.
    #[test]
    fn a_foreign_stylesheet_link_is_caught() {
        let html = r#"<link rel="stylesheet" href="https://fonts.googleapis.com/css?family=Foo">"#;
        let problems = foreign_origin_problems(html);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("fonts.googleapis.com"));
    }

    #[test]
    fn a_foreign_img_src_is_caught() {
        let html = r#"<img src="https://example.com/tracker.png">"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn a_foreign_css_url_is_caught() {
        let html = r#"<style>body { background: url("https://example.com/bg.png"); }</style>"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn a_foreign_css_import_url_form_is_caught_once() {
        let html = r#"<style>@import url("https://example.com/font.css");</style>"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn a_foreign_css_import_quoted_string_form_is_caught() {
        let html = r#"<style>@import "https://example.com/font.css";</style>"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn a_protocol_relative_src_is_caught() {
        let html = r#"<script src="//example.com/analytics.js"></script>"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn an_anchor_to_another_origin_is_not_a_problem() {
        let html = r#"<a href="https://github.com/forskscope/forskscope">GitHub</a>"#;
        assert!(foreign_origin_problems(html).is_empty());
    }

    #[test]
    fn a_data_attribute_ending_in_src_is_not_mistaken_for_src() {
        let html = r#"<div data-src="https://example.com/x.png"></div>"#;
        assert!(foreign_origin_problems(html).is_empty());
    }

    // ── F190 (review 158): case, whitespace, srcset/poster/object ──────────

    #[test]
    fn an_uppercase_src_attribute_is_caught() {
        let html = r#"<img SRC="https://example.com/tracker.png">"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn an_uppercase_link_tag_is_caught() {
        let html = r#"<LINK rel="stylesheet" href="https://fonts.googleapis.com/css">"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn an_uppercase_scheme_is_caught() {
        let html = r#"<img src="HTTPS://example.com/tracker.png">"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn whitespace_around_the_equals_sign_is_tolerated() {
        let html = r#"<img src = "https://example.com/tracker.png">"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn a_foreign_candidate_anywhere_in_srcset_is_caught() {
        let html =
            r#"<img src="local.png" srcset="local-2x.png 2x, https://example.com/hero.jpg 3x">"#;
        let problems = foreign_origin_problems(html);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("example.com/hero.jpg"));
    }

    #[test]
    fn a_foreign_poster_is_caught() {
        let html = r#"<video poster="https://example.com/poster.jpg"></video>"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    #[test]
    fn a_foreign_object_data_is_caught() {
        let html = r#"<object data="https://example.com/thing.svg"></object>"#;
        assert_eq!(foreign_origin_problems(html).len(), 1);
    }

    // ── Link check ───────────────────────────────────────────────────────

    #[test]
    fn a_relative_link_to_an_existing_file_resolves() {
        let dir = tempdir();
        fs::write(dir.join("about.html"), "").unwrap();
        fs::write(dir.join("index.html"), r#"<a href="about.html">About</a>"#).unwrap();
        let mut problems = Vec::new();
        check_html_file(&dir, &dir.join("index.html"), &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
        cleanup(&dir);
    }

    #[test]
    fn a_link_to_a_directory_resolves_to_its_index() {
        let dir = tempdir();
        fs::create_dir_all(dir.join("guide")).unwrap();
        fs::write(dir.join("guide/index.html"), "").unwrap();
        fs::write(dir.join("index.html"), r#"<a href="guide/">Guide</a>"#).unwrap();
        let mut problems = Vec::new();
        check_html_file(&dir, &dir.join("index.html"), &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
        cleanup(&dir);
    }

    #[test]
    fn a_fragment_only_link_is_not_checked_against_the_filesystem() {
        let dir = tempdir();
        fs::write(
            dir.join("index.html"),
            r##"<a href="#download">Download</a>"##,
        )
        .unwrap();
        let mut problems = Vec::new();
        check_html_file(&dir, &dir.join("index.html"), &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
        cleanup(&dir);
    }

    #[test]
    fn a_deep_link_with_an_anchor_resolves_on_its_file_part() {
        let dir = tempdir();
        fs::create_dir_all(dir.join("guide/users")).unwrap();
        fs::write(dir.join("guide/users/installation.html"), "").unwrap();
        fs::write(
            dir.join("index.html"),
            r##"<a href="guide/users/installation.html#verifying-your-download">Verify</a>"##,
        )
        .unwrap();
        let mut problems = Vec::new();
        check_html_file(&dir, &dir.join("index.html"), &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
        cleanup(&dir);
    }

    /// Falsifies the check: a link to a file that was never written must be
    /// reported, not silently accepted.
    #[test]
    fn a_broken_link_is_reported() {
        let dir = tempdir();
        fs::write(
            dir.join("index.html"),
            r#"<a href="guide/missing.html">Gone</a>"#,
        )
        .unwrap();
        let mut problems = Vec::new();
        check_html_file(&dir, &dir.join("index.html"), &mut problems);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("guide/missing.html"));
        cleanup(&dir);
    }

    #[test]
    fn a_page_declaring_its_own_base_href_is_skipped() {
        let dir = tempdir();
        fs::write(
            dir.join("404.html"),
            r#"<base href="/forskscope/guide/"><a href="definitely-missing.html">x</a>"#,
        )
        .unwrap();
        let mut problems = Vec::new();
        check_html_file(&dir, &dir.join("404.html"), &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
        cleanup(&dir);
    }

    #[test]
    fn an_external_link_is_never_checked_against_the_filesystem() {
        let dir = tempdir();
        fs::write(
            dir.join("index.html"),
            r#"<a href="https://github.com/forskscope/forskscope">GitHub</a>"#,
        )
        .unwrap();
        let mut problems = Vec::new();
        check_html_file(&dir, &dir.join("index.html"), &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
        cleanup(&dir);
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "forskscope-pages-check-test-{}-{}",
            process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }
}
