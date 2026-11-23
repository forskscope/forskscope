//! F226 (handoff 107): a test-only page, never shipped, comparing four
//! ways to change a live region's own text — the same four shapes the
//! app's own four real regions already use one of — against role, so the
//! investigation has an isolated, repeatable surface instead of reasoning
//! about the product's own regions directly (whose text also carries
//! real state, and whose mount/unmount timing is entangled with whatever
//! else that view is doing).
//!
//! A crate example, not a Cargo feature behind a release-binary marker
//! check (RFC-098's own `layout-probe` precedent) — chosen because an
//! example is *never* part of `cargo build --release`'s own binary at
//! all (`cargo build --bin forskscope` never touches `examples/`), so
//! there is no risk of it shipping to guard against in the first place,
//! and no second feature flag / env-var marker / release.yml grep step
//! to add and maintain for a page this narrow in scope.
//!
//! Five regions:
//! - **A — update in place:** always mounted; its own text signal is set
//!   directly. (The ignore-lists and Explorer regions both work this
//!   way today — reply 191c's own finding.)
//! - **B — insert with the text:** absent until triggered; mounted only
//!   once a short delay confirms it was actually absent for at least one
//!   full render (never patched in place from an existing "absent"
//!   state within the same tick). (The toast works this way.)
//! - **C — clear, then set next frame:** always mounted, like A, but its
//!   own text is blanked first and the real message arrives one frame
//!   later.
//! - **D — replace the node:** always mounted as a wrapper, but the
//!   child inside carries a `key` that changes on every trigger, forcing
//!   Dioxus to unmount the old instance and mount a new one in its
//!   place, continuously-present wrapper or not.
//! - **Alert — same mechanism as A** (update in place), `role="alert"`
//!   instead of `role="status"`, to isolate the role's own effect from
//!   the update shape's.
//!
//! Each region has its own uniquely-named trigger button
//! (`"Trigger A"`/`"Trigger B"`/`"Trigger C"`/`"Trigger D"`/`"Trigger
//! Alert"`), found and clicked over AT-SPI by a driver in
//! `.git-exclude/gui/drivers/`, inside this project's own private
//! session/accessibility-bus harness (handoff 106) — never a real
//! desktop.
//!
//! Run via `.git-exclude/gui/session.sh ... -- cargo run -p forskscope
//! --example announce_probe`, the same way every other GUI driver in
//! this project launches the real binary.

use dioxus::prelude::*;
use dioxus_desktop::tao::dpi::LogicalSize;
use dioxus_desktop::{Config, WindowBuilder};

fn main() {
    let window = WindowBuilder::new()
        .with_title("F226 announce probe")
        .with_inner_size(LogicalSize::new(640.0, 480.0));
    let config = Config::new().with_window(window).with_menu(None);
    dioxus_desktop::launch::launch(App, Vec::new(), vec![Box::new(config)]);
}

#[component]
fn App() -> Element {
    let mut counter = use_signal(|| 0u32);
    let mut next_message = move || {
        let n = *counter.read() + 1;
        counter.set(n);
        format!("message {n}")
    };

    // ── A: update in place ──────────────────────────────────────────────
    let mut text_a = use_signal(|| "a: (not yet triggered)".to_string());

    // ── B: insert with the text ─────────────────────────────────────────
    // `None` for at least one full render before `Some`, enforced with a
    // real async yield (`tokio::time::sleep`), not just a second
    // synchronous `.set()` in the same handler - Dioxus coalesces
    // same-handler writes to one signal into a single effective render,
    // which would make "absent, then present-with-text" indistinguishable
    // from "always present-with-text" to anything watching the DOM.
    let mut shape_b: Signal<Option<String>> = use_signal(|| None);

    // ── C: clear, then set next frame ───────────────────────────────────
    let mut text_c = use_signal(|| "c: (not yet triggered)".to_string());

    // ── D: replace the node (a new keyed element) ───────────────────────
    let mut key_d = use_signal(|| 0u32);
    let mut text_d = use_signal(|| "d: (not yet triggered)".to_string());

    // ── Alert: same mechanism as A, role="alert" ────────────────────────
    let mut text_alert = use_signal(|| "alert: (not yet triggered)".to_string());

    rsx! {
        div { style: "font-family: sans-serif; padding: 1rem;",
            h1 { "F226 announce probe" }

            div {
                h2 { "A — update in place" }
                p { id: "region-a", role: "status", "aria-live": "polite", "{text_a}" }
                button {
                    aria_label: "Trigger A",
                    onclick: move |_| { text_a.set(format!("a: {}", next_message())); },
                    "Trigger A"
                }
            }

            div {
                h2 { "B — insert with the text" }
                div { id: "region-b-container",
                    if let Some(t) = shape_b.read().as_ref() {
                        span { id: "region-b", role: "status", "aria-live": "polite", "{t}" }
                    } else {
                        span { id: "region-b-empty", "aria-hidden": "true", "(absent)" }
                    }
                }
                button {
                    aria_label: "Trigger B",
                    onclick: move |_| {
                        let msg = format!("b: {}", next_message());
                        shape_b.set(None);
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            shape_b.set(Some(msg));
                        });
                    },
                    "Trigger B"
                }
            }

            div {
                h2 { "C — clear, then set next frame" }
                p { id: "region-c", role: "status", "aria-live": "polite", "{text_c}" }
                button {
                    aria_label: "Trigger C",
                    onclick: move |_| {
                        let msg = format!("c: {}", next_message());
                        text_c.set(String::new());
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            text_c.set(msg);
                        });
                    },
                    "Trigger C"
                }
            }

            div {
                h2 { "D — replace the node (new keyed element)" }
                div { id: "region-d-container",
                    for k in [*key_d.read()] {
                        p {
                            key: "{k}",
                            id: "region-d",
                            role: "status",
                            "aria-live": "polite",
                            "{text_d}"
                        }
                    }
                }
                button {
                    aria_label: "Trigger D",
                    onclick: move |_| {
                        text_d.set(format!("d: {}", next_message()));
                        let k = *key_d.read();
                        key_d.set(k + 1);
                    },
                    "Trigger D"
                }
            }

            div {
                h2 { "Alert — update in place, role=\"alert\"" }
                p { id: "region-alert", role: "alert", "{text_alert}" }
                button {
                    aria_label: "Trigger Alert",
                    onclick: move |_| { text_alert.set(format!("alert: {}", next_message())); },
                    "Trigger Alert"
                }
            }
        }
    }
}
