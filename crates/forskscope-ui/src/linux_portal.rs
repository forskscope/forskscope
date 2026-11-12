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

fn read_color_scheme(proxy: &gio::DBusProxy, timeout_msec: i32) -> Option<SystemColorScheme> {
    let args = (NAMESPACE, KEY).to_variant();
    let reply = proxy
        .call_sync(
            "ReadOne",
            Some(&args),
            gio::DBusCallFlags::NONE,
            timeout_msec,
            gio::Cancellable::NONE,
        )
        .or_else(|_| {
            // Older portals (no `ReadOne`) only have `Read`.
            proxy.call_sync(
                "Read",
                Some(&args),
                gio::DBusCallFlags::NONE,
                timeout_msec,
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

        if let Some(scheme) = read_color_scheme(&proxy, 5000) {
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

/// RFC-097 stage 2 §2.1, review 211 §3: a bounded, synchronous read, used
/// only once at startup, *before* settings resolve - never a substitute
/// for [`watch_color_scheme`]'s own async, unbounded subscription, which
/// still starts afterward regardless of what this returns. Exists so a
/// saved `Theme::System` (or a brand-new install) renders its *first*
/// paint correctly on Linux, rather than flashing `Dark` for one frame
/// while the async reader is still connecting - review 211 measured that
/// flash live (reply 185's own fake-portal captures, taken before this
/// fix, started on `Dark` even for a portal seeded at value 2/light).
///
/// `timeout_ms` bounds *wall-clock* time, not just the D-Bus call's own
/// timeout: `for_bus_sync` itself takes no deadline parameter, so the
/// whole read runs on a detached thread and this function gives up (and
/// returns `None`, read as "no portal" - the same outcome a real absence
/// produces) if nothing comes back in time. The thread is not joined; on
/// a timeout it keeps running until the D-Bus call itself gives up
/// (`read_color_scheme`'s own `timeout_msec`, passed through unchanged)
/// and then exits quietly - one short-lived leaked thread, once per
/// process start, never accumulating.
pub(crate) fn read_color_scheme_once_with_timeout(timeout_ms: u64) -> Option<SystemColorScheme> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let context = glib::MainContext::new();
        let result = context.with_thread_default(|| {
            let proxy = gio::DBusProxy::for_bus_sync(
                gio::BusType::Session,
                gio::DBusProxyFlags::NONE,
                None,
                BUS_NAME,
                OBJECT_PATH,
                SETTINGS_IFACE,
                gio::Cancellable::NONE,
            )
            .ok()?;
            read_color_scheme(&proxy, timeout_ms.min(i32::MAX as u64) as i32)
        });
        let _ = tx.send(result.ok().flatten());
    });
    rx.recv_timeout(std::time::Duration::from_millis(timeout_ms))
        .ok()
        .flatten()
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
