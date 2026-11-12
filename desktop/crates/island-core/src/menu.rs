//! The island's own menu (tray icon, right click): a dark grey rounded panel
//! like the island, with switches. Sizes in DIPs.

use crate::shape::Rect;

pub const WIDTH: f32 = 360.0;
/// Around the rows, inside the panel.
pub const PAD: f32 = 10.0;
/// The name and version on top.
pub const HEADER: f32 = 52.0;
pub const ROW: f32 = 46.0;
pub const SEPARATOR: f32 = 13.0;
pub const RADIUS: f32 = 14.0;
/// The highlight under the pointer.
pub const ROW_RADIUS: f32 = 10.0;
/// Where a row's words start.
pub const TEXT_X: f32 = 22.0;
const SWITCH: (f32, f32) = (46.0, 26.0);
const KNOB_INSET: f32 = 3.0;
/// Between the menu and the edges of the screen's work area.
pub const SCREEN_MARGIN: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A setting with a switch: choosing it flips the switch, the menu stays.
    Toggle,
    /// Does something and closes the menu.
    Action,
    Separator,
}

/// Each entry's place, top to bottom, under the header.
pub fn rows(kinds: &[Kind]) -> Vec<Rect> {
    let mut y = PAD + HEADER;
    kinds
        .iter()
        .map(|k| {
            let h = if *k == Kind::Separator {
                SEPARATOR
            } else {
                ROW
            };
            let r = Rect {
                x: PAD,
                y,
                width: WIDTH - 2.0 * PAD,
                height: h,
            };
            y += h;
            r
        })
        .collect()
}

pub fn height(kinds: &[Kind]) -> f32 {
    rows(kinds).last().map_or(PAD + HEADER, |r| r.y + r.height) + PAD
}

/// The entry under (x, y), separators left out.
pub fn hit(kinds: &[Kind], x: f32, y: f32) -> Option<usize> {
    rows(kinds).iter().zip(kinds).position(|(r, k)| {
        *k != Kind::Separator && x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
    })
}

/// A toggle row's switch, at its right end.
pub fn switch(row: Rect) -> Rect {
    Rect {
        x: row.x + row.width - 14.0 - SWITCH.0,
        y: row.y + (row.height - SWITCH.1) / 2.0,
        width: SWITCH.0,
        height: SWITCH.1,
    }
}

/// The switch's knob: on the right when on. `t` from 0 (off) to 1 (on).
pub fn knob(switch: Rect, t: f32) -> Rect {
    let d = switch.height - 2.0 * KNOB_INSET;
    let travel = switch.width - 2.0 * KNOB_INSET - d;
    Rect {
        x: switch.x + KNOB_INSET + travel * t.clamp(0.0, 1.0),
        y: switch.y + KNOB_INSET,
        width: d,
        height: d,
    }
}

/// Where the menu goes, in px: from the pointer down and to the right,
/// flipped up or left where it wouldn't fit, and kept inside `work` (the
/// screen without the taskbar).
pub fn place(pointer: (f32, f32), size: (f32, f32), work: Rect, margin: f32) -> (f32, f32) {
    let (px, py) = pointer;
    let (w, h) = size;
    let x = if px + w <= work.x + work.width - margin {
        px
    } else {
        px - w
    };
    let y = if py + h <= work.y + work.height - margin {
        py
    } else {
        py - h
    };
    let fit =
        |v: f32, lo: f32, len: f32, size: f32| v.min(lo + len - margin - size).max(lo + margin);
    (
        fit(x, work.x, work.width, w),
        fit(y, work.y, work.height, h),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use Kind::*;

    const MENU: [Kind; 7] = [Toggle, Toggle, Separator, Toggle, Action, Separator, Action];

    #[test]
    fn rows_stack_under_the_header() {
        let r = rows(&MENU);
        assert_eq!(r[0].y, PAD + HEADER);
        for w in r.windows(2) {
            assert_eq!(w[0].y + w[0].height, w[1].y);
        }
        assert_eq!(r[2].height, SEPARATOR);
        assert_eq!(
            height(&MENU),
            PAD + HEADER + 5.0 * ROW + 2.0 * SEPARATOR + PAD
        );
    }

    #[test]
    fn hits_rows_but_not_separators_or_the_header() {
        let r = rows(&MENU);
        for (i, row) in r.iter().enumerate() {
            let got = hit(&MENU, row.x + 20.0, row.y + row.height / 2.0);
            assert_eq!(got, (MENU[i] != Separator).then_some(i));
        }
        assert_eq!(hit(&MENU, 50.0, HEADER / 2.0), None);
        assert_eq!(hit(&MENU, 1.0, r[0].y + 5.0), None);
        assert_eq!(hit(&MENU, 50.0, height(&MENU) + 1.0), None);
    }

    #[test]
    fn switches_sit_inside_their_row() {
        let row = rows(&MENU)[0];
        let s = switch(row);
        assert!(s.x > TEXT_X + 120.0 && s.x + s.width < row.x + row.width);
        assert!(s.y > row.y && s.y + s.height < row.y + row.height);
        let (off, on) = (knob(s, 0.0), knob(s, 1.0));
        assert!(off.x > s.x && on.x + on.width < s.x + s.width + 0.01);
        assert!(on.x > off.x);
        assert_eq!(off.y, on.y);
    }

    #[test]
    fn stays_on_screen() {
        let work = Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1040.0,
        };
        // From the island at the top: down and right.
        assert_eq!(
            place((900.0, 12.0), (260.0, 300.0), work, 8.0),
            (900.0, 12.0)
        );
        // From the tray, in a corner above the taskbar: up and left.
        assert_eq!(
            place((1880.0, 1060.0), (260.0, 300.0), work, 8.0),
            (1620.0, 732.0)
        );
        // A secondary screen to the left, negative x.
        let left = Rect { x: -2560.0, ..work };
        let (x, _) = place((-700.0, 10.0), (260.0, 300.0), left, 8.0);
        assert!(x >= -2552.0 && x + 260.0 <= -640.0 + 8.0);
    }
}
