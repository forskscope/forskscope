//! Shared test infrastructure for permission-dependent tests (F143, handoff
//! 063 §1) - see `forskscope-core`'s own `src/tests/support.rs` for the full
//! rationale; this is the same fix, duplicated once per crate because
//! `ui/view/explorer.rs` and `state/compare/tests.rs` share no other common
//! test module today.

/// Whether this process can rely on Unix file-permission checks actually
/// doing something. `false` only when running as root (euid 0) - the one
/// case where a permission guard reporting no effect is a known, deliberate
/// exemption rather than an unexplained one.
#[cfg(unix)]
pub(crate) fn permission_enforcement_is_active() -> bool {
    // SAFETY: geteuid takes no arguments, reads process state, and cannot fail.
    unsafe { libc::geteuid() != 0 }
}

/// Call when a `chmod`-based guard reports no effect. Panics if permission
/// enforcement should be active (not root) - there is no known reason the
/// change should have failed, so silently skipping would hide a genuine
/// regression. Returns normally - the caller then skips, per the usual
/// cleanup-and-return - only when running as root.
#[cfg(unix)]
pub(crate) fn permission_guard_failed() {
    assert!(
        !permission_enforcement_is_active(),
        "a permission-based guard had no effect even though this process is \
         not running as root - permission enforcement should be active, so \
         this is not the known root exemption; investigate rather than let \
         this skip silently (F143)"
    );
    eprintln!(
        "skipping: running as root, which bypasses chmod-based unreadability \
         entirely"
    );
}
