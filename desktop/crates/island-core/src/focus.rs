//! The focus timer (Pomodoro): focus, a break, and a longer break every few
//! rounds. Pure: the app passes the time (ms on a steady clock), so nothing
//! here ticks; the app sets one timer for when the phase ends.
//!
//! As in the plugin, a break starts by itself when focus ends, and the
//! next focus waits for a click.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Focus,
    Break,
    LongBreak,
}

/// How long each phase is, in minutes, and how many rounds of focus come
/// before a long break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lengths {
    pub focus: u32,
    pub short: u32,
    pub long: u32,
    pub every: u32,
}

impl Default for Lengths {
    fn default() -> Self {
        Self {
            focus: 25,
            short: 5,
            long: 15,
            every: 4,
        }
    }
}

impl Lengths {
    pub const FOCUS: (u32, u32) = (5, 120);
    pub const SHORT: (u32, u32) = (1, 30);
    pub const LONG: (u32, u32) = (5, 60);
    pub const EVERY: (u32, u32) = (2, 8);

    pub fn minutes(&self, phase: Phase) -> u32 {
        match phase {
            Phase::Focus => self.focus,
            Phase::Break => self.short,
            Phase::LongBreak => self.long,
        }
    }

    fn ms(&self, phase: Phase) -> u64 {
        u64::from(self.minutes(phase)) * 60_000
    }
}

/// What just ended, for the glance that tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// Focus is over; a break of this many minutes has started.
    Focus { break_minutes: u32, long: bool },
    /// The break is over; focus waits for a click.
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    pub phase: Phase,
    /// When the phase ends, while it runs.
    until: Option<u64>,
    /// What is left of it while paused.
    left: u64,
    /// Rounds of focus finished since the last long break.
    pub rounds: u32,
}

impl Timer {
    pub fn new(l: &Lengths) -> Self {
        Self {
            phase: Phase::Focus,
            until: None,
            left: l.ms(Phase::Focus),
            rounds: 0,
        }
    }

    pub fn running(&self) -> bool {
        self.until.is_some()
    }

    /// Running, or paused part way: a session is on.
    pub fn active(&self, l: &Lengths) -> bool {
        self.running() || self.phase != Phase::Focus || self.left != l.ms(Phase::Focus)
    }

    pub fn left_ms(&self, now: u64) -> u64 {
        self.until.map_or(self.left, |u| u.saturating_sub(now))
    }

    /// How much of the phase is done, 0 to 1.
    pub fn progress(&self, now: u64, l: &Lengths) -> f32 {
        let total = l.ms(self.phase).max(1);
        1.0 - self.left_ms(now).min(total) as f32 / total as f32
    }

    /// When the phase ends, while it runs (for the app's one timer).
    pub fn ends_at(&self) -> Option<u64> {
        self.until
    }

    pub fn toggle(&mut self, now: u64) {
        match self.until.take() {
            Some(u) => self.left = u.saturating_sub(now),
            None => self.until = Some(now + self.left),
        }
    }

    /// Back to a fresh round of focus, stopped.
    pub fn reset(&mut self, l: &Lengths) {
        *self = Self::new(l);
    }

    /// Ends the phase now, as if its time were up.
    pub fn skip(&mut self, now: u64, l: &Lengths) -> Ended {
        self.end(now, l)
    }

    /// Moves on if the phase's time is up.
    pub fn finish(&mut self, now: u64, l: &Lengths) -> Option<Ended> {
        match self.until {
            Some(u) if now >= u => Some(self.end(u.max(now.saturating_sub(1000)), l)),
            _ => None,
        }
    }

    fn end(&mut self, at: u64, l: &Lengths) -> Ended {
        match self.phase {
            Phase::Focus => {
                self.rounds += 1;
                let long = self.rounds >= l.every.max(1);
                self.phase = if long { Phase::LongBreak } else { Phase::Break };
                let ms = l.ms(self.phase);
                self.left = ms;
                // A skip while paused leaves the break paused too.
                self.until = self.until.map(|_| at + ms);
                Ended::Focus {
                    break_minutes: l.minutes(self.phase),
                    long,
                }
            }
            Phase::Break | Phase::LongBreak => {
                if self.phase == Phase::LongBreak {
                    self.rounds = 0;
                }
                self.phase = Phase::Focus;
                self.left = l.ms(Phase::Focus);
                self.until = None;
                Ended::Break
            }
        }
    }

    /// New lengths: a stopped, fresh phase takes its new length at once;
    /// one under way keeps its own.
    pub fn set_lengths(&mut self, old: &Lengths, new: &Lengths) {
        if !self.running() && self.left == old.ms(self.phase) {
            self.left = new.ms(self.phase);
        }
    }
}

/// Minutes and seconds left, as the island shows them: `24:59`, `1:05:00`.
pub fn clock(ms: u64) -> String {
    // Rounded up, so it reads 25:00 at the start and 0:00 only at the end.
    let s = ms.div_ceil(1000);
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// A length stepped by `delta` minutes (the wheel moves 5), kept in range
/// and on a multiple of the step.
pub fn step(minutes: u32, delta: i32, (lo, hi): (u32, u32)) -> u32 {
    let step = delta.unsigned_abs().max(1);
    let m = (minutes as i64 + delta as i64).clamp(lo as i64, hi as i64) as u32;
    (m / step * step).clamp(lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    #[test]
    fn runs_pauses_and_resumes_without_drift() {
        let l = Lengths::default();
        let mut t = Timer::new(&l);
        assert!(!t.active(&l));
        assert_eq!(t.left_ms(0), 25 * MIN);
        t.toggle(1000);
        assert!(t.running() && t.active(&l));
        assert_eq!(t.left_ms(1000 + 10 * MIN), 15 * MIN);
        t.toggle(1000 + 10 * MIN);
        // Paused: time passing doesn't count.
        assert_eq!(t.left_ms(1000 + 50 * MIN), 15 * MIN);
        t.toggle(2000 + 50 * MIN);
        assert_eq!(t.ends_at(), Some(2000 + 65 * MIN));
        assert!((t.progress(2000 + 50 * MIN, &l) - 0.4).abs() < 1e-4);
    }

    #[test]
    fn a_break_follows_focus_and_a_long_one_every_four() {
        let l = Lengths::default();
        let mut t = Timer::new(&l);
        let mut now = 0;
        for round in 1..=4 {
            t.toggle(now);
            assert_eq!(t.finish(now + 24 * MIN, &l), None, "not yet");
            now += 25 * MIN;
            let ended = t.finish(now, &l);
            let long = round == 4;
            assert_eq!(
                ended,
                Some(Ended::Focus {
                    break_minutes: if long { 15 } else { 5 },
                    long
                })
            );
            // The break runs by itself.
            assert!(t.running());
            now += l.minutes(t.phase) as u64 * MIN;
            assert_eq!(t.finish(now, &l), Some(Ended::Break));
            // Focus waits for a click.
            assert_eq!(t.phase, Phase::Focus);
            assert!(!t.running());
            assert_eq!(t.left_ms(now), 25 * MIN);
        }
        // After the long break, a new set of rounds.
        assert_eq!(t.rounds, 0);
    }

    #[test]
    fn a_late_wake_does_not_shorten_the_break() {
        let l = Lengths::default();
        let mut t = Timer::new(&l);
        t.toggle(0);
        // The PC slept through the end: the break counts from a second ago.
        t.finish(25 * MIN + 30 * MIN, &l);
        assert_eq!(t.left_ms(55 * MIN), 5 * MIN - 1000);
    }

    #[test]
    fn skip_and_reset() {
        let l = Lengths::default();
        let mut t = Timer::new(&l);
        assert_eq!(
            t.skip(0, &l),
            Ended::Focus {
                break_minutes: 5,
                long: false
            }
        );
        assert_eq!(t.skip(MIN, &l), Ended::Break);
        assert_eq!(t.rounds, 1);
        t.reset(&l);
        assert_eq!(t, Timer::new(&l));
    }

    #[test]
    fn a_skip_keeps_the_timer_running_or_paused() {
        let l = Lengths::default();
        let mut t = Timer::new(&l);
        t.skip(0, &l);
        assert!(!t.running());
        assert_eq!(t.left_ms(MIN), 5 * MIN);
        t.reset(&l);
        t.toggle(0);
        t.skip(MIN, &l);
        assert_eq!(t.left_ms(MIN), 5 * MIN);
        assert!(t.running());
    }

    #[test]
    fn new_lengths_apply_to_a_fresh_phase_only() {
        let old = Lengths::default();
        let new = Lengths { focus: 50, ..old };
        let mut t = Timer::new(&old);
        t.set_lengths(&old, &new);
        assert_eq!(t.left_ms(0), 50 * MIN);
        let mut t = Timer::new(&old);
        t.toggle(0);
        t.set_lengths(&old, &new);
        assert_eq!(t.left_ms(0), 25 * MIN);
    }

    #[test]
    fn clock_and_steps() {
        assert_eq!(clock(25 * MIN), "25:00");
        assert_eq!(clock(25 * MIN - 1), "25:00");
        assert_eq!(clock(65_000), "1:05");
        assert_eq!(clock(0), "0:00");
        assert_eq!(clock(90 * MIN), "1:30:00");
        assert_eq!(step(25, 5, Lengths::FOCUS), 30);
        assert_eq!(step(25, -5, Lengths::FOCUS), 20);
        assert_eq!(step(5, -5, Lengths::FOCUS), 5);
        assert_eq!(step(120, 5, Lengths::FOCUS), 120);
        assert_eq!(step(27, 5, Lengths::FOCUS), 30);
    }
}
