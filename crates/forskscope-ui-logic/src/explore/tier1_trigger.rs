//! When a tier-1 directory walk starts (RFC-080 §5, acceptance criterion 7).
//!
//! Selection changes on every keypress while arrowing through rows, so a walk
//! that started on selection would start twenty of them going down twenty
//! directories. The rule: **a walk starts only for a row that has been rested on
//! for [`TIER1_DEBOUNCE`]**, and is cancelled the moment the selection moves or
//! the pane navigates. At most one walk is ever in flight.
//!
//! This is the whole rule as a pure state machine: selection events and a
//! caller-supplied instant in, "start a walk for this row" and "cancel the
//! walk" out. The view supplies the clock (as a [`Duration`] since any fixed
//! origin, so tests need no real time) and does the spawning and cancelling —
//! through the same `DigestEpoch` every other comparison uses, so there is one
//! cancellation mechanism, not two.

use std::time::Duration;

/// How long a row must be rested on before its tier-1 walk starts.
///
/// **250 ms.** Key repeat during arrow-key navigation is far faster than this
/// (a held arrow repeats every ~30 ms), so a row must genuinely be rested on
/// before anything starts, while 250 ms still reads as immediate to someone who
/// stops on a row deliberately. It is the difference between this feature being
/// free and being a hazard, and it is one number in one place: change it here.
pub const TIER1_DEBOUNCE: Duration = Duration::from_millis(250);

/// What the view must do besides starting a walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tier1Action {
    /// The selection moved (or the pane navigated) while a walk was in flight:
    /// cancel it, and discard whatever it would have produced.
    CancelWalk,
}

/// The debounce state machine, generic over what identifies a row.
#[derive(Debug, Clone)]
pub struct Tier1Trigger<K> {
    interval: Duration,
    /// The row being rested on, and when the resting began.
    pending: Option<(K, Duration)>,
    /// The row whose walk is running. At most one.
    in_flight: Option<K>,
}

impl<K: Clone + PartialEq> Default for Tier1Trigger<K> {
    fn default() -> Self {
        Self::new(TIER1_DEBOUNCE)
    }
}

impl<K: Clone + PartialEq> Tier1Trigger<K> {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            pending: None,
            in_flight: None,
        }
    }

    /// A selection event. `row` is the directory row now selected that a walk
    /// could be started for, or `None` when the selection is not such a row
    /// (moved onto a file, cleared) or the pane navigated.
    ///
    /// Returns [`Tier1Action::CancelWalk`] when a walk is in flight for a row
    /// that is no longer the one selected. Re-selecting the row already being
    /// waited on keeps its timer: it is still the same rest.
    pub fn selected(&mut self, now: Duration, row: Option<K>) -> Option<Tier1Action> {
        match row {
            Some(k) if self.in_flight.as_ref() == Some(&k) => None,
            Some(k) if self.pending.as_ref().is_some_and(|(p, _)| *p == k) => None,
            Some(k) => {
                self.pending = Some((k, now));
                self.cancel_in_flight()
            }
            None => {
                self.pending = None;
                self.cancel_in_flight()
            }
        }
    }

    /// The clock has reached `now`. Returns the row to start a walk for when it
    /// has been rested on for the full interval — at most once per rest.
    pub fn tick(&mut self, now: Duration) -> Option<K> {
        let (key, since) = self.pending.as_ref()?;
        if now.saturating_sub(*since) < self.interval {
            return None;
        }
        let key = key.clone();
        self.pending = None;
        self.in_flight = Some(key.clone());
        Some(key)
    }

    /// The walk for `key` finished (or was abandoned).
    pub fn finished(&mut self, key: &K) {
        if self.in_flight.as_ref() == Some(key) {
            self.in_flight = None;
        }
    }

    /// The row whose walk is running, if any.
    pub fn in_flight(&self) -> Option<&K> {
        self.in_flight.as_ref()
    }

    /// When the current rest will have lasted long enough, if one is pending —
    /// for a caller that wants to sleep exactly that long.
    pub fn due_at(&self) -> Option<Duration> {
        self.pending
            .as_ref()
            .map(|(_, since)| *since + self.interval)
    }

    fn cancel_in_flight(&mut self) -> Option<Tier1Action> {
        self.in_flight.take().map(|_| Tier1Action::CancelWalk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Criterion 7: arrowing through rows — a selection every 30 ms, the clock
    /// ticked after each — starts **no** walk, however many rows are passed.
    /// Falsify by starting a walk in `selected` (no debounce): the count is 20.
    #[test]
    fn moving_through_rows_starts_no_walk() {
        let mut t: Tier1Trigger<u32> = Tier1Trigger::default();
        let mut walks = 0;
        for row in 0..20u32 {
            let now = ms(u64::from(row) * 30);
            t.selected(now, Some(row));
            if t.tick(now).is_some() {
                walks += 1;
            }
            // and a tick 100 ms later, still inside the next row's window
            if t.tick(now + ms(29)).is_some() {
                walks += 1;
            }
        }
        assert_eq!(walks, 0, "twenty rows passed through; no walk may start");
        assert_eq!(t.in_flight(), None);
    }

    /// Resting on one row past the interval starts exactly one walk, for that row.
    #[test]
    fn resting_past_the_interval_starts_one_walk_for_that_row() {
        let mut t: Tier1Trigger<u32> = Tier1Trigger::default();
        for row in 0..5u32 {
            t.selected(ms(u64::from(row) * 30), Some(row));
        }
        let rested = ms(4 * 30);
        assert_eq!(
            t.tick(rested + ms(249)),
            None,
            "one ms short of the interval"
        );
        assert_eq!(t.tick(rested + TIER1_DEBOUNCE), Some(4));
        assert_eq!(
            t.tick(rested + ms(10_000)),
            None,
            "a rest starts a walk once"
        );
        assert_eq!(t.in_flight(), Some(&4));
    }

    /// The selection moving cancels the walk in flight, and the next walk waits
    /// for the new rest; at most one walk is ever in flight.
    #[test]
    fn selection_moving_cancels_the_walk_and_never_leaves_two_in_flight() {
        let mut t: Tier1Trigger<u32> = Tier1Trigger::default();
        t.selected(ms(0), Some(1));
        assert_eq!(t.tick(ms(250)), Some(1));

        assert_eq!(t.selected(ms(300), Some(2)), Some(Tier1Action::CancelWalk));
        assert_eq!(
            t.in_flight(),
            None,
            "the cancelled walk is no longer in flight"
        );
        assert_eq!(t.tick(ms(400)), None);
        assert_eq!(t.tick(ms(550)), Some(2));
        assert_eq!(t.in_flight(), Some(&2));
    }

    /// Navigation (or selecting something that is not a candidate) cancels the
    /// in-flight walk and the pending rest.
    #[test]
    fn navigating_away_cancels_the_walk_and_the_pending_rest() {
        let mut t: Tier1Trigger<u32> = Tier1Trigger::default();
        t.selected(ms(0), Some(1));
        assert_eq!(t.tick(ms(250)), Some(1));
        assert_eq!(t.selected(ms(260), None), Some(Tier1Action::CancelWalk));

        t.selected(ms(300), Some(7));
        assert_eq!(
            t.selected(ms(310), None),
            None,
            "nothing in flight to cancel"
        );
        assert_eq!(t.tick(ms(10_000)), None, "the pending rest was dropped");
    }

    /// Re-selecting the row already being waited on (or walked) is the same rest.
    #[test]
    fn reselecting_the_same_row_keeps_its_timer_and_its_walk() {
        let mut t: Tier1Trigger<u32> = Tier1Trigger::default();
        t.selected(ms(0), Some(1));
        t.selected(ms(200), Some(1));
        assert_eq!(t.tick(ms(250)), Some(1), "the timer was not restarted");
        assert_eq!(t.selected(ms(300), Some(1)), None, "the walk keeps running");
        assert_eq!(t.in_flight(), Some(&1));
    }

    #[test]
    fn finishing_frees_the_slot() {
        let mut t: Tier1Trigger<u32> = Tier1Trigger::default();
        t.selected(ms(0), Some(1));
        t.tick(ms(250));
        t.finished(&9);
        assert_eq!(t.in_flight(), Some(&1), "another row's finish is not ours");
        t.finished(&1);
        assert_eq!(t.in_flight(), None);
    }

    #[test]
    fn due_at_names_when_the_rest_will_have_lasted_long_enough() {
        let mut t: Tier1Trigger<u32> = Tier1Trigger::default();
        assert_eq!(t.due_at(), None);
        t.selected(ms(100), Some(1));
        assert_eq!(t.due_at(), Some(ms(350)));
    }
}
