//! The desktop apps' words, in English and Spanish.

use crate::settings::Language;

#[derive(Debug, PartialEq, Eq)]
pub struct Texts {
    pub nothing_playing: &'static str,
    /// The tray icon's tooltip.
    pub name: &'static str,
    pub lyrics: &'static str,
    pub open_lyrics: &'static str,
    /// Windows' banners off: notifications only in the island.
    pub only_island: &'static str,
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
    // The glances.
    pub volume: &'static str,
    pub muted: &'static str,
    pub microphone_on: &'static str,
    pub microphone_off: &'static str,
    pub camera_on: &'static str,
    pub camera_off: &'static str,
    pub connected: &'static str,
    pub disconnected: &'static str,
    /// A notification whose app has no name.
    pub notification: &'static str,
    /// When a notification arrived: just now.
    pub now: &'static str,
    /// After installing: the glance, and where the island now is.
    pub installed: &'static str,
    pub in_start_menu: &'static str,
    /// Opened from outside its install folder: what it offers.
    pub install: &'static str,
    pub update: &'static str,
    pub portable: &'static str,
    /// The install failed: the glance, and what is left.
    pub install_failed: &'static str,
    pub still_portable: &'static str,
    /// An update failed: the installed one is back.
    pub update_failed: &'static str,
    pub still_installed: &'static str,
    /// "Check for updates" in the menu, and while it looks.
    pub check_updates: &'static str,
    pub checking: &'static str,
    /// What it found: the glances.
    pub up_to_date: &'static str,
    pub no_connection: &'static str,
    pub check_failed: &'static str,
    pub try_later: &'static str,
    /// A copy that doesn't update itself: where the new one is.
    pub new_version: &'static str,
    pub on_github: &'static str,
    /// The found update's offer: not now (asked again next time).
    pub not_now: &'static str,
}

pub const EN: Texts = Texts {
    nothing_playing: "Nothing playing",
    name: "Island",
    lyrics: "Synced lyrics",
    open_lyrics: "Lyrics in the open island",
    only_island: "Notifications only in the island",
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
    volume: "Volume",
    muted: "Muted",
    microphone_on: "Microphone in use",
    microphone_off: "Microphone off",
    camera_on: "Camera in use",
    camera_off: "Camera off",
    connected: "Connected",
    disconnected: "Disconnected",
    notification: "Notification",
    now: "now",
    installed: "Island installed",
    in_start_menu: "In the Start menu",
    install: "Install Island",
    update: "Update Island",
    portable: "Use without installing",
    install_failed: "Couldn't install Island",
    still_portable: "It runs without installing",
    update_failed: "Couldn't update Island",
    still_installed: "The old one stays",
    check_updates: "Check for updates",
    checking: "Checking…",
    up_to_date: "Island is up to date",
    no_connection: "No connection",
    check_failed: "Couldn't check",
    try_later: "Try again later",
    new_version: "A new version",
    on_github: "on GitHub",
    not_now: "Not now",
};

pub const ES: Texts = Texts {
    nothing_playing: "Nada en reproducción",
    name: "Island",
    lyrics: "Letras sincronizadas",
    open_lyrics: "Letra en la isla abierta",
    only_island: "Notificaciones solo en la isla",
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
    volume: "Volumen",
    muted: "Silenciado",
    microphone_on: "Micrófono en uso",
    microphone_off: "Micrófono apagado",
    camera_on: "Cámara en uso",
    camera_off: "Cámara apagada",
    connected: "Conectado",
    disconnected: "Desconectado",
    notification: "Notificación",
    now: "ahora",
    installed: "Isla instalada",
    in_start_menu: "En el menú Inicio",
    install: "Instalar Island",
    update: "Actualizar Island",
    portable: "Usar sin instalar",
    install_failed: "No se pudo instalar Island",
    still_portable: "Sigue sin instalar",
    update_failed: "No se pudo actualizar Island",
    still_installed: "Sigue la anterior",
    check_updates: "Buscar actualizaciones",
    checking: "Buscando…",
    up_to_date: "Island está al día",
    no_connection: "Sin conexión",
    check_failed: "No se pudo buscar",
    try_later: "Prueba más tarde",
    new_version: "Versión nueva",
    on_github: "en GitHub",
    not_now: "Ahora no",
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
                t.only_island,
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
                t.volume,
                t.muted,
                t.microphone_on,
                t.microphone_off,
                t.camera_on,
                t.camera_off,
                t.connected,
                t.disconnected,
                t.installed,
                t.in_start_menu,
                t.install,
                t.update,
                t.portable,
                t.install_failed,
                t.still_portable,
                t.update_failed,
                t.still_installed,
                t.check_updates,
                t.checking,
                t.up_to_date,
                t.no_connection,
                t.check_failed,
                t.try_later,
                t.new_version,
                t.on_github,
                t.not_now,
                t.notification,
                t.now,
            ] {
                assert!(!s.trim().is_empty());
                // The tray's tooltip and menu take up to 127 characters.
                assert!(s.chars().count() < 64);
            }
        }
    }
}
