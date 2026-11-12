//! The island's icon: a dark pill with a light rim, seen on light and dark
//! taskbars alike. Drawn from code, for the tray and for the exe's own icon
//! (made into an `.ico` when it is built), so there is no image to keep.

/// One pixel of the pill on an `s`-wide square, as BGRA with straight alpha.
pub fn pill_pixel(x: f32, y: f32, s: f32) -> u32 {
    let r = s * 0.22;
    let half = s * 0.46 - r;
    let (px, py) = (x - s / 2.0, y - s / 2.0);
    let qx = (px.abs() - half).max(0.0);
    // Distance to the pill's edge: negative inside.
    let d = (qx * qx + py * py).sqrt() - r;
    let rim = (s / 16.0).max(1.0);
    let alpha = (0.5 - d).clamp(0.0, 1.0);
    let body = (0.5 - (d + rim)).clamp(0.0, 1.0);
    // Black body, light grey rim.
    let c = (0xD8 as f32 * (1.0 - body)) as u32;
    let a = (alpha * 255.0) as u32;
    (a << 24) | (c << 16) | (c << 8) | c
}

/// The pill at `size`, rows from the top.
pub fn pill(size: u32) -> Vec<u32> {
    let s = size as f32;
    (0..size * size)
        .map(|i| pill_pixel((i % size) as f32 + 0.5, (i / size) as f32 + 0.5, s))
        .collect()
}

/// An `.ico` file with the pill at each of `sizes` (1 to 255), as 32-bit
/// bitmaps.
pub fn ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = sizes.iter().map(|&s| bitmap(s)).collect();
    let mut out = Vec::new();
    // ICONDIR: reserved, type 1 (icon), count.
    for v in [0u16, 1, sizes.len() as u16] {
        out.extend(v.to_le_bytes());
    }
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&s, img) in sizes.iter().zip(&images) {
        // ICONDIRENTRY: width, height, no palette, reserved, planes, bits.
        out.extend([s as u8, s as u8, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((img.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in images {
        out.extend(img);
    }
    out
}

/// One image of the `.ico`: the header, the pixels from the bottom row up,
/// and the (empty) mask the format asks for.
fn bitmap(size: u32) -> Vec<u8> {
    let mut out = Vec::new();
    // BITMAPINFOHEADER; the height counts the mask too.
    out.extend(40u32.to_le_bytes());
    out.extend((size as i32).to_le_bytes());
    out.extend((2 * size as i32).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend([0u8; 24]);
    let pixels = pill(size);
    for row in pixels.chunks(size as usize).rev() {
        for p in row {
            out.extend(p.to_le_bytes());
        }
    }
    // One bit a pixel, rows padded to 4 bytes; the alpha already says it all.
    let mask_row = size.div_ceil(32) * 4;
    out.resize(out.len() + (mask_row * size) as usize, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pill_is_clear_at_the_corners_and_solid_inside() {
        let p = pill(32);
        assert_eq!(p[0] >> 24, 0);
        let middle = p[16 * 32 + 16];
        assert_eq!(middle >> 24, 255);
        // Black body.
        assert_eq!(middle & 0xFF, 0);
    }

    #[test]
    fn the_ico_holds_every_size_where_it_says() {
        let sizes = [16, 32, 48];
        let f = ico(&sizes);
        assert_eq!(&f[..6], &[0, 0, 1, 0, 3, 0]);
        for (i, &s) in sizes.iter().enumerate() {
            let e = &f[6 + 16 * i..6 + 16 * (i + 1)];
            assert_eq!(e[0] as u32, s);
            let len = u32::from_le_bytes(e[8..12].try_into().unwrap()) as usize;
            let at = u32::from_le_bytes(e[12..16].try_into().unwrap()) as usize;
            // 40-byte header, 4 bytes a pixel, then the mask.
            assert_eq!(
                len,
                40 + (s * s * 4) as usize + (s.div_ceil(32) * 4 * s) as usize
            );
            assert_eq!(u32::from_le_bytes(f[at..at + 4].try_into().unwrap()), 40);
            assert!(at + len <= f.len());
        }
    }
}
