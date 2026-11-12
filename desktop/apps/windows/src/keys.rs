//! The volume and media keys, taken before Windows sees them, so its own
//! flyout doesn't show over the island's glance. Windows 11 draws that
//! flyout inside the shell, with no window of its own that could be hidden
//! from outside it; a key Windows never gets is the one way left.
//!
//! A low-level keyboard hook on a thread of its own: it waits in
//! `GetMessageW`, so nothing runs between key presses, and the window's
//! thread being busy (drawing, say) never holds up the keyboard. The hook
//! only posts the key to the window, which does the rest: the volume as
//! Windows' keys move it, the song through the media controls. The media
//! keys are taken only while some app is in those controls; else they go on
//! to Windows as ever. Every other key passes untouched.
//!
//! Windows doesn't call the hook for keys meant for an app running as
//! administrator: those keep Windows' flyout.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_VOLUME_DOWN, VK_VOLUME_MUTE,
    VK_VOLUME_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE, PeekMessageW,
    PostMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    VolumeUp,
    VolumeDown,
    Mute,
    PlayPause,
    Next,
    Prev,
}

impl Key {
    fn from_vk(vk: u32) -> Option<Self> {
        Some(match vk {
            v if v == u32::from(VK_VOLUME_UP.0) => Self::VolumeUp,
            v if v == u32::from(VK_VOLUME_DOWN.0) => Self::VolumeDown,
            v if v == u32::from(VK_VOLUME_MUTE.0) => Self::Mute,
            v if v == u32::from(VK_MEDIA_PLAY_PAUSE.0) => Self::PlayPause,
            v if v == u32::from(VK_MEDIA_NEXT_TRACK.0) => Self::Next,
            v if v == u32::from(VK_MEDIA_PREV_TRACK.0) => Self::Prev,
            _ => return None,
        })
    }

    fn is_media(self) -> bool {
        matches!(self, Self::PlayPause | Self::Next | Self::Prev)
    }

    const ALL: [Self; 6] = [
        Self::VolumeUp,
        Self::VolumeDown,
        Self::Mute,
        Self::PlayPause,
        Self::Next,
        Self::Prev,
    ];

    /// A key message's key.
    pub fn read(wp: WPARAM) -> Option<Self> {
        Self::ALL.get(wp.0).copied()
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|k| *k == self).unwrap_or(0)
    }
}

// What the hook needs, which it can't be handed: there is one hook at most.
static TARGET: AtomicIsize = AtomicIsize::new(0);
static MESSAGE: AtomicU32 = AtomicU32::new(0);
static MEDIA: AtomicBool = AtomicBool::new(false);

/// Whether some app is in the media controls now (its keys are ours then).
pub fn set_media(on: bool) {
    MEDIA.store(on, Ordering::Relaxed);
}

unsafe extern "system" fn on_key(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for HC_ACTION, lParam points to the key's KBDLLHOOKSTRUCT.
        let vk = unsafe { (*(lp.0 as *const KBDLLHOOKSTRUCT)).vkCode };
        if let Some(key) = Key::from_vk(vk)
            && (!key.is_media() || MEDIA.load(Ordering::Relaxed))
        {
            // Held down, the key repeats its down: one step each, as in
            // Windows. Its up is kept from Windows too, and does nothing.
            if wp.0 == WM_KEYDOWN as usize || wp.0 == WM_SYSKEYDOWN as usize {
                let hwnd = HWND(TARGET.load(Ordering::Relaxed) as *mut _);
                let msg = MESSAGE.load(Ordering::Relaxed);
                unsafe {
                    let _ = PostMessageW(Some(hwnd), msg, WPARAM(key.index()), LPARAM(0));
                }
            }
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

pub struct Keys {
    thread: u32,
    join: Option<JoinHandle<()>>,
}

impl Keys {
    /// Takes the keys, each press posted to `hwnd` as `msg` (read it with
    /// [`Key::read`]). `None` if Windows refused the hook: its flyout stays.
    pub fn start(hwnd: HWND, msg: u32) -> Option<Self> {
        TARGET.store(hwnd.0 as isize, Ordering::Relaxed);
        MESSAGE.store(msg, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        let join = std::thread::Builder::new()
            .name("keys".into())
            .spawn(move || run(tx))
            .ok()?;
        match rx.recv() {
            Ok(Some(thread)) => Some(Self {
                thread,
                join: Some(join),
            }),
            _ => {
                let _ = join.join();
                None
            }
        }
    }
}

/// The hook's thread: tells its id once the hook is in (`None`: refused),
/// then waits for messages until told to quit, and takes the hook out.
fn run(tx: mpsc::Sender<Option<u32>>) {
    unsafe {
        // A message queue first, so the quit posted later always lands.
        let mut msg = MSG::default();
        let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
        let module = GetModuleHandleW(None).ok();
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(on_key), module.map(Into::into), 0);
        let Ok(hook) = hook else {
            let _ = tx.send(None);
            return;
        };
        let _ = tx.send(Some(GetCurrentThreadId()));
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {}
        let _ = UnhookWindowsHookEx(hook);
    }
}

impl Drop for Keys {
    /// Gives the keys back to Windows.
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_message_reads_back() {
        for k in Key::ALL {
            assert_eq!(Key::read(WPARAM(k.index())), Some(k));
        }
        assert_eq!(Key::read(WPARAM(99)), None);
    }

    #[test]
    fn only_volume_and_media_keys_are_ours() {
        assert_eq!(Key::from_vk(u32::from(VK_VOLUME_UP.0)), Some(Key::VolumeUp));
        assert_eq!(Key::from_vk(u32::from(b'A')), None);
        assert!(Key::PlayPause.is_media());
        assert!(!Key::Mute.is_media());
    }
}
