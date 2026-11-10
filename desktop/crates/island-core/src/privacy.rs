//! The microphone and the camera in use, as Windows' privacy settings keep
//! it: under `CapabilityAccessManager\ConsentStore\<device>` each app that
//! used one has a key, with `LastUsedTimeStop` 0 while it still does.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Microphone,
    Camera,
}

impl Device {
    pub const ALL: [Device; 2] = [Device::Microphone, Device::Camera];

    /// Its key under `ConsentStore`.
    pub fn key(self) -> &'static str {
        match self {
            Device::Microphone => "microphone",
            Device::Camera => "webcam",
        }
    }

    /// Segoe Fluent Icons (and MDL2).
    pub fn icon(self) -> &'static str {
        match self {
            Device::Microphone => "\u{E720}",
            Device::Camera => "\u{E722}",
        }
    }

    /// The icon's colour (linear RGB): orange for the microphone, green for
    /// the camera, as their indicator lights usually are.
    pub fn tint(self) -> [f32; 3] {
        match self {
            Device::Microphone => [1.0, 0.62, 0.2],
            Device::Camera => [0.3, 0.85, 0.45],
        }
    }
}

/// An app's key as a name to show: `Microsoft.WindowsCamera_8wekyb3d8bbwe`
/// is `WindowsCamera`, `C:#Program Files#Brave#brave.exe` is `brave`.
pub fn app_name(key: &str) -> &str {
    if let Some((_, file)) = key.rsplit_once('#') {
        let n = file.len();
        return match file.get(n.saturating_sub(4)..) {
            Some(ext) if ext.eq_ignore_ascii_case(".exe") => &file[..n - 4],
            _ => file,
        };
    }
    let family = key.split('_').next().unwrap_or(key);
    family.rsplit('.').next().unwrap_or(family)
}

/// What a change of who uses a device tells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change<'a> {
    /// This app began to use it.
    On(&'a str),
    /// No app uses it any more.
    Off,
}

/// From the apps using the device `before` to those `now`.
pub fn change<'a>(before: &[String], now: &'a [String]) -> Option<Change<'a>> {
    match now.iter().find(|a| !before.contains(a)) {
        Some(a) => Some(Change::On(app_name(a))),
        None if now.is_empty() && !before.is_empty() => Some(Change::Off),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apps_read_by_their_short_name() {
        assert_eq!(
            app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe"),
            "WindowsCamera"
        );
        assert_eq!(
            app_name("C:#Program Files#BraveSoftware#Brave-Browser#Application#brave.exe"),
            "brave"
        );
        assert_eq!(app_name("C:#Tools#rec.EXE"), "rec");
        assert_eq!(app_name("C:#Tools#a"), "a");
        assert_eq!(app_name("zoom"), "zoom");
    }

    #[test]
    fn a_new_app_or_the_last_one_leaving_is_a_change() {
        let none: Vec<String> = vec![];
        let brave = vec!["C:#B#brave.exe".to_string()];
        let both = vec!["C:#B#brave.exe".to_string(), "Zoom_x".to_string()];
        assert_eq!(change(&none, &brave), Some(Change::On("brave")));
        assert_eq!(change(&brave, &both), Some(Change::On("Zoom")));
        assert_eq!(change(&both, &brave), None);
        assert_eq!(change(&brave, &none), Some(Change::Off));
        assert_eq!(change(&none, &none), None);
        assert_eq!(change(&brave, &brave), None);
    }
}
