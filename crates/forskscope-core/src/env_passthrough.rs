//! RFC-091 §2: when this process overrides an environment variable for its
//! own purposes (portable/private mode's `XDG_CACHE_HOME` et al.), a child
//! process ForskScope launches — the browser *Check for updates* opens, an
//! external tool, a `git` invocation — must not inherit the override. It
//! should see what the user's own environment actually had. This module is
//! the one place that remembers what to restore, so every `Command::new`
//! site only has to make one call.
//!
//! General-purpose on purpose: this crate has no notion of "portable mode"
//! or XDG — it just remembers variables a caller is about to override, and
//! reapplies the pre-override values onto a `Command`. The RFC-091-specific
//! part (which variables, and the new values) lives entirely in the caller.

use std::process::Command;
use std::sync::OnceLock;

/// One variable's original value, captured before the caller overrides it.
/// `None` means the variable was not set at all — restored by removing it
/// from the child's environment, not by setting it to an empty string.
#[derive(Debug, Clone)]
struct OriginalVar {
    name: &'static str,
    value: Option<String>,
}

static ORIGINAL_ENV: OnceLock<Vec<OriginalVar>> = OnceLock::new();

/// Captures `names`' current values. Call this immediately before actually
/// overriding any of them (e.g. with `std::env::set_var`) — capturing a
/// value after overriding it would "restore" the override itself, defeating
/// the whole point. A second call, in the same process, is a no-op: the
/// first call's capture is what every `restore_onto` call applies for the
/// rest of the run (`OnceLock`, like
/// [`crate::platform`]'s process-wide state).
pub fn record_before_override(names: &[&'static str]) {
    let captured = names
        .iter()
        .map(|&name| OriginalVar {
            name,
            value: std::env::var(name).ok(),
        })
        .collect();
    let _ = ORIGINAL_ENV.set(captured);
}

/// Applies every recorded variable's original value onto `cmd` — a present
/// variable is set back to what it was, an absent one is explicitly removed
/// from the child's environment (which otherwise inherits this process's
/// current, overridden value; `Command` does not strip the parent
/// environment unless told to). A no-op if [`record_before_override`] was
/// never called — nothing was overridden, so there is nothing to undo, and
/// every `Command` behaves exactly as it did before this module existed.
pub fn restore_onto(cmd: &mut Command) {
    let Some(vars) = ORIGINAL_ENV.get() else {
        return;
    };
    for var in vars {
        match &var.value {
            Some(value) => {
                cmd.env(var.name, value);
            }
            None => {
                cmd.env_remove(var.name);
            }
        }
    }
}
