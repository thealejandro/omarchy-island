//! Colors for the black island: a cover's or the system's color made vivid
//! enough to fill a bar or a button, and what reads on top of it.

pub type Rgb = [f32; 3];

pub const WHITE: Rgb = [1.0; 3];

/// Saturation (HSV) under which a color is taken as gray.
const GRAY: f32 = 0.12;

/// `c` (0 to 1) as HSV: hue in 0..6, saturation, value.
fn hsv([r, g, b]: Rgb) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let s = if max > 0.0 { d / max } else { 0.0 };
    let h = if d <= 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h, s, max)
}

fn rgb(h: f32, s: f32, v: f32) -> Rgb {
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

/// Whether `c` has no hue to speak of.
pub fn is_gray(c: Rgb) -> bool {
    hsv(c).1 < GRAY
}

/// `c`'s hue, saturated and bright enough to fill a bar or a button on
/// black and still be its color; a gray stays white.
pub fn vivid(c: Rgb) -> Rgb {
    let (h, s, _) = hsv(c);
    if s < GRAY {
        return WHITE;
    }
    rgb(h, s.clamp(0.55, 0.8), 1.0)
}

/// `c` mixed `k` (0 to 1) toward white.
pub fn toward_white(c: Rgb, k: f32) -> Rgb {
    c.map(|v| v + (1.0 - v) * k)
}

/// Black or white, whichever reads better on `c`.
pub fn ink_on([r, g, b]: Rgb) -> Rgb {
    // Relative luminance, near enough (the colors here are bright).
    let l = 0.2126 * r * r + 0.7152 * g * g + 0.0722 * b * b;
    if l > 0.3 { [0.0; 3] } else { WHITE }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Rgb, b: Rgb) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.02)
    }

    #[test]
    fn vivid_keeps_the_hue_and_lights_it() {
        // A dull pink cover: still pink, at full brightness.
        let v = vivid([0.6, 0.2, 0.4]);
        assert_eq!(v[0], 1.0);
        assert!(v[1] < v[2] && v[2] < v[0]);
        assert!(close(vivid([0.0, 0.0, 0.5]), [0.2, 0.2, 1.0]));
        // A washed-out blue gets some saturation back.
        let (_, s, _) = hsv(vivid([0.8, 0.8, 0.95]));
        assert!(s >= 0.55);
    }

    #[test]
    fn grays_are_white() {
        assert!(is_gray([0.85; 3]));
        assert_eq!(vivid([0.85; 3]), WHITE);
        assert_eq!(vivid([0.0; 3]), WHITE);
        assert!(!is_gray([1.0, 0.4, 0.6]));
    }

    #[test]
    fn ink_reads_on_the_color() {
        assert_eq!(ink_on(WHITE), [0.0; 3]);
        assert_eq!(ink_on([1.0, 0.9, 0.3]), [0.0; 3]);
        assert_eq!(ink_on([0.25, 0.2, 1.0]), WHITE);
        assert!(close(toward_white([0.0, 0.5, 1.0], 0.5), [0.5, 0.75, 1.0]));
    }
}
