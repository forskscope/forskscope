//! ForskScope desktop entry point.

mod app;
mod i18n;
mod state;
mod ui;

use std::path::PathBuf;

use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::{Config, WindowBuilder};

use app::{App, STARTUP_PAIR};

fn main() {
    // `forskscope <left> <right>` opens a comparison at startup (RFC-034).
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pair = match args.as_slice() {
        [left, right] => Some((PathBuf::from(left), PathBuf::from(right))),
        _ => None,
    };
    let _ = STARTUP_PAIR.set(pair);

    let window = WindowBuilder::new()
        .with_title("ForskScope")
        .with_inner_size(LogicalSize::new(1180.0, 760.0));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window))
        .launch(App);
}
