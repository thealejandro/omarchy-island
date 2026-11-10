//! Notifications as they arrive, read with the system's own listener. That
//! listener tells of new ones only to packaged apps, so two cues wake a
//! thread that reads the list and keeps the new ones (see
//! `island_core::notifications`) for the window:
//!
//! - The shell's count of notifications changes: a state the shell
//!   publishes (WNF, `WNF_SHEL_NOTIFICATIONS`) that any process of the
//!   user may follow. It changes with every new one, also when Windows
//!   shows no banner for it (banners turned off, an app in full screen).
//! - The window of the shell that shows a banner becomes visible
//!   (uncloaked), through accessibility: in case a Windows update ever
//!   stopped that state.
//!
//! Both only wait, out of process; between notifications nothing runs and
//! the system is never touched. Do Not Disturb is followed the same way
//! (`WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED`) so the window can keep
//! quiet with it.

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use island_core::notifications::{Seen, Toast};
use windows::Foundation::Size;
use windows::Storage::Streams::DataReader;
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
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
use windows::core::{PWSTR, Result, s, w};

/// The shell's processes that show notifications (by Windows version).
const SHELLS: [&str; 2] = ["ShellExperienceHost.exe", "ShellHost.exe"];
const WINDOW: &str = "Windows.UI.Core.CoreWindow";

/// The event the cues set for the reader (one watch at a time).
static WAKE: AtomicIsize = AtomicIsize::new(0);
/// Do Not Disturb's profile: 0 when off.
static QUIET: AtomicU32 = AtomicU32::new(0);

/// The shell's states followed (well-known WNF names).
const COUNT: u64 = 0x0D83_063E_A3BC_1035;
const QUIET_PROFILE: u64 = 0x0D83_063E_A3BF_1C75;

type WnfCallback =
    unsafe extern "system" fn(u64, u32, *const c_void, *const c_void, *const u8, u32) -> i32;
type Subscribe = unsafe extern "system" fn(
    *mut *mut c_void,
    u64,
    u32,
    WnfCallback,
    *const c_void,
    *const c_void,
    u32,
    u32,
) -> i32;
type Unsubscribe = unsafe extern "system" fn(*mut c_void) -> i32;

/// A state changed (also called once at start, with how it is).
unsafe extern "system" fn changed(
    name: u64,
    _: u32,
    _: *const c_void,
    _: *const c_void,
    data: *const u8,
    size: u32,
) -> i32 {
    if name == QUIET_PROFILE {
        let profile = if data.is_null() || size < 4 {
            0
        } else {
            unsafe { data.cast::<u32>().read_unaligned() }
        };
        QUIET.store(profile, Ordering::Relaxed);
    } else {
        let wake = WAKE.load(Ordering::Relaxed);
        if wake != 0 {
            unsafe {
                let _ = SetEvent(HANDLE(wake as *mut _));
            }
        }
    }
    0
}

/// ntdll's functions for following a state; `None` if this Windows has
/// none.
fn wnf() -> Option<(Subscribe, Unsubscribe)> {
    unsafe {
        let nt = GetModuleHandleW(w!("ntdll.dll")).ok()?;
        let sub = GetProcAddress(nt, s!("RtlSubscribeWnfStateChangeNotification"))?;
        let unsub = GetProcAddress(nt, s!("RtlUnsubscribeWnfStateChangeNotification"))?;
        Some((
            std::mem::transmute::<unsafe extern "system" fn() -> isize, Subscribe>(sub),
            std::mem::transmute::<unsafe extern "system" fn() -> isize, Unsubscribe>(unsub),
        ))
    }
}

/// Follows `names`; the subscriptions to end later.
fn follow(names: &[u64]) -> Vec<*mut c_void> {
    let Some((subscribe, _)) = wnf() else {
        return Vec::new();
    };
    names
        .iter()
        .filter_map(|&name| {
            let mut sub = std::ptr::null_mut();
            let r = unsafe {
                subscribe(
                    &mut sub,
                    name,
                    0,
                    changed,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    0,
                )
            };
            (r >= 0 && !sub.is_null()).then_some(sub)
        })
        .collect()
}

/// Whether Do Not Disturb is on.
pub fn quiet() -> bool {
    QUIET.load(Ordering::Relaxed) != 0
}

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
    let texts = n
        .Notification()
        .and_then(|n| n.Visual())
        .and_then(|v| v.GetBinding(&KnownNotificationBindings::ToastGeneric()?))
        .and_then(|b| b.GetTextElements());
    let line = |i: u32| {
        texts
            .as_ref()
            .ok()
            .and_then(|t| t.GetAt(i).and_then(|t| t.Text()).ok())
            .map(|s| Toast::line(&s.to_string()))
            .unwrap_or_default()
    };
    Ok(Toast {
        id: n.Id()?,
        app: if app.is_empty() { fallback.into() } else { app },
        app_id: n
            .AppInfo()
            .and_then(|a| a.AppUserModelId())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        title: line(0),
        body: line(1),
    })
}

/// The app's logo as the system keeps it (PNG bytes), for the one glanced
/// at only; `None` if it has none.
fn logo(l: &UserNotificationListener, id: u32) -> Option<Vec<u8>> {
    let r = l
        .GetNotification(id)
        .and_then(|n| n.AppInfo())
        .and_then(|a| a.DisplayInfo())
        .and_then(|d| {
            d.GetLogo(Size {
                Width: 64.0,
                Height: 64.0,
            })
        })
        .ok()?;
    let stream = r.OpenReadAsync().ok()?.join().ok()?;
    let size = u32::try_from(stream.Size().ok()?).ok()?;
    // A logo is a few KB: anything huge is not one.
    if size == 0 || size > 4 << 20 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size).ok()?.join().ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    Some(bytes)
}

/// A notification that showed, with its app's logo.
pub struct Arrived {
    pub toast: Toast,
    pub logo: Option<Arc<Vec<u8>>>,
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
    /// The shell's states followed (WNF subscriptions).
    states: Vec<isize>,
    /// Set to end the thread; the other wakes it to read.
    stop: isize,
    wake: isize,
    thread: Option<std::thread::JoinHandle<()>>,
    new: Arc<Mutex<Vec<Arrived>>>,
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
        let states = follow(&[COUNT, QUIET_PROFILE])
            .into_iter()
            .map(|s| s as isize)
            .collect();
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
            states,
            stop: stop.0 as isize,
            wake: wake.0 as isize,
            thread: Some(thread),
            new,
        };
        // Either cue is enough.
        (!hook.is_invalid() || !me.states.is_empty()).then_some(me)
    }

    /// The notifications that showed since the last call, oldest first.
    pub fn take(&self) -> Vec<Arrived> {
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
    new: Arc<Mutex<Vec<Arrived>>>,
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
        // Only the newest is glanced at: only its logo is read.
        let last = fresh.len() - 1;
        let arrived = fresh.into_iter().enumerate().map(|(i, toast)| Arrived {
            logo: (i == last)
                .then(|| logo(&l, toast.id))
                .flatten()
                .map(Arc::new),
            toast,
        });
        if let Ok(mut n) = new.lock() {
            n.extend(arrived);
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
            if let Some((_, unsubscribe)) = wnf() {
                for &s in &self.states {
                    unsubscribe(s as *mut c_void);
                }
            }
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
