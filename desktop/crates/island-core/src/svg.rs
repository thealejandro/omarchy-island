//! SVG path data (the `d` attribute) read into absolute commands a 2D
//! API can draw: moves, lines, cubic curves, arcs and closes. Quadratic
//! curves become cubic ones and the shorthand forms are made explicit, so
//! a drawer only needs those five.

use crate::shape::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cmd {
    /// Starts a figure.
    Move(Point),
    Line(Point),
    /// Two control points, then the end.
    Cubic(Point, Point, Point),
    /// As SVG has it: radii, x-axis rotation (degrees), the flags and the
    /// end.
    Arc {
        radius: (f32, f32),
        rotation: f32,
        large: bool,
        sweep: bool,
        to: Point,
    },
    Close,
}

struct Reader<'a> {
    s: &'a [u8],
    i: usize,
}

impl Reader<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_whitespace() || self.s[self.i] == b',')
        {
            self.i += 1;
        }
    }

    /// A command letter next, if there is one.
    fn letter(&mut self) -> Option<u8> {
        self.skip();
        let c = *self.s.get(self.i)?;
        if c.is_ascii_alphabetic() && c != b'e' && c != b'E' {
            self.i += 1;
            Some(c)
        } else {
            None
        }
    }

    /// A number next (without consuming a command letter).
    fn has_number(&mut self) -> bool {
        self.skip();
        self.s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }

    fn number(&mut self) -> Option<f32> {
        self.skip();
        let start = self.i;
        let s = self.s;
        if matches!(s.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        let mut dot = false;
        while let Some(&c) = s.get(self.i) {
            if c.is_ascii_digit() {
                self.i += 1;
            } else if c == b'.' && !dot {
                dot = true;
                self.i += 1;
            } else {
                break;
            }
        }
        if matches!(s.get(self.i), Some(b'e' | b'E')) {
            let back = self.i;
            self.i += 1;
            if matches!(s.get(self.i), Some(b'-' | b'+')) {
                self.i += 1;
            }
            let digits = self.i;
            while s.get(self.i).is_some_and(|c| c.is_ascii_digit()) {
                self.i += 1;
            }
            if self.i == digits {
                self.i = back;
            }
        }
        std::str::from_utf8(&s[start..self.i]).ok()?.parse().ok()
    }

    /// An arc's flag: a single 0 or 1, which may run into what follows.
    fn flag(&mut self) -> Option<bool> {
        self.skip();
        let c = *self.s.get(self.i)?;
        self.i += 1;
        match c {
            b'0' => Some(false),
            b'1' => Some(true),
            _ => None,
        }
    }

    fn point(&mut self) -> Option<Point> {
        Some(Point {
            x: self.number()?,
            y: self.number()?,
        })
    }
}

fn add(a: Point, b: Point) -> Point {
    Point {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

fn mirror(p: Point, about: Point) -> Point {
    Point {
        x: 2.0 * about.x - p.x,
        y: 2.0 * about.y - p.y,
    }
}

/// `d` as absolute commands; `None` if it isn't valid path data.
pub fn parse(d: &str) -> Option<Vec<Cmd>> {
    let mut r = Reader {
        s: d.as_bytes(),
        i: 0,
    };
    let mut out = Vec::new();
    let origin = Point { x: 0.0, y: 0.0 };
    let (mut at, mut start) = (origin, origin);
    // The last control point, for the smooth forms (S and T).
    let mut last_cubic: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut cmd = r.letter()?;
    loop {
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { at } else { origin };
        let pt = |r: &mut Reader| r.point().map(|p| add(p, base));
        let (mut cubic, mut quad) = (None, None);
        match cmd.to_ascii_uppercase() {
            b'M' => {
                at = pt(&mut r)?;
                start = at;
                out.push(Cmd::Move(at));
                // More pairs after a move are lines.
                cmd = if rel { b'l' } else { b'L' };
            }
            b'L' => {
                at = pt(&mut r)?;
                out.push(Cmd::Line(at));
            }
            b'H' => {
                let x = r.number()? + if rel { at.x } else { 0.0 };
                at = Point { x, ..at };
                out.push(Cmd::Line(at));
            }
            b'V' => {
                let y = r.number()? + if rel { at.y } else { 0.0 };
                at = Point { y, ..at };
                out.push(Cmd::Line(at));
            }
            b'C' => {
                let (c1, c2, to) = (pt(&mut r)?, pt(&mut r)?, pt(&mut r)?);
                out.push(Cmd::Cubic(c1, c2, to));
                (cubic, at) = (Some(c2), to);
            }
            b'S' => {
                let c1 = last_cubic.map_or(at, |c| mirror(c, at));
                let (c2, to) = (pt(&mut r)?, pt(&mut r)?);
                out.push(Cmd::Cubic(c1, c2, to));
                (cubic, at) = (Some(c2), to);
            }
            b'Q' | b'T' => {
                let q = if cmd.eq_ignore_ascii_case(&b'Q') {
                    pt(&mut r)?
                } else {
                    last_quad.map_or(at, |c| mirror(c, at))
                };
                let to = pt(&mut r)?;
                // The same curve as a cubic one.
                let k = 2.0 / 3.0;
                let c1 = Point {
                    x: at.x + k * (q.x - at.x),
                    y: at.y + k * (q.y - at.y),
                };
                let c2 = Point {
                    x: to.x + k * (q.x - to.x),
                    y: to.y + k * (q.y - to.y),
                };
                out.push(Cmd::Cubic(c1, c2, to));
                (quad, at) = (Some(q), to);
            }
            b'A' => {
                let radius = (r.number()?.abs(), r.number()?.abs());
                let rotation = r.number()?;
                let (large, sweep) = (r.flag()?, r.flag()?);
                let to = pt(&mut r)?;
                out.push(Cmd::Arc {
                    radius,
                    rotation,
                    large,
                    sweep,
                    to,
                });
                at = to;
            }
            b'Z' => {
                out.push(Cmd::Close);
                at = start;
            }
            _ => return None,
        }
        (last_cubic, last_quad) = (cubic, quad);
        // The same command again while numbers follow (not after a close).
        if !cmd.eq_ignore_ascii_case(&b'Z') && r.has_number() {
            continue;
        }
        match r.letter() {
            Some(c) => cmd = c,
            None => {
                r.skip();
                return (r.i == r.s.len()).then_some(out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    #[test]
    fn relative_and_shorthand_commands_become_absolute() {
        let d = parse("M2 3h4v-1l1,1z m1 1 2 2").unwrap();
        assert_eq!(
            d,
            vec![
                Cmd::Move(p(2.0, 3.0)),
                Cmd::Line(p(6.0, 3.0)),
                Cmd::Line(p(6.0, 2.0)),
                Cmd::Line(p(7.0, 3.0)),
                Cmd::Close,
                // After a close, relative to the figure's start.
                Cmd::Move(p(3.0, 4.0)),
                // Pairs after a move are lines.
                Cmd::Line(p(5.0, 6.0)),
            ]
        );
    }

    #[test]
    fn numbers_run_together_as_svg_allows() {
        let d = parse("M.5.5L-1-2e1").unwrap();
        assert_eq!(d, vec![Cmd::Move(p(0.5, 0.5)), Cmd::Line(p(-1.0, -20.0))]);
    }

    #[test]
    fn arcs_read_packed_flags() {
        let d = parse("M0 0a2 2 0 011 1").unwrap();
        assert_eq!(
            d[1],
            Cmd::Arc {
                radius: (2.0, 2.0),
                rotation: 0.0,
                large: false,
                sweep: true,
                to: p(1.0, 1.0),
            }
        );
    }

    #[test]
    fn smooth_and_quadratic_curves_become_cubic() {
        let d = parse("M0 0C1 0 2 1 2 2S3 4 4 4Q6 4 6 6").unwrap();
        // S mirrors the last control point about the current one.
        assert_eq!(d[2], Cmd::Cubic(p(2.0, 3.0), p(3.0, 4.0), p(4.0, 4.0)));
        let Cmd::Cubic(c1, c2, to) = d[3] else {
            panic!()
        };
        assert_eq!(to, p(6.0, 6.0));
        assert!((c1.x - 5.333).abs() < 0.01 && (c1.y - 4.0).abs() < 0.01);
        assert!((c2.x - 6.0).abs() < 0.01 && (c2.y - 4.667).abs() < 0.01);
    }

    #[test]
    fn bad_data_is_none() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("L1 1"), Some(vec![Cmd::Line(p(1.0, 1.0))]));
        assert_eq!(parse("M1"), None);
        assert_eq!(parse("M1 1 X"), None);
    }
}
