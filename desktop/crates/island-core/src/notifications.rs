//! Notifications as they arrive: the system's list read again when a new
//! one shows, and only those not seen before told. The list read at start
//! is what was there already: no news.

use crate::shape::Rect;

/// Segoe Fluent Icons (and MDL2): the notification bell.
pub const ICON: crate::icons::Icon = crate::icons::Icon::Bell;

/// A notification's glance: bigger than the song's, so it never passes for
/// it, with room for the app's logo and two lines (DIPs).
pub const SIZE: (f32, f32) = (420.0, 76.0);

/// How long the ring around the island pulses as one arrives, in ms.
pub const PULSE_MS: f32 = 1800.0;

/// How bright the ring in the app's color is, `ms` after it arrived: three
/// beats, then a steady glow while the glance stays.
pub fn ring(ms: f32) -> f32 {
    const REST: f32 = 0.3;
    if !(0.0..PULSE_MS).contains(&ms) {
        return REST;
    }
    let beat = (std::f32::consts::PI * ms / (PULSE_MS / 3.0)).sin();
    REST + (1.0 - REST) * beat * beat
}

/// Where a notification's glance puts things: the logo at the left; the
/// app and "now" on top, then the title and, if there is one, the body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToastLayout {
    pub logo: Rect,
    pub app: Rect,
    pub when: Rect,
    pub title: Rect,
    pub body: Rect,
}

pub fn layout((w, h): (f32, f32), has_body: bool) -> ToastLayout {
    let (pad, side) = (16.0, 44.0);
    let logo = Rect {
        x: pad,
        y: (h - side) / 2.0,
        width: side,
        height: side,
    };
    let x = pad + side + 12.0;
    let right = w - 18.0;
    // The lines, centered as a block.
    let (app_h, title_h) = (16.0, 20.0);
    let body_h = if has_body { 18.0 } else { 0.0 };
    let top = (h - app_h - title_h - body_h) / 2.0;
    let when = Rect {
        x: right - 64.0,
        y: top,
        width: 64.0,
        height: app_h,
    };
    let app = Rect {
        x,
        y: top,
        width: when.x - 8.0 - x,
        height: app_h,
    };
    let title = Rect {
        x,
        y: top + app_h,
        width: right - x,
        height: title_h,
    };
    let body = Rect {
        x,
        y: title.y + title_h,
        width: right - x,
        height: body_h,
    };
    ToastLayout {
        logo,
        app,
        when,
        title,
        body,
    }
}

/// A notification, as far as a glance tells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub id: u32,
    /// The app that sent it.
    pub app: String,
    /// Its title (its first line of text).
    pub title: String,
    /// Its second line, if any.
    pub body: String,
}

impl Toast {
    /// One line of text, whatever the app sent (line breaks, runs of spaces).
    pub fn line(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// The notifications seen, by id.
#[derive(Debug, Default)]
pub struct Seen {
    ids: Vec<u32>,
    primed: bool,
}

impl Seen {
    /// The list now; the ones in it not seen before, oldest first (none the
    /// first time). The ones gone are forgotten.
    pub fn new_ones(&mut self, now: Vec<Toast>) -> Vec<Toast> {
        let ids: Vec<u32> = now.iter().map(|t| t.id).collect();
        let fresh = if self.primed {
            now.into_iter()
                .filter(|t| !self.ids.contains(&t.id))
                .collect()
        } else {
            Vec::new()
        };
        self.ids = ids;
        self.primed = true;
        fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u32) -> Toast {
        Toast {
            id,
            app: "Mail".into(),
            title: format!("n{id}"),
            body: String::new(),
        }
    }

    #[test]
    fn only_new_ones_are_told() {
        let mut s = Seen::default();
        assert_eq!(s.new_ones(vec![t(1), t(2)]), vec![]);
        assert_eq!(s.new_ones(vec![t(1), t(2), t(3)]), vec![t(3)]);
        // Read again with nothing new (another window showed): nothing.
        assert_eq!(s.new_ones(vec![t(1), t(2), t(3)]), vec![]);
        // One dismissed, two new.
        assert_eq!(s.new_ones(vec![t(2), t(3), t(4), t(5)]), vec![t(4), t(5)]);
        // A dismissed one is forgotten; its id coming back is new.
        assert_eq!(s.new_ones(vec![t(1)]), vec![t(1)]);
    }

    #[test]
    fn the_ring_beats_then_rests() {
        assert_eq!(ring(0.0), ring(PULSE_MS));
        assert!(ring(PULSE_MS / 6.0) > 0.99);
        assert!(ring(PULSE_MS / 3.0) < 0.31);
        assert_eq!(ring(10_000.0), ring(PULSE_MS));
        // No jump where the beats end.
        assert!((ring(PULSE_MS - 1.0) - ring(PULSE_MS)).abs() < 0.01);
    }

    #[test]
    fn the_glance_fits_logo_and_lines() {
        for has_body in [true, false] {
            let l = layout(SIZE, has_body);
            let inside = |r: Rect| {
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.width <= SIZE.0 && r.y + r.height <= SIZE.1
            };
            for r in [l.logo, l.app, l.when, l.title, l.body] {
                assert!(inside(r), "{r:?}");
            }
            assert!(l.logo.x + l.logo.width < l.app.x);
            assert!(l.app.x + l.app.width < l.when.x);
            assert!(l.app.y + l.app.height <= l.title.y);
            assert!(l.title.y + l.title.height <= l.body.y);
        }
        assert_eq!(layout(SIZE, false).body.height, 0.0);
        // Taller than the song's glance, so it never passes for it.
        const { assert!(SIZE.1 > crate::media::PEEK.1 + 20.0) };
    }

    #[test]
    fn text_is_one_line() {
        assert_eq!(Toast::line("  Hola\n  mundo\t !"), "Hola mundo !");
        assert_eq!(Toast::line("\n"), "");
    }
}
