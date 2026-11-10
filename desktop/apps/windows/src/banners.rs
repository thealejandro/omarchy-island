//! Windows' own banners, as an earlier version of the island left them.
//!
//! With `windowsBanners: false` it turned them off app by app (each app's
//! `ShowBanner`, the switch under Settings, System, Notifications), noting
//! each one in `%LOCALAPPDATA%\island\banners.txt`. The island no longer
//! touches them; at start it turns those back on, if still as it left them,
//! and forgets the note. Only for a version or two.

use std::path::PathBuf;

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegDeleteKeyValueW, RegGetValueW,
};
use windows::core::{HSTRING, w};

const SETTINGS: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings";

/// The file of the apps turned off then; nothing is made if there is none.
fn noted() -> Option<PathBuf> {
    Some(
        PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
            .join("island")
            .join("banners.txt"),
    )
}
/// An id that names a key under the settings, and nothing outside them.
fn usable(app_id: &str) -> bool {
    !app_id.is_empty() && !app_id.contains("..") && !app_id.starts_with('\\')
}

/// The app's `ShowBanner`: `None` if it isn't set (the default: on).
fn show_banner(app_id: &str) -> Option<u32> {
    let key = HSTRING::from(format!("{SETTINGS}\\{app_id}"));
    let (mut v, mut size) = (0u32, 4u32);
    let e = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            w!("ShowBanner"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut v as *mut u32).cast()),
            Some(&mut size),
        )
    };
    (e == ERROR_SUCCESS).then_some(v)
}

/// Turns back on the banners an earlier version turned off, and forgets them.
pub fn restore() {
    let Some(path) = noted() else { return };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    for id in text.lines().filter(|l| usable(l)) {
        // Only if still as the island left it: the user may have changed it.
        if show_banner(id) == Some(0) {
            let key = HSTRING::from(format!("{SETTINGS}\\{id}"));
            unsafe {
                let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &key, w!("ShowBanner"));
            }
        }
    }
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_apps_ids() {
        assert!(usable("Microsoft.WindowsTerminal_8wekyb3d8bbwe!App"));
        assert!(usable("Chrome"));
        assert!(!usable(""));
        assert!(!usable("..\\..\\Run"));
        assert!(!usable("\\Software"));
    }
}
