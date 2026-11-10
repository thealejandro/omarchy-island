//! Glances that tell something (the volume, the focus timer's end) instead
//! of the song: an icon, a title, a detail and, for a level, a bar. Sizes
//! in DIPs, in the glance's own space (see [`crate::media::PEEK`]).

use crate::media::PEEK_TEXT;
use crate::shape::Rect;

/// How long a level's glance stays after its last change: short, it
/// follows the keys or the wheel.
pub const LEVEL_MS: u32 = 1500;

/// Beside a level's bar, the title and detail take this much.
pub const LEVEL_TEXT: Rect = Rect {
    width: 190.0,
    ..PEEK_TEXT
};

/// The level's bar, at the glance's right.
pub const LEVEL_BAR: Rect = Rect {
    x: 256.0,
    y: 20.0,
    width: 126.0,
    height: 4.0,
};

/// The volume as a whole percentage, as the system's own flyout shows it.
pub fn percent(level: f32) -> u32 {
    (level.clamp(0.0, 1.0) * 100.0).round() as u32
}

/// Segoe Fluent Icons (and MDL2): a speaker with as many waves as the
/// volume is loud, crossed out when muted.
pub fn volume_icon(level: f32, muted: bool) -> &'static str {
    match percent(level) {
        _ if muted => "\u{E74F}",
        0 => "\u{E992}",
        1..=33 => "\u{E993}",
        34..=66 => "\u{E994}",
        _ => "\u{E995}",
    }
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
        assert_eq!(volume_icon(0.0, false), "\u{E992}");
        assert_eq!(volume_icon(0.2, false), "\u{E993}");
        assert_eq!(volume_icon(0.5, false), "\u{E994}");
        assert_eq!(volume_icon(0.9, false), "\u{E995}");
        assert_eq!(volume_icon(0.9, true), "\u{E74F}");
    }

    #[test]
    fn only_a_new_percent_or_mute_is_a_change() {
        assert!(volume_changed(None, (0.5, false)));
        assert!(!volume_changed(Some((0.501, false)), (0.499, false)));
        assert!(volume_changed(Some((0.5, false)), (0.52, false)));
        assert!(volume_changed(Some((0.5, false)), (0.5, true)));
    }

    #[test]
    fn the_bar_sits_right_of_the_text_inside_the_glance() {
        const {
            assert!(LEVEL_TEXT.x + LEVEL_TEXT.width < LEVEL_BAR.x);
            assert!(LEVEL_BAR.x + LEVEL_BAR.width <= PEEK.0 - 14.0);
            assert!(LEVEL_BAR.y + LEVEL_BAR.height < PEEK.1);
        }
    }
}
