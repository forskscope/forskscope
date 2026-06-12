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

mod app;
mod i18n;
mod state;
mod ui;

use std::path::PathBuf;

use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::{Config, WindowBuilder};

use app::{App, STARTUP_MERGED, STARTUP_PAIR};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // --diagnostics: print platform info and exit without launching the UI.
    // Useful for debugging startup failures and filing bug reports.
    if args.iter().any(|a| a == "--diagnostics") {
        let info = forskscope_core::platform::PlatformInfo::collect();
        println!("{}", info.to_report());
        return;
    }

    match args.as_slice() {
        [left, right] => {
            let _ = STARTUP_PAIR.set(Some((PathBuf::from(left), PathBuf::from(right))));
            let _ = STARTUP_MERGED.set(None);
        }
        [local, remote, merged] => {
            let _ = STARTUP_PAIR.set(Some((PathBuf::from(local), PathBuf::from(remote))));
            let _ = STARTUP_MERGED.set(Some(PathBuf::from(merged)));
        }
        _ => {
            let _ = STARTUP_PAIR.set(None);
            let _ = STARTUP_MERGED.set(None);
        }
    }

    let window = WindowBuilder::new()
        .with_title("ForskScope")
        .with_inner_size(LogicalSize::new(1180.0, 760.0));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window))
        .launch(App);
}
