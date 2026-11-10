// No console window in release builds; debug builds keep it for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod banners;
mod bars;
mod bluetooth;
mod lyrics;
mod media;
mod notifications;
mod pacer;
mod privacy;
mod render;
mod settings_file;
mod startup;
mod tray;
mod volume;

use std::cell::RefCell;
use std::time::Instant;

use island_core::bluetooth::Event as BtEvent;
use island_core::focus::{self, Ended, Lengths, Phase};
use island_core::glance;
use island_core::media as layout;
use island_core::menu::{self, Kind as MenuKind};
use island_core::monitor::{self, Monitor, MonitorChoice};
use island_core::pages::{self, FocusButton, Page};
use island_core::privacy::{Change, Device};
use island_core::settings::Settings;
use island_core::shape::{self, Placement, Rect};
use island_core::spring::{self, Spring};
use island_core::texts::{self, Texts};
use serde_json::Value;
use windows::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Globalization::{GetTimeFormatEx, GetUserDefaultLocaleName, TIME_NOSECONDS};
use windows::Win32::Graphics::Gdi::{
    CreateRectRgn, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST,
    MONITORINFO, MonitorFromPoint, ScreenToClient, SetWindowRgn,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
    SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_ESCAPE,
};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW, MA_NOACTIVATE, MONITORINFOF_PRIMARY, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SW_SHOW,
    SW_SHOWNOACTIVATE, SW_SHOWNORMAL, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow,
    SetTimer, SetWindowPos, ShowWindow, TranslateMessage, WM_ACTIVATE, WM_APP, WM_CLOSE,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_DWMCOLORIZATIONCOLORCHANGED, WM_KEYDOWN,
    WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONUP, WM_SETTINGCHANGE,
    WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::BOOL;
use windows::core::{Error, HSTRING, PCWSTR, Result, w};

// Sizes in DIPs (1/96 inch); the springs run in DIPs too, so a DPI change
// doesn't jolt the motion.
const CLOSED: (f32, f32) = (180.0, 32.0);
const OPEN: (f32, f32) = (layout::OPEN_WIDTH, pages::OPEN_HEIGHT);
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
/// The system's volume changed (see `volume.rs`).
const WM_APP_VOLUME: u32 = WM_APP + 7;
/// Other speakers are the default now.
const WM_APP_SPEAKERS: u32 = WM_APP + 8;
/// Who uses the microphone or camera changed (see `privacy.rs`).
const WM_APP_PRIVACY: u32 = WM_APP + 9;
/// A Bluetooth device connected or disconnected (see `bluetooth.rs`).
const WM_APP_BLUETOOTH: u32 = WM_APP + 10;
/// A notification showed (see `notifications.rs`).
const WM_APP_NOTIFICATION: u32 = WM_APP + 11;

/// How long the glance's content takes to change in place, in ms.
const SWAP_MS: f32 = 300.0;

/// What the glance showed before a change in place (see `render::Before`).
enum Before {
    Notice(Notice),
    Line(String),
    Song,
}

/// A glance's content, kept until it ends.
#[derive(Clone, Default)]
struct Notice {
    icon: island_core::icons::Icon,
    tint: [f32; 3],
    title: std::borrow::Cow<'static, str>,
    detail: String,
    level: Option<f32>,
    /// A notification's: its own, bigger glance with the app's logo, a
    /// second line and a ring in the logo's color.
    toast: bool,
    body: String,
    logo: Option<std::sync::Arc<Vec<u8>>>,
    /// The app that sent the notification, opened by a click on it.
    app_id: String,
}

/// Opens the app with this AppUserModelID, as its tile in Start would.
/// Windows lets no other app click a notification itself, so this is as
/// close to it as the island gets.
fn open_app(app_id: &str) {
    // An id the shell gave, inside its apps folder: nothing that could leave
    // it or pass on arguments.
    if app_id.is_empty() || app_id.contains(['"', ':', '/']) || app_id.contains("..") {
        return;
    }
    let target = HSTRING::from(format!("shell:AppsFolder\\{app_id}"));
    unsafe {
        ShellExecuteW(None, w!("open"), &target, None, None, SW_SHOWNORMAL);
    }
}

/// How long a notification's glance stays: a touch longer than the others,
/// it has more to read.
const TOAST_MS: u32 = 5000;
/// Reads the settings once an editor has finished saving them.
const SETTINGS_TIMER: usize = 6;
const SETTINGS_WAIT_MS: u32 = 150;
/// When the focus timer's phase ends (one shot, set again each time).
const FOCUS_TIMER: usize = 7;
/// When a Bluetooth change still settling will have held.
const BLUETOOTH_TIMER: usize = 8;
/// The wheel changes the focus length by this many minutes.
const WHEEL_MINUTES: i32 = 5;
// The menu's items.
const MENU_LYRICS: u32 = 1;
const MENU_OPEN_LYRICS: u32 = 2;
const MENU_STARTUP: u32 = 3;
const MENU_SETTINGS: u32 = 4;
const MENU_QUIT: u32 = 5;
const MENU_ONLY_ISLAND: u32 = 6;
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
    /// The open island's width setting (DIPs, steps of 10): 560 by default,
    /// with the column or without.
    open_width_setting: f32,
    /// That width as it fits the screen now ([`layout::fit_open_width`]).
    open_width: f32,
    /// The most the song's glance grows to on this screen.
    peek_most: f32,
    /// Windows' banners are turned off for the island to tell alone; `None`
    /// before the first settings, which also bring back any a crashed
    /// island left off.
    banners_off: Option<bool>,
    /// The song's glance and the closed island with music, as wide as their
    /// text needs ([`layout::peek_width`], [`layout::closed_width`]).
    peek_w: f32,
    closed_w: f32,
    /// The open island's page: the one last chosen (while the app runs).
    page: Page,
    timer: focus::Timer,
    /// The steady clock the timer counts on.
    start: Instant,
    /// What the glance tells instead of the song (the timer's end).
    notice: Option<Notice>,
    /// Core Audio, while the volume glance is on, and the volume last
    /// told (a change is told only if it differs).
    volume: Option<volume::Volume>,
    last_volume: Option<(f32, bool)>,
    /// The default speakers are Bluetooth ones: read again on a new song or
    /// a change of speakers, for the lyrics' "auto" delay.
    on_bluetooth: bool,
    /// The watch on the microphone and camera, while that glance is on,
    /// and the apps last seen using each (`Device::ALL`'s order).
    privacy: Option<privacy::Privacy>,
    users: [Vec<String>; 2],
    /// The watch on paired Bluetooth devices, while that glance is on.
    bluetooth: Option<bluetooth::Bluetooth>,
    /// The watch on notifications, while that glance is on and Windows
    /// lets the island read them.
    notifications: Option<notifications::Notifications>,
    /// The glance's content being left in place, and since when.
    swap: Option<(Before, Instant)>,
    /// When the last notification arrived: its ring beats for a moment.
    pulse: Option<Instant>,
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
        self.peek_most = layout::fit_open_width(layout::PEEK_MOST, mon.width / scale);
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
        // The focus timer's words, for this frame.
        let now = self.now_ms();
        let (t, tm, l) = (self.texts, &self.timer, &self.settings.focus);
        let left = focus::clock(tm.left_ms(now));
        let active = tm.active(l);
        let (phase, round) = match tm.phase {
            Phase::Focus => (t.focus, (tm.rounds + 1).min(l.every)),
            Phase::Break => (t.short_break, tm.rounds),
            Phase::LongBreak => (t.long_break, l.every),
        };
        let label = format!("{phase} · {}", texts::fill(t.round, &[round, l.every]));
        let next = if !active {
            t.wheel_hint.to_string()
        } else if tm.phase == Phase::Focus {
            let long = tm.rounds + 1 >= l.every;
            let minutes = if long { l.long } else { l.short };
            texts::fill(t.then_break, &[minutes])
        } else {
            t.then_focus.to_string()
        };
        let clock = if tm.running() {
            left.clone()
        } else {
            time_of_day()
        };
        let focus_view = render::FocusView {
            label: &label,
            time: &left,
            next: &next,
            progress: tm.progress(now, l),
            rounds: tm.rounds,
            every: l.every,
            running: tm.running(),
            resting: tm.phase != Phase::Focus,
        };
        let ring = island_core::notifications::ring(
            self.pulse
                .map_or(f32::MAX, |t| t.elapsed().as_secs_f32() * 1000.0),
        );
        let orbit = self
            .pulse
            .and_then(|t| island_core::notifications::orbit(t.elapsed().as_secs_f32() * 1000.0));
        fn shown<'a>(
            n: &'a Notice,
            when: &'a str,
            ring: f32,
            orbit: Option<(f32, f32)>,
        ) -> render::Notice<'a> {
            render::Notice {
                icon: n.icon,
                tint: n.tint,
                title: &n.title,
                detail: &n.detail,
                level: n.level,
                toast: n.toast,
                body: &n.body,
                logo: n.logo.as_ref(),
                when,
                ring,
                orbit,
            }
        }
        let when = self.texts.now;
        let view = |n| shown(n, when, ring, orbit);
        let scene = render::Scene {
            page: self.page,
            tabs_clock: &clock,
            tabs_timer: tm.running(),
            focus: focus_view,
            notice: self.notice.as_ref().map(view),
            closed_timer: (active && self.track.is_none()).then_some(left.as_str()),
            swap: self.swap.as_ref().map(|(b, at)| render::Swap {
                before: match b {
                    Before::Notice(n) => render::Before::Notice(view(n)),
                    Before::Line(l) => render::Before::Line(l),
                    Before::Song => render::Before::Song,
                },
                t: {
                    let x = (at.elapsed().as_secs_f32() * 1000.0 / SWAP_MS).min(1.0);
                    1.0 - (1.0 - x).powi(3)
                },
            }),
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
            peek_width: self.peek_size().0,
            closed_width: self.closed_w,
            openness: self.openness(),
            peek_view: self.peek_view,
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

    /// Wakes up when what is shown changes, only while it is on screen: the
    /// song's second (open, on Home), the focus timer's second, or the
    /// minute of the clock on the tabs.
    fn tick(&self) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TICK_TIMER);
        }
        let mut wait: Option<u32> = None;
        let mut sooner = |ms: u32| wait = Some(wait.map_or(ms, |w| w.min(ms)));
        if let Some(t) = &self.track
            && self.open
            && self.page == Page::Home
            && t.playing
            && t.duration_ms > 0
        {
            sooner(1000 - (t.position_now() % 1000) as u32 + 15);
        }
        let shown = self.open || self.track.is_none();
        if self.timer.running() && shown {
            // It shows the second rounded up: it changes on a whole one.
            let left = self.timer.left_ms(self.now_ms());
            sooner((left % 1000) as u32 + 15);
        } else if self.open {
            let t = unsafe { GetLocalTime() };
            let into = u32::from(t.wSecond) * 1000 + u32::from(t.wMilliseconds);
            sooner(60_000u32.saturating_sub(into) + 15);
        }
        if let Some(ms) = wait {
            unsafe { SetTimer(Some(self.hwnd), TICK_TIMER, ms.max(16), None) };
        }
    }

    /// Milliseconds on the steady clock, for the focus timer.
    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// One timer for when the focus phase ends, while it runs.
    fn arm_focus(&self) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), FOCUS_TIMER);
            if let Some(at) = self.timer.ends_at() {
                let ms = at.saturating_sub(self.now_ms()).min(u32::MAX as u64 / 2) as u32;
                SetTimer(Some(self.hwnd), FOCUS_TIMER, ms + 5, None);
            }
        }
    }

    /// The phase is over: a glance tells it, and the next one is set.
    fn focus_ended(&mut self) {
        let (now, l) = (self.now_ms(), self.settings.focus);
        if let Some(ended) = self.timer.finish(now, &l) {
            let t = self.texts;
            let (title, detail, tint) = match ended {
                Ended::Focus { break_minutes, .. } => (
                    t.focus_done,
                    texts::fill(t.take_break, &[break_minutes]),
                    render::BREAK_TINT,
                ),
                Ended::Break => (t.break_over, t.ready.to_string(), render::FOCUS_TINT),
            };
            self.notify(
                Notice {
                    icon: island_core::icons::Icon::Timer,
                    tint,
                    title: title.into(),
                    detail,
                    level: None,
                    ..Default::default()
                },
                PEEK_MS,
            );
        }
        self.arm_focus();
        self.redraw();
    }

    /// What the glance shows now, to change from in place.
    fn showing(&self) -> Before {
        if let Some(n) = &self.notice {
            return Before::Notice(n.clone());
        }
        let line = self
            .lyric_index
            .and_then(|i| self.lines.get(i))
            .map(|l| l.text.clone())
            .filter(|l| self.glance() && !l.trim().is_empty());
        line.map_or(Before::Song, Before::Line)
    }

    /// The glance's content is about to change: if the glance is out and
    /// at rest, `before` fades out as the new content comes in. (Growing
    /// or shrinking, the body's own move carries the change.)
    fn swap_from(&mut self, before: Before) {
        let at_rest = !self.open
            && self.peek_view
            && self.height.target > CLOSED.1
            && (self.height.value - self.height.target).abs() < 0.5
            && self.alphas()[1] > 0.99;
        if !at_rest {
            return;
        }
        self.swap = Some((before, Instant::now()));
        if self.last_frame.is_none() {
            self.last_frame = Some(Instant::now());
            self.pacer.start();
        }
    }

    /// A glance that tells something, for `ms` after its last change.
    fn notify(&mut self, n: Notice, ms: u32) {
        // A notice of the same kind (the volume again) just updates.
        let before = self.showing();
        if !matches!(&before, Before::Notice(b) if b.title == n.title && b.icon == n.icon) {
            self.swap_from(before);
        }
        self.notice = Some(n);
        unsafe {
            let _ = KillTimer(Some(self.hwnd), PEEK_TIMER);
            SetTimer(Some(self.hwnd), PEEK_TIMER, ms, None);
        }
        self.peek = true;
        self.retarget();
    }

    /// The volume changed: a glance, if it's a change worth one.
    fn volume_changed(&mut self, now: (f32, bool)) {
        let before = self.last_volume.replace(now);
        if !glance::volume_changed(before, now) {
            return;
        }
        let (level, muted) = now;
        let t = self.texts;
        let n = Notice {
            icon: glance::volume_icon(level, muted),
            tint: render::WHITE,
            title: if muted { t.muted } else { t.volume }.into(),
            // Muted, the volume kept for later means nothing now.
            detail: if muted {
                String::new()
            } else {
                format!("{} %", glance::percent(level))
            },
            level: Some(if muted { 0.0 } else { level }),
            ..Default::default()
        };
        self.notify(n, glance::LEVEL_MS);
    }

    /// A paired Bluetooth device connected or disconnected: a glance for
    /// the last change told (a burst ends where it ended up).
    fn bluetooth_changed(&mut self) {
        let Some(b) = &self.bluetooth else { return };
        let (mut told, next) = b.due();
        unsafe {
            let _ = KillTimer(Some(self.hwnd), BLUETOOTH_TIMER);
            if let Some(ms) = next {
                SetTimer(Some(self.hwnd), BLUETOOTH_TIMER, ms as u32 + 20, None);
            }
        }
        let Some(e) = told.pop() else { return };
        let t = self.texts;
        let (title, name) = match e {
            BtEvent::Connected(n) => (t.connected, n),
            BtEvent::Disconnected(n) => (t.disconnected, n),
        };
        let n = Notice {
            icon: island_core::bluetooth::ICON,
            tint: island_core::bluetooth::TINT,
            title: title.into(),
            detail: name,
            level: None,
            ..Default::default()
        };
        self.notify(n, PEEK_MS);
    }

    /// Notifications showed: a glance for the last one (a burst ends on
    /// the newest): its app, title, body and logo, and a ring that beats.
    fn notified(&mut self) {
        let Some(n) = &self.notifications else { return };
        let Some(a) = n.take().pop() else { return };
        // As the user is now: presenting, a game, Do Not Disturb.
        let state = unsafe { SHQueryUserNotificationState() }.map_or(5, |s| s.0);
        let moment = island_core::notifications::Moment::from_state(state);
        let full_screen = self.settings.notifications_in_full_screen;
        if !island_core::notifications::glances(moment, notifications::quiet(), full_screen) {
            return;
        }
        if moment == island_core::notifications::Moment::FullScreen {
            // A full-screen window may have gone over the island: on top
            // again (without taking the focus).
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
                );
            }
        }
        let t = a.toast;
        // An app new to the island: its next ones come without a banner.
        if self.banners_off == Some(true) {
            banners::quiet(&t.app_id);
        }
        let n = Notice {
            icon: island_core::notifications::ICON,
            tint: render::WHITE,
            title: t.app.into(),
            detail: t.title,
            toast: true,
            body: t.body,
            logo: a.logo,
            app_id: t.app_id,
            ..Default::default()
        };
        self.pulse = Some(Instant::now());
        self.notify(n, TOAST_MS);
    }

    /// Who uses `Device::ALL[i]` may have changed: a glance if an app
    /// began to, or the last one stopped.
    fn privacy_changed(&mut self, i: usize) {
        let Some(&d) = Device::ALL.get(i) else {
            return;
        };
        let now = privacy::using(d);
        let t = self.texts;
        let n = match island_core::privacy::change(&self.users[i], &now) {
            Some(Change::On(app)) => Notice {
                icon: d.icon(),
                tint: d.tint(),
                title: if d == Device::Microphone {
                    t.microphone_on
                } else {
                    t.camera_on
                }
                .into(),
                detail: app.to_string(),
                level: None,
                ..Default::default()
            },
            Some(Change::Off) => Notice {
                icon: d.icon(),
                tint: render::WHITE,
                title: if d == Device::Microphone {
                    t.microphone_off
                } else {
                    t.camera_off
                }
                .into(),
                detail: String::new(),
                level: None,
                ..Default::default()
            },
            None => {
                self.users[i] = now;
                return;
            }
        };
        self.users[i] = now;
        self.notify(n, PEEK_MS);
    }

    fn redraw(&mut self) {
        if self.last_frame.is_none() {
            self.draw();
        }
        self.tick();
    }

    /// From window px to the open layout's DIPs: it is centered in the
    /// body; y from the island's top.
    fn to_open(&self, x: f32, y: f32) -> (f32, f32) {
        let s = self.scale;
        let body = shape::body_rect(
            self.placement,
            self.window.width,
            self.width.value * s,
            self.height.value * s,
            EAR * s,
        );
        let lx = (x - body.x - shape::ear_room(self.placement, EAR * s)) / s
            - (self.width.value - self.open_size().0) / 2.0;
        (lx, (y - body.y) / s)
    }

    /// A notification's glance is out (not yet giving way).
    fn toast_shown(&self) -> bool {
        self.peek && self.notice.as_ref().is_some_and(|n| n.toast)
    }

    fn click(&mut self, x: f32, y: f32) {
        // On a notification: its app, and the glance has done its job.
        if !self.open && self.toast_shown() {
            if let Some(n) = &self.notice {
                open_app(&n.app_id);
            }
            self.set_peek(false);
            return;
        }
        if !self.open {
            return;
        }
        let (lx, ly) = self.to_open(x, y);
        if let Some(page) = pages::hit_tab(lx, ly) {
            if page != self.page {
                self.page = page;
                self.redraw();
            }
            return;
        }
        // The page is laid out from its own top.
        let py = ly - pages::PAGE_Y;
        match self.page {
            Page::Home => {
                if self.track.is_none() {
                    return;
                }
                let cmd = match layout::hit(lx, py, self.open_layout()) {
                    Some(layout::Button::Prev) => media::Command::Prev,
                    Some(layout::Button::PlayPause) => media::Command::PlayPause,
                    Some(layout::Button::Next) => media::Command::Next,
                    None => return,
                };
                self.media.send(cmd);
            }
            Page::Focus => {
                let (now, l) = (self.now_ms(), self.settings.focus);
                match pages::hit_focus(lx, py, self.open_width) {
                    Some(FocusButton::PlayPause) => self.timer.toggle(now),
                    Some(FocusButton::Reset) => self.timer.reset(&l),
                    Some(FocusButton::Skip) => {
                        self.timer.skip(now, &l);
                    }
                    None => return,
                }
                self.arm_focus();
                self.redraw();
            }
        }
    }

    /// The wheel over the focus time, before a session starts: its length,
    /// kept in `settings.json`.
    fn wheel(&mut self, x: i32, y: i32, delta: i16) {
        if !self.open || self.page != Page::Focus || self.timer.active(&self.settings.focus) {
            return;
        }
        let mut p = POINT { x, y };
        unsafe {
            let _ = ScreenToClient(self.hwnd, &mut p);
        }
        let (lx, ly) = self.to_open(p.x as f32, p.y as f32);
        if !pages::over_time(lx, ly - pages::PAGE_Y, self.open_width) {
            return;
        }
        let step = if delta > 0 {
            WHEEL_MINUTES
        } else {
            -WHEEL_MINUTES
        };
        let minutes = focus::step(self.settings.focus.focus, step, Lengths::FOCUS);
        if minutes == self.settings.focus.focus {
            return;
        }
        let mut file = self.file;
        file.focus.focus = minutes;
        let _ = settings_file::set("focusMinutes", Value::from(minutes));
        self.apply(file);
    }

    /// How open the body is, from the small view it grows from (the closed
    /// island or the glance) to the open one: 0 to 1.
    fn openness(&self) -> f32 {
        let small = if self.peek_view {
            self.peek_size().1
        } else {
            CLOSED.1
        };
        ((self.height.value - small) / (OPEN.1 - small)).clamp(0.0, 1.0)
    }

    /// How visible the closed, glance and open contents are, from the
    /// body's height: each one fades as the body leaves its size.
    fn alphas(&self) -> [f32; 3] {
        let h = self.height.value;
        let open = ((h - CLOSED.1) / (OPEN.1 - CLOSED.1)).clamp(0.0, 1.0);
        // The closed content leaves fast; the open one comes in once there
        // is room for it. It shrinks with the body (scaled to it), so
        // closing it can stay until the body is small, while the cover
        // travels on its own.
        let open_a = if self.shrinking {
            (open - 0.3) / 0.45
        } else {
            (open - 0.25) / 0.55
        }
        .clamp(0.0, 1.0);
        // The small views leave fast as the body grows; shrinking, they come
        // back only once it is nearly their size, not over a tall body.
        let pace = if self.shrinking { 2.4 } else { 3.0 };
        let leaving = (1.0 - open * pace).clamp(0.0, 1.0);
        if self.peek_view {
            let peek_h = self.peek_size().1;
            // One gives way to the other halfway, so they never overlap. A
            // taller glance (a notification's) is fully there by the song's
            // glance's height, so growing from that one never shows the
            // closed island in between; the body uncovers the rest.
            let small_h = peek_h.min(layout::PEEK.1);
            let grown = ((h - CLOSED.1) / (small_h - CLOSED.1)).clamp(0.0, 1.0);
            let peek = ((grown - 0.5) * 2.0).clamp(0.0, 1.0);
            let closed = ((0.5 - grown) * 2.0).clamp(0.0, 1.0);
            // It leaves as the body grows past its own height, not the
            // closed one: at rest it is fully visible.
            let past = ((h - peek_h) / (OPEN.1 - peek_h)).clamp(0.0, 1.0);
            let leaving = (1.0 - past * pace).clamp(0.0, 1.0);
            [closed, peek * leaving, open_a]
        } else {
            [leaving, 0.0, open_a]
        }
    }

    /// The window is sized for the widest the island gets.
    fn widest(&self) -> f32 {
        self.open_width.max(self.peek_most)
    }

    /// Measures the closed island's and the song's glance's text, for
    /// their widths: the song's name, and the line being sung if it shows.
    fn fit_text(&mut self) {
        let Some(r) = &self.renderer else { return };
        let Some(t) = &self.track else { return };
        let name = self.song_text.spans(&t.title, &t.artist);
        self.closed_w = layout::closed_width(r.text_width(&name, true));
        let line = match self.showing() {
            Before::Line(l) => vec![(l, false)],
            _ => name,
        };
        self.peek_w = layout::peek_width(r.text_width(&line, false), self.peek_most);
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

    /// The glance's size: a level on its own is barely bigger than the
    /// closed island; over the lyrics' glance it takes that one's size.
    fn peek_size(&self) -> (f32, f32) {
        let level = self.notice.as_ref().is_some_and(|n| n.level.is_some());
        if self.notice.as_ref().is_some_and(|n| n.toast) {
            island_core::notifications::SIZE
        } else if level && !self.glance() {
            glance::LEVEL
        } else if self.notice.is_some() {
            layout::PEEK
        } else {
            (self.peek_w, layout::PEEK.1)
        }
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
        // The sound comes out later than the player's position says.
        let pos = t
            .position_now()
            .saturating_sub(u64::from(self.settings.lyrics_delay(self.on_bluetooth)));
        let w = island_core::lyrics::window(&self.lines, pos);
        let index = w.index;
        let next_at = w.next_at;
        let playing = t.playing;
        if index != self.lyric_index {
            // Closed, the glance's line changes in place.
            if self.notice.is_none() && !self.open {
                let b = self.showing();
                self.swap_from(b);
            }
            // One line on, open: the column glides up to it. A seek, or
            // closed: it is just there.
            let step = matches!((self.lyric_index, index), (Some(a), Some(b)) if b == a + 1);
            self.lyric_index = index;
            // Closed, the glance grows or shrinks to the new line.
            if !self.open && self.glance() {
                self.retarget();
            }
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
        self.fit_text();
        let (w, h) = if self.open {
            self.open_size()
        } else if self.peeking() {
            self.peek_size()
        } else if self.track.is_some() {
            // Wide enough for the song's name.
            (self.closed_w, CLOSED.1)
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
        if let Some((_, at)) = &self.swap {
            if at.elapsed().as_secs_f32() * 1000.0 < SWAP_MS {
                moving = true;
            } else {
                self.swap = None;
            }
        }
        if let Some(at) = self.pulse {
            let toast = self.notice.as_ref().is_some_and(|n| n.toast);
            if toast && at.elapsed().as_secs_f32() * 1000.0 < island_core::notifications::PULSE_MS {
                moving = true;
            } else {
                self.pulse = None;
            }
        }
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
            if !self.peek {
                self.notice = None;
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
        if s.default_page != self.settings.default_page {
            self.page = s.default_page;
        }
        self.timer.set_lengths(&self.settings.focus, &s.focus);
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
        if s.volume_glance != self.volume.is_some() {
            self.volume = s
                .volume_glance
                .then(|| volume::Volume::start(self.hwnd, WM_APP_VOLUME, WM_APP_SPEAKERS).ok())
                .flatten();
            self.last_volume = self.volume.as_ref().and_then(|v| v.now());
        }
        if s.privacy_glance != self.privacy.is_some() {
            self.privacy = s
                .privacy_glance
                .then(|| privacy::Privacy::start(self.hwnd, WM_APP_PRIVACY))
                .flatten();
            // What is in use already was told by Windows' own light.
            self.users = Device::ALL.map(privacy::using);
        }
        if s.bluetooth_glance != self.bluetooth.is_some() {
            self.bluetooth = s
                .bluetooth_glance
                .then(|| bluetooth::Bluetooth::start(self.hwnd, WM_APP_BLUETOOTH).ok())
                .flatten();
        }
        if s.notification_glance != self.notifications.is_some() {
            self.notifications = s
                .notification_glance
                .then(|| {
                    notifications::Notifications::start(
                        self.hwnd,
                        WM_APP_NOTIFICATION,
                        self.texts.notification,
                    )
                })
                .flatten();
        }
        // Windows' banners off only while the island can tell them instead.
        let alone = !s.windows_banners && self.notifications.is_some();
        if Some(alone) != self.banners_off {
            self.banners_off = Some(alone);
            if alone {
                banners::quiet_all();
            } else {
                banners::restore();
            }
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
            (
                MENU_ONLY_ISLAND,
                row(
                    toggle,
                    t.only_island,
                    !self.settings.windows_banners,
                    self.settings.notification_glance,
                ),
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
            MENU_ONLY_ISLAND => ("windowsBanners", &mut file.windows_banners),
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
            MENU_ONLY_ISLAND => self.settings.windows_banners,
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
                // Over a notification the pointer is there to click it: the
                // player waits until it is gone.
                if self.tracking && self.toast_shown() {
                    unsafe { SetTimer(Some(self.hwnd), HOVER_TIMER, HOVER_DELAY_MS, None) };
                } else if self.tracking {
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
            WM_TIMER if wp.0 == PEEK_TIMER => {
                // The lyrics' glance stays: the notice gives way to it.
                if self.notice.is_some() && self.glance() {
                    let b = self.showing();
                    self.swap_from(b);
                }
                // Otherwise it stays while the island shrinks back (so it
                // keeps its size and its look on the way) and goes once still.
                if self.glance() {
                    self.notice = None;
                }
                self.set_peek(false);
            }
            WM_TIMER if wp.0 == FOCUS_TIMER => self.focus_ended(),
            WM_APP_VOLUME if self.volume.is_some() => self.volume_changed(volume::read(wp, lp)),
            WM_APP_PRIVACY if self.privacy.is_some() => self.privacy_changed(wp.0),
            WM_APP_BLUETOOTH => self.bluetooth_changed(),
            WM_APP_NOTIFICATION => self.notified(),
            WM_TIMER if wp.0 == BLUETOOTH_TIMER => self.bluetooth_changed(),
            WM_APP_SPEAKERS => {
                // New speakers come with their own volume: no glance for it.
                if let Some(v) = &mut self.volume {
                    v.follow();
                    self.last_volume = v.now();
                }
                self.on_bluetooth = volume::on_bluetooth();
                self.follow_lyrics();
            }
            WM_MOUSEWHEEL => {
                let (x, y) = (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32);
                self.wheel(x, y, (wp.0 >> 16) as i16);
            }
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
                    self.on_bluetooth = volume::on_bluetooth();
                    if !first {
                        self.notice = None;
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
            // Windows' colors may have changed: the accent is read again.
            WM_SETTINGCHANGE | WM_DWMCOLORIZATIONCOLORCHANGED => render::forget_accent(),
            WM_RBUTTONUP => self.open_menu(),
            WM_APP_TRAY => {
                if matches!((lp.0 & 0xFFFF) as u32, WM_LBUTTONUP | WM_RBUTTONUP) {
                    self.open_menu();
                }
            }
            WM_DESTROY => {
                // Gone, the island tells nothing: the banners come back.
                if self.banners_off == Some(true) {
                    banners::restore();
                }
                unsafe { PostQuitMessage(0) }
            }
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

/// The time of day as the user's locale writes it, without seconds.
fn time_of_day() -> String {
    // A custom format can take up to 80 characters (LOCALE_STIMEFORMAT).
    let mut buf = [0u16; 81];
    let n = unsafe {
        GetTimeFormatEx(
            PCWSTR::null(),
            TIME_NOSECONDS,
            None,
            PCWSTR::null(),
            Some(&mut buf),
        )
    };
    String::from_utf16_lossy(&buf[..(n.max(1) - 1) as usize])
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
    page: Option<Page>,
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
            page: arg("--page").and_then(|s| Page::parse(&s)),
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
        s.default_page = self.page.unwrap_or(s.default_page);
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
            peek_most: layout::PEEK.0,
            banners_off: None,
            peek_w: layout::PEEK.0,
            closed_w: layout::CLOSED_MEDIA_WIDTH,
            page: defaults.default_page,
            timer: focus::Timer::new(&defaults.focus),
            start: Instant::now(),
            notice: None,
            // Started by the settings (on by default).
            volume: None,
            last_volume: None,
            on_bluetooth: false,
            privacy: None,
            users: Default::default(),
            bluetooth: None,
            notifications: None,
            swap: None,
            pulse: None,
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
