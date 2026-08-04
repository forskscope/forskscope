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
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod i18n;
mod state;
mod ui;

use dioxus_desktop::tao::dpi::LogicalSize;
use dioxus_desktop::{Config, WindowBuilder};
use forskscope_ui_logic::parse_startup_args;

use app::{App, STARTUP_REQUEST};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // --diagnostics: print platform info and exit without launching the UI.
    // Useful for debugging startup failures and filing bug reports.
    if args.iter().any(|a| a == "--diagnostics") {
        let info = forskscope_core::platform::PlatformInfo::collect();
        println!("{}", info.to_report());
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

    let window = WindowBuilder::new()
        .with_title("ForskScope")
        .with_inner_size(LogicalSize::new(1180.0, 760.0));

    dioxus_desktop::launch::launch(
        App,
        Vec::new(),
        vec![Box::new(Config::new().with_window(window).with_menu(None))],
    );
}
