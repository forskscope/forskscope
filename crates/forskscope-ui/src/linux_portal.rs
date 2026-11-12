//! RFC-097 stage 2 §2.1: Linux-only. WebKitGTK 4.1 (GTK 3) doesn't follow
//! the desktop's dark/light preference itself (measured, reply 184/review
//! 210), so `Theme::System` on Linux reads the freedesktop settings portal
//! (`org.freedesktop.portal.Settings`) directly, through `gio`, and follows
//! its `SettingChanged` signal for the rest of the session. If the portal
//! never answers — no session bus, no portal implementation, an unexpected
//! reply shape — `System` simply stays unoffered this session (`state::
//! settings::system_theme_offered`); that is the promise the handoff asks
//! for, not an error to report.

use crate::state::SystemColorScheme;
use gio::prelude::*;

const BUS_NAME: &str = "org.freedesktop.portal.Desktop";
const OBJECT_PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS_IFACE: &str = "org.freedesktop.portal.Settings";
const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "color-scheme";

/// `org.freedesktop.appearance`'s `color-scheme` value: `1` is dark, `2` is
/// light, `0` (no preference) is light (RFC-097 §2.1's own reading of the
/// portal spec). Anything else is treated the same as "no preference" —
/// the portal's own contract never defines a fourth value, so there is
/// nothing more specific to fall back to.
fn parse_color_scheme_value(n: u32) -> SystemColorScheme {
    if n == 1 {
        SystemColorScheme::Dark
    } else {
        SystemColorScheme::Light
    }
}

/// `ReadOne`'s reply is `(v)`: one variant holding the value directly.
/// Older portals have only `Read`, whose reply is also `(v)` but with the
/// value wrapped one `Variant` layer deeper still — this unwraps either
/// shape by trying the direct `u32` first, falling back to one more
/// `.get::<Variant>()` unwrap before giving up.
fn parse_reply_value(value: &glib::Variant) -> Option<SystemColorScheme> {
    if let Some(n) = value.get::<u32>() {
        return Some(parse_color_scheme_value(n));
    }
    // `.get::<Variant>()` on a value that isn't itself a "v" logs a
    // GLib-CRITICAL assertion (still returns `None`, but noisily) - `.is`
    // checks the type first so a reply in neither shape returns `None`
    // quietly.
    if !value.is::<glib::Variant>() {
        return None;
    }
    value
        .get::<glib::Variant>()
        .and_then(|inner| inner.get::<u32>())
        .map(parse_color_scheme_value)
}

fn read_color_scheme(proxy: &gio::DBusProxy) -> Option<SystemColorScheme> {
    let args = (NAMESPACE, KEY).to_variant();
    let reply = proxy
        .call_sync(
            "ReadOne",
            Some(&args),
            gio::DBusCallFlags::NONE,
            5000,
            gio::Cancellable::NONE,
        )
        .or_else(|_| {
            // Older portals (no `ReadOne`) only have `Read`.
            proxy.call_sync(
                "Read",
                Some(&args),
                gio::DBusCallFlags::NONE,
                5000,
                gio::Cancellable::NONE,
            )
        })
        .ok()?;
    parse_reply_value(&reply.child_value(0))
}

/// Blocking: connects to the session bus, reads the current value once,
/// then runs a private main loop for the rest of the process's life,
/// sending every reading (the initial one, then each `SettingChanged`) to
/// `tx`. Runs on its own OS thread (`state::settings::
/// spawn_system_preference_bridges`), never on the Dioxus/tokio executor —
/// every call here is a blocking D-Bus round trip, and the main loop blocks
/// forever by design.
///
/// Returns early, having sent nothing, if the session bus or the portal
/// isn't reachable at all — `system_theme_offered` reads "never sent
/// anything" and "not offered" as the same thing, so there is nothing
/// further to signal here.
pub(crate) fn watch_color_scheme(tx: tokio::sync::mpsc::UnboundedSender<SystemColorScheme>) {
    let context = glib::MainContext::new();
    let _ = context.with_thread_default(|| {
        let Ok(proxy) = gio::DBusProxy::for_bus_sync(
            gio::BusType::Session,
            gio::DBusProxyFlags::NONE,
            None,
            BUS_NAME,
            OBJECT_PATH,
            SETTINGS_IFACE,
            gio::Cancellable::NONE,
        ) else {
            return;
        };

        if let Some(scheme) = read_color_scheme(&proxy) {
            let _ = tx.send(scheme);
        }

        proxy.connect_g_signal(move |_proxy, _sender, signal_name, parameters| {
            if signal_name != "SettingChanged" {
                return;
            }
            let Some((namespace, key, value)) = parameters.get::<(String, String, glib::Variant)>()
            else {
                return;
            };
            if namespace == NAMESPACE
                && key == KEY
                && let Some(scheme) = parse_reply_value(&value)
            {
                let _ = tx.send(scheme);
            }
        });

        glib::MainLoop::new(Some(&context), false).run();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_scheme_value_1_is_dark() {
        assert_eq!(parse_color_scheme_value(1), SystemColorScheme::Dark);
    }

    #[test]
    fn color_scheme_value_2_is_light() {
        assert_eq!(parse_color_scheme_value(2), SystemColorScheme::Light);
    }

    #[test]
    fn color_scheme_value_0_no_preference_is_light() {
        assert_eq!(parse_color_scheme_value(0), SystemColorScheme::Light);
    }

    #[test]
    fn an_undefined_value_falls_back_to_light() {
        // The portal spec never defines a fourth value; this is the same
        // "no preference" fallback as 0, not a panic or a guess.
        assert_eq!(parse_color_scheme_value(99), SystemColorScheme::Light);
    }

    #[test]
    fn reply_shape_direct_u32_parses() {
        let value = 1u32.to_variant();
        assert_eq!(parse_reply_value(&value), Some(SystemColorScheme::Dark));
    }

    #[test]
    fn reply_shape_one_variant_layer_deeper_parses() {
        // The older `Read` method's reply wraps the value one layer deeper
        // than `ReadOne`'s - this is that extra wrapping.
        let inner = 2u32.to_variant();
        let outer = inner.to_variant();
        assert_eq!(parse_reply_value(&outer), Some(SystemColorScheme::Light));
    }

    #[test]
    fn an_unparseable_reply_is_none() {
        let value = "not a number".to_variant();
        assert_eq!(parse_reply_value(&value), None);
    }
}
