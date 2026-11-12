//! The island's own menu (the tray icon's): a dark grey rounded panel like
//! the island, with switches. Sizes in DIPs.
//!
//! Two sizes: compact for the usual menu, near Windows' own menus, and large
//! for the offer to install or update, which asks for a choice and is shown
//! in the middle of the screen.

use crate::shape::Rect;

/// Between the menu and the edges of the screen's work area.
pub const SCREEN_MARGIN: f32 = 8.0;

/// Which of the two sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Compact,
    Large,
}

/// A menu's measures and its text's sizes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    /// Around the rows, inside the panel.
    pub pad: f32,
    /// The name and version on top.
    pub header: f32,
    pub row: f32,
    pub separator: f32,
    pub radius: f32,
    /// The highlight under the pointer.
    pub row_radius: f32,
    /// Where a row's words start.
    pub text_x: f32,
    switch: (f32, f32),
    knob_inset: f32,
    /// Between a switch and the row's right end.
    switch_end: f32,
    /// The rows', the name's and the version's text.
    pub font: f32,
    pub head_font: f32,
    pub version_font: f32,
}

impl Look {
    pub const ALL: [Self; 2] = [Self::Compact, Self::Large];

    pub const fn size(self) -> Size {
        match self {
            // Room for the longest row ("Notifications only in the island")
            // beside its switch.
            Self::Compact => Size {
                width: 312.0,
                pad: 6.0,
                header: 38.0,
                row: 36.0,
                separator: 9.0,
                radius: 12.0,
                row_radius: 8.0,
                text_x: 16.0,
                switch: (38.0, 20.0),
                knob_inset: 2.5,
                switch_end: 12.0,
                font: 14.0,
                head_font: 14.5,
                version_font: 12.0,
            },
            Self::Large => Size {
                width: 360.0,
                pad: 10.0,
                header: 52.0,
                row: 46.0,
                separator: 13.0,
                radius: 14.0,
                row_radius: 10.0,
                text_x: 22.0,
                switch: (46.0, 26.0),
                knob_inset: 3.0,
                switch_end: 14.0,
                font: 17.0,
                head_font: 18.0,
                version_font: 14.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A setting with a switch: choosing it flips the switch, the menu stays.
    Toggle,
    /// Does something and closes the menu.
    Action,
    Separator,
}

impl Size {
    /// Each entry's place, top to bottom, under the header.
    pub fn rows(&self, kinds: &[Kind]) -> Vec<Rect> {
        let mut y = self.pad + self.header;
        kinds
            .iter()
            .map(|k| {
                let h = if *k == Kind::Separator {
                    self.separator
                } else {
                    self.row
                };
                let r = Rect {
                    x: self.pad,
                    y,
                    width: self.width - 2.0 * self.pad,
                    height: h,
                };
                y += h;
                r
            })
            .collect()
    }

    pub fn height(&self, kinds: &[Kind]) -> f32 {
        self.rows(kinds)
            .last()
            .map_or(self.pad + self.header, |r| r.y + r.height)
            + self.pad
    }

    /// The entry under (x, y), separators left out.
    pub fn hit(&self, kinds: &[Kind], x: f32, y: f32) -> Option<usize> {
        self.rows(kinds).iter().zip(kinds).position(|(r, k)| {
            *k != Kind::Separator && x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
        })
    }

    /// A toggle row's switch, at its right end.
    pub fn switch(&self, row: Rect) -> Rect {
        let (w, h) = self.switch;
        Rect {
            x: row.x + row.width - self.switch_end - w,
            y: row.y + (row.height - h) / 2.0,
            width: w,
            height: h,
        }
    }

    /// The switch's knob: on the right when on. `t` from 0 (off) to 1 (on).
    pub fn knob(&self, switch: Rect, t: f32) -> Rect {
        let d = switch.height - 2.0 * self.knob_inset;
        let travel = switch.width - 2.0 * self.knob_inset - d;
        Rect {
            x: switch.x + self.knob_inset + travel * t.clamp(0.0, 1.0),
            y: switch.y + self.knob_inset,
            width: d,
            height: d,
        }
    }

    /// How wide a toggle row's words may be, before its switch.
    pub fn label_width(&self, row: Rect) -> f32 {
        self.switch(row).x - 8.0 - self.text_x
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
        for s in Look::ALL.map(Look::size) {
            let r = s.rows(&MENU);
            assert_eq!(r[0].y, s.pad + s.header);
            for w in r.windows(2) {
                assert_eq!(w[0].y + w[0].height, w[1].y);
            }
            assert_eq!(r[2].height, s.separator);
            assert_eq!(
                s.height(&MENU),
                s.pad + s.header + 5.0 * s.row + 2.0 * s.separator + s.pad
            );
        }
    }

    #[test]
    fn hits_rows_but_not_separators_or_the_header() {
        for s in Look::ALL.map(Look::size) {
            let r = s.rows(&MENU);
            for (i, row) in r.iter().enumerate() {
                let got = s.hit(&MENU, row.x + 20.0, row.y + row.height / 2.0);
                assert_eq!(got, (MENU[i] != Separator).then_some(i));
            }
            assert_eq!(s.hit(&MENU, 50.0, s.header / 2.0), None);
            assert_eq!(s.hit(&MENU, 1.0, r[0].y + 5.0), None);
            assert_eq!(s.hit(&MENU, 50.0, s.height(&MENU) + 1.0), None);
        }
    }

    #[test]
    fn switches_sit_inside_their_row() {
        for s in Look::ALL.map(Look::size) {
            let row = s.rows(&MENU)[0];
            let sw = s.switch(row);
            assert!(sw.x + sw.width < row.x + row.width);
            assert!(sw.y > row.y && sw.y + sw.height < row.y + row.height);
            let (off, on) = (s.knob(sw, 0.0), s.knob(sw, 1.0));
            assert!(off.x > sw.x && on.x + on.width < sw.x + sw.width + 0.01);
            assert!(on.x > off.x);
            assert_eq!(off.y, on.y);
        }
    }

    #[test]
    fn the_longest_row_fits_beside_its_switch() {
        // "Notifications only in the island", 32 characters: at most 0.5 em
        // each in Segoe UI. Only the compact menu has switches (the large
        // one is the offer's two choices).
        let s = Look::Compact.size();
        let row = s.rows(&MENU)[0];
        assert!(s.label_width(row) >= 32.0 * 0.5 * s.font);
    }

    #[test]
    fn the_usual_menu_is_the_smaller_one() {
        let (c, l) = (Look::Compact.size(), Look::Large.size());
        assert!(c.height(&MENU) < l.height(&MENU));
        assert!(c.width <= l.width && c.row < l.row && c.font < l.font);
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
