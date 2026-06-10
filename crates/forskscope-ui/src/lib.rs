//! Library entry point for `forskscope-ui`.
//!
//! Exposing a `[lib]` target alongside the `[[bin]]` lets `cargo test --lib`
//! run `#[cfg(test)]` blocks in modules that have no GTK/WebView dependency.
//! Most pure logic now lives in `forskscope-ui-logic`; the binary entry
//! point remains `main.rs`.

pub mod app;
pub mod i18n;
pub mod state;
pub mod ui;
