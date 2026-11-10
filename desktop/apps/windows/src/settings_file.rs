//! `%APPDATA%\island\settings.json`: read, written key by key, and watched
//! so a change made by hand applies at once.

use std::path::PathBuf;

use island_core::settings::{self, Settings};
use serde_json::Value;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::HSTRING;

const NAME: &str = "settings.json";

fn folder() -> PathBuf {
    let base = std::env::var_os("APPDATA").map_or_else(|| PathBuf::from("."), PathBuf::from);
    base.join("island")
}

pub fn path() -> PathBuf {
    folder().join(NAME)
}

/// The file's text: empty when there is none, `None` when it can't be read
/// now (another app has it open to save it).
fn read() -> Option<String> {
    match std::fs::read(path()) {
        // Past the limit it isn't read anyway.
        Ok(b) if b.len() > settings::MAX_FILE => None,
        Ok(b) => Some(String::from_utf8_lossy(&b).into_owned()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(String::new()),
        Err(_) => None,
    }
}

/// The settings; `None` while the file can't be used (keep the ones in use).
pub fn load() -> Option<Settings> {
    Settings::parse(&read()?)
}

fn write(out: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(folder())?;
    // Written whole and then moved over, so the watcher never reads half.
    let tmp = folder().join("settings.json.tmp");
    std::fs::write(&tmp, out)?;
    std::fs::rename(&tmp, path())
}

/// Sets one key, keeping the rest of the file (and keys it doesn't know).
pub fn set(key: &str, value: Value) -> std::io::Result<()> {
    let text = match read() {
        Some(t) if t.trim().is_empty() => settings::TEMPLATE.to_string(),
        // Never over a file being saved, or one with a typo to fix by hand.
        Some(t) if Settings::parse(&t).is_some() => t,
        _ => return Err(std::io::Error::other("settings.json can't be used now")),
    };
    write(&settings::set(&text, key, value))
}

/// Makes the file, with every key at its default, if there is none.
pub fn ensure() -> std::io::Result<PathBuf> {
    if !path().exists() {
        write(settings::TEMPLATE)?;
    }
    Ok(path())
}

/// Posts `msg` to `hwnd` when `settings.json` changes. The thread sleeps in
/// the kernel until then: nothing is polled.
pub fn watch(hwnd: HWND, msg: u32) {
    let hwnd = hwnd.0 as isize;
    std::thread::spawn(move || {
        let _ = std::fs::create_dir_all(folder());
        let Ok(dir) = (unsafe {
            CreateFileW(
                &HSTRING::from(folder().as_os_str()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            )
        }) else {
            return;
        };
        // u32s, so the records inside are aligned.
        let mut buf = [0u32; 1024];
        loop {
            let mut got = 0u32;
            let read = unsafe {
                ReadDirectoryChangesW(
                    dir,
                    buf.as_mut_ptr().cast(),
                    size_of_val(&buf) as u32,
                    false,
                    FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_FILE_NAME,
                    Some(&mut got),
                    None,
                    None,
                )
            };
            if read.is_err() {
                break;
            }
            // 0 bytes: too many changes to list; any of them may be ours.
            if got == 0
                || names(&buf, got)
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(NAME))
            {
                let h = HWND(hwnd as *mut _);
                if unsafe { PostMessageW(Some(h), msg, WPARAM(0), LPARAM(0)) }.is_err() {
                    break;
                }
            }
        }
        unsafe {
            let _ = CloseHandle(dir);
        }
    });
}

/// The file names in a ReadDirectoryChangesW answer of `len` bytes.
fn names(buf: &[u32], len: u32) -> Vec<String> {
    let base = buf.as_ptr().cast::<u8>();
    let len = (len as usize).min(size_of_val(buf));
    let mut out = Vec::new();
    let mut at = 0usize;
    let head = std::mem::offset_of!(FILE_NOTIFY_INFORMATION, FileName);
    while at + head <= len {
        // SAFETY: within the buffer (checked above and below); the kernel
        // writes the records DWORD-aligned.
        let info = unsafe { &*base.add(at).cast::<FILE_NOTIFY_INFORMATION>() };
        let bytes = info.FileNameLength as usize;
        if at + head + bytes > len {
            break;
        }
        let name =
            unsafe { std::slice::from_raw_parts(base.add(at + head).cast::<u16>(), bytes / 2) };
        out.push(String::from_utf16_lossy(name));
        if info.NextEntryOffset == 0 {
            break;
        }
        at += info.NextEntryOffset as usize;
    }
    out
}
