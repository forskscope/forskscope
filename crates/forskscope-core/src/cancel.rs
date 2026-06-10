//! Lightweight cancellation token for long-running blocking operations
//! (RFC-037 §"Cancellation", RFC-008 §"Background Job Model").
//!
//! The token is a thin wrapper around `Arc<AtomicBool>`. The caller creates
//! a token, keeps the handle, and passes clones into blocking tasks.
//! When the caller calls [`CancellationToken::cancel`], all clones observe
//! the cancellation on their next [`CancellationToken::is_cancelled`] check.
//!
//! There is no async machinery here; the token is polled explicitly by
//! blocking loops. Async cancellation (e.g. `tokio_util::CancellationToken`)
//! is a UI-layer concern and is intentionally not introduced into core.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A cloneable cancellation signal for blocking operations.
///
/// # Example
/// ```
/// use forskscope_core::CancellationToken;
///
/// let token = CancellationToken::new();
/// let worker_token = token.clone();
///
/// // In a blocking task:
/// // while !worker_token.is_cancelled() { /* work */ }
///
/// token.cancel();
/// assert!(worker_token.is_cancelled());
/// ```
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Create a new, uncancelled token.
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Signal cancellation. All clones will observe this on their next
    /// [`is_cancelled`](Self::is_cancelled) call.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// `true` once [`cancel`](Self::cancel) has been called on any clone.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
