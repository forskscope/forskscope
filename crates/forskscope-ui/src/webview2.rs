//! Windows-only WebView2 Runtime presence check (F109).
//!
//! `dioxus-desktop` panics inside `wry` if the runtime is missing, and a
//! release build has no console to show why. `main.rs` wires the OS calls
//! (`wry::webview_version`, `rfd::MessageDialog`, opening the download page)
//! to [`decide`], which stays a plain function so it can be unit-tested
//! without a window or a real WebView2 install.

use crate::i18n::t;
use crate::state::Lang;

/// What `main.rs` should do once the check has run.
///
/// Only `main.rs`'s `#[cfg(windows)]` startup path calls [`decide`] outside
/// of tests — `allow(dead_code)` on other targets keeps this unit-testable
/// from any platform without a Windows-only build warning under `-D
/// warnings`.
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Decision {
    /// The runtime is present; startup continues.
    Proceed,
    /// The runtime is missing. `stderr_line` is what `main.rs` prints before
    /// showing the message box; `dialog_text` is the message box's body.
    Stop {
        stderr_line: String,
        dialog_text: String,
    },
}

/// Pure decision from `wry::webview_version()`'s result: `Ok(version)` when
/// the runtime is installed, `Err(message)` otherwise (the OS call itself
/// lives in `main.rs`, which is `#[cfg(windows)]`).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn decide(version: Result<String, String>, lang: Lang) -> Decision {
    match version {
        Ok(_) => Decision::Proceed,
        Err(error) => Decision::Stop {
            stderr_line: format!(
                "{} ({error})",
                t(
                    lang,
                    "forskscope: the Microsoft Edge WebView2 Runtime is not installed"
                )
            ),
            dialog_text: t(
                lang,
                "ForskScope needs the Microsoft Edge WebView2 Runtime, which is not installed on this computer. Open the download page?",
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_proceeds() {
        assert!(matches!(
            decide(Ok("120.0.6099.130".to_string()), Lang::En),
            Decision::Proceed
        ));
    }

    #[test]
    fn absent_stops_with_a_non_empty_bilingual_message() {
        let mut dialog_texts = Vec::new();
        for lang in [Lang::En, Lang::Ja] {
            match decide(Err("not found".to_string()), lang) {
                Decision::Stop {
                    stderr_line,
                    dialog_text,
                } => {
                    assert!(!stderr_line.is_empty());
                    assert!(!dialog_text.is_empty());
                    assert!(stderr_line.contains("not found"));
                    dialog_texts.push(dialog_text);
                }
                Decision::Proceed => panic!("expected Stop when the runtime is absent"),
            }
        }
        // Review 109 §5: a missing `ja()` entry would make `t` fall back to
        // the English key, and the assertions above would still pass. The
        // two languages' text must actually differ, not merely both be
        // non-empty.
        assert_ne!(
            dialog_texts[0], dialog_texts[1],
            "the Ja dialog text must differ from the En one — a missing i18n.rs entry would silently fall back to English"
        );
    }
}
