//! Glances that tell something (the volume, the focus timer's end) instead
//! of the song: an icon, a title, a detail and, for a level, a bar. Sizes
//! in DIPs, in the glance's own space (see [`crate::media::PEEK`]).

use crate::icons::Icon;
use crate::shape::Rect;

/// How long a level's glance stays after its last change: short, it
/// follows the keys or the wheel.
pub const LEVEL_MS: u32 = 1500;

/// A level's glance on its own: about as wide as the closed island with a song
/// and only a touch taller, enough to catch the eye. Over the lyrics'
/// glance it takes that one's size instead.
pub const LEVEL: (f32, f32) = (300.0, 36.0);

/// Where a level's glance puts things, at `size`: the icon at the left,
/// the bar filling the middle and the percentage at the right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelLayout {
    pub icon: Rect,
    pub bar: Rect,
    /// The percentage, right-aligned (or, muted, the word in the bar's place).
    pub text: Rect,
}

pub fn level_layout((w, h): (f32, f32)) -> LevelLayout {
    let side = (h - 12.0).clamp(20.0, 28.0);
    let pad = 14.0;
    let icon = Rect {
        x: pad,
        y: (h - side) / 2.0,
        width: side,
        height: side,
    };
    let text = Rect {
        x: w - pad - 40.0,
        y: 0.0,
        width: 40.0,
        height: h,
    };
    let x = icon.x + icon.width + 12.0;
    let bar = Rect {
        x,
        y: (h - 4.0) / 2.0,
        width: text.x - 10.0 - x,
        height: 4.0,
    };
    LevelLayout { icon, bar, text }
}

/// The volume as a whole percentage, as the system's own flyout shows it.
pub fn percent(level: f32) -> u32 {
    (level.clamp(0.0, 1.0) * 100.0).round() as u32
}

/// A speaker with as many waves as the
/// volume is loud, crossed out when muted.
pub fn volume_icon(level: f32, muted: bool) -> Icon {
    match percent(level) {
        _ if muted => Icon::SpeakerMute,
        0 => Icon::Speaker0,
        1..=50 => Icon::Speaker1,
        _ => Icon::Speaker2,
    }
}

/// The volume after one press of a volume key, as Windows' own keys move it:
/// 2 % up or down from the percentage shown, never past 0 or 100.
pub fn volume_step(level: f32, up: bool) -> f32 {
    let p = percent(level) as i32 + if up { 2 } else { -2 };
    p.clamp(0, 100) as f32 / 100.0
}

/// A change worth a glance: the percentage or the mute (a change of
/// balance between the speakers alone tells nothing).
pub fn volume_changed(before: Option<(f32, bool)>, now: (f32, bool)) -> bool {
    before.is_none_or(|(l, m)| percent(l) != percent(now.0) || m != now.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::PEEK;

    #[test]
    fn levels_read_as_percent_and_icon() {
        assert_eq!(percent(0.456), 46);
        assert_eq!(percent(1.2), 100);
        assert_eq!(percent(-1.0), 0);
        assert_eq!(volume_icon(0.0, false), Icon::Speaker0);
        assert_eq!(volume_icon(0.2, false), Icon::Speaker1);
        assert_eq!(volume_icon(0.5, false), Icon::Speaker1);
        assert_eq!(volume_icon(0.9, false), Icon::Speaker2);
        assert_eq!(volume_icon(0.9, true), Icon::SpeakerMute);
    }

    #[test]
    fn a_volume_key_moves_two_percent() {
        assert_eq!(percent(volume_step(0.30, true)), 32);
        assert_eq!(percent(volume_step(0.31, false)), 29);
        assert_eq!(percent(volume_step(0.995, true)), 100);
        assert_eq!(percent(volume_step(0.01, false)), 0);
        assert_eq!(volume_step(1.0, true), 1.0);
        assert_eq!(volume_step(0.0, false), 0.0);
    }

    #[test]
    fn only_a_new_percent_or_mute_is_a_change() {
        assert!(volume_changed(None, (0.5, false)));
        assert!(!volume_changed(Some((0.501, false)), (0.499, false)));
        assert!(volume_changed(Some((0.5, false)), (0.52, false)));
        assert!(volume_changed(Some((0.5, false)), (0.5, true)));
    }

    #[test]
    fn icon_bar_and_percent_fill_the_glance_in_order() {
        for size in [LEVEL, PEEK] {
            let l = level_layout(size);
            assert!(l.icon.x + l.icon.width < l.bar.x);
            assert!(l.bar.x + l.bar.width < l.text.x);
            assert!(l.text.x + l.text.width <= size.0 - 14.0);
            assert!(l.icon.y >= 0.0 && l.icon.y + l.icon.height <= size.1);
            // Most of the width is the bar.
            assert!(l.bar.width > size.0 * 0.5);
        }
        // Barely taller than the closed island.
        const { assert!(LEVEL.1 - crate::media::CLOSED_TEXT.height <= 4.0) };
    }
}
