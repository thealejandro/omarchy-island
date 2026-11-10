//! The microphone and camera in use, from the keys Windows' privacy
//! settings keep (see `island_core::privacy`). A thread sleeps on the
//! registry's own change notification and posts the device that changed;
//! the window's thread then reads who uses it. Nothing is polled.

use island_core::privacy::Device;
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_NOTIFY, KEY_READ, REG_NOTIFY_CHANGE_LAST_SET,
    REG_NOTIFY_CHANGE_NAME, RRF_RT_REG_QWORD, RegCloseKey, RegEnumKeyExW, RegGetValueW,
    RegNotifyChangeKeyValue, RegOpenKeyExW,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::{HSTRING, PWSTR, w};

const STORE: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore";

fn open(path: &str, access: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<HKEY> {
    let mut key = HKEY::default();
    let e = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(path),
            None,
            access,
            &mut key,
        )
    };
    (e == ERROR_SUCCESS).then_some(key)
}

/// The names of `key`'s subkeys.
fn subkeys(key: HKEY) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = [0u16; 512];
    for i in 0.. {
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
    out
}

/// Still in use: it started and has not stopped (`LastUsedTimeStop` 0).
fn in_use(key: HKEY, app: &str) -> bool {
    let mut stop = 1u64;
    let mut size = size_of::<u64>() as u32;
    let e = unsafe {
        RegGetValueW(
            key,
            &HSTRING::from(app),
            w!("LastUsedTimeStop"),
            RRF_RT_REG_QWORD,
            None,
            Some((&mut stop as *mut u64).cast()),
            Some(&mut size),
        )
    };
    e == ERROR_SUCCESS && stop == 0
}

fn using_in(path: &str, out: &mut Vec<String>) {
    let Some(key) = open(path, KEY_READ) else {
        return;
    };
    for app in subkeys(key) {
        if app.eq_ignore_ascii_case("NonPackaged") {
            using_in(&format!("{path}\\{app}"), out);
        } else if in_use(key, &app) {
            out.push(app);
        }
    }
    unsafe {
        let _ = RegCloseKey(key);
    }
}

/// The apps (their keys) using `device` now.
pub fn using(device: Device) -> Vec<String> {
    let mut out = Vec::new();
    using_in(&format!("{STORE}\\{}", device.key()), &mut out);
    out
}

/// Watches both devices while it lives.
pub struct Privacy {
    /// Set to end the thread.
    stop: isize,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Privacy {
    /// A change to `Device::ALL[i]` goes to `hwnd` as `msg` with `i` as
    /// its WPARAM.
    pub fn start(hwnd: HWND, msg: u32) -> Option<Self> {
        let stop = unsafe { CreateEventW(None, true, false, None) }.ok()?;
        let (hwnd, stop_raw) = (hwnd.0 as isize, stop.0 as isize);
        let thread = std::thread::spawn(move || watch(hwnd, msg, HANDLE(stop_raw as *mut _)));
        Some(Self {
            stop: stop.0 as isize,
            thread: Some(thread),
        })
    }
}

fn watch(hwnd: isize, msg: u32, stop: HANDLE) {
    let mut keys = Vec::new();
    let mut events = vec![stop];
    for d in Device::ALL {
        let (Some(key), Ok(ev)) = (open(&format!("{STORE}\\{}", d.key()), KEY_NOTIFY), unsafe {
            CreateEventW(None, false, false, None)
        }) else {
            return;
        };
        keys.push(key);
        events.push(ev);
    }
    let arm = |i: usize| unsafe {
        RegNotifyChangeKeyValue(
            keys[i],
            true,
            REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
            Some(events[i + 1]),
            true,
        ) == ERROR_SUCCESS
    };
    if (0..keys.len()).all(arm) {
        loop {
            let got = unsafe { WaitForMultipleObjects(&events, false, INFINITE) }.0 as usize;
            if got == 0 || got > keys.len() {
                break;
            }
            let i = got - 1;
            unsafe {
                let _ = PostMessageW(Some(HWND(hwnd as *mut _)), msg, WPARAM(i), LPARAM(0));
            }
            if !arm(i) {
                break;
            }
        }
    }
    for k in keys {
        unsafe {
            let _ = RegCloseKey(k);
        }
    }
    for e in &events[1..] {
        unsafe {
            let _ = CloseHandle(*e);
        }
    }
}

impl Drop for Privacy {
    fn drop(&mut self) {
        let stop = HANDLE(self.stop as *mut _);
        unsafe {
            let _ = SetEvent(stop);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        unsafe {
            let _ = CloseHandle(stop);
        }
    }
}
