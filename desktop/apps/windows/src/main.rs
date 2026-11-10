// No console window in release builds; debug builds keep it for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bars;
mod media;
mod render;

use std::cell::RefCell;
use std::time::Instant;

use island_core::media as layout;
use island_core::monitor::{self, Monitor, MonitorChoice};
use island_core::shape::{self, Placement, Rect};
use island_core::spring::{self, Spring};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::Graphics::Gdi::{
    CreateRectRgn, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, SetWindowRgn,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
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
    WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
    WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
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
/// Moves the progress bar on, once a second while open and playing.
const TICK_TIMER: usize = 3;
/// Ends the glance on a new song.
const PEEK_TIMER: usize = 4;
/// How long that glance stays (`peekDuration` in the plugin).
const PEEK_MS: u32 = 3500;
const WM_APP_LAYOUT: u32 = WM_APP + 1;
/// The media thread has a new snapshot.
const WM_APP_MEDIA: u32 = WM_APP + 2;
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
    /// Glancing at a new song.
    peek: bool,
    /// The glance is what shows (or fades) while not open: from when it
    /// starts until the island opens or has closed.
    peek_view: bool,
    /// Title and artist of the song on screen, to tell a new one.
    song: Option<(String, String)>,
    /// The pointer is over the island (WM_MOUSELEAVE is armed).
    tracking: bool,
    /// The window region last set, in px: left, top, right, bottom.
    region: [i32; 4],
    /// While springs move: the last frame. `None`: no frame timer.
    last_frame: Option<Instant>,
    renderer: Option<render::Renderer>,
    media: media::Media,
    track: Option<media::Track>,
    empty: &'static str,
    song_text: layout::SongText,
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
        // The pointer only reaches the island inside its body, ears left
        // out; everywhere else, clicks go to what is below. The region takes
        // the bigger of the size now and the target, so it changes only when
        // the island starts to open and when it has finished closing.
        let (rw, rh) = (
            self.width.value.max(self.width.target) * s,
            self.height.value.max(self.height.target) * s,
        );
        let inset = shape::ear_room(self.placement, ear);
        let rb = shape::body_rect(self.placement, self.window.width, rw, rh, ear);
        let region = [
            (rb.x + inset).floor() as i32,
            0,
            (rb.x + rb.width - inset).ceil() as i32,
            rh.ceil() as i32,
        ];
        if region != self.region {
            self.region = region;
            unsafe {
                let rgn = CreateRectRgn(region[0], region[1], region[2], region[3]);
                SetWindowRgn(self.hwnd, Some(rgn), false);
            }
        }
        let scene = render::Scene {
            width: self.window.width as u32,
            height: self.window.height as u32,
            body,
            outline: &outline,
            scale: s,
            inset: shape::ear_room(self.placement, ear),
            size: (self.width.value.max(0.0), self.height.value.max(0.0)),
            alpha: self.alphas(),
            track: self.track.as_ref(),
            position_ms: self.track.as_ref().map_or(0, |t| t.position_now()),
            empty: self.empty,
            song: self.song_text,
        };
        let drawn = match &mut self.renderer {
            Some(r) => r.draw(&scene),
            None => Ok(()),
        };
        // A lost device (driver update, GPU reset): start over once.
        if drawn.is_err() {
            self.renderer = render::Renderer::new(self.hwnd).ok();
            if let Some(r) = &mut self.renderer {
                let _ = r.draw(&scene);
            }
        }
    }

    /// Wakes up when the shown second changes, only while that is on screen.
    fn tick(&self) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TICK_TIMER);
        }
        if let Some(t) = &self.track
            && self.open
            && t.playing
            && t.duration_ms > 0
        {
            let ms = 1000 - (t.position_now() % 1000) as u32 + 15;
            unsafe { SetTimer(Some(self.hwnd), TICK_TIMER, ms, None) };
        }
    }

    fn click(&self, x: f32, y: f32) {
        if !self.open || self.track.is_none() {
            return;
        }
        let s = self.scale;
        let body = shape::body_rect(
            self.placement,
            self.window.width,
            self.width.value * s,
            self.height.value * s,
            EAR * s,
        );
        // To the open layout's DIPs: it is centered in the body.
        let lx = (x - body.x - shape::ear_room(self.placement, EAR * s)) / s
            - (self.width.value - OPEN.0) / 2.0;
        let ly = (y - body.y) / s;
        let cmd = match layout::hit(lx, ly) {
            Some(layout::Button::Prev) => media::Command::Prev,
            Some(layout::Button::PlayPause) => media::Command::PlayPause,
            Some(layout::Button::Next) => media::Command::Next,
            None => return,
        };
        self.media.send(cmd);
    }

    /// How visible the closed, glance and open contents are, from the
    /// body's height: each one fades as the body leaves its size.
    fn alphas(&self) -> [f32; 3] {
        let h = self.height.value;
        let open = ((h - CLOSED.1) / (OPEN.1 - CLOSED.1)).clamp(0.0, 1.0);
        // The closed content leaves fast; the open one comes in once there
        // is room for it.
        let open_a = ((open - 0.4) / 0.6).clamp(0.0, 1.0);
        let leaving = (1.0 - open * 3.0).clamp(0.0, 1.0);
        if self.peek_view {
            // One gives way to the other halfway, so they never overlap.
            let grown = ((h - CLOSED.1) / (layout::PEEK.1 - CLOSED.1)).clamp(0.0, 1.0);
            let peek = ((grown - 0.5) * 2.0).clamp(0.0, 1.0);
            let closed = ((0.5 - grown) * 2.0).clamp(0.0, 1.0);
            [closed, peek * leaving, open_a]
        } else {
            [leaving, 0.0, open_a]
        }
    }

    fn set_open(&mut self, open: bool) {
        if self.open == open {
            return;
        }
        self.open = open;
        if open {
            self.peek_view = false;
        }
        self.retarget();
    }

    fn set_peek(&mut self, peek: bool) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), PEEK_TIMER);
            if peek {
                SetTimer(Some(self.hwnd), PEEK_TIMER, PEEK_MS, None);
            }
        }
        self.peek = peek;
        if peek && !self.open {
            self.peek_view = true;
        }
        self.retarget();
    }

    fn retarget(&mut self) {
        let (w, h) = if self.open {
            OPEN
        } else if self.peek {
            layout::PEEK
        } else if self.track.is_some() {
            // Wide enough for the song's name.
            (layout::CLOSED_MEDIA_WIDTH, CLOSED.1)
        } else {
            CLOSED
        };
        self.width.target = w;
        self.height.target = h;
        if self.last_frame.is_none() {
            self.last_frame = Some(Instant::now());
            unsafe { SetTimer(Some(self.hwnd), FRAME_TIMER, 10, None) };
        }
        self.tick();
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let ms = self
            .last_frame
            .map_or(16.0, |t| (now - t).as_secs_f32() * 1000.0);
        self.last_frame = Some(now);
        let (pw, ph) = if self.open || self.peek {
            (spring::OPEN_WIDTH, spring::OPEN_HEIGHT)
        } else {
            (spring::CLOSE_WIDTH, spring::CLOSE_HEIGHT)
        };
        let moving = self.width.advance(&pw, ms) | self.height.advance(&ph, ms);
        self.draw();
        if !moving {
            // Nothing runs while the island is still.
            if !self.open && !self.peek {
                self.peek_view = false;
            }
            self.last_frame = None;
            unsafe {
                let _ = KillTimer(Some(self.hwnd), FRAME_TIMER);
            }
        }
    }

    fn handle(&mut self, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
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
            WM_TIMER if wp.0 == TICK_TIMER => {
                if self.last_frame.is_none() {
                    self.draw();
                }
                self.tick();
            }
            WM_TIMER if wp.0 == PEEK_TIMER => self.set_peek(false),
            WM_APP_MEDIA => {
                self.track = self.media.latest();
                // A new song gets a glance; the one playing at start doesn't.
                let song = self
                    .track
                    .as_ref()
                    .map(|t| (t.title.clone(), t.artist.clone()));
                if song.is_some() && song != self.song {
                    let first = self.song.is_none();
                    self.song = song;
                    if !first {
                        self.set_peek(true);
                    }
                }
                // Music starting or stopping widens or narrows the closed island.
                self.retarget();
                if self.last_frame.is_none() {
                    self.draw();
                }
                self.tick();
            }
            WM_LBUTTONUP => {
                let (x, y) = (lp.0 as i16 as f32, (lp.0 >> 16) as i16 as f32);
                self.click(x, y);
            }
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
        Ok(mut app) => app.as_mut().and_then(|a| a.handle(msg, wp, lp)),
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

/// "Nothing playing" in the user's language (all the texts come in phase 5).
fn nothing_playing() -> &'static str {
    let mut name = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(&mut name) };
    let locale = String::from_utf16_lossy(&name[..(n.max(1) - 1) as usize]);
    if locale.starts_with("es") {
        "Nada en reproducción"
    } else {
        "Nothing playing"
    }
}

/// The value after `--name` on the command line (settings come in phase 5).
fn arg(name: &str) -> Option<String> {
    std::env::args().skip_while(|a| a != name).nth(1)
}

fn main() -> Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // WIC is COM; the UI thread is its apartment.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
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
            peek: false,
            peek_view: false,
            song: None,
            tracking: false,
            region: [0; 4],
            last_frame: None,
            renderer: Some(render::Renderer::new(hwnd)?),
            media: media::Media::start(hwnd, WM_APP_MEDIA),
            track: None,
            empty: nothing_playing(),
            song_text: arg("--song-text")
                .and_then(|s| layout::SongText::parse(&s))
                .unwrap_or_default(),
        };
        if let Some(r) = &app.renderer {
            r.align_song(
                arg("--text-align")
                    .and_then(|s| layout::TextAlign::parse(&s))
                    .unwrap_or_default(),
            )?;
        }
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
