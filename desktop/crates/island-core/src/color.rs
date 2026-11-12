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

/// `#rrggbb` or `#rgb` (spaces around are fine) as 0 to 1.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let hex = s.trim().strip_prefix('#')?;
    // Checked first, so the slicing below never splits a character.
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let digit = |i: usize, n: usize| u8::from_str_radix(&hex[i..i + n], 16).ok();
    let rgb = match hex.len() {
        6 => [digit(0, 2)?, digit(2, 2)?, digit(4, 2)?],
        3 => [digit(0, 1)? * 17, digit(1, 1)? * 17, digit(2, 1)? * 17],
        _ => return None,
    };
    Some(rgb.map(|v| f32::from(v) / 255.0))
}

/// The contrast white text needs on the island (WCAG 1.4.3).
pub const TEXT_LEGIBLE: f32 = 4.5;

/// `c` as is when white text reads on it, else mixed toward black just
/// enough that it does (a light theme's background would hide the text).
pub fn under_white_text(c: Rgb) -> Rgb {
    let contrast = |c: Rgb| 1.05 / (luminance(c) + 0.05);
    if contrast(c) >= TEXT_LEGIBLE {
        return c;
    }
    let toward_black = |k: f32| c.map(|v| v * (1.0 - k));
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..20 {
        let k = (lo + hi) / 2.0;
        if contrast(toward_black(k)) >= TEXT_LEGIBLE {
            hi = k;
        } else {
            lo = k;
        }
    }
    toward_black(hi)
}

/// WCAG's relative luminance of an sRGB color.
fn luminance(c: Rgb) -> f32 {
    let lin = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

/// The contrast between two colors, 1 to 21.
pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// `c`'s contrast against the island's black.
pub fn contrast_on_black(c: Rgb) -> f32 {
    contrast(c, [0.0; 3])
}

/// The least contrast a bar or ring needs to be seen (WCAG 1.4.11).
pub const LEGIBLE: f32 = 3.0;

/// `c` as is when it reads on the island's `body`, else mixed toward white
/// just enough that it does (a dark color chosen by hand, or the system's
/// on a lighter body, would hardly show). A body dark enough for the white
/// text always leaves room for it.
pub fn legible_on(c: Rgb, body: Rgb) -> Rgb {
    if contrast(c, body) >= LEGIBLE {
        return c;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..20 {
        let k = (lo + hi) / 2.0;
        let m = toward_white(c, k);
        if contrast(m, body) >= LEGIBLE && luminance(m) >= luminance(body) {
            hi = k;
        } else {
            lo = k;
        }
    }
    toward_white(c, hi)
}

/// [`legible_on`] the island's black.
pub fn legible_on_black(c: Rgb) -> Rgb {
    legible_on(c, [0.0; 3])
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
    fn dark_colors_are_lifted_to_read_on_black() {
        for dark in [[0.125; 3], [0.0, 0.0, 0.4], [0.0; 3], [0.3, 0.0, 0.0]] {
            let c = legible_on_black(dark);
            assert!(contrast_on_black(c) >= LEGIBLE, "{dark:?}");
            // Only as much as needed.
            assert!(contrast_on_black(c) < LEGIBLE + 0.05, "{dark:?}");
        }
        // Bright ones stay as chosen.
        assert_eq!(legible_on_black([1.0, 0.5, 0.0]), [1.0, 0.5, 0.0]);
        assert_eq!(legible_on_black(WHITE), WHITE);
        // On the lightest body the white text allows, it still shows.
        let body = under_white_text([0.93, 0.97, 0.99]);
        let c = legible_on(legible_on_black([0.0, 0.2, 0.6]), body);
        assert!(contrast(c, body) >= LEGIBLE);
        assert!(luminance(c) > luminance(body));
    }

    #[test]
    fn light_bodies_are_darkened_under_white_text() {
        // A light theme's background (Aero's #eef8fc).
        let c = under_white_text([0.933, 0.973, 0.988]);
        let on = |c: Rgb| 1.05 / (luminance(c) + 0.05);
        assert!(on(c) >= TEXT_LEGIBLE && on(c) < TEXT_LEGIBLE + 0.05);
        // Its hue is kept: still bluish.
        assert!(c[2] > c[0]);
        let dark = [0.1, 0.12, 0.2];
        assert_eq!(under_white_text(dark), dark);
        assert_eq!(under_white_text([0.0; 3]), [0.0; 3]);
    }

    #[test]
    fn hex_colors() {
        assert_eq!(parse_hex(" #ff0000 "), Some([1.0, 0.0, 0.0]));
        assert_eq!(parse_hex("#fff"), Some(WHITE));
        for bad in ["ff0000", "#12345", "#ggg", "#é12", ""] {
            assert_eq!(parse_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn ink_reads_on_the_color() {
        assert_eq!(ink_on(WHITE), [0.0; 3]);
        assert_eq!(ink_on([1.0, 0.9, 0.3]), [0.0; 3]);
        assert_eq!(ink_on([0.25, 0.2, 1.0]), WHITE);
        assert!(close(toward_white([0.0, 0.5, 1.0], 0.5), [0.5, 0.75, 1.0]));
    }
}
