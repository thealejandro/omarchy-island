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
    // The focus timer. `{}` stands for a number.
    pub focus: &'static str,
    pub short_break: &'static str,
    pub long_break: &'static str,
    /// "round 2 of 4".
    pub round: &'static str,
    /// What comes after focus, and after a break.
    pub then_break: &'static str,
    pub then_focus: &'static str,
    pub focus_done: &'static str,
    /// "{} min break".
    pub take_break: &'static str,
    pub break_over: &'static str,
    pub ready: &'static str,
    /// Under the time while it's stopped.
    pub wheel_hint: &'static str,
}

pub const EN: Texts = Texts {
    nothing_playing: "Nothing playing",
    name: "Island",
    lyrics: "Synced lyrics",
    open_lyrics: "Lyrics in the open island",
    start_with_windows: "Start with Windows",
    open_settings: "Open settings",
    quit: "Quit",
    focus: "Focus",
    short_break: "Break",
    long_break: "Long break",
    round: "round {} of {}",
    then_break: "then a {} min break",
    then_focus: "then focus",
    focus_done: "Focus done",
    take_break: "{} min break",
    break_over: "Break over",
    ready: "Ready for another round",
    wheel_hint: "scroll to change",
};

pub const ES: Texts = Texts {
    nothing_playing: "Nada en reproducción",
    name: "Island",
    lyrics: "Letras sincronizadas",
    open_lyrics: "Letra en la isla abierta",
    start_with_windows: "Iniciar con Windows",
    open_settings: "Abrir ajustes",
    quit: "Salir",
    focus: "Enfoque",
    short_break: "Descanso",
    long_break: "Descanso largo",
    round: "ronda {} de {}",
    then_break: "luego {} min de descanso",
    then_focus: "luego, enfoque",
    focus_done: "Enfoque terminado",
    take_break: "Descanso de {} min",
    break_over: "Descanso terminado",
    ready: "Listo para otra ronda",
    wheel_hint: "rueda para cambiar",
};

/// `text` with each `{}` replaced by the next of `args`.
pub fn fill(text: &str, args: &[u32]) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    let mut args = args.iter();
    let mut rest = text;
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        if let Some(a) = args.next() {
            out.push_str(&a.to_string());
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

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
    fn numbers_fill_in() {
        assert_eq!(fill(ES.round, &[2, 4]), "ronda 2 de 4");
        assert_eq!(fill(EN.take_break, &[5]), "5 min break");
        assert_eq!(fill("{} y {}", &[1]), "1 y ");
        assert_eq!(fill("sin nada", &[3]), "sin nada");
        for t in [&EN, &ES] {
            assert_eq!(t.round.matches("{}").count(), 2);
            assert_eq!(t.then_break.matches("{}").count(), 1);
            assert_eq!(t.take_break.matches("{}").count(), 1);
        }
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
                t.focus,
                t.short_break,
                t.long_break,
                t.round,
                t.then_break,
                t.then_focus,
                t.focus_done,
                t.take_break,
                t.break_over,
                t.ready,
                t.wheel_hint,
            ] {
                assert!(!s.trim().is_empty());
                // The tray's tooltip and menu take up to 127 characters.
                assert!(s.chars().count() < 64);
            }
        }
    }
}
