//! The desktop island's settings, as `settings.json` keeps them: pure
//! parsing and editing, so the file can be read on any machine's tests.
//!
//! Like `IslandSettings.js` in the plugin, anything missing or invalid
//! falls back to its default, key by key: a typo never loses the rest.
//! Keys shared with the plugin keep its names (`language`, `lyrics`,
//! `openWidth`).

use serde_json::{Map, Value};

use crate::color;
use crate::focus::Lengths;
use crate::media::{self, SongText, TextAlign};
use crate::monitor::MonitorChoice;
use crate::pages::Page;
use crate::shape::Placement;

/// Bigger than any real settings file: anything past it is not read.
pub const MAX_FILE: usize = 64 * 1024;

/// The texts' language: the system's unless chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    System,
    En,
    Es,
}

impl Language {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "" => Some(Self::System),
            "en" => Some(Self::En),
            "es" => Some(Self::Es),
            _ => None,
        }
    }

    /// English or Spanish, given the system's locale name (`es-MX`).
    pub fn resolve(self, locale: &str) -> Self {
        match self {
            Self::System if locale.to_ascii_lowercase().starts_with("es") => Self::Es,
            Self::System => Self::En,
            chosen => chosen,
        }
    }
}

/// The color of the glances' levels (volume and the like) and of a
/// notification with no logo of its own.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Accent {
    /// Windows' accent, followed when it changes.
    #[default]
    System,
    White,
    /// The accent of the Windows theme applied with `tema` (its
    /// `actual.json`); the system's while there is none.
    Theme,
    /// `#rrggbb` or `#rgb`, as 0 to 1.
    Color([f32; 3]),
}

impl Accent {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "system" => Some(Self::System),
            "white" => Some(Self::White),
            "theme" => Some(Self::Theme),
            _ => color::parse_hex(s).map(Self::Color),
        }
    }
}

/// The island's own color: black like a notch by default.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum IslandColor {
    #[default]
    Black,
    /// The background of the Windows theme applied with `tema`; black
    /// while there is none.
    Theme,
    Color([f32; 3]),
}

impl IslandColor {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "black" => Some(Self::Black),
            "theme" => Some(Self::Theme),
            _ => color::parse_hex(s).map(Self::Color),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub language: Language,
    pub placement: Placement,
    pub monitor: MonitorChoice,
    pub text_align: TextAlign,
    pub song_text: SongText,
    /// Synced lyrics from lrclib.net (off: it sends the song's name there).
    pub lyrics: bool,
    /// The open island shows them in a column beside the player.
    pub open_lyrics: bool,
    /// The open island's width; `None`: by whether there is a column.
    pub open_width: Option<f32>,
    /// The page the island opens on, until another is chosen.
    pub default_page: Page,
    /// The focus timer's lengths.
    pub focus: Lengths,
    /// A glance on each change of the system's volume.
    pub volume_glance: bool,
    /// A glance when an app starts or stops using the microphone or camera.
    pub privacy_glance: bool,
    /// A glance when a paired Bluetooth device connects or disconnects.
    pub bluetooth_glance: bool,
    /// A glance when a notification shows: its app and title.
    pub notification_glance: bool,
    /// Those glances while an app is in full screen too (never while
    /// presenting or in a game).
    pub notifications_in_full_screen: bool,
    /// How late the lyrics follow the position the player tells, in ms:
    /// the sound comes out later than that (more so over Bluetooth).
    /// `None` ("auto"): by the speakers in use, see [`Settings::lyrics_delay`].
    pub lyrics_delay: Option<u32>,
    /// How big the closed island and its glances are, times this; the open
    /// island keeps its size.
    pub island_scale: f32,
    pub accent: Accent,
    pub island_color: IslandColor,
}

/// `lyricsDelay`'s range, in ms.
pub const LYRICS_DELAY: (u32, u32) = (0, 2000);
/// `islandScale`'s range: the closed island and its glances, times this.
pub const ISLAND_SCALE: (f32, f32) = (0.8, 1.4);
/// "auto" over Bluetooth speakers or headphones: what they add, about.
/// Wired ones add next to nothing.
pub const BLUETOOTH_DELAY: u32 = 200;

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::default(),
            placement: Placement::default(),
            monitor: MonitorChoice::default(),
            text_align: TextAlign::default(),
            song_text: SongText::default(),
            lyrics: false,
            open_lyrics: true,
            open_width: None,
            default_page: Page::default(),
            focus: Lengths::default(),
            volume_glance: true,
            privacy_glance: true,
            bluetooth_glance: true,
            notification_glance: true,
            notifications_in_full_screen: true,
            lyrics_delay: None,
            island_scale: 1.0,
            accent: Accent::System,
            island_color: IslandColor::Black,
        }
    }
}

/// What a new `settings.json` holds: every key, at its default, so they
/// can be found and changed by hand.
pub const TEMPLATE: &str = r#"{
  "language": "",
  "placement": "top",
  "monitor": "primary",
  "textAlign": "center",
  "songText": "title-artist",
  "lyrics": false,
  "openLyrics": true,
  "openWidth": "auto",
  "defaultPage": "home",
  "islandScale": 1.0,
  "accent": "system",
  "islandColor": "black",
  "focusMinutes": 25,
  "breakMinutes": 5,
  "longBreakMinutes": 15,
  "longBreakEvery": 4,
  "volumeGlance": true,
  "privacyGlance": true,
  "bluetoothGlance": true,
  "notificationGlance": true,
  "notificationsInFullScreen": true,
  "lyricsDelay": "auto"
}
"#;

impl Settings {
    /// Reads `settings.json`'s text; whatever can't be used is the default.
    /// `None` when it isn't a JSON object at all (half saved, a missing
    /// comma): better to keep the settings in use than to drop them all.
    /// Empty text is a file with nothing set yet.
    pub fn parse(text: &str) -> Option<Self> {
        let d = Self::default();
        let text = text.trim_start_matches('\u{feff}');
        if text.trim().is_empty() {
            return Some(d);
        }
        if text.len() > MAX_FILE {
            return None;
        }
        let Ok(Value::Object(o)) = serde_json::from_str::<Value>(text) else {
            return None;
        };
        let text = |k: &str| o.get(k).and_then(Value::as_str);
        let flag = |k: &str, def: bool| o.get(k).and_then(Value::as_bool).unwrap_or(def);
        // Whole minutes in range; anything else is the default.
        let minutes = |k: &str, (lo, hi): (u32, u32), def: u32| {
            o.get(k)
                .and_then(Value::as_u64)
                .filter(|m| (u64::from(lo)..=u64::from(hi)).contains(m))
                .map_or(def, |m| m as u32)
        };
        let df = d.focus;
        Some(Self {
            language: text("language")
                .and_then(Language::parse)
                .unwrap_or(d.language),
            placement: text("placement")
                .and_then(Placement::parse)
                .unwrap_or(d.placement),
            // A screen's number may come as a number or as text; "all" or
            // a list ([1, 3] in JSON, "1,3" as text) puts an island on each.
            monitor: match o.get("monitor") {
                Some(Value::String(s)) => MonitorChoice::parse(s),
                Some(Value::Number(n)) => MonitorChoice::parse(&n.to_string()),
                Some(Value::Array(a)) => {
                    let ns: Option<Vec<usize>> = a
                        .iter()
                        .map(|v| {
                            v.as_u64()
                                .and_then(|n| usize::try_from(n).ok())
                                .filter(|n| *n >= 1)
                        })
                        .collect();
                    ns.filter(|ns| !ns.is_empty()).map(MonitorChoice::List)
                }
                _ => None,
            }
            .unwrap_or(d.monitor),
            text_align: text("textAlign")
                .and_then(TextAlign::parse)
                .unwrap_or(d.text_align),
            song_text: text("songText")
                .and_then(SongText::parse)
                .unwrap_or(d.song_text),
            lyrics: flag("lyrics", d.lyrics),
            open_lyrics: flag("openLyrics", d.open_lyrics),
            open_width: o
                .get("openWidth")
                .and_then(Value::as_f64)
                .and_then(|w| media::parse_open_width(&w.to_string())),
            default_page: text("defaultPage")
                .and_then(Page::parse)
                .unwrap_or(d.default_page),
            focus: Lengths {
                focus: minutes("focusMinutes", Lengths::FOCUS, df.focus),
                short: minutes("breakMinutes", Lengths::SHORT, df.short),
                long: minutes("longBreakMinutes", Lengths::LONG, df.long),
                every: minutes("longBreakEvery", Lengths::EVERY, df.every),
            },
            volume_glance: flag("volumeGlance", d.volume_glance),
            privacy_glance: flag("privacyGlance", d.privacy_glance),
            bluetooth_glance: flag("bluetoothGlance", d.bluetooth_glance),
            notification_glance: flag("notificationGlance", d.notification_glance),
            notifications_in_full_screen: flag(
                "notificationsInFullScreen",
                d.notifications_in_full_screen,
            ),
            // A number in range; "auto" or anything else: by the speakers.
            lyrics_delay: o
                .get("lyricsDelay")
                .and_then(Value::as_u64)
                .filter(|m| (u64::from(LYRICS_DELAY.0)..=u64::from(LYRICS_DELAY.1)).contains(m))
                .map(|m| m as u32),
            // A number is clamped into range; anything else: 1.0 (as made).
            island_scale: o
                .get("islandScale")
                .and_then(Value::as_f64)
                .map_or(d.island_scale, |v| {
                    (v as f32).clamp(ISLAND_SCALE.0, ISLAND_SCALE.1)
                }),
            accent: text("accent").and_then(Accent::parse).unwrap_or(d.accent),
            island_color: text("islandColor")
                .and_then(IslandColor::parse)
                .unwrap_or(d.island_color),
        })
    }

    /// The lyrics' delay in ms: the one chosen, or by whether the sound goes
    /// out over Bluetooth.
    pub fn lyrics_delay(&self, bluetooth: bool) -> u32 {
        self.lyrics_delay
            .unwrap_or(if bluetooth { BLUETOOTH_DELAY } else { 0 })
    }

    /// The open island's width setting: the chosen one, or 560 (with a
    /// column of lyrics or without, so turning them on doesn't resize it).
    pub fn open_width(&self) -> f32 {
        self.open_width
            .unwrap_or(if self.lyrics && self.open_lyrics {
                media::OPEN_LYRICS_WIDTH
            } else {
                media::OPEN_WIDTH
            })
    }
}

/// `text` with one key set to `value`, everything else kept as it was. Text
/// that isn't a JSON object starts over as one.
pub fn set(text: &str, key: &str, value: Value) -> String {
    let mut o = match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
        Ok(Value::Object(o)) if text.len() <= MAX_FILE => o,
        _ => Map::new(),
    };
    o.insert(key.to_string(), value);
    let mut out = serde_json::to_string_pretty(&Value::Object(o)).unwrap_or_default();
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(text: &str) -> Settings {
        Settings::parse(text).unwrap()
    }

    #[test]
    fn the_accent_is_the_system_white_or_a_color() {
        assert_eq!(Settings::default().accent, Accent::System);
        assert_eq!(parse(r#"{"accent": "white"}"#).accent, Accent::White);
        assert_eq!(parse(r#"{"accent": "theme"}"#).accent, Accent::Theme);
        assert_eq!(Settings::default().island_color, IslandColor::Black);
        assert_eq!(
            parse(r#"{"islandColor": "Theme"}"#).island_color,
            IslandColor::Theme
        );
        assert_eq!(
            parse(r##"{"islandColor": "#102030"}"##).island_color,
            IslandColor::Color([16.0 / 255.0, 32.0 / 255.0, 48.0 / 255.0])
        );
        assert_eq!(
            parse(r#"{"islandColor": "white"}"#).island_color,
            IslandColor::Black
        );
        assert_eq!(parse(r#"{"accent": " System "}"#).accent, Accent::System);
        assert_eq!(
            parse(r##"{"accent": "#ff8000"}"##).accent,
            Accent::Color([1.0, 128.0 / 255.0, 0.0])
        );
        assert_eq!(
            parse(r##"{"accent": "#0F0"}"##).accent,
            Accent::Color([0.0, 1.0, 0.0])
        );
        for bad in [
            r#""red""#,
            r##""#12345""##,
            r##""#gggggg""##,
            "3",
            r##""#+1+2+3""##,
        ] {
            let s = parse(&format!(r#"{{"accent": {bad}}}"#));
            assert_eq!(s.accent, Accent::System, "{bad}");
        }
    }

    #[test]
    fn empty_files_give_the_defaults_and_broken_ones_nothing() {
        for text in ["", "  \n", "\u{feff}", "{}", TEMPLATE] {
            assert_eq!(Settings::parse(text), Some(Settings::default()), "{text:?}");
        }
        for text in [
            "nope",
            "[1, 2]",
            "null",
            "{\"lyrics\": ",
            "{\"lyrics\": true,}",
        ] {
            assert_eq!(Settings::parse(text), None, "{text:?}");
        }
        let d = Settings::default();
        assert!(!d.lyrics && d.open_lyrics && d.open_width.is_none());
        assert_eq!(d.open_width(), media::OPEN_WIDTH);
    }

    #[test]
    fn each_key_is_read_on_its_own() {
        let s = parse(
            r#"{
                "language": "es", "placement": "floating", "monitor": 2,
                "textAlign": "left", "songText": "title", "lyrics": true,
                "openLyrics": false, "openWidth": 544, "other": 1
            }"#,
        );
        assert_eq!(s.language, Language::Es);
        assert_eq!(s.placement, Placement::Floating);
        assert_eq!(s.monitor, MonitorChoice::parse("2").unwrap());
        assert_eq!(s.text_align, TextAlign::Left);
        assert_eq!(s.song_text, SongText::Title);
        assert!(s.lyrics && !s.open_lyrics);
        assert_eq!(s.open_width, Some(540.0));
        // A bad value loses only itself.
        let s = parse(r#"{"language": "fr", "lyrics": "yes", "placement": "top-left"}"#);
        assert_eq!(s.language, Language::System);
        assert!(!s.lyrics);
        assert_eq!(s.placement, Placement::TopLeft);
        // "auto", or anything that isn't a number: by the lyrics.
        assert_eq!(parse(r#"{"openWidth": "auto"}"#).open_width, None);
        assert_eq!(
            parse(r#"{"monitor": "1"}"#).monitor,
            MonitorChoice::Number(1)
        );
        // The focus timer: whole minutes in range, each on its own.
        let s = parse(
            r#"{"defaultPage": "focus", "focusMinutes": 50, "breakMinutes": 0,
                "longBreakMinutes": 20.5, "longBreakEvery": 3}"#,
        );
        assert_eq!(s.default_page, Page::Focus);
        assert_eq!(
            s.focus,
            Lengths {
                focus: 50,
                short: 5,
                long: 15,
                every: 3
            }
        );
        assert_eq!(parse(r#"{"defaultPage": "nope"}"#).default_page, Page::Home);
        assert!(!parse(r#"{"volumeGlance": false}"#).volume_glance);
        assert!(parse(r#"{"volumeGlance": 0}"#).volume_glance);
        assert!(!parse(r#"{"privacyGlance": false}"#).privacy_glance);
        assert!(!parse(r#"{"bluetoothGlance": false}"#).bluetooth_glance);
        assert!(!parse(r#"{"notificationGlance": false}"#).notification_glance);
        assert!(!parse(r#"{"notificationsInFullScreen": false}"#).notifications_in_full_screen);
        // The lyrics' delay: a number wins over the speakers; else by them.
        let s = parse(r#"{"lyricsDelay": 350}"#);
        assert_eq!((s.lyrics_delay(true), s.lyrics_delay(false)), (350, 350));
        assert_eq!(parse(r#"{"lyricsDelay": 0}"#).lyrics_delay(true), 0);
        for auto in [r#""auto""#, "-5", "9000", r#""fast""#] {
            let s = parse(&format!(r#"{{"lyricsDelay": {auto}}}"#));
            assert_eq!(s.lyrics_delay(true), BLUETOOTH_DELAY, "{auto}");
            assert_eq!(s.lyrics_delay(false), 0, "{auto}");
        }
        // The island's scale: a number is clamped into range; else 1.0.
        assert_eq!(parse(r#"{"islandScale": 1.2}"#).island_scale, 1.2);
        assert_eq!(
            parse(r#"{"islandScale": 2.5}"#).island_scale,
            ISLAND_SCALE.1
        );
        assert_eq!(
            parse(r#"{"islandScale": 0.5}"#).island_scale,
            ISLAND_SCALE.0
        );
        assert_eq!(parse(r#"{"islandScale": "big"}"#).island_scale, 1.0);
        assert_eq!(parse(r#"{"islandScale": true}"#).island_scale, 1.0);
        assert_eq!(Settings::default().island_scale, 1.0);
        // A BOM (Notepad's UTF-8) is fine.
        assert!(parse("\u{feff}{\"lyrics\": true}").lyrics);
        // Too big to be a settings file.
        let huge = format!("{{\"lyrics\": true, \"x\": \"{}\"}}", "a".repeat(MAX_FILE));
        assert_eq!(Settings::parse(&huge), None);
    }

    #[test]
    fn the_monitor_takes_a_number_all_or_a_list() {
        assert_eq!(
            parse(r#"{"monitor": "primary"}"#).monitor,
            MonitorChoice::Primary
        );
        assert_eq!(parse(r#"{"monitor": 2}"#).monitor, MonitorChoice::Number(2));
        assert_eq!(parse(r#"{"monitor": "all"}"#).monitor, MonitorChoice::All);
        // A list: an array in JSON, comma-separated as text.
        assert_eq!(
            parse(r#"{"monitor": [1, 3]}"#).monitor,
            MonitorChoice::List(vec![1, 3])
        );
        assert_eq!(
            parse(r#"{"monitor": "1,3"}"#).monitor,
            MonitorChoice::List(vec![1, 3])
        );
        assert_eq!(
            parse(r#"{"monitor": "2, 1"}"#).monitor,
            MonitorChoice::List(vec![2, 1])
        );
        // A bad value is the default alone.
        for bad in [
            r#""0""#,
            r#""nope""#,
            r#""1,0""#,
            r#""1,x""#,
            "-1",
            "1.5",
            "[]",
            "[0]",
            r#"[1, "a"]"#,
            "true",
        ] {
            let s = parse(&format!(r#"{{"monitor": {bad}}}"#));
            assert_eq!(s.monitor, MonitorChoice::Primary, "{bad}");
        }
    }

    #[test]
    fn the_width_follows_the_lyrics_unless_chosen() {
        let s = parse(r#"{"lyrics": true}"#);
        assert_eq!(s.open_width(), media::OPEN_LYRICS_WIDTH);
        let s = parse(r#"{"lyrics": true, "openLyrics": false}"#);
        assert_eq!(s.open_width(), media::OPEN_WIDTH);
        let s = parse(r#"{"lyrics": true, "openWidth": 5000}"#);
        assert_eq!(s.open_width(), media::OPEN_WIDTH_MAX);
    }

    #[test]
    fn setting_a_key_keeps_the_rest() {
        let text = set(r#"{"lyrics": false, "mine": [1]}"#, "lyrics", json!(true));
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["lyrics"], json!(true));
        assert_eq!(v["mine"], json!([1]));
        assert!(parse(&text).lyrics);
        let text = set(TEMPLATE, "openLyrics", json!(false));
        assert!(!parse(&text).open_lyrics);
        // The keys stay where they were.
        let keys: Vec<String> = serde_json::from_str::<Map<String, Value>>(&text)
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let template: Vec<String> = serde_json::from_str::<Map<String, Value>>(TEMPLATE)
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys, template);
        assert_eq!(keys[0], "language");
        // Nothing usable there: a new object.
        let text = set("nope", "openLyrics", json!(false));
        assert!(!parse(&text).open_lyrics);
    }

    #[test]
    fn language_resolves_from_the_system() {
        assert_eq!(Language::System.resolve("es-MX"), Language::Es);
        assert_eq!(Language::System.resolve("ES-es"), Language::Es);
        assert_eq!(Language::System.resolve("en-US"), Language::En);
        assert_eq!(Language::System.resolve(""), Language::En);
        assert_eq!(Language::En.resolve("es-MX"), Language::En);
        assert_eq!(Language::parse(""), Some(Language::System));
    }
}
