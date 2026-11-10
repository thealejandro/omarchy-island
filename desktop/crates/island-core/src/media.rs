//! The music view: times, the playback position, and where things sit.
//! Rects are in DIPs from the body's top-left corner (ears not included).

use crate::shape::Rect;

/// `m:ss`, or `h:mm:ss` from an hour on.
pub fn fmt_time(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// The position now, from `at_ms` measured `elapsed_ms` ago. It never runs
/// past the end, and stands still while paused.
pub fn position(at_ms: u64, elapsed_ms: u64, duration_ms: u64, playing: bool) -> u64 {
    let p = if playing {
        at_ms.saturating_add(elapsed_ms)
    } else {
        at_ms
    };
    if duration_ms > 0 {
        p.min(duration_ms)
    } else {
        p
    }
}

/// The cover's color, from a few of its pixels (RGB): the most vivid ones,
/// brightened to read on black. A gray cover gives a light gray.
pub fn accent(pixels: &[[u8; 3]]) -> [f32; 3] {
    let mut scored: Vec<(f32, [f32; 3])> = pixels
        .iter()
        .map(|p| {
            let c = p.map(|v| v as f32 / 255.0);
            let max = c[0].max(c[1]).max(c[2]);
            let min = c[0].min(c[1]).min(c[2]);
            let sat = if max > 0.0 { (max - min) / max } else { 0.0 };
            (sat * max, c)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top = &scored[..scored.len().div_ceil(4)];
    if top.is_empty() || top[0].0 < 0.12 {
        return [0.85; 3];
    }
    let mut sum = [0.0f32; 3];
    for (_, c) in top {
        for i in 0..3 {
            sum[i] += c[i];
        }
    }
    let max = sum[0].max(sum[1]).max(sum[2]);
    // Its hue at nearly full brightness, a touch toward white.
    sum.map(|v| (v / max * 0.95) * 0.85 + 0.15)
}

/// How the song's line sits in the closed island and the peek.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
}

impl TextAlign {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "left" => Some(Self::Left),
            "center" => Some(Self::Center),
            _ => None,
        }
    }
}

/// What the song's line says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SongText {
    #[default]
    TitleArtist,
    ArtistTitle,
    Title,
    Artist,
}

impl SongText {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "title-artist" => Some(Self::TitleArtist),
            "artist-title" => Some(Self::ArtistTitle),
            "title" => Some(Self::Title),
            "artist" => Some(Self::Artist),
            _ => None,
        }
    }

    /// The line; a missing part leaves just the other one.
    pub fn line(self, title: &str, artist: &str) -> String {
        self.spans(title, artist).into_iter().map(|s| s.0).collect()
    }

    /// The line in pieces, `true` on the ones shown dimmer: next to the
    /// title, the artist and the separator, so the title stands out.
    pub fn spans(self, title: &str, artist: &str) -> Vec<(String, bool)> {
        let parts = self.parts(title, artist);
        let both = parts.len() > 1;
        parts
            .into_iter()
            .enumerate()
            .map(|(i, s)| (s.to_string(), both && (i == 1 || s != title)))
            .collect()
    }

    /// The text pieces in order: one, or first, separator, second.
    fn parts<'a>(self, title: &'a str, artist: &'a str) -> Vec<&'a str> {
        let (a, b) = match self {
            Self::TitleArtist => (title, artist),
            Self::ArtistTitle => (artist, title),
            Self::Title => (title, ""),
            Self::Artist if artist.is_empty() => (title, ""),
            Self::Artist => (artist, ""),
        };
        match (a.is_empty(), b.is_empty()) {
            (false, false) => vec![a, "  ·  ", b],
            (true, _) => vec![b],
            (false, true) => vec![a],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Prev,
    PlayPause,
    Next,
}

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

// The open island, 180 tall; its width is a setting, in steps of 10.
pub const OPEN_WIDTH: f32 = 560.0;
/// The width by default with a column of lyrics.
pub const OPEN_LYRICS_WIDTH: f32 = 720.0;
pub const OPEN_WIDTH_MIN: f32 = 480.0;
pub const OPEN_WIDTH_MAX: f32 = 960.0;
const OPEN_WIDTH_STEP: f32 = 10.0;
pub const ART: Rect = rect(24.0, 24.0, 88.0, 88.0);
const TEXT_X: f32 = 128.0;
const MARGIN: f32 = 24.0;
const BUTTON: f32 = 40.0;
const BUTTONS_Y: f32 = 128.0;
const BUTTONS_GAP: f32 = 64.0;

// With synced lyrics the open island is not taller: the lines run in a
// column right of the player, as tall as it, so nothing is left empty.
const LYRICS_TOP: f32 = 14.0;
const LYRICS_HEIGHT: f32 = 152.0;
const LYRICS_GAP_X: f32 = 20.0;
/// The column's share of the room beside the cover.
const LYRICS_SHARE: f32 = 0.42;
/// Narrower than this the lines would break every few words: no column.
const LYRICS_MIN_WIDTH: f32 = 180.0;

/// The open island's width from a setting: a whole number of DIPs,
/// rounded to a step of 10 and kept between the smallest and largest.
pub fn parse_open_width(s: &str) -> Option<f32> {
    let w: f32 = s.trim().parse().ok().filter(|w: &f32| w.is_finite())?;
    let w = (w / OPEN_WIDTH_STEP).round() * OPEN_WIDTH_STEP;
    Some(w.clamp(OPEN_WIDTH_MIN, OPEN_WIDTH_MAX))
}

/// The most of a screen's width the open island takes.
const SCREEN_SHARE: f32 = 0.6;

/// The open island's width on a screen `screen` DIPs wide: the setting, but
/// at most a share of the screen (in steps of 10), so it stays a bar at the
/// top on a 1280 x 1024 monitor or a laptop at 150 % too. Never below the
/// smallest width unless the screen itself is narrower.
pub fn fit_open_width(setting: f32, screen: f32) -> f32 {
    let most = (screen * SCREEN_SHARE / OPEN_WIDTH_STEP).floor() * OPEN_WIDTH_STEP;
    setting.min(most.max(OPEN_WIDTH_MIN)).min(screen.max(0.0))
}

/// How the open island is laid out: its width, and whether the lyrics
/// take a column of it. The island keeps its width with or without lines,
/// so it doesn't jump when they arrive; the player fills what's left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Open {
    pub width: f32,
    pub lyrics: bool,
}

impl Open {
    pub const fn new(width: f32, lyrics: bool) -> Self {
        Self { width, lyrics }
    }

    /// The column's width: a share of the room, in steps of 10; none if
    /// that is too narrow to read.
    fn column_width(self) -> f32 {
        if !self.lyrics {
            return 0.0;
        }
        let room = self.width - TEXT_X - MARGIN - LYRICS_GAP_X;
        let w = (room * LYRICS_SHARE / OPEN_WIDTH_STEP).round() * OPEN_WIDTH_STEP;
        if w < LYRICS_MIN_WIDTH { 0.0 } else { w }
    }

    /// The player's text width, beside the column if there is one.
    fn text_width(self) -> f32 {
        let col = self.column_width();
        let beside = if col > 0.0 { col + LYRICS_GAP_X } else { 0.0 };
        self.width - TEXT_X - MARGIN - beside
    }

    /// Where the lines go, if they have a column.
    pub fn lyrics_rect(self) -> Option<Rect> {
        let w = self.column_width();
        (w > 0.0).then(|| rect(self.width - MARGIN - w, LYRICS_TOP, w, LYRICS_HEIGHT))
    }
}

/// Where the player's parts go in the open island, alone or beside lyrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    pub title: Rect,
    pub artist: Rect,
    pub bar: Rect,
    pub times: Rect,
    buttons_center: f32,
}

pub fn player(open: Open) -> Player {
    let w = open.text_width();
    Player {
        title: rect(TEXT_X, 26.0, w, 26.0),
        artist: rect(TEXT_X, 54.0, w, 22.0),
        bar: rect(TEXT_X, 96.0, w, 4.0),
        times: rect(TEXT_X, 104.0, w, 16.0),
        // The controls sit under the bar's middle.
        buttons_center: TEXT_X + w / 2.0,
    }
}
/// Where the line being sung starts, from the column's top: room for the
/// one before it above, and for what comes below.
pub const LYRICS_ANCHOR: f32 = 42.0;
pub const LYRICS_GAP: f32 = 8.0;
/// A new line scrolls the column up over this long.
pub const LYRICS_SCROLL_MS: f32 = 320.0;
/// Lines fade out over this much at the column's edges.
pub const LYRICS_FADE: f32 = 18.0;
/// Stands for a line without words (a pause in the singing).
pub const LYRICS_PAUSE: &str = "• • •";

/// How much of the scroll to a new line is left `elapsed_ms` after it
/// starts: 1 to 0, eased out.
pub fn lyrics_scroll(elapsed_ms: f32) -> f32 {
    let t = (elapsed_ms / LYRICS_SCROLL_MS).clamp(0.0, 1.0);
    (1.0 - t).powi(3)
}

/// The tops of stacked lines of these `heights` in the column: the
/// `current` one at [`LYRICS_ANCHOR`], lowered by `scroll` of the step it
/// just took (the line before it and a gap), so a change glides up.
pub fn lyrics_tops(heights: &[f32], current: usize, scroll: f32) -> Vec<f32> {
    let mut tops = vec![0.0; heights.len()];
    if current >= heights.len() {
        return tops;
    }
    let back = current
        .checked_sub(1)
        .map_or(0.0, |i| heights[i] + LYRICS_GAP);
    let base = LYRICS_ANCHOR + scroll.clamp(0.0, 1.0) * back;
    let mut y = base;
    for i in current..heights.len() {
        tops[i] = y;
        y += heights[i] + LYRICS_GAP;
    }
    let mut y = base;
    for i in (0..current).rev() {
        y -= heights[i] + LYRICS_GAP;
        tops[i] = y;
    }
    tops
}

/// How bright a line is, by how far it is from the one in focus (in
/// lines; fractional while scrolling): sung ones fade faster than the
/// ones to come.
pub fn lyrics_alpha(distance: f32) -> f32 {
    if distance < 0.0 {
        (1.0 + distance * 0.62).max(0.16)
    } else {
        (1.0 - distance * 0.42).max(0.2)
    }
}

/// How much of a line at `top` with `height` shows at the column's edges.
pub fn lyrics_edge(top: f32, height: f32) -> f32 {
    let above = (top + height) / LYRICS_FADE;
    let below = (LYRICS_HEIGHT - top) / LYRICS_FADE;
    above.min(below).clamp(0.0, 1.0)
}

// The glance on a new song (and, later, on each line of lyrics), 400 x 44.
pub const PEEK: (f32, f32) = (400.0, 44.0);
pub const PEEK_ART: Rect = rect(14.0, 8.0, 28.0, 28.0);
pub const PEEK_TEXT: Rect = rect(54.0, 0.0, 300.0, 44.0);
pub const PEEK_BARS: Rect = rect(368.0, 16.0, 18.0, 12.0);

// The closed island, 180 x 32; 300 wide while something plays, for the
// song's name.
pub const CLOSED_MEDIA_WIDTH: f32 = 300.0;
pub const SMALL_ART: Rect = rect(12.0, 6.0, 20.0, 20.0);
pub const CLOSED_TEXT: Rect = rect(40.0, 0.0, 214.0, 32.0);
pub const BARS: Rect = rect(270.0, 10.0, 18.0, 12.0);

// A view coming in grows from a touch smaller and drops into place from
// above (the plugin's peek: scale 0.96 and a 6 px drop, OutCubic).
pub const ENTER_SCALE: f32 = 0.94;
pub const ENTER_DROP: f32 = 6.0;

/// How a view shows while its visibility `a` goes from 0 to 1: opacity,
/// scale about its top center, and how far above its place it is (DIPs).
/// Eased out: it shows quickly and settles gently; leaving runs it back.
pub fn entrance(a: f32) -> (f32, f32, f32) {
    let a = a.clamp(0.0, 1.0);
    let e = 1.0 - (1.0 - a).powi(3);
    (
        e,
        ENTER_SCALE + (1.0 - ENTER_SCALE) * e,
        ENTER_DROP * (1.0 - e),
    )
}

pub fn button(b: Button, open: Open) -> Rect {
    let dx = match b {
        Button::Prev => -BUTTONS_GAP,
        Button::PlayPause => 0.0,
        Button::Next => BUTTONS_GAP,
    };
    rect(
        player(open).buttons_center + dx - BUTTON / 2.0,
        BUTTONS_Y,
        BUTTON,
        BUTTON,
    )
}

/// The button under (`x`, `y`), if any.
pub fn hit(x: f32, y: f32, open: Open) -> Option<Button> {
    [Button::Prev, Button::PlayPause, Button::Next]
        .into_iter()
        .find(|&b| {
            let r = button(b, open);
            x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lyrics_fill_the_open_island_beside_the_player() {
        // Every width a setting can give, with and without lyrics.
        let mut w = OPEN_WIDTH_MIN;
        while w <= OPEN_WIDTH_MAX {
            for lyrics in [false, true] {
                let o = Open::new(w, lyrics);
                let p = player(o);
                assert!(p.bar.width >= 200.0, "{w}: room for the bar");
                let end = p.bar.x + p.bar.width;
                match o.lyrics_rect() {
                    Some(l) => {
                        assert!(lyrics);
                        assert!(l.x >= end + 16.0, "{w}");
                        assert_eq!(l.x + l.width, w - 24.0, "{w}");
                        // The open island's height (180).
                        assert!(l.y + l.height <= 180.0);
                        assert!(l.width >= 180.0 && l.width % 10.0 == 0.0, "{w}");
                    }
                    None => assert_eq!(end, w - 24.0, "{w}"),
                }
            }
            w += 10.0;
        }
        // The default with lyrics: the player a bit narrower than alone.
        let o = Open::new(OPEN_LYRICS_WIDTH, true);
        assert_eq!(o.lyrics_rect().map(|l| l.width), Some(230.0));
        assert_eq!(player(o).bar.width, 318.0);
        assert_eq!(player(Open::new(OPEN_WIDTH, false)).bar.width, 408.0);
        // Too narrow for a readable column: just the player.
        assert_eq!(Open::new(540.0, true).lyrics_rect(), None);
        assert!(Open::new(600.0, true).lyrics_rect().is_some());
    }

    #[test]
    fn open_width_comes_in_steps_of_ten_within_limits() {
        assert_eq!(parse_open_width("540"), Some(540.0));
        assert_eq!(parse_open_width(" 724 "), Some(720.0));
        assert_eq!(parse_open_width("725.5"), Some(730.0));
        assert_eq!(parse_open_width("100"), Some(OPEN_WIDTH_MIN));
        assert_eq!(parse_open_width("5000"), Some(OPEN_WIDTH_MAX));
        assert_eq!(parse_open_width("ancho"), None);
        assert_eq!(parse_open_width("NaN"), None);
        assert_eq!(parse_open_width("inf"), None);
    }

    #[test]
    fn open_width_fits_small_screens() {
        // A 2560 screen at 100 %: the setting as it is.
        assert_eq!(fit_open_width(960.0, 2560.0), 960.0);
        assert_eq!(fit_open_width(720.0, 1920.0), 720.0);
        // A square 17" (1280 x 1024) still fits the default.
        assert_eq!(fit_open_width(720.0, 1280.0), 720.0);
        assert_eq!(fit_open_width(960.0, 1280.0), 760.0);
        // 1366 at 125 % (1093 DIPs) and 1024 x 768.
        assert_eq!(fit_open_width(720.0, 1092.8), 650.0);
        assert_eq!(fit_open_width(720.0, 1024.0), 610.0);
        // Smaller still: the smallest width, never past the screen.
        assert_eq!(fit_open_width(720.0, 800.0), OPEN_WIDTH_MIN);
        assert_eq!(fit_open_width(720.0, 400.0), 400.0);
        // A narrower setting is kept.
        assert_eq!(fit_open_width(540.0, 1024.0), 540.0);
        // At the width a 1024 screen gets, there is still a lyrics column.
        assert!(Open::new(610.0, true).lyrics_rect().is_some());
    }

    #[test]
    fn lyrics_stack_around_the_line_being_sung() {
        let h = [22.0, 44.0, 22.0, 22.0];
        let tops = lyrics_tops(&h, 1, 0.0);
        assert_eq!(tops[1], LYRICS_ANCHOR);
        assert_eq!(tops[0], LYRICS_ANCHOR - 22.0 - LYRICS_GAP);
        assert_eq!(tops[2], LYRICS_ANCHOR + 44.0 + LYRICS_GAP);
        assert_eq!(tops[3], tops[2] + 22.0 + LYRICS_GAP);
        // Just changed: everything is still one step down, where the line
        // before was the one in focus.
        let start = lyrics_tops(&h, 1, 1.0);
        assert_eq!(start[0], LYRICS_ANCHOR);
        // The first line has nothing to scroll from; out of range, nothing.
        assert_eq!(lyrics_tops(&h, 0, 1.0)[0], LYRICS_ANCHOR);
        assert_eq!(lyrics_tops(&h, 9, 0.0), vec![0.0; 4]);
    }

    #[test]
    fn lyrics_scroll_and_fade_smoothly() {
        assert_eq!(lyrics_scroll(0.0), 1.0);
        assert_eq!(lyrics_scroll(LYRICS_SCROLL_MS), 0.0);
        assert_eq!(lyrics_scroll(1e6), 0.0);
        assert!(lyrics_scroll(LYRICS_SCROLL_MS / 2.0) < 0.2);
        assert_eq!(lyrics_alpha(0.0), 1.0);
        assert!(lyrics_alpha(-1.0) < lyrics_alpha(1.0));
        assert!(lyrics_alpha(1.0) > lyrics_alpha(2.0));
        assert!(lyrics_alpha(-9.0) > 0.0 && lyrics_alpha(9.0) > 0.0);
        assert_eq!(lyrics_edge(LYRICS_ANCHOR, 22.0), 1.0);
        assert_eq!(lyrics_edge(-30.0, 22.0), 0.0);
        assert_eq!(lyrics_edge(LYRICS_HEIGHT, 22.0), 0.0);
    }

    #[test]
    fn entrance_eases_out_to_its_place() {
        assert_eq!(entrance(0.0), (0.0, ENTER_SCALE, ENTER_DROP));
        assert_eq!(entrance(1.0), (1.0, 1.0, 0.0));
        assert_eq!(entrance(-1.0), entrance(0.0));
        assert_eq!(entrance(2.0), entrance(1.0));
        // Past halfway in time, most of the way there.
        let (o, k, d) = entrance(0.5);
        assert!(o > 0.85 && k > 0.99 && d < 1.0, "{o} {k} {d}");
        // Always moving forward.
        let mut last = entrance(0.0);
        for i in 1..=20 {
            let now = entrance(i as f32 / 20.0);
            assert!(now.0 > last.0 && now.1 > last.1 && now.2 < last.2);
            last = now;
        }
    }

    #[test]
    fn times() {
        assert_eq!(fmt_time(0), "0:00");
        assert_eq!(fmt_time(61_999), "1:01");
        assert_eq!(fmt_time(3_723_000), "1:02:03");
    }

    #[test]
    fn position_runs_only_while_playing_and_stops_at_the_end() {
        assert_eq!(position(1000, 500, 200_000, true), 1500);
        assert_eq!(position(1000, 500, 200_000, false), 1000);
        assert_eq!(position(199_000, 5000, 200_000, true), 200_000);
        // A stream without a known length just runs.
        assert_eq!(position(1000, 500, 0, true), 1500);
    }

    #[test]
    fn accent_takes_the_vivid_part_and_brightens_it() {
        // Mostly dark gray with a red patch: red wins, bright.
        let mut px = vec![[40, 40, 40]; 12];
        px.extend([[200, 30, 20]; 4]);
        let a = accent(&px);
        assert!(a[0] > 0.9 && a[1] < 0.4 && a[2] < 0.4, "{a:?}");
        // A gray cover, and no pixels at all.
        assert_eq!(accent(&[[90, 90, 90]; 8]), [0.85; 3]);
        assert_eq!(accent(&[]), [0.85; 3]);
        assert_eq!(accent(&[[0, 0, 0]; 4]), [0.85; 3]);
    }

    #[test]
    fn song_line_styles() {
        let line = |s, t, a| SongText::line(s, t, a);
        assert_eq!(
            line(SongText::TitleArtist, "casa", "Riosse"),
            "casa  ·  Riosse"
        );
        assert_eq!(
            line(SongText::ArtistTitle, "casa", "Riosse"),
            "Riosse  ·  casa"
        );
        assert_eq!(line(SongText::Title, "casa", "Riosse"), "casa");
        assert_eq!(line(SongText::Artist, "casa", "Riosse"), "Riosse");
        // Missing parts: no stray separator, and never an empty line.
        assert_eq!(line(SongText::TitleArtist, "casa", ""), "casa");
        assert_eq!(line(SongText::ArtistTitle, "casa", ""), "casa");
        assert_eq!(line(SongText::Artist, "casa", ""), "casa");
        // The artist and the separator are dimmer; the title never.
        let s = |v: &[(&str, bool)]| {
            v.iter()
                .map(|(t, d)| (t.to_string(), *d))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            SongText::TitleArtist.spans("casa", "Riosse"),
            s(&[("casa", false), ("  ·  ", true), ("Riosse", true)])
        );
        assert_eq!(
            SongText::ArtistTitle.spans("casa", "Riosse"),
            s(&[("Riosse", true), ("  ·  ", true), ("casa", false)])
        );
        assert_eq!(
            SongText::Artist.spans("casa", "Riosse"),
            s(&[("Riosse", false)])
        );
        assert_eq!(
            SongText::Title.spans("casa", "Riosse"),
            s(&[("casa", false)])
        );
        // Only a title (or the same text twice): nothing dims.
        assert_eq!(SongText::Artist.spans("casa", ""), s(&[("casa", false)]));
        assert_eq!(SongText::parse("artist"), Some(SongText::Artist));
        assert_eq!(SongText::parse("nope"), None);
        assert_eq!(TextAlign::parse("left"), Some(TextAlign::Left));
        assert_eq!(TextAlign::default(), TextAlign::Center);
    }

    #[test]
    fn buttons_are_hit_inside_and_missed_between() {
        let alone = Open::new(OPEN_WIDTH, false);
        assert_eq!(hit(332.0, 148.0, alone), Some(Button::PlayPause));
        assert_eq!(hit(268.0, 148.0, alone), Some(Button::Prev));
        assert_eq!(hit(396.0, 148.0, alone), Some(Button::Next));
        assert_eq!(hit(300.0, 148.0, alone), None);
        assert_eq!(hit(332.0, 60.0, alone), None);
        // Beside lyrics, they follow the narrower bar's middle.
        let beside = Open::new(OPEN_LYRICS_WIDTH, true);
        assert_eq!(hit(287.0, 148.0, beside), Some(Button::PlayPause));
        assert_eq!(hit(396.0, 148.0, beside), None);
        // They fit in the open island, left of the lyrics, at any width.
        let mut w = OPEN_WIDTH_MIN;
        while w <= OPEN_WIDTH_MAX {
            for lyrics in [false, true] {
                let o = Open::new(w, lyrics);
                let next = button(Button::Next, o);
                assert!(next.y + next.height <= 180.0);
                let end = o.lyrics_rect().map_or(w, |l| l.x);
                assert!(next.x + next.width <= end, "{w}");
                assert!(button(Button::Prev, o).x >= ART.x + ART.width, "{w}");
            }
            w += 10.0;
        }
    }
}
