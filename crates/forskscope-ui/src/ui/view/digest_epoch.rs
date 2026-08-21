//! F78: one shared comparison mechanism for both `explorer.rs` and
//! `deep_compare.rs`, replacing the two half-mechanisms they had — a
//! generation guard with no concurrency bound (Explorer), and a
//! concurrency bound with no generation guard or cancellable digest calls
//! (Deep Compare).
//!
//! `DigestEpoch` owns exactly three concerns: which run a result belongs
//! to (generation), how to stop outstanding work early (a
//! `CancellationToken`), and how many blocking comparisons may run at once
//! (a `tokio::sync::Semaphore`, created once and never replaced — see
//! `restart()`).

use std::sync::Arc;

use tokio::sync::Semaphore;

use forskscope_core::CancellationToken;

/// An opaque marker for "the epoch as it was when this comparison was
/// spawned". Only [`DigestEpoch::begin_task`] can produce one — there is no
/// public constructor and no public field — so a caller cannot pass the
/// *current* generation where the *spawn-time* one belongs; the two are
/// different types, not just different values of the same `u64`. This is
/// F77's recorded regression exposure (review 074 §2), closed by
/// construction rather than by convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochStamp(u64);

/// The shared mechanism. One instance per view (Explorer, Deep Compare),
/// held in a `Signal` so `restart()` can be called from a `use_effect`.
pub struct DigestEpoch {
    generation: u64,
    token: CancellationToken,
    semaphore: Arc<Semaphore>,
}

impl DigestEpoch {
    /// `concurrency_limit` bounds how many comparisons spawned through this
    /// epoch's semaphore may run at once, for the lifetime of the epoch —
    /// not per generation. See `restart()`.
    pub fn new(concurrency_limit: usize) -> Self {
        Self {
            generation: 0,
            token: CancellationToken::new(),
            semaphore: Arc::new(Semaphore::new(concurrency_limit)),
        }
    }

    /// Called exactly where a view's inputs (its compare roots) change.
    /// Cancels outstanding work under the old roots, installs a fresh
    /// token for the new run, and bumps the generation so stamps taken
    /// before this call stop being current.
    ///
    /// The semaphore is deliberately left alone: replacing it per-epoch
    /// would let old-epoch tasks that are cancelled but still draining run
    /// alongside a full new epoch's worth of permits, bounding fan-out per
    /// epoch instead of overall.
    pub fn restart(&mut self) {
        self.token.cancel();
        self.token = CancellationToken::new();
        self.generation += 1;
    }

    /// Call once per about-to-be-spawned comparison, before the `spawn`.
    /// Returns the stamp to carry into the task (check it with
    /// `is_current` before applying the result), the token to pass into
    /// the cancellable comparison call, and the semaphore to acquire a
    /// permit from — inside the spawned task, after any signal `read()`
    /// guard has been dropped, never while holding one across an `await`.
    #[must_use]
    pub fn begin_task(&self) -> (EpochStamp, CancellationToken, Arc<Semaphore>) {
        (
            EpochStamp(self.generation),
            self.token.clone(),
            self.semaphore.clone(),
        )
    }

    /// Whether `stamp` still belongs to the current run. `false` means the
    /// comparison it was taken for has been superseded by a later
    /// `restart()` — its result must not be applied.
    pub fn is_current(&self, stamp: EpochStamp) -> bool {
        stamp.0 == self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // §8.2: `restart()` must invalidate a stamp taken before it.
    // Falsified by commenting out `self.generation += 1;` in `restart()`
    // above and re-running — `stale_stamp` stays current, and this test
    // fails. Restored.
    #[test]
    fn restart_invalidates_a_stamp_taken_before_it() {
        let mut epoch = DigestEpoch::new(4);
        let (stale_stamp, _, _) = epoch.begin_task();
        assert!(epoch.is_current(stale_stamp));

        epoch.restart();

        assert!(
            !epoch.is_current(stale_stamp),
            "restart() must invalidate stamps taken before it"
        );
    }

    // §8.2: `restart()` must cancel the token it handed out before it, so
    // outstanding work under the old roots actually stops instead of only
    // being ignored on arrival.
    #[test]
    fn restart_cancels_the_token_it_handed_out_before_it() {
        let mut epoch = DigestEpoch::new(4);
        let (_, token, _) = epoch.begin_task();
        assert!(!token.is_cancelled());

        epoch.restart();

        assert!(
            token.is_cancelled(),
            "restart() must cancel the token handed out before it"
        );
    }

    #[test]
    fn a_stamp_taken_after_restart_is_current() {
        let mut epoch = DigestEpoch::new(4);
        epoch.restart();
        let (stamp, _, _) = epoch.begin_task();

        assert!(epoch.is_current(stamp));
    }

    // The whole point of a persistent semaphore (§7a): replacing it per
    // `restart()` would bound fan-out per epoch instead of overall.
    #[test]
    fn the_semaphore_is_the_same_instance_across_restart() {
        let mut epoch = DigestEpoch::new(4);
        let (_, _, sem_before) = epoch.begin_task();

        epoch.restart();
        let (_, _, sem_after) = epoch.begin_task();

        assert!(
            Arc::ptr_eq(&sem_before, &sem_after),
            "the semaphore must survive restart(), not be replaced by it"
        );
    }
}
