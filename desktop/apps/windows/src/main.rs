// No console window in release builds; debug builds keep it for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bars;
mod lyrics;
mod media;
mod pacer;
mod render;
mod settings_file;
mod startup;
mod tray;

use std::cell::RefCell;
use std::time::Instant;

use island_core::media as layout;
use island_core::menu::{self, Kind as MenuKind};
use island_core::monitor::{self, Monitor, MonitorChoice};
use island_core::settings::Settings;
use island_core::shape::{self, Placement, Rect};
use island_core::spring::{self, Spring};
use island_core::texts::{self, Texts};
use serde_json::Value;
use windows::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::Graphics::Gdi::{
    CreateRectRgn, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST,
    MONITORINFO, MonitorFromPoint, SetWindowRgn,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
    SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_ESCAPE,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW, MA_NOACTIVATE, MONITORINFOF_PRIMARY, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SW_SHOW,
    SW_SHOWNOACTIVATE, SW_SHOWNORMAL, SWP_NOACTIVATE, SetForegroundWindow, SetTimer, SetWindowPos,
    ShowWindow, TranslateMessage, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_RBUTTONUP,
    WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::BOOL;
use windows::core::{Error, HSTRING, PCWSTR, Result, w};

// Sizes in DIPs (1/96 inch); the springs run in DIPs too, so a DPI change
// doesn't jolt the motion.
const CLOSED: (f32, f32) = (180.0, 32.0);
const OPEN: (f32, f32) = (layout::OPEN_WIDTH, 180.0);
const RADIUS: f32 = 18.0;
const EAR: f32 = 10.0;
const MARGIN: f32 = 8.0;
const HOVER_DELAY_MS: u32 = 120;

const HOVER_TIMER: usize = 1;
/// Moves the progress bar on, once a second while open and playing.
const TICK_TIMER: usize = 3;
/// Ends the glance on a new song.
const PEEK_TIMER: usize = 4;
/// How long that glance stays (`peekDuration` in the plugin).
const PEEK_MS: u32 = 3500;
/// Moves the lyrics to the next line, when it starts.
const LYRIC_TIMER: usize = 5;
const WM_APP_LAYOUT: u32 = WM_APP + 1;
/// The media thread has a new snapshot.
const WM_APP_MEDIA: u32 = WM_APP + 2;
/// The lyrics thread has an answer.
const WM_APP_LYRICS: u32 = WM_APP + 3;
/// Time for a frame, while the island moves (see `pacer.rs`).
const WM_APP_FRAME: u32 = WM_APP + 4;
/// `settings.json` changed.
const WM_APP_SETTINGS: u32 = WM_APP + 5;
/// A click on the tray icon.
const WM_APP_TRAY: u32 = WM_APP + 6;
/// Reads the settings once an editor has finished saving them.
const SETTINGS_TIMER: usize = 6;
const SETTINGS_WAIT_MS: u32 = 150;
// The menu's items.
const MENU_LYRICS: u32 = 1;
const MENU_OPEN_LYRICS: u32 = 2;
const MENU_STARTUP: u32 = 3;
const MENU_SETTINGS: u32 = 4;
const MENU_QUIT: u32 = 5;
const MENU_CLASS: PCWSTR = w!("IslandMenu");

/// The menu window, while it is open.
struct MenuWindow {
    hwnd: HWND,
    target: render::MenuTarget,
    /// Its screen's scale, and its size in px.
    scale: f32,
    size: (u32, u32),
    /// The entry under the pointer.
    hover: Option<usize>,
    /// WM_MOUSELEAVE is armed.
    tracking: bool,
    /// Starting with Windows is on (read from the registry on opening).
    startup: bool,
}
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
    /// While springs move: the last frame. `None`: no frames asked for.
    last_frame: Option<Instant>,
    pacer: pacer::Pacer,
    /// The body is on its way to a smaller size.
    shrinking: bool,
    renderer: Option<render::Renderer>,
    media: media::Media,
    track: Option<media::Track>,
    /// As `settings.json` has them, and as they are in use (with what the
    /// command line sets on top).
    file: Settings,
    settings: Settings,
    args: Overrides,
    /// The system's locale name (`es-MX`), for the texts' language.
    locale: String,
    texts: &'static Texts,
    tray: Option<tray::Tray>,
    /// The menu, while open.
    menu: Option<MenuWindow>,
    /// Sent to every window when Explorer (and the taskbar) restarts.
    taskbar_created: u32,
    song_text: layout::SongText,
    /// `None` while lyrics are off (the default: they send the song's name
    /// to lrclib.net).
    lyrics: Option<lyrics::Lyrics>,
    /// The song the lyrics are for ([`island_core::lyrics::cache_key`]).
    lyrics_key: String,
    /// Its synced lines; empty without them.
    lines: Vec<island_core::lyrics::Line>,
    /// The line being sung; `None` before the first.
    lyric_index: Option<usize>,
    /// When the open column started scrolling to it.
    lyric_moved: Option<Instant>,
    /// The open island shows the lyrics in a column beside the player (on
    /// by default); off, it is just the player.
    lyrics_column: bool,
    /// The open island's width setting (DIPs, steps of 10): 720 by default
    /// with the column, 560 without.
    open_width_setting: f32,
    /// That width as it fits the screen now ([`layout::fit_open_width`]).
    open_width: f32,
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
        // Small or scaled-up screens get a narrower open island.
        let fit = layout::fit_open_width(self.open_width_setting, mon.width / scale);
        if fit != self.open_width {
            self.open_width = fit;
            if self.open {
                self.retarget();
            }
        }
        let s = self.scale;
        let r = shape::window_rect(
            self.placement,
            mon,
            self.widest() * s,
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
        let scroll = self.lyric_scroll();
        let lyrics = (!self.lines.is_empty()).then_some(render::LyricsView {
            lines: &self.lines,
            index: self.lyric_index,
            scroll,
        });
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
            empty: self.texts.nothing_playing,
            song: self.song_text,
            lyrics,
            open: self.open_layout(),
            glance: self.glance() && lyrics.is_some_and(|l| !l.current().trim().is_empty()),
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
            - (self.width.value - self.open_size().0) / 2.0;
        let ly = (y - body.y) / s;
        let cmd = match layout::hit(lx, ly, self.open_layout()) {
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
        // Growing, it comes in early so the body is never empty; shrinking,
        // it leaves early so the body never cuts it.
        let open_a = if self.shrinking {
            (open - 0.6) / 0.4
        } else {
            (open - 0.25) / 0.55
        }
        .clamp(0.0, 1.0);
        // The small views leave fast as the body grows, and come back early
        // as it shrinks, so it is never empty on the way.
        let pace = if self.shrinking { 1.4 } else { 3.0 };
        let leaving = (1.0 - open * pace).clamp(0.0, 1.0);
        if self.peek_view {
            // One gives way to the other halfway, so they never overlap.
            let grown = ((h - CLOSED.1) / (layout::PEEK.1 - CLOSED.1)).clamp(0.0, 1.0);
            let peek = ((grown - 0.5) * 2.0).clamp(0.0, 1.0);
            let closed = ((0.5 - grown) * 2.0).clamp(0.0, 1.0);
            // It leaves as the body grows past its own height, not the
            // closed one: at rest it is fully visible.
            let past = ((h - layout::PEEK.1) / (OPEN.1 - layout::PEEK.1)).clamp(0.0, 1.0);
            let leaving = (1.0 - past * pace).clamp(0.0, 1.0);
            [closed, peek * leaving, open_a]
        } else {
            [leaving, 0.0, open_a]
        }
    }

    /// The window is sized for the widest the island gets.
    fn widest(&self) -> f32 {
        self.open_width
    }

    /// The open island's layout now: a column for the lines if there are.
    fn open_layout(&self) -> layout::Open {
        layout::Open::new(
            self.open_width,
            self.lyrics_column && !self.lines.is_empty(),
        )
    }

    /// The open island: the same width with or without lines.
    fn open_size(&self) -> (f32, f32) {
        (self.open_width, OPEN.1)
    }

    /// How much of the scroll to the line being sung is left.
    fn lyric_scroll(&self) -> f32 {
        self.lyric_moved.map_or(0.0, |t| {
            layout::lyrics_scroll(t.elapsed().as_secs_f32() * 1000.0)
        })
    }

    /// The lyrics glance: while synced lyrics play, the closed island shows
    /// the line being sung (`lyricsPeek` in the plugin).
    fn glance(&self) -> bool {
        self.lyrics.is_some()
            && !self.lines.is_empty()
            && self.track.as_ref().is_some_and(|t| t.playing)
    }

    fn peeking(&self) -> bool {
        self.peek || self.glance()
    }

    /// Asks for the song's lyrics once it is known well enough: names and a
    /// length lrclib.net takes (without one the match isn't reliable).
    fn ask_lyrics(&mut self) {
        let Some(lyr) = &self.lyrics else { return };
        let Some(t) = &self.track else { return };
        let duration = ((t.duration_ms + 500) / 1000) as u32;
        let key = island_core::lyrics::cache_key(&t.artist, &t.title, &t.album, duration as f64);
        if key == self.lyrics_key {
            return;
        }
        self.lines.clear();
        self.lyric_index = None;
        self.lyric_moved = None;
        if t.artist.is_empty() || t.title.is_empty() || !(10..=3600).contains(&duration) {
            return;
        }
        self.lyrics_key = key.clone();
        lyr.ask(lyrics::Query {
            key,
            artist: t.artist.clone(),
            title: t.title.clone(),
            album: t.album.clone(),
            duration,
        });
    }

    /// The line for the position now, and a timer for the next one; only
    /// while a line is on screen and the song plays. Never a poll: the
    /// position is read again each time, so it can't drift.
    fn follow_lyrics(&mut self) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), LYRIC_TIMER);
        }
        let Some(t) = &self.track else { return };
        if self.lines.is_empty() {
            return;
        }
        let pos = t.position_now();
        let w = island_core::lyrics::window(&self.lines, pos);
        let index = w.index;
        let next_at = w.next_at;
        let playing = t.playing;
        if index != self.lyric_index {
            // One line on, open: the column glides up to it. A seek, or
            // closed: it is just there.
            let step = matches!((self.lyric_index, index), (Some(a), Some(b)) if b == a + 1);
            self.lyric_index = index;
            let column = self.open_layout().lyrics_rect().is_some();
            self.lyric_moved = (self.open && column && step).then(Instant::now);
            if self.lyric_moved.is_some() && self.last_frame.is_none() {
                self.last_frame = Some(Instant::now());
                self.pacer.start();
            } else if self.last_frame.is_none() {
                self.draw();
            }
        }
        if playing
            && (self.open || self.glance())
            && let Some(at) = next_at
        {
            let ms = at.saturating_sub(pos).clamp(40, u32::MAX as u64) as u32;
            unsafe { SetTimer(Some(self.hwnd), LYRIC_TIMER, ms, None) };
        }
    }

    fn set_open(&mut self, open: bool) {
        if self.open == open {
            return;
        }
        // The glance stays the view being left while the island opens (it
        // is dropped once open), so the song's line never flashes between.
        self.open = open;
        self.retarget();
        self.follow_lyrics();
    }

    fn set_peek(&mut self, peek: bool) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), PEEK_TIMER);
            if peek {
                SetTimer(Some(self.hwnd), PEEK_TIMER, PEEK_MS, None);
            }
        }
        self.peek = peek;
        self.retarget();
    }

    fn retarget(&mut self) {
        if self.peeking() && !self.open {
            self.peek_view = true;
        }
        let (w, h) = if self.open {
            self.open_size()
        } else if self.peeking() {
            layout::PEEK
        } else if self.track.is_some() {
            // Wide enough for the song's name.
            (layout::CLOSED_MEDIA_WIDTH, CLOSED.1)
        } else {
            CLOSED
        };
        if h != self.height.target {
            self.shrinking = h < self.height.value;
        }
        self.width.target = w;
        self.height.target = h;
        if self.last_frame.is_none() {
            self.last_frame = Some(Instant::now());
            self.pacer.start();
        }
        self.tick();
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let ms = self
            .last_frame
            .map_or(16.0, |t| (now - t).as_secs_f32() * 1000.0);
        self.last_frame = Some(now);
        let peeking = self.peeking();
        // Growing is loose and a little wobbly; shrinking (also from open
        // to the glance) is firm, so it never dips under where it stops.
        let (pw, ph) = if self.shrinking {
            (spring::CLOSE_WIDTH, spring::CLOSE_HEIGHT)
        } else {
            (spring::OPEN_WIDTH, spring::OPEN_HEIGHT)
        };
        let mut moving = self.width.advance(&pw, ms) | self.height.advance(&ph, ms);
        if self.lyric_moved.is_some() {
            if self.lyric_scroll() > 0.0 {
                moving = true;
            } else {
                self.lyric_moved = None;
            }
        }
        self.draw();
        if !moving {
            // Nothing runs while the island is still.
            if self.open || !peeking {
                self.peek_view = false;
            }
            self.last_frame = None;
            self.pacer.stop();
        }
    }

    /// Puts `file`'s settings to use, changing only what changed: switching
    /// the lyrics on starts their thread, off ends it.
    fn apply(&mut self, file: Settings) {
        self.file = file;
        let s = self.args.over(file);
        self.settings = s;
        self.texts = texts::texts(s.language, &self.locale);
        self.placement = s.placement;
        self.monitor = s.monitor;
        self.song_text = s.song_text;
        if let Some(r) = &self.renderer {
            let _ = r.align_song(s.text_align);
        }
        if s.lyrics != self.lyrics.is_some() {
            self.lyrics = s
                .lyrics
                .then(|| lyrics::Lyrics::start(self.hwnd, WM_APP_LYRICS));
            self.lyrics_key.clear();
            self.lines.clear();
            self.lyric_index = None;
            self.lyric_moved = None;
            self.ask_lyrics();
            self.follow_lyrics();
        }
        self.lyrics_column = s.open_lyrics;
        self.open_width_setting = s.open_width();
        self.layout();
        self.retarget();
    }

    /// The file again, once it has changed; kept as is while it can't be
    /// read or has a typo.
    fn reload(&mut self) {
        if let Some(file) = settings_file::load()
            && file != self.file
        {
            self.apply(file);
        }
    }

    /// The menu's entries and what each does.
    fn menu_rows(&self, startup_on: bool) -> Vec<(u32, render::MenuRow<'static>)> {
        let t = self.texts;
        let row = |kind, label, on, enabled| render::MenuRow {
            kind,
            label,
            on,
            enabled,
        };
        let (toggle, action, line) = (MenuKind::Toggle, MenuKind::Action, MenuKind::Separator);
        let lyrics = self.settings.lyrics;
        vec![
            (MENU_LYRICS, row(toggle, t.lyrics, lyrics, true)),
            (
                MENU_OPEN_LYRICS,
                row(toggle, t.open_lyrics, self.settings.open_lyrics, lyrics),
            ),
            (0, row(line, "", false, false)),
            (
                MENU_STARTUP,
                row(toggle, t.start_with_windows, startup_on, true),
            ),
            (MENU_SETTINGS, row(action, t.open_settings, false, true)),
            (0, row(line, "", false, false)),
            (MENU_QUIT, row(action, t.quit, false, true)),
        ]
    }

    /// Opens the menu at the pointer (or closes it, if open).
    fn open_menu(&mut self) {
        if self.menu.is_some() {
            self.close_menu();
            return;
        }
        let Some(renderer) = &self.renderer else {
            return;
        };
        let startup_on = startup::is_on();
        let kinds: Vec<MenuKind> = self
            .menu_rows(startup_on)
            .iter()
            .map(|r| r.1.kind)
            .collect();
        unsafe {
            let mut at = POINT::default();
            let _ = GetCursorPos(&mut at);
            let mon = MonitorFromPoint(at, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(mon, &mut mi);
            let (mut dx, mut dy) = (96, 96);
            let _ = GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
            let k = dx.max(96) as f32 / 96.0;
            let w = mi.rcWork;
            let work = Rect {
                x: w.left as f32,
                y: w.top as f32,
                width: (w.right - w.left) as f32,
                height: (w.bottom - w.top) as f32,
            };
            let size = ((menu::WIDTH * k).ceil(), (menu::height(&kinds) * k).ceil());
            let (x, y) = menu::place(
                (at.x as f32, at.y as f32),
                size,
                work,
                menu::SCREEN_MARGIN * k,
            );
            let Ok(hwnd) = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOREDIRECTIONBITMAP,
                MENU_CLASS,
                w!("Island menu"),
                WS_POPUP,
                x as i32,
                y as i32,
                size.0 as i32,
                size.1 as i32,
                None,
                None,
                None,
                None,
            ) else {
                return;
            };
            let Ok(target) = renderer.menu_target(hwnd) else {
                let _ = DestroyWindow(hwnd);
                return;
            };
            self.menu = Some(MenuWindow {
                hwnd,
                target,
                scale: k,
                size: (size.0 as u32, size.1 as u32),
                hover: None,
                tracking: false,
                startup: startup_on,
            });
            self.draw_menu();
            let _ = ShowWindow(hwnd, SW_SHOW);
            // Focused, so a click anywhere else (it loses the focus) or Esc
            // closes it.
            let _ = SetForegroundWindow(hwnd);
        }
    }

    fn close_menu(&mut self) {
        if let Some(m) = self.menu.take() {
            unsafe {
                let _ = DestroyWindow(m.hwnd);
            }
        }
    }

    fn draw_menu(&mut self) {
        let rows: Vec<render::MenuRow> = match &self.menu {
            Some(m) => self.menu_rows(m.startup).into_iter().map(|r| r.1).collect(),
            None => return,
        };
        let (Some(r), Some(m)) = (&mut self.renderer, &mut self.menu) else {
            return;
        };
        let view = render::MenuView {
            width: m.size.0,
            height: m.size.1,
            scale: m.scale,
            title: self.texts.name,
            version: concat!("v", env!("CARGO_PKG_VERSION")),
            rows: &rows,
            hover: m.hover,
        };
        let _ = r.draw_menu(&mut m.target, &view);
    }

    /// The menu window's messages. `true` when it asks to open the
    /// settings, which is done with nothing borrowed.
    fn menu_message(&mut self, msg: u32, wp: WPARAM, lp: LPARAM) -> (Option<LRESULT>, bool) {
        let Some(m) = &self.menu else {
            return (None, false);
        };
        let (scale, startup, hwnd) = (m.scale, m.startup, m.hwnd);
        let at = (
            lp.0 as i16 as f32 / scale,
            (lp.0 >> 16) as i16 as f32 / scale,
        );
        let kinds: Vec<MenuKind> = self.menu_rows(startup).iter().map(|r| r.1.kind).collect();
        match msg {
            WM_MOUSEMOVE => {
                let m = self.menu.as_mut().unwrap();
                if !m.tracking {
                    let mut t = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: m.hwnd,
                        dwHoverTime: 0,
                    };
                    m.tracking = unsafe { TrackMouseEvent(&mut t) }.is_ok();
                }
                let hover = menu::hit(&kinds, at.0, at.1);
                if hover != m.hover {
                    m.hover = hover;
                    self.draw_menu();
                }
            }
            WM_MOUSELEAVE => {
                let m = self.menu.as_mut().unwrap();
                m.tracking = false;
                if m.hover.take().is_some() {
                    self.draw_menu();
                }
            }
            WM_LBUTTONUP => {
                let Some(i) = menu::hit(&kinds, at.0, at.1) else {
                    return (Some(LRESULT(0)), false);
                };
                let (id, row) = self.menu_rows(startup).swap_remove(i);
                if !row.enabled {
                    return (Some(LRESULT(0)), false);
                }
                if row.kind == MenuKind::Toggle {
                    // The switch flips; the menu stays to flip more.
                    self.command(id);
                    if let Some(m) = &mut self.menu {
                        m.startup = startup::is_on();
                    }
                    self.draw_menu();
                } else {
                    self.close_menu();
                    if id == MENU_SETTINGS {
                        return (Some(LRESULT(0)), true);
                    }
                    self.command(id);
                }
            }
            WM_KEYDOWN if wp.0 == VK_ESCAPE.0 as usize => self.close_menu(),
            // Focus elsewhere: a click outside it.
            WM_ACTIVATE if wp.0 & 0xFFFF == 0 => unsafe {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            },
            WM_CLOSE => self.close_menu(),
            _ => return (None, false),
        }
        (Some(LRESULT(0)), false)
    }

    /// What a menu item does. The switches go to the file, and take effect
    /// at once (also if it can't be written now).
    fn command(&mut self, id: u32) {
        let mut file = self.file;
        let (key, on) = match id {
            MENU_LYRICS => ("lyrics", &mut file.lyrics),
            MENU_OPEN_LYRICS => ("openLyrics", &mut file.open_lyrics),
            MENU_STARTUP => {
                let _ = startup::set(!startup::is_on());
                return;
            }
            MENU_QUIT => {
                // The icon goes first: the taskbar keeps a dead one otherwise.
                self.tray = None;
                unsafe {
                    let _ = DestroyWindow(self.hwnd);
                }
                return;
            }
            _ => return,
        };
        // From what is in use; the command line no longer holds it.
        *on = !match id {
            MENU_LYRICS => std::mem::take(&mut self.args.lyrics) | self.settings.lyrics,
            _ => self
                .args
                .open_lyrics
                .take()
                .unwrap_or(self.settings.open_lyrics),
        };
        let _ = settings_file::set(key, Value::Bool(*on));
        self.apply(file);
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
            WM_APP_FRAME => {
                self.pacer.drawn();
                // A frame already on its way when the island came to rest.
                if self.last_frame.is_some() {
                    self.frame();
                }
            }
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
                self.ask_lyrics();
                // Music starting or stopping widens or narrows the closed island.
                self.retarget();
                if self.last_frame.is_none() {
                    self.draw();
                }
                self.tick();
                // A seek, a pause or a new song: the line again.
                self.follow_lyrics();
            }
            WM_APP_LYRICS => {
                let answer = self.lyrics.as_ref().and_then(|l| l.latest());
                if let Some((key, found)) = answer
                    && key == self.lyrics_key
                {
                    // Only synced lyrics can be followed; plain ones give
                    // no timing.
                    self.lines = if found.kind == island_core::lyrics::Kind::Synced {
                        found.lines
                    } else {
                        Vec::new()
                    };
                    self.lyric_index = None;
                    self.lyric_moved = None;
                    self.retarget();
                    self.follow_lyrics();
                    if self.last_frame.is_none() {
                        self.draw();
                    }
                }
            }
            WM_TIMER if wp.0 == LYRIC_TIMER => self.follow_lyrics(),
            WM_LBUTTONUP => {
                let (x, y) = (lp.0 as i16 as f32, (lp.0 >> 16) as i16 as f32);
                self.click(x, y);
            }
            WM_DPICHANGED | WM_DISPLAYCHANGE | WM_APP_LAYOUT => self.layout(),
            // Clicking the island never takes the focus from the app in use.
            WM_MOUSEACTIVATE => return Some(LRESULT(MA_NOACTIVATE as isize)),
            // An editor saves in steps: read once it is done.
            WM_APP_SETTINGS => unsafe {
                SetTimer(Some(self.hwnd), SETTINGS_TIMER, SETTINGS_WAIT_MS, None);
            },
            WM_TIMER if wp.0 == SETTINGS_TIMER => {
                unsafe {
                    let _ = KillTimer(Some(self.hwnd), SETTINGS_TIMER);
                }
                self.reload();
            }
            WM_RBUTTONUP => self.open_menu(),
            WM_APP_TRAY => {
                if matches!((lp.0 & 0xFFFF) as u32, WM_LBUTTONUP | WM_RBUTTONUP) {
                    self.open_menu();
                }
            }
            WM_DESTROY => unsafe { PostQuitMessage(0) },
            _ if msg == self.taskbar_created && msg != 0 => {
                if let Some(t) = &self.tray {
                    t.add(self.texts.name);
                }
            }
            _ => return None,
        }
        Some(LRESULT(0))
    }
}

/// Opens `settings.json` in the app for .json files, else in Notepad.
fn open_settings() {
    let Ok(path) = settings_file::ensure() else {
        return;
    };
    let file = HSTRING::from(path.as_os_str());
    unsafe {
        let done = ShellExecuteW(None, w!("open"), &file, None, None, SW_SHOWNORMAL);
        if done.0 as isize <= 32 {
            let quoted = HSTRING::from(format!("\"{}\"", path.display()));
            ShellExecuteW(
                None,
                w!("open"),
                w!("notepad.exe"),
                &quoted,
                None,
                SW_SHOWNORMAL,
            );
        }
    }
}

/// The menu window's messages go to the app, like the island's.
extern "system" fn menu_wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let (handled, settings) = APP.with(|cell| match cell.try_borrow_mut() {
        Ok(mut app) => app
            .as_mut()
            .map_or((None, false), |a| a.menu_message(msg, wp, lp)),
        Err(_) => (None, false),
    });
    if settings {
        open_settings();
    }
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, msg, wp, lp) })
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

/// The user's locale name, like `es-MX`.
fn locale() -> String {
    let mut name = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(&mut name) };
    String::from_utf16_lossy(&name[..(n.max(1) - 1) as usize])
}

/// The value after `--name` on the command line.
fn arg(name: &str) -> Option<String> {
    std::env::args().skip_while(|a| a != name).nth(1)
}

/// Settings given on the command line, for trying things out: they win
/// over `settings.json`.
#[derive(Default)]
struct Overrides {
    placement: Option<Placement>,
    monitor: Option<MonitorChoice>,
    text_align: Option<layout::TextAlign>,
    song_text: Option<layout::SongText>,
    lyrics: bool,
    open_lyrics: Option<bool>,
    open_width: Option<f32>,
}

impl Overrides {
    fn from_args() -> Self {
        Self {
            placement: arg("--placement").and_then(|s| Placement::parse(&s)),
            monitor: arg("--monitor").and_then(|s| MonitorChoice::parse(&s)),
            text_align: arg("--text-align").and_then(|s| layout::TextAlign::parse(&s)),
            song_text: arg("--song-text").and_then(|s| layout::SongText::parse(&s)),
            lyrics: std::env::args().any(|a| a == "--lyrics"),
            open_lyrics: arg("--open-lyrics").map(|s| s != "off"),
            open_width: arg("--open-width").and_then(|s| layout::parse_open_width(&s)),
        }
    }

    fn over(&self, mut s: Settings) -> Settings {
        s.placement = self.placement.unwrap_or(s.placement);
        s.monitor = self.monitor.unwrap_or(s.monitor);
        s.text_align = self.text_align.unwrap_or(s.text_align);
        s.song_text = self.song_text.unwrap_or(s.song_text);
        s.lyrics |= self.lyrics;
        s.open_lyrics = self.open_lyrics.unwrap_or(s.open_lyrics);
        s.open_width = self.open_width.or(s.open_width);
        s
    }
}

fn main() -> Result<()> {
    unsafe {
        // One island per user session: a second start (the exe opened
        // twice, or by hand while it starts with Windows) just leaves. The
        // handle lives as long as the process.
        let _single = CreateMutexW(None, false, w!("Local\\IslandDesktop"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
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
        let menu_class = WNDCLASSW {
            lpfnWndProc: Some(menu_wndproc),
            hInstance: instance.into(),
            lpszClassName: MENU_CLASS,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        if RegisterClassW(&menu_class) == 0 {
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
        let defaults = Settings::default();
        let locale = locale();
        let texts = texts::texts(defaults.language, &locale);
        let app = App {
            hwnd,
            placement: defaults.placement,
            monitor: defaults.monitor,
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
            pacer: pacer::Pacer::new(hwnd, WM_APP_FRAME),
            shrinking: false,
            track: None,
            file: defaults,
            settings: defaults,
            args: Overrides::from_args(),
            locale,
            texts,
            tray: Some(tray::Tray::new(hwnd, WM_APP_TRAY, texts.name)),
            menu: None,
            taskbar_created: RegisterWindowMessageW(w!("TaskbarCreated")),
            song_text: defaults.song_text,
            // Started by the settings, if they turn them on.
            lyrics: None,
            lyrics_key: String::new(),
            lines: Vec::new(),
            lyric_index: None,
            lyric_moved: None,
            lyrics_column: defaults.open_lyrics,
            open_width_setting: defaults.open_width(),
            // Fitted to the screen by the first layout.
            open_width: defaults.open_width(),
        };
        APP.with_borrow_mut(|a| *a = Some(app));
        // A file with a typo at start: the defaults until it is fixed.
        let file = settings_file::load().unwrap_or_default();
        APP.with_borrow_mut(|a| a.as_mut().map(|a| a.apply(file)));
        settings_file::watch(hwnd, WM_APP_SETTINGS);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        // Takes the tray icon away too, if it is still there.
        APP.with_borrow_mut(|a| *a = None);
    }
    Ok(())
}
