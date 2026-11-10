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
        let (a, b) = match self {
            Self::TitleArtist => (title, artist),
            Self::ArtistTitle => (artist, title),
            Self::Title => (title, ""),
            Self::Artist if artist.is_empty() => (title, ""),
            Self::Artist => (artist, ""),
        };
        match (a.is_empty(), b.is_empty()) {
            (false, false) => format!("{a}  ·  {b}"),
            (true, _) => b.to_string(),
            (false, true) => a.to_string(),
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

// The open island, 560 x 180.
pub const ART: Rect = rect(24.0, 24.0, 88.0, 88.0);
pub const TITLE: Rect = rect(128.0, 26.0, 408.0, 26.0);
pub const ARTIST: Rect = rect(128.0, 54.0, 408.0, 22.0);
pub const BAR: Rect = rect(128.0, 96.0, 408.0, 4.0);
pub const TIMES: Rect = rect(128.0, 104.0, 408.0, 16.0);
const BUTTON: f32 = 40.0;
const BUTTONS_Y: f32 = 128.0;
const BUTTONS_CENTER: f32 = 332.0;
const BUTTONS_GAP: f32 = 64.0;

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

pub fn button(b: Button) -> Rect {
    let dx = match b {
        Button::Prev => -BUTTONS_GAP,
        Button::PlayPause => 0.0,
        Button::Next => BUTTONS_GAP,
    };
    rect(
        BUTTONS_CENTER + dx - BUTTON / 2.0,
        BUTTONS_Y,
        BUTTON,
        BUTTON,
    )
}

/// The button under (`x`, `y`), if any.
pub fn hit(x: f32, y: f32) -> Option<Button> {
    [Button::Prev, Button::PlayPause, Button::Next]
        .into_iter()
        .find(|&b| {
            let r = button(b);
            x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
        })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(SongText::parse("artist"), Some(SongText::Artist));
        assert_eq!(SongText::parse("nope"), None);
        assert_eq!(TextAlign::parse("left"), Some(TextAlign::Left));
        assert_eq!(TextAlign::default(), TextAlign::Center);
    }

    #[test]
    fn buttons_are_hit_inside_and_missed_between() {
        assert_eq!(hit(332.0, 148.0), Some(Button::PlayPause));
        assert_eq!(hit(268.0, 148.0), Some(Button::Prev));
        assert_eq!(hit(396.0, 148.0), Some(Button::Next));
        assert_eq!(hit(300.0, 148.0), None);
        assert_eq!(hit(332.0, 60.0), None);
        // They fit in the open island.
        assert!(button(Button::Next).y + button(Button::Next).height <= 180.0);
    }
}
