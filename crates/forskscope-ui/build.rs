//! F180 (handoff 073): `about.rs` reads `FORSKSCOPE_CHANNEL` through
//! `option_env!` at compile time. Without this, Cargo has no reason to
//! know that env var affects this crate's output, and can skip rebuilding
//! it when only that var changes between two builds of the same tree —
//! which is exactly how `PKGBUILD` and `store-build.ps1` select the
//! channel. In practice both are one-shot clean builds, so this would
//! rarely bite in CI, but it is cheap and correct to declare.
fn main() {
    println!("cargo:rerun-if-env-changed=FORSKSCOPE_CHANNEL");
}
