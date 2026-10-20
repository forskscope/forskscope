//! `env_passthrough` — restoring a pre-override environment variable onto a
//! child `Command` (RFC-091 §2).

use std::process::Command;

use crate::env_passthrough::{record_before_override, restore_onto};

/// Falsify by having `restore_onto` skip the `None` arm (never call
/// `env_remove`) and the "was unset" assertion below fails — the child
/// would inherit this process's override as a real value where the user's
/// environment actually had nothing.
#[test]
fn restore_onto_undoes_an_override_and_removes_a_variable_that_was_never_set() {
    const SET_VAR: &str = "FORSKSCOPE_ENV_PASSTHROUGH_TEST_SET";
    const UNSET_VAR: &str = "FORSKSCOPE_ENV_PASSTHROUGH_TEST_UNSET";

    // SAFETY: both names are fabricated for this test alone and are not
    // read or written anywhere else in the process.
    unsafe {
        std::env::set_var(SET_VAR, "original-value");
        std::env::remove_var(UNSET_VAR);
    }

    record_before_override(&[SET_VAR, UNSET_VAR]);

    // Simulate the caller's own override, same as main() would do to
    // XDG_CACHE_HOME — restore_onto must undo exactly this.
    // SAFETY: see above.
    unsafe {
        std::env::set_var(SET_VAR, "overridden-value");
        std::env::set_var(UNSET_VAR, "should-not-leak-to-a-child");
    }

    let mut cmd = Command::new("true");
    restore_onto(&mut cmd);

    let applied: std::collections::HashMap<_, _> = cmd.get_envs().collect();
    assert_eq!(
        applied.get(std::ffi::OsStr::new(SET_VAR)),
        Some(&Some(std::ffi::OsStr::new("original-value"))),
        "a variable that was set must be restored to its original value"
    );
    assert_eq!(
        applied.get(std::ffi::OsStr::new(UNSET_VAR)),
        Some(&None),
        "a variable that was unset must be explicitly removed from the \
         child's environment, not merely left at its overridden value"
    );
}

/// `record_before_override` is a process-wide `OnceLock` — a second call
/// in the same test binary is a no-op, so this cannot also assert nothing
/// was ever recorded; it only pins down that `restore_onto` makes no
/// `.env()`/`.env_remove()` call for a variable it was never told about,
/// using a name no other test in this suite touches.
#[test]
fn restore_onto_does_not_touch_a_variable_nobody_recorded() {
    let mut cmd = Command::new("true");
    restore_onto(&mut cmd);
    let applied: std::collections::HashMap<_, _> = cmd.get_envs().collect();
    assert!(
        !applied.contains_key(std::ffi::OsStr::new(
            "FORSKSCOPE_ENV_PASSTHROUGH_TEST_NEVER_RECORDED"
        )),
        "a variable nobody called record_before_override for must not appear on the command at all"
    );
}
