//! The Windows theme applied with `tema` (windows-themes): its colors, as
//! the `actual.json` it leaves in `%LOCALAPPDATA%\thealejandro\temas`, and
//! the island's colors from the settings and that theme.

use serde_json::Value;

use crate::color::{self, Rgb, WHITE};
use crate::settings::{Accent, IslandColor};

/// Bigger than any real `actual.json`: anything past it is not read.
pub const MAX_FILE: usize = 64 * 1024;

/// What the island takes from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ThemeColors {
    pub accent: Option<Rgb>,
    pub background: Option<Rgb>,
}

impl ThemeColors {
    /// `actual.json`'s text; `None` when it isn't a JSON object (half saved).
    /// A version other than 1 gives no colors: the island's own then.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim_start_matches('\u{feff}');
        if text.len() > MAX_FILE {
            return None;
        }
        let Ok(Value::Object(o)) = serde_json::from_str::<Value>(text) else {
            return None;
        };
        if o.get("version").and_then(Value::as_u64) != Some(1) {
            return Some(Self::default());
        }
        let hex = |k: &str| o.get(k).and_then(Value::as_str).and_then(color::parse_hex);
        Some(Self {
            accent: hex("accent"),
            background: hex("background"),
        })
    }
}

/// The levels' color on the island's `body`: `None` for the system's
/// accent (made legible on it where drawn).
pub fn accent(chosen: Accent, theme: Option<&ThemeColors>, body: Rgb) -> Option<Rgb> {
    match chosen {
        Accent::System => None,
        Accent::White => Some(WHITE),
        Accent::Theme => theme
            .and_then(|t| t.accent)
            .map(|c| color::legible_on(c, body)),
        // Too dark, it would hardly show on the island.
        Accent::Color(c) => Some(color::legible_on(c, body)),
    }
}

/// The island's body: dark enough, whatever was chosen, for its white text.
pub fn body(chosen: IslandColor, theme: Option<&ThemeColors>) -> Rgb {
    match chosen {
        IslandColor::Black => [0.0; 3],
        IslandColor::Theme => theme
            .and_then(|t| t.background)
            .map_or([0.0; 3], color::under_white_text),
        IslandColor::Color(c) => color::under_white_text(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AERO: &str = r##"{
        "version": 1, "slug": "aero", "mode": "light",
        "accent": "#0073b7", "background": "#eef8fc", "foreground": "#0f2a3a"
    }"##;

    #[test]
    fn reads_the_theme_s_colors() {
        let t = ThemeColors::parse(&format!("\u{feff}{AERO}")).unwrap();
        assert_eq!(t.accent, color::parse_hex("#0073b7"));
        assert_eq!(t.background, color::parse_hex("#eef8fc"));
        assert_eq!(
            ThemeColors::parse(r#"{"version": 1, "accent": 3}"#),
            Some(ThemeColors::default())
        );
        // A later format it doesn't know.
        assert_eq!(
            ThemeColors::parse(r##"{"version": 2, "accent": "#0073b7"}"##),
            Some(ThemeColors::default())
        );
        assert_eq!(ThemeColors::parse("{ half"), None);
    }

    #[test]
    fn the_island_s_colors_from_settings_and_theme() {
        let t = ThemeColors::parse(AERO).unwrap();
        assert_eq!(body(IslandColor::Black, Some(&t)), [0.0; 3]);
        // No theme applied: black, and the system's accent.
        assert_eq!(body(IslandColor::Theme, None), [0.0; 3]);
        assert_eq!(accent(Accent::Theme, None, [0.0; 3]), None);
        assert_eq!(accent(Accent::System, Some(&t), [0.0; 3]), None);
        // A light theme's background is darkened for the white text.
        let b = body(IslandColor::Theme, Some(&t));
        assert!(b != [0.0; 3] && b[2] > b[0] && b.iter().all(|v| *v < 0.5));
        // Aero's accent reads on black as is.
        assert_eq!(accent(Accent::Theme, Some(&t), [0.0; 3]), t.accent);
        // On the theme's own body, lightened until it shows there.
        let a = accent(Accent::Theme, Some(&t), b).unwrap();
        assert!(color::contrast(a, b) >= color::LEGIBLE);
    }
}
