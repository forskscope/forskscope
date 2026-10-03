//! F136: debouncing a text field that is cheap to display but expensive to
//! commit.
//!
//! *Ignore directory names*/*Ignore file extensions* (RFC-056) used to write
//! `store.settings` on every keystroke. `AppSettings::ignore_rules()` reads
//! those fields, the Explorer memoises it, and its tree-rebuild effects
//! depend on that memo — so typing `target` (six characters) rebuilt both
//! panes' trees six times and collapsed an expanded one to its root six
//! times, a direct consequence of F112's fix making settings changes reach a
//! running Explorer at all.
//!
//! [`FieldDebounce`] is [`crate::Tier1Trigger`]'s worked example, copied: a
//! pure state machine over a caller-supplied clock, so the interval is
//! provable on a fake one rather than a real-time test. It is simpler than
//! `Tier1Trigger` because there is nothing to cancel — no walk is in flight,
//! only a value waiting to be committed, and the latest `changed()` always
//! wins. The view keeps displaying the field's own immediate input value
//! (never debounced — only the *commit*, the `store.settings` write and
//! everything downstream of it, waits).

use std::time::Duration;

/// How long a field must go unedited before its value commits.
///
/// **400 ms.** Longer than [`crate::TIER1_DEBOUNCE`]'s 250 ms on purpose: that
/// interval only has to outlast key-repeat (~30 ms) during arrow navigation,
/// but ordinary typing has real pauses between words and often between
/// characters — a touch-typist's inter-keystroke interval is commonly
/// 100-200 ms, and a slower typist's more than that. 250 ms would still
/// collapse *some* genuine mid-word pauses into a commit; 400 ms comfortably
/// outlasts a keystroke-to-keystroke gap while still reading as immediate
/// once typing actually stops.
pub const FIELD_DEBOUNCE: Duration = Duration::from_millis(400);

/// The debounce state machine, generic over the value being committed.
#[derive(Debug, Clone)]
pub struct FieldDebounce<V> {
    interval: Duration,
    /// The latest value, and when it arrived. `None` once committed, until
    /// the next edit.
    pending: Option<(V, Duration)>,
}

impl<V: Clone + PartialEq> Default for FieldDebounce<V> {
    fn default() -> Self {
        Self::new(FIELD_DEBOUNCE)
    }
}

impl<V: Clone + PartialEq> FieldDebounce<V> {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            pending: None,
        }
    }

    /// The field changed. Starts or restarts the rest — every edit pushes
    /// the commit back by the full interval, so a value only ever commits
    /// once editing has genuinely stopped.
    pub fn changed(&mut self, now: Duration, value: V) {
        self.pending = Some((value, now));
    }

    /// The clock has reached `now`. Returns the value to commit once it has
    /// rested long enough — at most once per rest, and `None` on every other
    /// call, including a call that arrives after an already-consumed rest
    /// (the common shape when several delayed checks were scheduled across
    /// a run of keystrokes and only the last one finds anything pending).
    pub fn tick(&mut self, now: Duration) -> Option<V> {
        let (value, since) = self.pending.as_ref()?;
        if now.saturating_sub(*since) < self.interval {
            return None;
        }
        let value = value.clone();
        self.pending = None;
        Some(value)
    }

    /// When the current rest will have lasted long enough, if a value is
    /// pending — for a caller that wants to sleep exactly that long.
    pub fn due_at(&self) -> Option<Duration> {
        self.pending
            .as_ref()
            .map(|(_, since)| *since + self.interval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// The defect itself, falsified: six keystrokes (one every 30 ms, well
    /// under the interval) must commit **once**, not six times. Falsify by
    /// committing on every `changed()` (no debounce): the count is 6.
    #[test]
    fn six_keystrokes_30ms_apart_commit_once_not_six_times() {
        let mut d: FieldDebounce<String> = FieldDebounce::default();
        let mut commits = 0;
        let word = "target";
        for (i, _) in word.char_indices() {
            let now = ms(u64::from(i as u32) * 30);
            d.changed(now, word[..=i].to_string());
            if d.tick(now).is_some() {
                commits += 1;
            }
        }
        // Keep checking well past the last keystroke, the way the view's
        // repeated spawned checks do - only one of them may ever find
        // something to commit.
        for extra in 1..=20u64 {
            if d.tick(ms(150 + extra * 30)).is_some() {
                commits += 1;
            }
        }
        assert_eq!(commits, 1, "six keystrokes must commit exactly once");
    }

    /// The committed value is the *last* one typed, not an intermediate one -
    /// a debounce that fired early would commit a partial word.
    #[test]
    fn the_committed_value_is_the_last_one_typed() {
        let mut d: FieldDebounce<String> = FieldDebounce::default();
        for (i, _) in "target".char_indices() {
            d.changed(ms(u64::from(i as u32) * 30), "target"[..=i].to_string());
        }
        let last_edit = ms(5 * 30);
        assert_eq!(d.tick(last_edit + ms(399)), None, "one ms short");
        assert_eq!(
            d.tick(last_edit + FIELD_DEBOUNCE),
            Some("target".to_string())
        );
    }

    /// Resuming typing before the interval elapses restarts the rest -
    /// nothing commits while edits keep arriving, however long the overall
    /// session runs.
    #[test]
    fn typing_again_before_the_interval_elapses_restarts_the_rest() {
        let mut d: FieldDebounce<String> = FieldDebounce::default();
        d.changed(ms(0), "t".to_string());
        assert_eq!(d.tick(ms(399)), None);
        d.changed(ms(399), "ta".to_string());
        // Had the first edit's timer not been restarted, this would commit
        // "t" at ms(400).
        assert_eq!(
            d.tick(ms(400)),
            None,
            "the second edit must restart the rest"
        );
        assert_eq!(d.tick(ms(399 + 400)), Some("ta".to_string()));
    }

    #[test]
    fn due_at_names_when_the_rest_will_have_lasted_long_enough() {
        let mut d: FieldDebounce<String> = FieldDebounce::default();
        assert_eq!(d.due_at(), None);
        d.changed(ms(100), "x".to_string());
        assert_eq!(d.due_at(), Some(ms(100) + FIELD_DEBOUNCE));
    }

    /// Once consumed, a pending value does not commit again - the state the
    /// "several delayed checks in flight" shape above depends on.
    #[test]
    fn a_consumed_commit_does_not_fire_again() {
        let mut d: FieldDebounce<String> = FieldDebounce::default();
        d.changed(ms(0), "x".to_string());
        assert_eq!(d.tick(ms(500)), Some("x".to_string()));
        assert_eq!(d.tick(ms(600)), None);
        assert_eq!(d.tick(ms(10_000)), None);
    }
}
