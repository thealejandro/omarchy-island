//! The tray icon and its menu: the island's way out and its switches.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
    GetSystemMetrics, HICON, ICONINFO, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING,
    PostMessageW, SM_CXSMICON, SetForegroundWindow, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenu, WM_NULL,
};
use windows::core::HSTRING;

pub enum Item<'a> {
    Check {
        id: u32,
        text: &'a str,
        checked: bool,
        enabled: bool,
    },
    Action {
        id: u32,
        text: &'a str,
    },
    Separator,
}

pub struct Tray {
    hwnd: HWND,
    msg: u32,
    icon: HICON,
}

impl Tray {
    /// Adds the icon; clicks on it come to `hwnd` as `msg`.
    pub fn new(hwnd: HWND, msg: u32, tip: &str) -> Self {
        let tray = Self {
            hwnd,
            msg,
            icon: pill_icon().unwrap_or_default(),
        };
        tray.add(tip);
        tray
    }

    fn data(&self, tip: &str) -> NOTIFYICONDATAW {
        let mut d = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: self.msg,
            hIcon: self.icon,
            ..Default::default()
        };
        for (to, c) in d.szTip.iter_mut().zip(tip.encode_utf16().take(127)) {
            *to = c;
        }
        d
    }

    /// Again when Explorer restarts (the taskbar forgets its icons).
    pub fn add(&self, tip: &str) {
        let d = self.data(tip);
        unsafe {
            if !Shell_NotifyIconW(NIM_ADD, &d).as_bool() {
                // Still there from before: just update it.
                let _ = Shell_NotifyIconW(NIM_MODIFY, &d);
            }
        }
    }
}

/// Shows a menu at the pointer for `hwnd`; the chosen item's id, if any.
/// It runs its own message loop until closed, so call it with nothing
/// borrowed: the window's messages keep coming meanwhile.
pub fn menu(hwnd: HWND, items: &[Item]) -> Option<u32> {
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        for item in items {
            let _ = match item {
                Item::Check {
                    id,
                    text,
                    checked,
                    enabled,
                } => {
                    let mut f = MF_STRING;
                    if *checked {
                        f |= MF_CHECKED;
                    }
                    if !enabled {
                        f |= MF_GRAYED;
                    }
                    AppendMenuW(menu, f, *id as usize, &HSTRING::from(*text))
                }
                Item::Action { id, text } => {
                    AppendMenuW(menu, MF_STRING, *id as usize, &HSTRING::from(*text))
                }
                Item::Separator => AppendMenuW(menu, MF_SEPARATOR, 0, None),
            };
        }
        let mut at = POINT::default();
        let _ = GetCursorPos(&mut at);
        // Without it the menu doesn't close when clicking elsewhere.
        let _ = SetForegroundWindow(hwnd);
        let id = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            at.x,
            at.y,
            None,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        (id.0 > 0).then_some(id.0 as u32)
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        let d = self.data("");
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &d);
            if !self.icon.is_invalid() {
                let _ = DestroyIcon(self.icon);
            }
        }
    }
}

/// The island as an icon: a dark pill with a light rim, seen on light and
/// dark taskbars alike. Drawn here, so there is no file to ship.
fn pill_icon() -> Option<HICON> {
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.clamp(16, 64);
    let s = size as f32;
    let pixels: Vec<u32> = (0..size * size)
        .map(|i| pill_pixel((i % size) as f32 + 0.5, (i / size) as f32 + 0.5, s))
        .collect();
    unsafe {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                // Negative: rows from the top.
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let color = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u32>(), pixels.len());
        // The alpha says what shows; the mask is only required.
        let mask = CreateBitmap(size, size, 1, 1, None);
        let icon = CreateIconIndirect(&ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        });
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mask.into());
        icon.ok()
    }
}

/// One pixel of the pill, as BGRA with straight alpha.
fn pill_pixel(x: f32, y: f32, s: f32) -> u32 {
    let r = s * 0.22;
    let half = s * 0.46 - r;
    let (px, py) = (x - s / 2.0, y - s / 2.0);
    let qx = (px.abs() - half).max(0.0);
    // Distance to the pill's edge: negative inside.
    let d = (qx * qx + py * py).sqrt() - r;
    let rim = (s / 16.0).max(1.0);
    let alpha = (0.5 - d).clamp(0.0, 1.0);
    let body = (0.5 - (d + rim)).clamp(0.0, 1.0);
    // Black body, light grey rim.
    let c = (0xD8 as f32 * (1.0 - body)) as u32;
    let a = (alpha * 255.0) as u32;
    (a << 24) | (c << 16) | (c << 8) | c
}
