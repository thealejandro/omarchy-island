//! The open island's pages: a row of tabs on top, and under it the page
//! (the player on Home, the focus timer on Focus). Sizes in DIPs, from the
//! open island's top left.

use crate::shape::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Home,
    Focus,
}

impl Page {
    pub const ALL: [Page; 2] = [Page::Home, Page::Focus];

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "home" => Some(Self::Home),
            "focus" => Some(Self::Focus),
            _ => None,
        }
    }
}

/// The row of tabs, on top.
const TABS_Y: f32 = 8.0;
const TAB: (f32, f32) = (34.0, 26.0);
const TABS_X: f32 = 16.0;
const TABS_GAP: f32 = 4.0;
/// The clock (or the timer, while it runs) at the row's right end.
const CLOCK_WIDTH: f32 = 120.0;

/// A page's content is laid out as if it began at the island's top; it is
/// drawn this much lower.
pub const PAGE_Y: f32 = 28.0;
/// The page's own height (the player's, before the tabs).
pub const PAGE_HEIGHT: f32 = 180.0;
pub const OPEN_HEIGHT: f32 = PAGE_Y + PAGE_HEIGHT;

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

pub fn tab(i: usize) -> Rect {
    rect(TABS_X + i as f32 * (TAB.0 + TABS_GAP), TABS_Y, TAB.0, TAB.1)
}

pub fn clock(width: f32) -> Rect {
    rect(width - TABS_X - CLOCK_WIDTH, TABS_Y, CLOCK_WIDTH, TAB.1)
}

/// The tab under (x, y), if any.
pub fn hit_tab(x: f32, y: f32) -> Option<Page> {
    Page::ALL.into_iter().enumerate().find_map(|(i, p)| {
        let r = tab(i);
        (x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height).then_some(p)
    })
}

/// The focus page, in the page's own coordinates (see [`PAGE_Y`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusPage {
    /// "Focus · round 2 of 4".
    pub label: Rect,
    /// The time left, big.
    pub time: Rect,
    /// What comes next, beside the time.
    pub next: Rect,
    pub bar: Rect,
    /// One dot per round before a long break.
    pub dots: Rect,
}

const MARGIN: f32 = 24.0;
pub const FOCUS_BUTTON: f32 = 40.0;
const FOCUS_BUTTONS_Y: f32 = 128.0;
const FOCUS_BUTTONS_GAP: f32 = 64.0;
pub const DOT: f32 = 6.0;
pub const DOT_GAP: f32 = 6.0;

pub fn focus_page(width: f32) -> FocusPage {
    let inner = width - 2.0 * MARGIN;
    FocusPage {
        label: rect(MARGIN, 18.0, inner, 20.0),
        time: rect(MARGIN, 38.0, 200.0, 54.0),
        next: rect(MARGIN + 200.0, 58.0, inner - 200.0, 22.0),
        bar: rect(MARGIN, 100.0, inner, 4.0),
        dots: rect(MARGIN, 112.0, inner, DOT),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusButton {
    Reset,
    PlayPause,
    Skip,
}

pub fn focus_button(b: FocusButton, width: f32) -> Rect {
    let dx = match b {
        FocusButton::Reset => -FOCUS_BUTTONS_GAP,
        FocusButton::PlayPause => 0.0,
        FocusButton::Skip => FOCUS_BUTTONS_GAP,
    };
    rect(
        width / 2.0 + dx - FOCUS_BUTTON / 2.0,
        FOCUS_BUTTONS_Y,
        FOCUS_BUTTON,
        FOCUS_BUTTON,
    )
}

/// The focus page's button under (x, y), in the page's coordinates.
pub fn hit_focus(x: f32, y: f32, width: f32) -> Option<FocusButton> {
    [
        FocusButton::Reset,
        FocusButton::PlayPause,
        FocusButton::Skip,
    ]
    .into_iter()
    .find(|&b| {
        let r = focus_button(b, width);
        x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
    })
}

/// The wheel changes the focus length over the time (while it's stopped).
pub fn over_time(x: f32, y: f32, width: f32) -> bool {
    let p = focus_page(width);
    let r = Rect {
        height: p.bar.y - p.label.y,
        y: p.label.y,
        ..p.time
    };
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{OPEN_WIDTH_MAX, OPEN_WIDTH_MIN};

    #[test]
    fn tabs_sit_in_their_row_left_of_the_clock() {
        for w in [OPEN_WIDTH_MIN, 560.0, OPEN_WIDTH_MAX] {
            let last = tab(Page::ALL.len() - 1);
            assert!(last.x + last.width < clock(w).x, "{w}");
            assert!(last.y + last.height <= PAGE_Y + 20.0);
        }
        assert_eq!(hit_tab(tab(0).x + 4.0, tab(0).y + 4.0), Some(Page::Home));
        assert_eq!(hit_tab(tab(1).x + 30.0, tab(1).y + 20.0), Some(Page::Focus));
        // Between two tabs, and under the row.
        assert_eq!(hit_tab(tab(0).x + TAB.0 + 1.0, 12.0), None);
        assert_eq!(hit_tab(tab(0).x + 4.0, PAGE_Y + 10.0), None);
    }

    #[test]
    fn the_focus_page_fits_at_any_width() {
        let mut w = OPEN_WIDTH_MIN;
        while w <= OPEN_WIDTH_MAX {
            let p = focus_page(w);
            assert!(p.next.width > 80.0, "{w}");
            assert!(p.bar.y > p.time.y + p.time.height);
            assert!(p.dots.y > p.bar.y);
            let next = focus_button(FocusButton::Skip, w);
            assert!(next.x + next.width < w - MARGIN);
            assert!(next.y + next.height <= PAGE_HEIGHT);
            assert!(p.dots.y + p.dots.height < focus_button(FocusButton::Reset, w).y);
            w += 10.0;
        }
    }

    #[test]
    fn focus_buttons_and_the_wheel_spot() {
        let w = 560.0;
        assert_eq!(hit_focus(280.0, 148.0, w), Some(FocusButton::PlayPause));
        assert_eq!(hit_focus(216.0, 148.0, w), Some(FocusButton::Reset));
        assert_eq!(hit_focus(344.0, 148.0, w), Some(FocusButton::Skip));
        assert_eq!(hit_focus(248.0, 148.0, w), None);
        assert!(over_time(60.0, 60.0, w));
        assert!(!over_time(60.0, 148.0, w));
        assert_eq!(Page::parse("focus"), Some(Page::Focus));
        assert_eq!(Page::parse("world"), None);
    }
}
