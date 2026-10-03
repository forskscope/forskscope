//! Shared test infrastructure for permission-dependent tests (F143, handoff
//! 063 §1).
//!
//! Every `chmod`-based "make this unreadable" helper in this suite
//! (`make_dir_unopenable`, `make_child_metadata_unreadable`,
//! `make_dir_readonly`, and inlined equivalents) returns `false` instead of
//! asserting when the change had no effect. Before this module existed,
//! every site treated "had no effect" itself as the reason to skip,
//! silently, with no check of *why*: a chmod that failed to take effect for
//! any reason - a misconfigured fixture, a filesystem that ignores
//! permissions, an actual regression in one of these helpers - passed the
//! test without executing a single assertion, and `cargo test` hides a
//! passing test's output, so nothing would ever show it.
//!
//! [`permission_guard_failed`] is the fix: the only *known* reason a correct
//! `chmod` can have no effect on Unix is running as root, which bypasses
//! file-permission checks entirely regardless of the mode bits. Call it
//! when a guard reports no effect; it panics unless this process is root,
//! so anything else - the real regression this was meant to catch - fails
//! the suite instead of passing it silently.

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
/// cleanup-and-return - only when running as root. The panicking test's own
/// name already appears in `cargo test`'s `thread '...' panicked at` line,
/// so this takes no name of its own.
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn permission_enforcement_is_active_matches_euid() {
        // SAFETY: geteuid takes no arguments and cannot fail.
        let is_root = unsafe { libc::geteuid() == 0 };
        assert_eq!(permission_enforcement_is_active(), !is_root);
    }

    #[test]
    fn permission_guard_failed_does_not_panic_as_root() {
        if permission_enforcement_is_active() {
            eprintln!(
                "skipping permission_guard_failed_does_not_panic_as_root: \
                 not running as root, nothing to check here"
            );
            return;
        }
        // Root: must return normally, never panic.
        permission_guard_failed();
    }
}
