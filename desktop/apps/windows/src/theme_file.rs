//! `%LOCALAPPDATA%\thealejandro\temas\actual.json`: the colors of the
//! Windows theme applied with `tema` (windows-themes). Only read, and
//! watched while a setting follows the theme.

use std::path::PathBuf;

use island_core::theme::{self, ThemeColors};
use windows::Win32::Foundation::HWND;

const NAME: &str = "actual.json";

fn folder() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("thealejandro").join("temas"))
}

/// The theme's colors, none with no theme applied (never applied, or
/// uninstalled); `None` while the file can't be used (keep the ones in use).
pub fn load() -> Option<ThemeColors> {
    let path = folder()?.join(NAME);
    match std::fs::read(path) {
        Ok(b) if b.len() > theme::MAX_FILE => None,
        Ok(b) => ThemeColors::parse(&String::from_utf8_lossy(&b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(ThemeColors::default()),
        // Held for a moment by another app (an antivirus): tried again.
        Err(_) => None,
    }
}

/// Posts `msg` to `hwnd` when the theme changes. `false` while `tema`
/// isn't installed (its folder isn't made here: it is the tool's own).
pub fn watch(hwnd: HWND, msg: u32) -> bool {
    folder().is_some_and(|f| crate::settings_file::watch_file(f, NAME, hwnd, msg))
}
