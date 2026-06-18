use std::path::Path;
use crate::state::settings::Lang;
use super::tab_title;

#[test]
fn same_filename_both_sides_shows_single_name() {
    assert_eq!(
        tab_title(Path::new("/old/src/main.rs"), Path::new("/new/src/main.rs"), Lang::En),
        "main.rs"
    );
}

#[test]
fn different_filenames_shows_both_with_arrow() {
    assert_eq!(
        tab_title(Path::new("/old/foo.txt"), Path::new("/new/bar.txt"), Lang::En),
        "foo.txt ↔ bar.txt"
    );
}

#[test]
fn left_only_filename_shows_left() {
    assert_eq!(
        tab_title(Path::new("/project/README.md"), Path::new("/"), Lang::En),
        "README.md"
    );
}

#[test]
fn both_missing_filenames_shows_fallback() {
    assert_eq!(
        tab_title(Path::new("/"), Path::new("/"), Lang::En),
        "comparison"
    );
}

#[test]
fn hidden_dotfile_names_match_correctly() {
    assert_eq!(
        tab_title(Path::new("/a/.gitignore"), Path::new("/b/.gitignore"), Lang::En),
        ".gitignore"
    );
}

#[test]
fn deeply_nested_same_filename_shows_single_name() {
    assert_eq!(
        tab_title(
            Path::new("/home/alice/projectA/src/lib/core/mod.rs"),
            Path::new("/home/bob/projectB/src/lib/core/mod.rs"),
            Lang::En,
        ),
        "mod.rs"
    );
}
