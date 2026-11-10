//! Notifications as they show, read with the system's own listener. That
//! listener tells of new ones only to packaged apps, so the cue is the one
//! accessibility gives: the window of the shell that shows them becomes
//! visible (uncloaked). That wakes a thread that reads the list and keeps
//! the new ones (see `island_core::notifications`) for the window.
//!
//! The hook takes that one event only, out of context (nothing is loaded
//! into other processes); between notifications nothing runs. If the shell
//! changed and the cue never came, the glance would just not show: the
//! system is never touched. With Do Not Disturb no notification shows, and
//! no glance either.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};

use island_core::notifications::{Seen, Toast};
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::{
    CreateEventW, INFINITE, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW, SetEvent, WaitForMultipleObjects,
};
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_UNCLOAKED, GetClassNameW, GetWindowThreadProcessId, OBJID_WINDOW, PostMessageW,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};
use windows::core::{PWSTR, Result};

/// The shell's processes that show notifications (by Windows version).
const SHELLS: [&str; 2] = ["ShellExperienceHost.exe", "ShellHost.exe"];
const WINDOW: &str = "Windows.UI.Core.CoreWindow";

/// The event the hook sets for the reader (one watch at a time).
static WAKE: AtomicIsize = AtomicIsize::new(0);

fn process_name(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut n);
        let _ = CloseHandle(h);
        ok.ok()?;
        let path = String::from_utf16_lossy(&buf[..n as usize]);
        Some(path.rsplit('\\').next().unwrap_or_default().to_string())
    }
}

/// A shell window showed: maybe a notification. Cheap checks first.
unsafe extern "system" fn uncloaked(
    _: HWINEVENTHOOK,
    _: u32,
    hwnd: HWND,
    object: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if hwnd.0.is_null() || object != OBJID_WINDOW.0 {
        return;
    }
    let mut class = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
    if String::from_utf16_lossy(&class[..n]) != WINDOW {
        return;
    }
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let shell =
        process_name(pid).is_some_and(|p| SHELLS.iter().any(|s| s.eq_ignore_ascii_case(&p)));
    let wake = WAKE.load(Ordering::Relaxed);
    if shell && wake != 0 {
        unsafe {
            let _ = SetEvent(HANDLE(wake as *mut _));
        }
    }
}

fn toast(n: &UserNotification, fallback: &str) -> Result<Toast> {
    let app = n
        .AppInfo()
        .and_then(|a| a.DisplayInfo())
        .and_then(|d| d.DisplayName())
        .map(|s| Toast::line(&s.to_string()))
        .unwrap_or_default();
    let title = n
        .Notification()
        .and_then(|n| n.Visual())
        .and_then(|v| v.GetBinding(&KnownNotificationBindings::ToastGeneric()?))
        .and_then(|b| b.GetTextElements())
        .and_then(|t| t.GetAt(0))
        .and_then(|t| t.Text())
        .map(|s| Toast::line(&s.to_string()))
        .unwrap_or_default();
    Ok(Toast {
        id: n.Id()?,
        app: if app.is_empty() { fallback.into() } else { app },
        title,
    })
}

fn read(l: &UserNotificationListener, fallback: &str) -> Result<Vec<Toast>> {
    let list = l.GetNotificationsAsync(NotificationKinds::Toast)?.join()?;
    let mut out = Vec::with_capacity(list.Size()? as usize);
    for i in 0..list.Size()? {
        if let Ok(t) = list.GetAt(i).and_then(|n| toast(&n, fallback)) {
            out.push(t);
        }
    }
    Ok(out)
}

/// Whether the island may read notifications ("Notification access" in
/// Windows' privacy settings).
pub fn allowed() -> bool {
    UserNotificationListener::Current()
        .and_then(|l| l.GetAccessStatus())
        .is_ok_and(|s| s == UserNotificationListenerAccessStatus::Allowed)
}

pub struct Notifications {
    hook: HWINEVENTHOOK,
    /// Set to end the thread; the other wakes it to read.
    stop: isize,
    wake: isize,
    thread: Option<std::thread::JoinHandle<()>>,
    new: Arc<Mutex<Vec<Toast>>>,
}

impl Notifications {
    /// New notifications go to `hwnd` as `msg`; then take them with
    /// [`Notifications::take`]. `None` without access to them. Call on the
    /// window's thread: the hook's calls come through its messages.
    pub fn start(hwnd: HWND, msg: u32, fallback: &'static str) -> Option<Self> {
        if !allowed() {
            return None;
        }
        let stop = unsafe { CreateEventW(None, true, false, None) }.ok()?;
        let Ok(wake) = (unsafe { CreateEventW(None, false, false, None) }) else {
            unsafe {
                let _ = CloseHandle(stop);
            }
            return None;
        };
        let new = Arc::new(Mutex::new(Vec::new()));
        let (h, s, w, n) = (
            hwnd.0 as isize,
            stop.0 as isize,
            wake.0 as isize,
            new.clone(),
        );
        let thread = std::thread::spawn(move || watch(h, msg, s, w, n, fallback));
        WAKE.store(wake.0 as isize, Ordering::Relaxed);
        let hook = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_UNCLOAKED,
                EVENT_OBJECT_UNCLOAKED,
                None,
                Some(uncloaked),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        let me = Self {
            hook,
            stop: stop.0 as isize,
            wake: wake.0 as isize,
            thread: Some(thread),
            new,
        };
        (!hook.is_invalid()).then_some(me)
    }

    /// The notifications that showed since the last call, oldest first.
    pub fn take(&self) -> Vec<Toast> {
        self.new
            .lock()
            .map(|mut n| std::mem::take(&mut *n))
            .unwrap_or_default()
    }
}

fn watch(
    hwnd: isize,
    msg: u32,
    stop: isize,
    wake: isize,
    new: Arc<Mutex<Vec<Toast>>>,
    fallback: &str,
) {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let Ok(l) = UserNotificationListener::Current() else {
        return;
    };
    let mut seen = Seen::default();
    // What is there at start is no news.
    if let Ok(now) = read(&l, fallback) {
        seen.new_ones(now);
    }
    let events = [HANDLE(stop as *mut _), HANDLE(wake as *mut _)];
    loop {
        let got = unsafe { WaitForMultipleObjects(&events, false, INFINITE) }.0;
        if got != 1 {
            break;
        }
        let Ok(now) = read(&l, fallback) else {
            continue;
        };
        let mut fresh = seen.new_ones(now);
        if fresh.is_empty() {
            // The list may lag the window a moment: once more, a little
            // later (another shell window showing ends here).
            let again = unsafe { WaitForMultipleObjects(&events, false, 400) }.0;
            if again == 0 {
                break;
            }
            let Ok(now) = read(&l, fallback) else {
                continue;
            };
            fresh = seen.new_ones(now);
            if fresh.is_empty() {
                continue;
            }
        }
        if let Ok(mut n) = new.lock() {
            n.extend(fresh);
        }
        unsafe {
            let _ = PostMessageW(Some(HWND(hwnd as *mut _)), msg, WPARAM(0), LPARAM(0));
        }
    }
}

impl Drop for Notifications {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWinEvent(self.hook);
        }
        WAKE.store(0, Ordering::Relaxed);
        unsafe {
            let _ = SetEvent(HANDLE(self.stop as *mut _));
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        unsafe {
            let _ = CloseHandle(HANDLE(self.stop as *mut _));
            let _ = CloseHandle(HANDLE(self.wake as *mut _));
        }
    }
}
