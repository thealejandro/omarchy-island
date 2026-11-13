//! Springs for the island's motion, stepped like Qt's `SpringAnimation`
//! (fixed 16 ms steps), so the desktop apps move like the Omarchy plugin.
//! A spring keeps its velocity when the target changes mid-flight: a quick
//! in-and-out of the pointer bends the motion instead of restarting it.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub spring: f32,
    pub damping: f32,
    pub mass: f32,
    /// Below this distance and speed the spring snaps to its target.
    pub epsilon: f32,
}

// Opening is loose and settles with a little wobble (the window leaves room
// for it, see `ROOM_WIDTH`); closing is stiffer and never overshoots, and a
// glance grows firm, without a wobble. Width is a touch softer than height
// so the body stretches before it widens.
pub const OPEN_WIDTH: Params = Params {
    spring: 4.4,
    damping: 0.41,
    mass: 1.0,
    epsilon: 0.25,
};
pub const OPEN_HEIGHT: Params = Params {
    spring: 5.2,
    damping: 0.40,
    mass: 1.0,
    epsilon: 0.25,
};
pub const CLOSE_WIDTH: Params = Params {
    spring: 6.4,
    damping: 0.61,
    mass: 1.0,
    epsilon: 0.25,
};
pub const CLOSE_HEIGHT: Params = Params {
    spring: 7.7,
    damping: 0.70,
    mass: 1.0,
    epsilon: 0.25,
};
/// Growing into a glance (not open), both ways.
pub const GLANCE: Params = Params {
    spring: 7.7,
    damping: 0.63,
    mass: 1.0,
    epsilon: 0.25,
};

/// Room the window keeps around the open body for its wobble, in DIPs: on
/// each side, and below. The widest open (960) overshoots by under 5 a side;
/// the tallest by under 8.
pub const ROOM_WIDTH: f32 = 16.0;
pub const ROOM_HEIGHT: f32 = 12.0;

/// The step the parameters are tuned for (Qt's).
const STEP_MS: f32 = 16.0;
/// The step actually taken: finer, so a frame of any length moves the
/// spring by just that time, and every frame moves (16 ms steps on a
/// 15.6 ms timer skipped one now and then, a visible hitch).
const SUBSTEP_MS: f32 = 4.0;
/// DIPs a second under which the motion is invisible (a tenth of a pixel a
/// frame): the spring stops there instead of redrawing for nothing.
const SETTLE_SPEED: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
}

impl Spring {
    pub fn at(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
            target: value,
        }
    }

    pub fn is_settled(&self) -> bool {
        self.value == self.target && self.velocity == 0.0
    }

    /// Advances by `elapsed_ms`; `true` while it is still moving.
    pub fn advance(&mut self, p: &Params, elapsed_ms: f32) -> bool {
        if self.is_settled() {
            return false;
        }
        // A long stall (a sleeping laptop, a busy thread) doesn't replay
        // seconds of motion: the spring just finishes.
        if elapsed_ms > 1000.0 {
            self.snap();
            return false;
        }
        let mut left = elapsed_ms.max(0.0);
        while left > 0.0 {
            let dt = left.min(SUBSTEP_MS);
            left -= dt;
            let diff = self.target - self.value;
            if diff.abs() < p.epsilon && self.velocity.abs() < SETTLE_SPEED.max(p.epsilon) {
                self.snap();
                return false;
            }
            // Qt's step, scaled to this one's share of it.
            self.velocity += (p.spring * diff - p.damping * self.velocity) / p.mass * dt / STEP_MS;
            self.value += self.velocity * dt / 1000.0;
        }
        true
    }

    fn snap(&mut self) {
        self.value = self.target;
        self.velocity = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The open island's height, as in `pages::OPEN_HEIGHT`.
    const OPEN_H: f32 = crate::pages::OPEN_HEIGHT;

    fn run(s: &mut Spring, p: &Params) -> (u32, f32) {
        let mut peak = s.value;
        for frame in 0..2000 {
            if !s.advance(p, 16.0) {
                return (frame, peak);
            }
            peak = peak.max(s.value);
        }
        panic!("never settled");
    }

    #[test]
    fn opening_overshoots_a_little_and_settles() {
        let mut s = Spring::at(32.0);
        s.target = 180.0;
        let (frames, peak) = run(&mut s, &OPEN_HEIGHT);
        assert_eq!(s.value, 180.0);
        assert!(peak > 180.0 && peak < 200.0, "peak {peak}");
        assert!(frames < 120, "{frames} frames");
    }

    #[test]
    fn it_stops_once_the_motion_is_invisible() {
        // Open and close at 60 Hz: done in about half a second each (the
        // opening's wobble included), not drawing sub-pixel steps for most
        // of a second.
        for (from, to, p) in [
            (32.0, 180.0, &OPEN_HEIGHT),
            (180.0, 32.0, &CLOSE_HEIGHT),
            (180.0, 560.0, &OPEN_WIDTH),
            (560.0, 180.0, &CLOSE_WIDTH),
        ] {
            let mut s = Spring::at(from);
            s.target = to;
            let mut frames = 0;
            while s.advance(p, 16.7) {
                frames += 1;
            }
            assert!(frames <= 36, "{from} to {to}: {frames} frames");
        }
    }

    #[test]
    fn closing_barely_overshoots() {
        let mut s = Spring::at(180.0);
        s.target = 32.0;
        let mut low = s.value;
        while s.advance(&CLOSE_HEIGHT, 16.0) {
            low = low.min(s.value);
        }
        assert_eq!(s.value, 32.0);
        assert!(low > 31.0, "low {low}");
    }

    #[test]
    fn the_wobble_fits_the_room() {
        for (from, to, p, room) in [
            (32.0, OPEN_H, &OPEN_HEIGHT, ROOM_HEIGHT),
            (180.0, 960.0, &OPEN_WIDTH, 2.0 * ROOM_WIDTH),
            (180.0, 560.0, &GLANCE, 0.0),
            (32.0, 64.0, &GLANCE, 0.0),
        ] {
            let mut s = Spring::at(from);
            s.target = to;
            let mut peak = s.value;
            while s.advance(p, 16.7) {
                peak = peak.max(s.value);
            }
            assert!(peak <= to + room, "{from} to {to}: peak {peak}");
        }
    }

    #[test]
    fn shrinking_never_dips_under() {
        for (from, to, p) in [
            (208.0, 32.0, &CLOSE_HEIGHT),
            (960.0, 180.0, &CLOSE_WIDTH),
            (64.0, 32.0, &GLANCE),
        ] {
            let mut s = Spring::at(from);
            s.target = to;
            let mut low = s.value;
            while s.advance(p, 16.7) {
                low = low.min(s.value);
            }
            assert!(low >= to - 0.01, "{from} to {to}: low {low}");
        }
    }

    #[test]
    fn a_settled_spring_does_nothing() {
        let mut s = Spring::at(5.0);
        assert!(!s.advance(&OPEN_WIDTH, 16.0));
        assert_eq!(s, Spring::at(5.0));
    }

    #[test]
    fn short_frames_add_up_and_long_stalls_finish() {
        let mut a = Spring::at(0.0);
        a.target = 100.0;
        let mut b = a;
        for _ in 0..4 {
            a.advance(&OPEN_WIDTH, 4.0);
        }
        b.advance(&OPEN_WIDTH, 16.0);
        assert_eq!(a.value, b.value);
        b.advance(&OPEN_WIDTH, 5000.0);
        assert!(b.is_settled() && b.value == 100.0);
    }
}
