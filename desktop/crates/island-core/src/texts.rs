//! The desktop apps' words, in English and Spanish.

use crate::settings::Language;

#[derive(Debug, PartialEq, Eq)]
pub struct Texts {
    pub nothing_playing: &'static str,
    /// The tray icon's tooltip.
    pub name: &'static str,
    pub lyrics: &'static str,
    pub open_lyrics: &'static str,
    pub start_with_windows: &'static str,
    pub open_settings: &'static str,
    pub quit: &'static str,
}

pub const EN: Texts = Texts {
    nothing_playing: "Nothing playing",
    name: "Island",
    lyrics: "Synced lyrics",
    open_lyrics: "Lyrics in the open island",
    start_with_windows: "Start with Windows",
    open_settings: "Open settings",
    quit: "Quit",
};

pub const ES: Texts = Texts {
    nothing_playing: "Nada en reproducción",
    name: "Island",
    lyrics: "Letras sincronizadas",
    open_lyrics: "Letra en la isla abierta",
    start_with_windows: "Iniciar con Windows",
    open_settings: "Abrir ajustes",
    quit: "Salir",
};

/// The texts for `language`, the system's being `locale` (`es-MX`).
pub fn texts(language: Language, locale: &str) -> &'static Texts {
    match language.resolve(locale) {
        Language::Es => &ES,
        _ => &EN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_by_setting_then_system() {
        assert_eq!(texts(Language::System, "es-ES"), &ES);
        assert_eq!(texts(Language::System, "de-DE"), &EN);
        assert_eq!(texts(Language::En, "es-ES"), &EN);
        assert_eq!(texts(Language::Es, "en-US"), &ES);
    }

    #[test]
    fn every_text_is_there() {
        for t in [&EN, &ES] {
            for s in [
                t.nothing_playing,
                t.name,
                t.lyrics,
                t.open_lyrics,
                t.start_with_windows,
                t.open_settings,
                t.quit,
            ] {
                assert!(!s.trim().is_empty());
                // The tray's tooltip and menu take up to 127 characters.
                assert!(s.chars().count() < 64);
            }
        }
    }
}
