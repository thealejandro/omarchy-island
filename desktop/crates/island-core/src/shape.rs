//! The island's silhouette and where it sits on a screen, in pixels.
//!
//! Hanging from the top edge it has concave "ears" on both top corners, so
//! it reads as the edge bulging down (`IslandShape.qml` in the plugin).
//! Floating, it is a rounded rectangle.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// Top center, hanging from the screen edge.
    #[default]
    Top,
    /// Top center, a margin below the edge.
    Floating,
    TopLeft,
    TopRight,
}

impl Placement {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "top" => Some(Self::Top),
            "floating" => Some(Self::Floating),
            "top-left" => Some(Self::TopLeft),
            "top-right" => Some(Self::TopRight),
            _ => None,
        }
    }

    pub fn hangs(self) -> bool {
        self == Self::Top
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

const fn pt(x: f32, y: f32) -> Point {
    Point { x, y }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Line(Point),
    /// A circular arc of `radius` to `to`; `clockwise` on screen (y down).
    Arc {
        to: Point,
        radius: f32,
        clockwise: bool,
    },
}

/// A closed outline: from `start` through `segments`, back to `start`.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    pub start: Point,
    pub segments: Vec<Segment>,
}

/// The body hanging from y = 0. It spans `width + 2 * ear` along x (the ears
/// stick out on both ends) and `height` down; `ear` and `radius` shrink to
/// fit a small body, like the plugin's.
pub fn hanging(width: f32, height: f32, radius: f32, ear: f32) -> Outline {
    let (w, h) = (width.max(0.0), height.max(0.0));
    let e = ear.min(h).min(w / 2.0).max(0.0);
    let r = radius.min(h - e).min(w / 2.0).max(0.0);
    let arc = |x, y, radius, clockwise| Segment::Arc {
        to: pt(x, y),
        radius,
        clockwise,
    };
    Outline {
        start: pt(0.0, 0.0),
        segments: vec![
            arc(e, e, e, true),
            Segment::Line(pt(e, h - r)),
            arc(e + r, h, r, false),
            Segment::Line(pt(e + w - r, h)),
            arc(e + w, h - r, r, false),
            Segment::Line(pt(e + w, e)),
            arc(w + 2.0 * e, 0.0, e, true),
        ],
    }
}

/// A rounded rectangle at (0, 0).
pub fn floating(width: f32, height: f32, radius: f32) -> Outline {
    let (w, h) = (width.max(0.0), height.max(0.0));
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let arc = |x, y| Segment::Arc {
        to: pt(x, y),
        radius: r,
        clockwise: true,
    };
    Outline {
        start: pt(r, 0.0),
        segments: vec![
            Segment::Line(pt(w - r, 0.0)),
            arc(w, r),
            Segment::Line(pt(w, h - r)),
            arc(w - r, h),
            Segment::Line(pt(r, h)),
            arc(0.0, h - r),
            Segment::Line(pt(0.0, r)),
            arc(r, 0.0),
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The extra width the ears add on each side.
pub fn ear_room(p: Placement, ear: f32) -> f32 {
    if p.hangs() { ear } else { 0.0 }
}

/// Where the window goes on a `monitor`, sized for the biggest body
/// (`max_width` x `max_height`, without ears).
pub fn window_rect(
    p: Placement,
    monitor: Rect,
    max_width: f32,
    max_height: f32,
    ear: f32,
    margin: f32,
) -> Rect {
    let width = max_width + 2.0 * ear_room(p, ear);
    let centered = monitor.x + ((monitor.width - width) / 2.0).round();
    let (x, y) = match p {
        Placement::Top => (centered, monitor.y),
        Placement::Floating => (centered, monitor.y + margin),
        Placement::TopLeft => (monitor.x + margin, monitor.y + margin),
        Placement::TopRight => (
            monitor.x + monitor.width - margin - width,
            monitor.y + margin,
        ),
    };
    Rect {
        x,
        y,
        width,
        height: max_height,
    }
}

/// Where a body of `width` x `height` (without ears) sits in a window of
/// `window_width`, ears included: centered, or against its corner.
pub fn body_rect(p: Placement, window_width: f32, width: f32, height: f32, ear: f32) -> Rect {
    let full = width + 2.0 * ear_room(p, ear);
    let x = match p {
        Placement::TopLeft => 0.0,
        Placement::TopRight => window_width - full,
        _ => ((window_width - full) / 2.0).round(),
    };
    Rect {
        x,
        y: 0.0,
        width: full,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end(o: &Outline) -> Point {
        match o.segments.last() {
            Some(Segment::Line(p)) | Some(Segment::Arc { to: p, .. }) => *p,
            None => o.start,
        }
    }

    #[test]
    fn hanging_spans_the_ears_and_ends_on_the_edge() {
        let o = hanging(200.0, 32.0, 18.0, 10.0);
        assert_eq!(end(&o), pt(220.0, 0.0));
        // The bottom corners leave room for the ears: r <= h - e.
        assert_eq!(
            o.segments[2],
            Segment::Arc {
                to: pt(28.0, 32.0),
                radius: 18.0,
                clockwise: false
            }
        );
    }

    #[test]
    fn hanging_shrinks_ears_and_corners_on_a_tiny_body() {
        let o = hanging(6.0, 4.0, 18.0, 10.0);
        // ear <= w / 2 = 3, radius <= h - ear = 1
        assert_eq!(
            o.segments[0],
            Segment::Arc {
                to: pt(3.0, 3.0),
                radius: 3.0,
                clockwise: true
            }
        );
        assert_eq!(
            o.segments[2],
            Segment::Arc {
                to: pt(4.0, 4.0),
                radius: 1.0,
                clockwise: false
            }
        );
        // A spring that dips below zero gives an empty shape, not a broken one.
        assert_eq!(end(&hanging(-3.0, -1.0, 18.0, 10.0)), pt(0.0, 0.0));
    }

    #[test]
    fn floating_is_a_closed_pill_when_short() {
        let o = floating(180.0, 32.0, 18.0);
        assert_eq!(o.start, pt(16.0, 0.0));
        assert_eq!(end(&o), o.start);
    }

    #[test]
    fn windows_and_bodies_by_placement() {
        let mon = Rect {
            x: -1920.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        };
        let top = window_rect(Placement::Top, mon, 560.0, 180.0, 10.0, 8.0);
        assert_eq!(
            top,
            Rect {
                x: -1920.0 + 670.0,
                y: 0.0,
                width: 580.0,
                height: 180.0
            }
        );
        assert_eq!(
            body_rect(Placement::Top, 580.0, 180.0, 32.0, 10.0),
            Rect {
                x: 190.0,
                y: 0.0,
                width: 200.0,
                height: 32.0
            }
        );
        let right = window_rect(Placement::TopRight, mon, 560.0, 180.0, 10.0, 8.0);
        assert_eq!((right.x, right.y, right.width), (-8.0 - 560.0, 8.0, 560.0));
        assert_eq!(
            body_rect(Placement::TopRight, 560.0, 180.0, 32.0, 10.0).x,
            380.0
        );
        assert_eq!(
            body_rect(Placement::TopLeft, 560.0, 180.0, 32.0, 10.0).x,
            0.0
        );
        assert_eq!(
            window_rect(Placement::Floating, mon, 560.0, 180.0, 10.0, 8.0).y,
            8.0
        );
    }

    #[test]
    fn placement_names() {
        for (s, p) in [
            ("top", Placement::Top),
            ("floating", Placement::Floating),
            ("top-left", Placement::TopLeft),
            ("top-right", Placement::TopRight),
        ] {
            assert_eq!(Placement::parse(s), Some(p));
        }
        assert_eq!(Placement::parse("bottom"), None);
    }
}
