//! Windows' own banners, turned off app by app when "Notifications only in
//! the island" is chosen in its menu, and back on when it is chosen again.
//!
//! It is the same switch as Settings > System > Notifications > (an app) >
//! "Show notification banners": the `ShowBanner` value under the app's key.
//! Only the apps the island turned off are noted, in
//! `%LOCALAPPDATA%\island\banners.txt`, and only those are turned back on;
//! an app the user had turned off stays so. `banners.on` beside it marks
//! that the switch is chosen. The notifications still go to
//! the notification center.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, REG_DWORD, RRF_RT_REG_DWORD, RegCloseKey,
    RegDeleteKeyValueW, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, RegSetKeyValueW,
};
use windows::core::{HSTRING, PWSTR, w};

const SETTINGS: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings";

/// The file of the apps turned off here.
fn noted() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("island");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("banners.txt"))
}

fn read_noted() -> Vec<String> {
    noted()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| {
            s.lines()
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Writes the notes; `false` if they couldn't be kept.
fn write_noted(ids: &[String]) -> bool {
    let Some(p) = noted() else { return false };
    if ids.is_empty() {
        let _ = std::fs::remove_file(p);
        true
    } else {
        std::fs::write(p, ids.join("\n") + "\n").is_ok()
    }
}

/// One reader or writer of the notes at a time.
static NOTES: Mutex<()> = Mutex::new(());

/// An id that names a key under the settings, and nothing outside them.
fn usable(app_id: &str) -> bool {
    !app_id.is_empty() && !app_id.contains("..") && !app_id.starts_with('\\')
}

/// Windows' own alerts (security, devices, updates) keep their banners.
fn system(app_id: &str) -> bool {
    app_id.starts_with("Windows.SystemToast.") || app_id.starts_with("Windows.Defender")
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

/// Turns off the banner of `app_id`, unless it is off already or one of
/// Windows' own; notes it to be turned back on.
pub fn quiet(app_id: &str) {
    if !usable(app_id) || system(app_id) || show_banner(app_id) == Some(0) {
        return;
    }
    let _notes = NOTES.lock().unwrap_or_else(|e| e.into_inner());
    let mut ids = read_noted();
    if ids.iter().any(|i| i == app_id) {
        return;
    }
    // Noted first: an app turned off and not noted would stay so for good.
    ids.push(app_id.to_string());
    if !write_noted(&ids) {
        return;
    }
    let key = HSTRING::from(format!("{SETTINGS}\\{app_id}"));
    let off = 0u32;
    let e = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &key,
            w!("ShowBanner"),
            REG_DWORD.0,
            Some((&off as *const u32).cast()),
            4,
        )
    };
    if e != ERROR_SUCCESS {
        ids.pop();
        write_noted(&ids);
    }
}

/// Every app that has sent notifications before (a key under the settings).
fn known() -> Vec<String> {
    let mut key = HKEY::default();
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(SETTINGS),
            None,
            KEY_READ,
            &mut key,
        )
    };
    if opened != ERROR_SUCCESS {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0.. {
        let mut buf = [0u16; 512];
        let mut n = buf.len() as u32;
        let e = unsafe {
            RegEnumKeyExW(
                key,
                i,
                Some(PWSTR(buf.as_mut_ptr())),
                &mut n,
                None,
                None,
                None,
                None,
            )
        };
        if e != ERROR_SUCCESS {
            break;
        }
        out.push(String::from_utf16_lossy(&buf[..n as usize]));
    }
    unsafe {
        let _ = RegCloseKey(key);
    }
    out
}

/// Whether the banners should be off now, for [`apply`].
static WANT: AtomicBool = AtomicBool::new(false);
/// One turning off or back on at a time.
static TURNING: Mutex<()> = Mutex::new(());

/// Notes that "Notifications only in the island" is chosen (`true`) or not.
/// It holds while the island is closed: the banners come back with it
/// closed, and go again as it starts ([`apply`]).
pub fn choose(on: bool) {
    let Some(m) = marker() else { return };
    let _ = if on {
        std::fs::write(m, "")
    } else {
        std::fs::remove_file(m)
    };
}

/// Turns the banners off (`true`) or back on, on a thread of its own. Asked
/// twice in a row, quickly, the last one is what holds: each thread waits
/// for the one before and then does what was asked last, whichever of them
/// the system runs first.
pub fn apply(off: bool) {
    WANT.store(off, Ordering::SeqCst);
    std::thread::spawn(|| {
        let _turning = TURNING.lock().unwrap_or_else(|e| e.into_inner());
        if WANT.load(Ordering::SeqCst) {
            for id in known() {
                quiet(&id);
            }
        } else {
            turn_back_on();
        }
    });
}

/// The banners back on now, before the island closes; the choice stays.
pub fn pause() {
    WANT.store(false, Ordering::SeqCst);
    let _turning = TURNING.lock().unwrap_or_else(|e| e.into_inner());
    turn_back_on();
}

/// The mark that "Notifications only in the island" is chosen.
fn marker() -> Option<PathBuf> {
    Some(noted()?.with_file_name("banners.on"))
}

/// Whether "Notifications only in the island" is chosen.
pub fn active() -> bool {
    // Noted apps alone count too: chosen before the mark existed.
    marker().is_some_and(|m| m.exists()) || {
        let _notes = NOTES.lock().unwrap_or_else(|e| e.into_inner());
        !read_noted().is_empty()
    }
}

/// Not chosen any more, and the banners back on.
pub fn restore() {
    choose(false);
    pause();
}

/// Turns back on the banners turned off here, and forgets them.
fn turn_back_on() {
    let _notes = NOTES.lock().unwrap_or_else(|e| e.into_inner());
    let ids = read_noted();
    for id in &ids {
        // Only if still as the island left it: the user may have changed it.
        if show_banner(id) == Some(0) {
            let key = HSTRING::from(format!("{SETTINGS}\\{id}"));
            unsafe {
                let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &key, w!("ShowBanner"));
            }
        }
    }
    write_noted(&[]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_apps_ids_and_not_windows_own() {
        assert!(usable("Microsoft.WindowsTerminal_8wekyb3d8bbwe!App"));
        assert!(usable("Chrome"));
        assert!(!usable(""));
        assert!(!usable("..\\..\\Run"));
        assert!(!usable("\\Software"));
        assert!(system("Windows.SystemToast.SecurityAndMaintenance"));
        assert!(!system("Microsoft.Office.OUTLOOK.EXE.15"));
    }
}
