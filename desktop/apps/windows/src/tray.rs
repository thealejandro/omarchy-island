//! The tray icon. A click on it opens the island's menu (`menu` in the core,
//! drawn like the island).

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, GetSystemMetrics, HICON, ICONINFO, SM_CXSMICON,
};

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

/// The island's pill (see `island_core::icon`) at the tray's size.
fn pill_icon() -> Option<HICON> {
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.clamp(16, 64);
    let pixels = island_core::icon::pill(size as u32);
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
