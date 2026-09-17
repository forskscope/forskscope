//! ForskScope desktop entry point.
//!
//! Startup modes:
//!
//! ```
//! forskscope                       # Explorer workspace
//! forskscope <left> <right>        # Two-file diff (git difftool compatible)
//! forskscope <local> <remote> <merged>  # git mergetool: diff local vs remote,
//!                                       # save result to <merged>
//! forskscope --diagnostics         # Print platform diagnostics and exit
//! ```
//!
//! Any other argument count is a startup error (non-zero exit), not a
//! silent fallback to the Explorer workspace (RFC-077).
//!
//! Exit codes: 1 is a startup-argument error; 3 (Windows only, F109) is a
//! missing Microsoft Edge WebView2 Runtime, raised whichever button the
//! user presses on the message box offering the download page.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod i18n;
mod keyboard;
mod state;
mod ui;
mod webview2;

use dioxus_desktop::tao::dpi::LogicalSize;
use dioxus_desktop::{Config, WindowBuilder};
use forskscope_ui_logic::parse_startup_args;

use app::{App, STARTUP_REQUEST};

/// Windows-only: the saved language, read without side effects (no
/// migration write, no recovery flow, no file created) via
/// [`forskscope_core::persist::schema::settings::SettingsRepository::load`].
/// Falls back to English — matching the owner's 2026-09-17 decision that a
/// first launch is English — whenever no settings file exists yet or it
/// cannot be read as a current/migrated value.
#[cfg(windows)]
fn saved_lang_no_side_effects() -> state::Lang {
    use forskscope_core::persist::schema::PersistenceLoad;
    use forskscope_core::persist::schema::settings::SettingsRepository;
    use forskscope_core::settings::LocaleId;

    let repo = SettingsRepository::new(state::config_file_path("settings.json"));
    let language = match repo.load() {
        PersistenceLoad::Current { value } | PersistenceLoad::MigratedLegacy { value, .. } => {
            Some(value.language)
        }
        PersistenceLoad::Missing { .. }
        | PersistenceLoad::FutureVersion { .. }
        | PersistenceLoad::Corrupt { .. } => None,
    };
    match language {
        Some(lang) if lang == LocaleId::japanese() => state::Lang::Ja,
        _ => state::Lang::En,
    }
}

/// Windows-only WebView2 presence check, run after argument parsing and
/// before the window is created (F109). Wires the OS calls to
/// [`webview2::decide`]: reads the runtime version, on failure prints to
/// stderr, shows a message box, and on "Yes" opens the download page via
/// `cmd /C start "" <url>` — the standard way to open a URL from a Windows
/// console app with no new dependency; passing the empty string as the
/// explicit window-title argument avoids `start` misreading the URL itself
/// as a quoted title, which is the usual quoting pitfall with this command.
#[cfg(windows)]
fn check_webview2_or_exit() {
    let version = wry::webview_version().map_err(|e| e.to_string());
    let lang = saved_lang_no_side_effects();
    if let webview2::Decision::Stop {
        stderr_line,
        dialog_text,
    } = webview2::decide(version, lang)
    {
        eprintln!("{stderr_line}");
        let pressed_yes = rfd::MessageDialog::new()
            .set_title("ForskScope")
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::YesNo)
            .set_description(dialog_text)
            .show()
            == rfd::MessageDialogResult::Yes;
        if pressed_yes {
            let _ = std::process::Command::new("cmd")
                .args([
                    "/C",
                    "start",
                    "",
                    "https://developer.microsoft.com/microsoft-edge/webview2/",
                ])
                .spawn();
        }
        std::process::exit(3);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // --diagnostics: print platform info and exit without launching the UI.
    // Useful for debugging startup failures and filing bug reports.
    if args.iter().any(|a| a == "--diagnostics") {
        let info = forskscope_core::platform::PlatformInfo::collect();
        println!("{}", info.to_report());
        #[cfg(windows)]
        match wry::webview_version() {
            Ok(version) => println!("WebView2 runtime: {version}"),
            Err(error) => println!("WebView2 runtime: not found ({error})"),
        }
        return;
    }

    match parse_startup_args(&args) {
        Ok(request) => {
            let _ = STARTUP_REQUEST.set(request);
        }
        Err(error) => {
            eprintln!("forskscope: {error}");
            std::process::exit(1);
        }
    }

    #[cfg(windows)]
    check_webview2_or_exit();

    let window = WindowBuilder::new()
        .with_title("ForskScope")
        .with_inner_size(LogicalSize::new(1180.0, 760.0));

    dioxus_desktop::launch::launch(
        App,
        Vec::new(),
        vec![Box::new(Config::new().with_window(window).with_menu(None))],
    );
}
