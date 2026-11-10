// No console window in release builds; debug builds keep it for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod render;

use std::cell::RefCell;
use std::time::Instant;

use island_core::monitor::{self, Monitor, MonitorChoice};
use island_core::shape::{self, Placement, Rect};
use island_core::spring::{self, Spring};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateRectRgn, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, SetWindowRgn,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
    SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_TOPMOST,
    IDC_ARROW, KillTimer, LoadCursorW, MA_NOACTIVATE, MONITORINFOF_PRIMARY, MSG, PostMessageW,
    PostQuitMessage, RegisterClassW, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SetTimer, SetWindowPos,
    ShowWindow, TranslateMessage, WM_APP, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED,
    WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::BOOL;
use windows::core::{Error, Result, w};

// Sizes in DIPs (1/96 inch); the springs run in DIPs too, so a DPI change
// doesn't jolt the motion.
const CLOSED: (f32, f32) = (180.0, 32.0);
const OPEN: (f32, f32) = (560.0, 180.0);
const RADIUS: f32 = 18.0;
const EAR: f32 = 10.0;
const MARGIN: f32 = 8.0;
const HOVER_DELAY_MS: u32 = 120;

const HOVER_TIMER: usize = 1;
const FRAME_TIMER: usize = 2;
const WM_APP_LAYOUT: u32 = WM_APP + 1;
// windows-rs only has it under the (big) Win32_UI_Controls feature.
const WM_MOUSELEAVE: u32 = 0x02A3;

struct App {
    hwnd: HWND,
    placement: Placement,
    monitor: MonitorChoice,
    scale: f32,
    /// The window on screen, in px: sized for the open island.
    window: Rect,
    width: Spring,
    height: Spring,
    open: bool,
    /// The pointer is over the island (WM_MOUSELEAVE is armed).
    tracking: bool,
    /// While springs move: the last frame. `None`: no frame timer.
    last_frame: Option<Instant>,
    renderer: Option<render::Renderer>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

unsafe extern "system" fn collect_monitor(
    mon: HMONITOR,
    _: HDC,
    _: *mut RECT,
    out: LPARAM,
) -> BOOL {
    let mut mi = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `out` is the &mut Vec that `monitors` passes, alive for the call.
    let list = unsafe { &mut *(out.0 as *mut Vec<(HMONITOR, Monitor)>) };
    if unsafe { GetMonitorInfoW(mon, &mut mi) }.as_bool() {
        let r = mi.rcMonitor;
        let rect = Rect {
            x: r.left as f32,
            y: r.top as f32,
            width: (r.right - r.left) as f32,
            height: (r.bottom - r.top) as f32,
        };
        list.push((
            mon,
            Monitor {
                rect,
                primary: mi.dwFlags & MONITORINFOF_PRIMARY != 0,
            },
        ));
    }
    true.into()
}

/// The connected screens, as Windows lists them now.
fn monitors() -> Vec<(HMONITOR, Monitor)> {
    let mut list: Vec<(HMONITOR, Monitor)> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM(&mut list as *mut _ as isize),
        );
    }
    list
}

/// The chosen screen and its scale; the primary when it is gone.
fn screen(choice: MonitorChoice) -> (Rect, f32) {
    let list = monitors();
    let mons: Vec<Monitor> = list.iter().map(|m| m.1).collect();
    let Some(i) = monitor::pick(&mons, choice) else {
        return (
            Rect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            1.0,
        );
    };
    let (mut dx, mut dy) = (96, 96);
    unsafe {
        let _ = GetDpiForMonitor(list[i].0, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
    }
    (mons[i].rect, dx.max(96) as f32 / 96.0)
}

impl App {
    /// Places the window on its screen at that screen's scale, and redraws.
    /// Runs again when screens are added, removed, moved or rescaled.
    fn layout(&mut self) {
        let (mon, scale) = screen(self.monitor);
        self.scale = scale;
        let s = self.scale;
        let r = shape::window_rect(
            self.placement,
            mon,
            OPEN.0 * s,
            OPEN.1 * s,
            EAR * s,
            MARGIN * s,
        );
        let r = Rect {
            x: r.x.round(),
            y: r.y.round(),
            width: r.width.ceil(),
            height: r.height.ceil(),
        };
        if r != self.window {
            self.window = r;
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    r.x as i32,
                    r.y as i32,
                    r.width as i32,
                    r.height as i32,
                    SWP_NOACTIVATE,
                );
            }
        }
        self.draw();
    }

    fn draw(&mut self) {
        let s = self.scale;
        let (w, h) = (
            self.width.value.max(0.0) * s,
            self.height.value.max(0.0) * s,
        );
        let (ear, radius) = (EAR * s, RADIUS * s);
        let body = shape::body_rect(self.placement, self.window.width, w, h, ear);
        let outline = if self.placement.hangs() {
            shape::hanging(w, h, radius, ear)
        } else {
            shape::floating(w, h, radius)
        };
        // The pointer only reaches the island inside its body; everywhere
        // else in the window, clicks go to what is below.
        unsafe {
            let rgn = CreateRectRgn(
                body.x.floor() as i32,
                0,
                (body.x + body.width).ceil() as i32,
                h.ceil() as i32,
            );
            SetWindowRgn(self.hwnd, Some(rgn), false);
        }
        let (ww, wh) = (self.window.width as u32, self.window.height as u32);
        let drawn = match &mut self.renderer {
            Some(r) => r.draw(ww, wh, body, &outline),
            None => Ok(()),
        };
        // A lost device (driver update, GPU reset): start over once.
        if drawn.is_err() {
            self.renderer = render::Renderer::new(self.hwnd).ok();
            if let Some(r) = &mut self.renderer {
                let _ = r.draw(ww, wh, body, &outline);
            }
        }
    }

    fn set_open(&mut self, open: bool) {
        if self.open == open {
            return;
        }
        self.open = open;
        let (w, h) = if open { OPEN } else { CLOSED };
        self.width.target = w;
        self.height.target = h;
        if self.last_frame.is_none() {
            self.last_frame = Some(Instant::now());
            unsafe { SetTimer(Some(self.hwnd), FRAME_TIMER, 10, None) };
        }
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let ms = self
            .last_frame
            .map_or(16.0, |t| (now - t).as_secs_f32() * 1000.0);
        self.last_frame = Some(now);
        let (pw, ph) = if self.open {
            (spring::OPEN_WIDTH, spring::OPEN_HEIGHT)
        } else {
            (spring::CLOSE_WIDTH, spring::CLOSE_HEIGHT)
        };
        let moving = self.width.advance(&pw, ms) | self.height.advance(&ph, ms);
        self.draw();
        if !moving {
            // Nothing runs while the island is still.
            self.last_frame = None;
            unsafe {
                let _ = KillTimer(Some(self.hwnd), FRAME_TIMER);
            }
        }
    }

    fn handle(&mut self, msg: u32, wp: WPARAM) -> Option<LRESULT> {
        match msg {
            WM_MOUSEMOVE if !self.tracking => {
                let mut t = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: self.hwnd,
                    dwHoverTime: 0,
                };
                self.tracking = unsafe { TrackMouseEvent(&mut t) }.is_ok();
                if !self.open {
                    unsafe { SetTimer(Some(self.hwnd), HOVER_TIMER, HOVER_DELAY_MS, None) };
                }
            }
            WM_MOUSELEAVE => {
                self.tracking = false;
                unsafe {
                    let _ = KillTimer(Some(self.hwnd), HOVER_TIMER);
                }
                self.set_open(false);
            }
            WM_TIMER if wp.0 == HOVER_TIMER => {
                unsafe {
                    let _ = KillTimer(Some(self.hwnd), HOVER_TIMER);
                }
                if self.tracking {
                    self.set_open(true);
                }
            }
            WM_TIMER if wp.0 == FRAME_TIMER => self.frame(),
            WM_DPICHANGED | WM_DISPLAYCHANGE | WM_APP_LAYOUT => self.layout(),
            // Clicking the island never takes the focus from the app in use.
            WM_MOUSEACTIVATE => return Some(LRESULT(MA_NOACTIVATE as isize)),
            // For now the way out; a tray icon replaces it (#64, phase 5).
            WM_RBUTTONUP => unsafe {
                let _ = DestroyWindow(self.hwnd);
            },
            WM_DESTROY => unsafe { PostQuitMessage(0) },
            _ => return None,
        }
        Some(LRESULT(0))
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let handled = APP.with(|cell| match cell.try_borrow_mut() {
        Ok(mut app) => app.as_mut().and_then(|a| a.handle(msg, wp)),
        // A message sent from inside a handler (SetWindowPos, DestroyWindow):
        // the app is busy, so what needs it waits for the queue.
        Err(_) => match msg {
            WM_DPICHANGED | WM_DISPLAYCHANGE => unsafe {
                let _ = PostMessageW(Some(hwnd), WM_APP_LAYOUT, WPARAM(0), LPARAM(0));
                Some(LRESULT(0))
            },
            WM_DESTROY => unsafe {
                PostQuitMessage(0);
                Some(LRESULT(0))
            },
            _ => None,
        },
    });
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, msg, wp, lp) })
}

/// The value after `--name` on the command line (settings come in phase 5).
fn arg(name: &str) -> Option<String> {
    std::env::args().skip_while(|a| a != name).nth(1)
}

fn main() -> Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let instance = GetModuleHandleW(None)?;
        let class = w!("IslandWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err(Error::from_thread());
        }
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
            class,
            w!("Island"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        let app = App {
            hwnd,
            placement: arg("--placement")
                .and_then(|s| Placement::parse(&s))
                .unwrap_or_default(),
            monitor: arg("--monitor")
                .and_then(|s| MonitorChoice::parse(&s))
                .unwrap_or_default(),
            scale: 1.0,
            window: Rect::default(),
            width: Spring::at(CLOSED.0),
            height: Spring::at(CLOSED.1),
            open: false,
            tracking: false,
            last_frame: None,
            renderer: Some(render::Renderer::new(hwnd)?),
        };
        APP.with_borrow_mut(|a| *a = Some(app));
        APP.with_borrow_mut(|a| a.as_mut().map(App::layout));
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}
