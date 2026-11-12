// No console window in release builds; debug builds keep it for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod banners;
mod bars;
mod bluetooth;
mod install;
mod keys;
mod lyrics;
mod media;
mod notifications;
mod pacer;
mod privacy;
mod render;
mod settings_file;
mod startup;
mod theme_file;
mod tray;
mod volume;

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Instant;

use island_core::bluetooth::Event as BtEvent;
use island_core::focus::{self, Ended, Lengths, Phase};
use island_core::glance;
use island_core::install::Offer;
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
    E_FAIL, ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Globalization::{GetTimeFormatEx, GetUserDefaultLocaleName, TIME_NOSECONDS};
use windows::Win32::Graphics::Gdi::{
    CreateRectRgn, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST,
    MONITORINFO, MonitorFromPoint, ScreenToClient, SetWindowRgn,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::{
    GetModuleHandleW, LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
};
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
    AllowSetForegroundWindow, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    FindWindowExW, GetCursorPos, GetMessageW, GetWindowThreadProcessId, HWND_MESSAGE, HWND_TOPMOST,
    IDC_ARROW, IsWindow, KillTimer, LoadCursorW, MA_NOACTIVATE, MONITORINFOF_PRIMARY, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SW_SHOW, SW_SHOWNORMAL,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow, SetTimer, SetWindowPos,
    ShowWindow, TranslateMessage, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_COPYDATA, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_DPICHANGED, WM_DWMCOLORIZATIONCOLORCHANGED, WM_KEYDOWN, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONUP, WM_SETTINGCHANGE, WM_TIMER,
    WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP, WS_VISIBLE,
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
/// Ends the glance on a new song (one for all the islands).
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
/// Time for a frame, while an island moves (see `pacer.rs`).
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
/// The `tema` theme changed (see `theme_file.rs`).
const WM_APP_THEME: u32 = WM_APP + 12;
/// The exe opened again while the island runs: its menu opens.
const WM_APP_OPEN: u32 = WM_APP + 13;
/// Installing or uninstalling: the island that runs ends.
const WM_APP_QUIT: u32 = WM_APP + 14;
/// The installed island's first start: a glance says it is installed.
const WM_APP_INSTALLED: u32 = WM_APP + 15;
/// Opened from outside its install folder: the offer to install, under the
/// island.
const WM_APP_OFFER: u32 = WM_APP + 16;
/// Started again after `--install` failed: it says so.
const WM_APP_INSTALL_FAILED: u32 = WM_APP + 17;
/// A volume or media key, kept from Windows (see `keys.rs`).
const WM_APP_KEY: u32 = WM_APP + 18;

const ISLAND_CLASS: PCWSTR = w!("IslandWindow");
/// The hidden window the shared things talk to: the sources (media, volume,
/// privacy, Bluetooth, notifications), the settings file's watch, the tray
/// and the glance and focus timers that belong to no single island.
const ANCHOR_CLASS: PCWSTR = w!("IslandAnchor");

/// How long the glance's content takes to change in place, in ms.
const SWAP_MS: f32 = 300.0;

/// What the glance showed before a change in place (see `render::Before`).
#[derive(Clone)]
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
/// The theme's file is read this long after it changes, and again as long
/// after a read that failed (another app had it open), a few times.
const THEME_TIMER: usize = 9;
const THEME_WAIT_MS: u32 = 150;
const THEME_TRIES: u8 = 3;
/// The wheel changes the focus length by this many minutes.
const WHEEL_MINUTES: i32 = 5;
// The menu's items.
const MENU_LYRICS: u32 = 1;
const MENU_OPEN_LYRICS: u32 = 2;
const MENU_STARTUP: u32 = 3;
const MENU_SETTINGS: u32 = 4;
const MENU_QUIT: u32 = 5;
const MENU_ONLY_ISLAND: u32 = 6;
const MENU_INSTALL: u32 = 7;
const MENU_PORTABLE: u32 = 8;
const MENU_CLASS: PCWSTR = w!("IslandMenu");

/// The menu window, while it is open.
struct MenuWindow {
    hwnd: HWND,
    target: render::MenuTarget,
    /// The island whose renderer made `target`: only that one draws it.
    owner: HWND,
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

/// One island window: everything that is its own, per screen. What an
/// island shows (the song, the glances, the page) is shared in `App`; how
/// its body moves and where it lives is here.
struct Surface {
    hwnd: HWND,
    /// The screen the window is on, to find it again on the next layout.
    mon: HMONITOR,
    monitor: Monitor,
    scale: f32,
    /// The window on screen, in px: sized for the open island.
    window: Rect,
    width: Spring,
    height: Spring,
    open: bool,
    /// The glance is what shows (or fades) while not open: from when it
    /// starts until this island opens or has closed.
    peek_view: bool,
    /// The pointer is over this island (WM_MOUSELEAVE is armed).
    tracking: bool,
    /// The window region last set, in px: left, top, right, bottom.
    region: [i32; 4],
    /// While this island's springs move: the last frame. `None`: no frames
    /// asked for.
    last_frame: Option<Instant>,
    pacer: pacer::Pacer,
    /// The body is on its way to a smaller size.
    shrinking: bool,
    renderer: Option<render::Renderer>,
    /// The open island's width setting as it fits this screen.
    open_width: f32,
    /// The most the song's glance grows to on this screen.
    peek_most: f32,
    /// What this island's glance is changing from in place, and since when.
    swap: Option<(Before, Instant)>,
}

impl Surface {
    /// A window on `monitor`, its renderer and its frame pacer.
    fn new(mon: HMONITOR, monitor: Monitor, scale: f32) -> Option<Self> {
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
                ISLAND_CLASS,
                w!("Island"),
                WS_POPUP | WS_VISIBLE,
                0,
                0,
                1,
                1,
                None,
                None,
                GetModuleHandleW(None).ok().map(|h| h.into()),
                None,
            )
        }
        .ok()?;
        Some(Self {
            hwnd,
            mon,
            monitor,
            scale,
            window: Rect::default(),
            width: Spring::at(CLOSED.0),
            height: Spring::at(CLOSED.1),
            open: false,
            peek_view: false,
            tracking: false,
            region: [0; 4],
            last_frame: None,
            pacer: pacer::Pacer::new(hwnd, WM_APP_FRAME),
            shrinking: false,
            renderer: render::Renderer::new(hwnd).ok(),
            // Fitted to the screen by the first place().
            open_width: layout::OPEN_WIDTH,
            peek_most: layout::PEEK.0,
            swap: None,
        })
    }

    /// The window is sized for the widest this island gets: the open one,
    /// or a glance at `islandScale`.
    fn widest(&self, k: f32) -> f32 {
        self.open_width.max(self.peek_most * k)
    }

    /// Places the window on its screen at that screen's scale.
    fn place(&mut self, placement: Placement, open_width_setting: f32, k: f32) {
        // Small or scaled-up screens get a narrower open island.
        let screen = self.monitor.rect.width / self.scale;
        self.open_width = layout::fit_open_width(open_width_setting, screen);
        self.peek_most = layout::fit_open_width(layout::PEEK_MOST, screen);
        let s = self.scale;
        let r = shape::window_rect(
            placement,
            self.monitor.rect,
            self.widest(k) * s,
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
    }

    /// The open island: the same width with or without lines.
    fn open_size(&self) -> (f32, f32) {
        (self.open_width, OPEN.1)
    }

    /// From window px to the open layout's DIPs: it is centered in the
    /// body; y from the island's top.
    fn to_open(&self, placement: Placement, x: f32, y: f32) -> (f32, f32) {
        let s = self.scale;
        let body = shape::body_rect(
            placement,
            self.window.width,
            self.width.value * s,
            self.height.value * s,
            EAR * s,
        );
        let lx = (x - body.x - shape::ear_room(placement, EAR * s)) / s
            - (self.width.value - self.open_size().0) / 2.0;
        (lx, (y - body.y) / s)
    }
}

/// How open a body is, from the small view it grows from (the closed island
/// or the glance, both at `islandScale`) to the open one: 0 to 1.
fn openness(sf: &Surface, k: f32, peek_h: f32) -> f32 {
    let small = if sf.peek_view { peek_h } else { CLOSED.1 * k };
    ((sf.height.value - small) / (OPEN.1 - small)).clamp(0.0, 1.0)
}

/// How visible the closed, glance and open contents are, from the body's
/// height: each one fades as the body leaves its size.
fn alphas(sf: &Surface, k: f32, peek_h: f32) -> [f32; 3] {
    let h = sf.height.value;
    let closed_h = CLOSED.1 * k;
    let open = ((h - closed_h) / (OPEN.1 - closed_h)).clamp(0.0, 1.0);
    // The closed content leaves fast; the open one comes in once there
    // is room for it. It shrinks with the body (scaled to it), so
    // closing it can stay until the body is small, while the cover
    // travels on its own.
    let open_a = if sf.shrinking {
        (open - 0.3) / 0.45
    } else {
        (open - 0.25) / 0.55
    }
    .clamp(0.0, 1.0);
    // The small views leave fast as the body grows; shrinking, they come
    // back only once it is nearly their size, not over a tall body.
    let pace = if sf.shrinking { 2.4 } else { 3.0 };
    let leaving = (1.0 - open * pace).clamp(0.0, 1.0);
    if sf.peek_view {
        // One gives way to the other halfway, so they never overlap. A
        // taller glance (a notification's) is fully there by the song's
        // glance's height, so growing from that one never shows the
        // closed island in between; the body uncovers the rest.
        let small_h = peek_h.min(layout::PEEK.1 * k);
        let grown = ((h - closed_h) / (small_h - closed_h)).clamp(0.0, 1.0);
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

/// The glance's size before `islandScale`, in DIPs: a fixed one for a
/// notice (a notification's, a level's, or the song's), as wide as the line
/// needs for the song (see `layout::peek_width`).
fn peek_unscaled(notice: &Option<Notice>, glance_on: bool, song_w: f32) -> (f32, f32) {
    let level = notice.as_ref().is_some_and(|n| n.level.is_some());
    if notice.as_ref().is_some_and(|n| n.toast) {
        island_core::notifications::SIZE
    } else if level && !glance_on {
        glance::LEVEL
    } else if notice.is_some() {
        layout::PEEK
    } else {
        (song_w, layout::PEEK.1)
    }
}

/// The glance's content is about to change: if this island's glance is out
/// and at rest, `before` fades out as the new content comes in. (Growing
/// or shrinking, the body's own move carries the change.) `peek_h` is the
/// glance's height on screen now.
fn swap_from(sf: &mut Surface, k: f32, peek_h: f32, before: Before) {
    let at_rest = !sf.open
        && sf.peek_view
        && sf.height.target > CLOSED.1 * k
        && (sf.height.value - sf.height.target).abs() < 0.5
        && alphas(sf, k, peek_h)[1] > 0.99;
    if !at_rest {
        return;
    }
    sf.swap = Some((before, Instant::now()));
    if sf.last_frame.is_none() {
        sf.last_frame = Some(Instant::now());
        sf.pacer.start();
    }
}

struct App {
    /// The hidden window the sources, the tray and the shared timers post
    /// to (see `ANCHOR_CLASS`).
    anchor: HWND,
    /// One island per chosen screen ([`monitor::pick_all`]).
    surfaces: Vec<Surface>,
    placement: Placement,
    monitor: MonitorChoice,
    media: media::Media,
    track: Option<media::Track>,
    /// Title and artist of the song on screen, to tell a new one.
    song: Option<(String, String)>,
    /// A glance is being shown (its timer is running): on every island.
    peek: bool,
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
    /// While the menu shows the offer to install (see `install::offer`).
    offer: Option<Offer>,
    /// The version and exe that offer is for (this one, or a newer one).
    offer_from: Option<(String, PathBuf)>,
    /// Sent to the anchor when Explorer (and the taskbar) restarts.
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
    /// The `tema` theme's colors, while a setting follows them.
    theme: Option<island_core::theme::ThemeColors>,
    /// Its `actual.json` is watched (from the first setting that follows it).
    theme_watched: bool,
    /// Reads of the theme's file that failed in a row.
    theme_failed: u8,
    /// Windows' banners are off for the island to tell notifications alone
    /// (chosen in the menu; kept, noted, across runs).
    banners_quiet: bool,
    /// The song's glance text width as measured (each island caps it to its
    /// screen, see `Surface::peek_most`).
    peek_w: f32,
    /// The closed island with music, as wide as the song's name needs
    /// ([`layout::closed_width`]); the same on every screen.
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
    /// The volume and media keys, while the island takes them from Windows.
    keys: Option<keys::Keys>,
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

/// A screen's scale (DIPs to px); 1.0 without a usable answer.
fn monitor_scale(mon: HMONITOR) -> f32 {
    let (mut dx, mut dy) = (96, 96);
    unsafe {
        let _ = GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
    }
    dx.max(96) as f32 / 96.0
}

impl App {
    /// One island per chosen screen: keeps the windows whose screen is
    /// still chosen, creates the missing ones and destroys the surplus.
    /// Runs again when screens are added, removed, moved or rescaled, and
    /// when the `monitor` setting changes.
    fn layout(&mut self) {
        let list = monitors();
        let mons: Vec<Monitor> = list.iter().map(|m| m.1).collect();
        let mut want: Vec<(HMONITOR, Monitor)> = monitor::pick_all(&mons, &self.monitor)
            .iter()
            .map(|&i| list[i])
            .collect();
        if want.is_empty() {
            // No screens at all: a made-up one, so the app keeps a window.
            want.push((
                HMONITOR::default(),
                Monitor {
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 1920.0,
                        height: 1080.0,
                    },
                    primary: true,
                },
            ));
        }
        let mut old = std::mem::take(&mut self.surfaces);
        // A window destroyed behind the app's back is made anew.
        old.retain(|s| unsafe { IsWindow(Some(s.hwnd)) }.as_bool());
        let mut surfaces = Vec::with_capacity(want.len());
        for (mon, m) in want {
            let scale = monitor_scale(mon);
            let sf = match old.iter().position(|s| s.mon == mon) {
                Some(pos) => {
                    let mut sf = old.remove(pos);
                    sf.monitor = m;
                    sf.scale = scale;
                    Some(sf)
                }
                None => Surface::new(mon, m, scale),
            };
            if let Some(sf) = sf {
                surfaces.push(sf);
            }
        }
        for sf in old {
            unsafe {
                let _ = DestroyWindow(sf.hwnd);
            }
        }
        self.surfaces = surfaces;
        // The menu draws with its island's renderer: gone with it.
        if let Some(m) = &self.menu
            && !self.surfaces.iter().any(|s| s.hwnd == m.owner)
        {
            self.close_menu();
        }
        // New islands too take the song's alignment.
        let align = self.settings.text_align;
        for sf in &self.surfaces {
            if let Some(r) = &sf.renderer {
                let _ = r.align_song(align);
            }
        }
        for i in 0..self.surfaces.len() {
            let (placement, setting, k) = (
                self.placement,
                self.open_width_setting,
                self.settings.island_scale,
            );
            self.surfaces[i].place(placement, setting, k);
        }
        self.fit_text();
        self.retarget_all();
        for i in 0..self.surfaces.len() {
            if self.surfaces[i].last_frame.is_none() {
                self.draw_surface(i);
            }
        }
    }

    /// Draws one island's window from the shared state and its own.
    fn draw_surface(&mut self, i: usize) {
        let k = self.settings.island_scale;
        let placement = self.placement;
        let now = self.now_ms();
        let peek_u = self.peek_unscaled_at(i);
        let peek_h = peek_u.1 * k;
        let alpha = alphas(&self.surfaces[i], k, peek_h);
        let open_a = openness(&self.surfaces[i], k, peek_h);
        let glance_on = self.glance();
        let scroll = self.lyric_scroll();
        let ring = self
            .pulse
            .map_or(f32::MAX, |t| t.elapsed().as_secs_f32() * 1000.0);
        let ring = island_core::notifications::ring(ring);
        let orbit = self
            .pulse
            .and_then(|t| island_core::notifications::orbit(t.elapsed().as_secs_f32() * 1000.0));
        let when = self.texts.now;
        let lyrics_filled = !self.lines.is_empty();

        let sf = &mut self.surfaces[i];
        let s = sf.scale;
        let (w, h) = (sf.width.value.max(0.0) * s, sf.height.value.max(0.0) * s);
        let (ear, radius) = (EAR * s, RADIUS * s);
        let body = shape::body_rect(placement, sf.window.width, w, h, ear);
        let outline = if placement.hangs() {
            shape::hanging(w, h, radius, ear)
        } else {
            shape::floating(w, h, radius)
        };
        // The pointer only reaches the island inside its body, ears left
        // out; everywhere else, clicks go to what is below. The region takes
        // the bigger of the size now and the target, so it changes only when
        // the island starts to open and when it has finished closing.
        let (rw, rh) = (
            sf.width.value.max(sf.width.target) * s,
            sf.height.value.max(sf.height.target) * s,
        );
        let inset = shape::ear_room(placement, ear);
        let rb = shape::body_rect(placement, sf.window.width, rw, rh, ear);
        let region = [
            (rb.x + inset).floor() as i32,
            0,
            (rb.x + rb.width - inset).ceil() as i32,
            rh.ceil() as i32,
        ];
        if region != sf.region {
            sf.region = region;
            unsafe {
                let rgn = CreateRectRgn(region[0], region[1], region[2], region[3]);
                SetWindowRgn(sf.hwnd, Some(rgn), false);
            }
        }
        let lyrics = lyrics_filled.then_some(render::LyricsView {
            lines: &self.lines,
            index: self.lyric_index,
            scroll,
        });
        // As `lyrics` moves into the scene: the line being sung, as the
        // glance would tell it.
        let glance_filled = glance_on
            && self
                .lyric_index
                .and_then(|x| self.lines.get(x))
                .is_some_and(|l| !l.text.trim().is_empty());
        // The focus timer's words, for this frame.
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
        let view = |n| shown(n, when, ring, orbit);
        let scene = render::Scene {
            page: self.page,
            tabs_clock: &clock,
            tabs_timer: tm.running(),
            focus: focus_view,
            notice: self.notice.as_ref().map(view),
            closed_timer: (active && self.track.is_none()).then_some(left.as_str()),
            swap: sf.swap.as_ref().map(|(b, at)| render::Swap {
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
            width: sf.window.width as u32,
            height: sf.window.height as u32,
            body,
            outline: &outline,
            scale: s,
            inset: shape::ear_room(placement, ear),
            size: (sf.width.value.max(0.0), sf.height.value.max(0.0)),
            alpha,
            track: self.track.as_ref(),
            position_ms: self.track.as_ref().map_or(0, |t| t.position_now()),
            empty: self.texts.nothing_playing,
            song: self.song_text,
            lyrics,
            open: layout::Open::new(sf.open_width, self.lyrics_column && lyrics_filled),
            glance: glance_filled,
            peek_width: peek_u.0,
            closed_width: self.closed_w,
            openness: open_a,
            peek_view: sf.peek_view,
            island_scale: k,
        };
        let drawn = match &mut sf.renderer {
            Some(r) => r.draw(&scene),
            None => Ok(()),
        };
        // A lost device (driver update, GPU reset): start over once.
        if drawn.is_err() {
            render::forget_device(sf.renderer.take().as_ref());
            sf.renderer = render::Renderer::new(sf.hwnd).ok();
            if let Some(r) = &mut sf.renderer {
                let _ = r.draw(&scene);
            }
        }
    }

    /// Wakes one island up when what is shown changes, only while it is on
    /// screen: the song's second (open, on Home), the focus timer's second,
    /// or the minute of the clock on the tabs.
    fn tick_surface(&self, i: usize) {
        let hwnd = self.surfaces[i].hwnd;
        unsafe {
            let _ = KillTimer(Some(hwnd), TICK_TIMER);
        }
        let mut wait: Option<u32> = None;
        let mut sooner = |ms: u32| wait = Some(wait.map_or(ms, |w| w.min(ms)));
        if let Some(t) = &self.track
            && self.surfaces[i].open
            && self.page == Page::Home
            && t.playing
            && t.duration_ms > 0
        {
            sooner(1000 - (t.position_now() % 1000) as u32 + 15);
        }
        let shown = self.surfaces[i].open || self.track.is_none();
        if self.timer.running() && shown {
            // It shows the second rounded up: it changes on a whole one.
            let left = self.timer.left_ms(self.now_ms());
            sooner((left % 1000) as u32 + 15);
        } else if self.surfaces[i].open {
            let t = unsafe { GetLocalTime() };
            let into = u32::from(t.wSecond) * 1000 + u32::from(t.wMilliseconds);
            sooner(60_000u32.saturating_sub(into) + 15);
        }
        if let Some(ms) = wait {
            unsafe { SetTimer(Some(hwnd), TICK_TIMER, ms.max(16), None) };
        }
    }

    /// Milliseconds on the steady clock, for the focus timer.
    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// One timer for when the focus phase ends, while it runs.
    fn arm_focus(&self) {
        unsafe {
            let _ = KillTimer(Some(self.anchor), FOCUS_TIMER);
            if let Some(at) = self.timer.ends_at() {
                let ms = at.saturating_sub(self.now_ms()).min(u32::MAX as u64 / 2) as u32;
                SetTimer(Some(self.anchor), FOCUS_TIMER, ms + 5, None);
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
        self.redraw_all();
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

    /// A glance that tells something, for `ms` after its last change.
    fn notify(&mut self, n: Notice, ms: u32) {
        // A notice of the same kind (the volume again) just updates.
        let before = self.showing();
        if !matches!(&before, Before::Notice(b) if b.title == n.title && b.icon == n.icon) {
            let k = self.settings.island_scale;
            for i in 0..self.surfaces.len() {
                let peek_h = self.peek_unscaled_at(i).1 * k;
                swap_from(&mut self.surfaces[i], k, peek_h, before.clone());
            }
        }
        self.notice = Some(n);
        unsafe {
            let _ = KillTimer(Some(self.anchor), PEEK_TIMER);
            SetTimer(Some(self.anchor), PEEK_TIMER, ms, None);
        }
        self.peek = true;
        self.retarget_all();
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

    /// A volume or media key, kept from Windows: done as Windows does it.
    fn key_pressed(&mut self, wp: WPARAM) {
        use keys::Key;
        let Some(key) = Key::read(wp) else { return };
        let up = match key {
            Key::VolumeUp => Some(true),
            Key::VolumeDown => Some(false),
            Key::Mute => None,
            Key::PlayPause => return self.media.send(media::Command::PlayPause),
            Key::Next => return self.media.send(media::Command::Next),
            Key::Prev => return self.media.send(media::Command::Prev),
        };
        let Some(v) = &self.volume else { return };
        let moved = v.press(up);
        let now = v.now();
        // At 0 or 100 already no change comes back; the key still gets its
        // glance, as with Windows' flyout.
        if let (false, Some(now)) = (moved, now) {
            self.last_volume = None;
            self.volume_changed(now);
        }
    }

    /// A paired Bluetooth device connected or disconnected: a glance for    /// the last change told (a burst ends where it ended up).
    fn bluetooth_changed(&mut self) {
        let Some(b) = &self.bluetooth else { return };
        let (mut told, next) = b.due();
        unsafe {
            let _ = KillTimer(Some(self.anchor), BLUETOOTH_TIMER);
            if let Some(ms) = next {
                SetTimer(Some(self.anchor), BLUETOOTH_TIMER, ms as u32 + 20, None);
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
            // A full-screen window may have gone over an island: on top
            // again (without taking the focus).
            for sf in &self.surfaces {
                unsafe {
                    let _ = SetWindowPos(
                        sf.hwnd,
                        Some(HWND_TOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
                    );
                }
            }
        }
        let t = a.toast;
        // An app new to the island: its next ones come without a banner.
        if self.banners_quiet {
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

    /// Redraws the islands that are at rest; each one keeps asking for
    /// frames on its own while it moves.
    fn redraw_all(&mut self) {
        for i in 0..self.surfaces.len() {
            if self.surfaces[i].last_frame.is_none() {
                self.draw_surface(i);
            }
            self.tick_surface(i);
        }
    }

    /// A notification's glance is out (not yet giving way).
    fn toast_shown(&self) -> bool {
        self.peek && self.notice.as_ref().is_some_and(|n| n.toast)
    }

    fn click(&mut self, i: usize, x: f32, y: f32) {
        // On a notification: its app, and the glance has done its job.
        if !self.surfaces[i].open && self.toast_shown() {
            if let Some(n) = &self.notice {
                open_app(&n.app_id);
            }
            self.set_peek(false);
            return;
        }
        if !self.surfaces[i].open {
            return;
        }
        let (lx, ly) = self.surfaces[i].to_open(self.placement, x, y);
        if let Some(page) = pages::hit_tab(lx, ly) {
            if page != self.page {
                self.page = page;
                self.redraw_all();
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
                let open = layout::Open::new(
                    self.surfaces[i].open_width,
                    self.lyrics_column && !self.lines.is_empty(),
                );
                let cmd = match layout::hit(lx, py, open) {
                    Some(layout::Button::Prev) => media::Command::Prev,
                    Some(layout::Button::PlayPause) => media::Command::PlayPause,
                    Some(layout::Button::Next) => media::Command::Next,
                    None => return,
                };
                self.media.send(cmd);
            }
            Page::Focus => {
                let (now, l) = (self.now_ms(), self.settings.focus);
                match pages::hit_focus(lx, py, self.surfaces[i].open_width) {
                    Some(FocusButton::PlayPause) => self.timer.toggle(now),
                    Some(FocusButton::Reset) => self.timer.reset(&l),
                    Some(FocusButton::Skip) => {
                        self.timer.skip(now, &l);
                    }
                    None => return,
                }
                self.arm_focus();
                self.redraw_all();
            }
        }
    }

    /// The wheel over the focus time, before a session starts: its length,
    /// kept in `settings.json`.
    fn wheel(&mut self, i: usize, x: i32, y: i32, delta: i16) {
        if !self.surfaces[i].open
            || self.page != Page::Focus
            || self.timer.active(&self.settings.focus)
        {
            return;
        }
        let mut p = POINT { x, y };
        unsafe {
            let _ = ScreenToClient(self.surfaces[i].hwnd, &mut p);
        }
        let (lx, ly) = self.surfaces[i].to_open(self.placement, p.x as f32, p.y as f32);
        if !pages::over_time(lx, ly - pages::PAGE_Y, self.surfaces[i].open_width) {
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
        let mut file = self.file.clone();
        file.focus.focus = minutes;
        let _ = settings_file::set("focusMinutes", Value::from(minutes));
        self.apply(file);
    }

    /// The lyrics' glance: while synced lyrics play, the closed island
    /// shows the line being sung (`lyricsPeek` in the plugin).
    fn glance(&self) -> bool {
        self.lyrics.is_some()
            && !self.lines.is_empty()
            && self.track.as_ref().is_some_and(|t| t.playing)
    }

    fn peeking(&self) -> bool {
        self.peek || self.glance()
    }

    /// The open island's layout now: a column for the lines if there are.
    fn open_layout(&self, i: usize) -> layout::Open {
        layout::Open::new(
            self.surfaces[i].open_width,
            self.lyrics_column && !self.lines.is_empty(),
        )
    }

    /// The glance's size before `islandScale` on one island's screen, in
    /// DIPs.
    fn peek_unscaled_at(&self, i: usize) -> (f32, f32) {
        let song_w = layout::peek_width(self.peek_w, self.surfaces[i].peek_most);
        peek_unscaled(&self.notice, self.glance(), song_w)
    }

    /// Measures the closed island's and the song's glance's text, for
    /// their widths: the song's name, and the line being sung if it shows.
    /// The closed width fits every screen; each island caps the glance's
    /// to its own (see `Surface::peek_most`).
    fn fit_text(&mut self) {
        let Some(r) = self.surfaces.iter().find_map(|s| s.renderer.as_ref()) else {
            return;
        };
        let Some(t) = &self.track else { return };
        let name = self.song_text.spans(&t.title, &t.artist);
        self.closed_w = layout::closed_width(r.text_width(&name, true));
        let line = match self.showing() {
            Before::Line(l) => vec![(l, false)],
            _ => name,
        };
        self.peek_w = r.text_width(&line, false);
    }

    /// How much of the scroll to the line being sung is left.
    fn lyric_scroll(&self) -> f32 {
        self.lyric_moved.map_or(0.0, |t| {
            layout::lyrics_scroll(t.elapsed().as_secs_f32() * 1000.0)
        })
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
        for sf in &self.surfaces {
            unsafe {
                let _ = KillTimer(Some(sf.hwnd), LYRIC_TIMER);
            }
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
            if self.notice.is_none() {
                let b = self.showing();
                let k = self.settings.island_scale;
                for i in 0..self.surfaces.len() {
                    if self.surfaces[i].open {
                        continue;
                    }
                    let peek_h = self.peek_unscaled_at(i).1 * k;
                    swap_from(&mut self.surfaces[i], k, peek_h, b.clone());
                }
            }
            let glance_on = self.glance();
            // One line on, open: the column glides up to it. A seek, or
            // closed: it is just there.
            let step = matches!((self.lyric_index, index), (Some(a), Some(b)) if b == a + 1);
            self.lyric_index = index;
            // Closed, the glance grows or shrinks to the new line.
            for i in 0..self.surfaces.len() {
                if !self.surfaces[i].open && glance_on {
                    self.retarget_surface(i);
                }
            }
            // The scroll is shared: on while any open island shows the
            // column, whichever comes last in the list.
            let glides: Vec<bool> = (0..self.surfaces.len())
                .map(|i| {
                    self.surfaces[i].open && step && self.open_layout(i).lyrics_rect().is_some()
                })
                .collect();
            self.lyric_moved = glides.contains(&true).then(Instant::now);
            for (i, glide) in glides.into_iter().enumerate() {
                if glide {
                    let sf = &mut self.surfaces[i];
                    if sf.last_frame.is_none() {
                        sf.last_frame = Some(Instant::now());
                        sf.pacer.start();
                    }
                } else if self.surfaces[i].last_frame.is_none() {
                    self.draw_surface(i);
                }
            }
        }
        if playing
            && (self.surfaces.iter().any(|s| s.open) || self.glance())
            && let Some(at) = next_at
        {
            let ms = at.saturating_sub(pos).clamp(40, u32::MAX as u64) as u32;
            for sf in &self.surfaces {
                unsafe { SetTimer(Some(sf.hwnd), LYRIC_TIMER, ms, None) };
            }
        }
    }

    fn set_open(&mut self, i: usize, open: bool) {
        if self.surfaces[i].open == open {
            return;
        }
        // The glance stays the view being left while the island opens (it
        // is dropped once open), so the song's line never flashes between.
        self.surfaces[i].open = open;
        self.retarget_surface(i);
        self.follow_lyrics();
    }

    fn set_peek(&mut self, peek: bool) {
        unsafe {
            let _ = KillTimer(Some(self.anchor), PEEK_TIMER);
            if peek {
                SetTimer(Some(self.anchor), PEEK_TIMER, PEEK_MS, None);
            }
        }
        self.peek = peek;
        self.retarget_all();
    }

    fn retarget_all(&mut self) {
        for i in 0..self.surfaces.len() {
            self.retarget_surface(i);
        }
    }

    /// Aims one island's springs at the size its view has now.
    fn retarget_surface(&mut self, i: usize) {
        {
            let peeking = self.peeking();
            let sf = &mut self.surfaces[i];
            if peeking && !sf.open {
                sf.peek_view = true;
            }
        }
        self.fit_text();
        let k = self.settings.island_scale;
        let peeking = self.peeking();
        let track_some = self.track.is_some();
        let peek_size = {
            let (w, h) = self.peek_unscaled_at(i);
            (w * k, h * k)
        };
        let sf = &mut self.surfaces[i];
        let (w, h) = if sf.open {
            sf.open_size()
        } else if peeking {
            peek_size
        } else if track_some {
            // Wide enough for the song's name.
            (self.closed_w * k, CLOSED.1 * k)
        } else {
            (CLOSED.0 * k, CLOSED.1 * k)
        };
        if h != sf.height.target {
            sf.shrinking = h < sf.height.value;
        }
        sf.width.target = w;
        sf.height.target = h;
        if sf.last_frame.is_none() {
            sf.last_frame = Some(Instant::now());
            sf.pacer.start();
        }
        self.tick_surface(i);
    }

    /// Advances one island's springs and draws a frame; it asks for more
    /// frames only while it moves, so a still island costs nothing.
    fn surface_frame(&mut self, i: usize) {
        let peeking = self.peeking();
        let mut moving;
        {
            let sf = &mut self.surfaces[i];
            let now = Instant::now();
            let ms = sf
                .last_frame
                .map_or(16.0, |t| (now - t).as_secs_f32() * 1000.0);
            sf.last_frame = Some(now);
            // Growing is loose and a little wobbly; shrinking (also from
            // open to the glance) is firm, so it never dips under where it
            // stops.
            let (pw, ph) = if sf.shrinking {
                (spring::CLOSE_WIDTH, spring::CLOSE_HEIGHT)
            } else {
                (spring::OPEN_WIDTH, spring::OPEN_HEIGHT)
            };
            moving = sf.width.advance(&pw, ms) | sf.height.advance(&ph, ms);
            if let Some((_, at)) = &sf.swap {
                if at.elapsed().as_secs_f32() * 1000.0 < SWAP_MS {
                    moving = true;
                } else {
                    sf.swap = None;
                }
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
        self.draw_surface(i);
        if !moving {
            let peek = self.peek;
            // Nothing runs while the island is still.
            let sf = &mut self.surfaces[i];
            if sf.open || !peeking {
                sf.peek_view = false;
            }
            sf.last_frame = None;
            sf.pacer.stop();
            // The notice goes once every island is still: the others may be
            // shrinking with it yet.
            if !peek && self.surfaces.iter().all(|s| s.last_frame.is_none()) {
                self.notice = None;
            }
        }
    }

    /// Puts `file`'s settings to use, changing only what changed: switching
    /// the lyrics on starts their thread, off ends it.
    fn apply(&mut self, file: Settings) {
        self.file = file.clone();
        let s = self.args.over(file);
        if s.default_page != self.settings.default_page {
            self.page = s.default_page;
        }
        self.timer.set_lengths(&self.settings.focus, &s.focus);
        self.settings = s;
        let s = self.settings.clone();
        self.texts = texts::texts(s.language, &self.locale);
        self.placement = s.placement;
        self.monitor = s.monitor.clone();
        self.song_text = s.song_text;
        self.recolor(false);
        if s.lyrics != self.lyrics.is_some() {
            self.lyrics = s
                .lyrics
                .then(|| lyrics::Lyrics::start(self.anchor, WM_APP_LYRICS));
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
                .then(|| volume::Volume::start(self.anchor, WM_APP_VOLUME, WM_APP_SPEAKERS).ok())
                .flatten();
            self.last_volume = self.volume.as_ref().and_then(|v| v.now());
        }
        // The keys only while the island has a glance of its own to show.
        let keys = s.hide_windows_flyout && self.volume.is_some();
        if keys != self.keys.is_some() {
            self.keys = keys
                .then(|| keys::Keys::start(self.anchor, WM_APP_KEY))
                .flatten();
        }
        if s.privacy_glance != self.privacy.is_some() {
            self.privacy = s
                .privacy_glance
                .then(|| privacy::Privacy::start(self.anchor, WM_APP_PRIVACY))
                .flatten();
            // What is in use already was told by Windows' own light.
            self.users = Device::ALL.map(privacy::using);
        }
        if s.bluetooth_glance != self.bluetooth.is_some() {
            self.bluetooth = s
                .bluetooth_glance
                .then(|| bluetooth::Bluetooth::start(self.anchor, WM_APP_BLUETOOTH).ok())
                .flatten();
        }
        if s.notification_glance != self.notifications.is_some() {
            self.notifications = s
                .notification_glance
                .then(|| {
                    notifications::Notifications::start(
                        self.anchor,
                        WM_APP_NOTIFICATION,
                        self.texts.notification,
                    )
                })
                .flatten();
        }
        self.lyrics_column = s.open_lyrics;
        self.open_width_setting = s.open_width();
        self.layout();
    }

    /// The levels' and the body's colors from the settings and, when one
    /// follows it, the `tema` theme (read again; kept while half written).
    /// `redraw`: the islands at rest show them now.
    fn recolor(&mut self, redraw: bool) {
        use island_core::settings::{Accent, IslandColor};
        use island_core::theme;
        let s = &self.settings;
        if s.accent == Accent::Theme || s.island_color == IslandColor::Theme {
            if !self.theme_watched {
                self.theme_watched = theme_file::watch(self.anchor, WM_APP_THEME);
            }
            match theme_file::load() {
                Some(t) => {
                    self.theme = Some(t);
                    self.theme_failed = 0;
                }
                None if self.theme_failed < THEME_TRIES => {
                    self.theme_failed += 1;
                    unsafe {
                        SetTimer(Some(self.anchor), THEME_TIMER, THEME_WAIT_MS, None);
                    }
                }
                None => self.theme_failed = 0,
            }
        }
        let t = self.theme.as_ref();
        let body = theme::body(s.island_color, t);
        render::choose_colors(theme::accent(s.accent, t, body), body);
        if redraw {
            for i in 0..self.surfaces.len() {
                if self.surfaces[i].last_frame.is_none() {
                    self.draw_surface(i);
                }
            }
        }
    }

    /// Windows' colors may have changed: its accent is read again, and
    /// `tema`, if it wasn't installed yet, may be now (it sets the Windows
    /// theme too), so its file is looked for again.
    fn colors_changed(&mut self) {
        use island_core::settings::{Accent, IslandColor};
        render::forget_accent();
        let s = &self.settings;
        if !self.theme_watched
            && (s.accent == Accent::Theme || s.island_color == IslandColor::Theme)
        {
            self.recolor(true);
        }
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

    /// A track or its position is new: every island retargets and the ones
    /// at rest redraw.
    fn media_update(&mut self) {
        self.track = self.media.latest();
        keys::set_media(self.track.is_some());
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
        self.retarget_all();
        for i in 0..self.surfaces.len() {
            if self.surfaces[i].last_frame.is_none() {
                self.draw_surface(i);
            }
            self.tick_surface(i);
        }
        // A seek, a pause or a new song: the line again.
        self.follow_lyrics();
    }

    /// The lyrics thread answered: keep the lines if they are for the song
    /// on screen, and follow them from the position now.
    fn lyrics_answer(&mut self) {
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
            self.retarget_all();
            self.follow_lyrics();
            for i in 0..self.surfaces.len() {
                if self.surfaces[i].last_frame.is_none() {
                    self.draw_surface(i);
                }
            }
        }
    }

    /// Other speakers are the default now: no glance for their volume, and
    /// the lyrics' delay is read again.
    fn speakers_changed(&mut self) {
        if let Some(v) = &mut self.volume {
            v.follow();
            self.last_volume = v.now();
        }
        self.on_bluetooth = volume::on_bluetooth();
        self.follow_lyrics();
    }

    /// The media thread's, and the shared timers', messages. They arrive
    /// at the anchor window, not at any island's.
    fn anchor_message(&mut self, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_APP_MEDIA => self.media_update(),
            WM_APP_LYRICS => self.lyrics_answer(),
            WM_APP_VOLUME if self.volume.is_some() => {
                self.volume_changed(volume::read(wp, lp));
            }
            WM_APP_KEY => self.key_pressed(wp),
            WM_APP_PRIVACY if self.privacy.is_some() => self.privacy_changed(wp.0),
            WM_APP_BLUETOOTH => self.bluetooth_changed(),
            WM_APP_NOTIFICATION => self.notified(),
            WM_APP_SPEAKERS => self.speakers_changed(),
            // A move over it comes as a few changes: one read once they end.
            WM_APP_THEME => unsafe {
                self.theme_failed = 0;
                SetTimer(Some(self.anchor), THEME_TIMER, THEME_WAIT_MS, None);
            },
            WM_TIMER if wp.0 == THEME_TIMER => {
                unsafe {
                    let _ = KillTimer(Some(self.anchor), THEME_TIMER);
                }
                self.recolor(true);
            }
            WM_TIMER if wp.0 == PEEK_TIMER => {
                // The lyrics' glance stays: the notice gives way to it.
                if self.notice.is_some() && self.glance() {
                    let b = self.showing();
                    let k = self.settings.island_scale;
                    for i in 0..self.surfaces.len() {
                        let peek_h = self.peek_unscaled_at(i).1 * k;
                        swap_from(&mut self.surfaces[i], k, peek_h, b.clone());
                    }
                }
                // Otherwise it stays while the islands shrink back (so it
                // keeps its size and its look on the way) and goes once
                // still.
                if self.glance() {
                    self.notice = None;
                }
                self.set_peek(false);
            }
            WM_TIMER if wp.0 == FOCUS_TIMER => self.focus_ended(),
            WM_TIMER if wp.0 == BLUETOOTH_TIMER => self.bluetooth_changed(),
            WM_APP_SETTINGS => unsafe {
                SetTimer(Some(self.anchor), SETTINGS_TIMER, SETTINGS_WAIT_MS, None);
            },
            WM_TIMER if wp.0 == SETTINGS_TIMER => {
                unsafe {
                    let _ = KillTimer(Some(self.anchor), SETTINGS_TIMER);
                }
                self.reload();
            }
            // Windows' colors may have changed: the accent is read again.
            WM_SETTINGCHANGE | WM_DWMCOLORIZATIONCOLORCHANGED => self.colors_changed(),
            WM_RBUTTONUP => self.open_menu(false),
            WM_APP_TRAY => {
                if matches!((lp.0 & 0xFFFF) as u32, WM_LBUTTONUP | WM_RBUTTONUP) {
                    self.open_menu(false);
                }
            }
            WM_APP_OPEN if self.menu.is_none() => self.open_menu(true),
            WM_APP_QUIT => self.quit(),
            WM_APP_OFFER if self.menu.is_none() => {
                if let (Some(offer), Ok(me)) = (install::offer(), std::env::current_exe()) {
                    self.show_offer(offer, env!("CARGO_PKG_VERSION").to_owned(), me);
                }
            }
            // A newer exe opened while this one runs: this one offers it,
            // and keeps running until the user chooses.
            WM_COPYDATA => {
                let (version, exe) = unsafe { install::read_offer(lp) }?;
                self.close_menu();
                self.show_offer(Offer::Update, version, exe);
                return Some(LRESULT(self.menu.is_some() as isize));
            }
            WM_APP_INSTALL_FAILED => {
                let n = Notice {
                    icon: island_core::icons::Icon::Home,
                    tint: render::WHITE,
                    title: self.texts.install_failed.into(),
                    detail: self.texts.still_portable.into(),
                    level: None,
                    ..Default::default()
                };
                self.notify(n, PEEK_MS);
            }
            WM_APP_INSTALLED => {
                let n = Notice {
                    icon: island_core::icons::Icon::Home,
                    tint: render::WHITE,
                    title: self.texts.installed.into(),
                    detail: self.texts.in_start_menu.into(),
                    level: None,
                    ..Default::default()
                };
                self.notify(n, PEEK_MS);
            }
            WM_DESTROY => {
                for sf in &self.surfaces {
                    unsafe {
                        let _ = DestroyWindow(sf.hwnd);
                    }
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

    /// One island window's messages: the pointer, the wheel, its own
    /// timers and frames. Glances and content are shared; an island opens
    /// and closes on its own.
    fn surface_message(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
        let Some(i) = self.surfaces.iter().position(|s| s.hwnd == hwnd) else {
            return match msg {
                WM_DPICHANGED | WM_DISPLAYCHANGE => unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_APP_LAYOUT, WPARAM(0), LPARAM(0));
                    Some(LRESULT(0))
                },
                _ => None,
            };
        };
        match msg {
            WM_MOUSEMOVE if !self.surfaces[i].tracking => {
                let mut t = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                self.surfaces[i].tracking = unsafe { TrackMouseEvent(&mut t) }.is_ok();
                if !self.surfaces[i].open {
                    unsafe { SetTimer(Some(hwnd), HOVER_TIMER, HOVER_DELAY_MS, None) };
                }
            }
            WM_MOUSELEAVE => {
                self.surfaces[i].tracking = false;
                unsafe {
                    let _ = KillTimer(Some(hwnd), HOVER_TIMER);
                }
                self.set_open(i, false);
            }
            WM_TIMER if wp.0 == HOVER_TIMER => {
                unsafe {
                    let _ = KillTimer(Some(hwnd), HOVER_TIMER);
                }
                // Over a notification the pointer is there to click it: the
                // player waits until it is gone.
                if self.surfaces[i].tracking && self.toast_shown() {
                    unsafe { SetTimer(Some(hwnd), HOVER_TIMER, HOVER_DELAY_MS, None) };
                } else if self.surfaces[i].tracking {
                    self.set_open(i, true);
                }
            }
            WM_APP_FRAME => {
                self.surfaces[i].pacer.drawn();
                // A frame already on its way when the island came to rest.
                if self.surfaces[i].last_frame.is_some() {
                    self.surface_frame(i);
                }
            }
            WM_TIMER if wp.0 == TICK_TIMER => {
                if self.surfaces[i].last_frame.is_none() {
                    self.draw_surface(i);
                }
                self.tick_surface(i);
            }
            WM_TIMER if wp.0 == LYRIC_TIMER => self.follow_lyrics(),
            WM_MOUSEWHEEL => {
                let (x, y) = (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32);
                self.wheel(i, x, y, (wp.0 >> 16) as i16);
            }
            WM_LBUTTONUP => {
                let (x, y) = (lp.0 as i16 as f32, (lp.0 >> 16) as i16 as f32);
                self.click(i, x, y);
            }
            WM_DPICHANGED | WM_DISPLAYCHANGE | WM_APP_LAYOUT => self.layout(),
            // Clicking the island never takes the focus from the app in use.
            WM_MOUSEACTIVATE => return Some(LRESULT(MA_NOACTIVATE as isize)),
            WM_RBUTTONUP => self.open_menu(false),
            WM_SETTINGCHANGE | WM_DWMCOLORIZATIONCOLORCHANGED => self.colors_changed(),
            // Asked to close (the anchor, message-only, never is): the app
            // ends.
            WM_CLOSE => self.quit(),
            // Destroyed from outside the app: it goes from the list (its
            // pacer's thread ends with it).
            WM_DESTROY => {
                if self.menu.as_ref().is_some_and(|m| m.owner == hwnd) {
                    self.close_menu();
                }
                self.surfaces.remove(i);
            }
            // Broadcast to the islands only (the anchor is message-only):
            // the first one puts the tray icon back.
            _ if msg == self.taskbar_created && msg != 0 => {
                if let (0, Some(t)) = (i, &self.tray) {
                    t.add(self.texts.name);
                }
            }
            _ => return None,
        }
        Some(LRESULT(0))
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
        if let Some(offer) = self.offer {
            let install = match offer {
                Offer::Install => t.install,
                Offer::Update => t.update,
            };
            return vec![
                (MENU_INSTALL, row(action, install, false, true)),
                (MENU_PORTABLE, row(action, t.portable, false, true)),
            ];
        }
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
                    self.banners_quiet,
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

    /// Opens the menu at the pointer, or `centered` on its screen (the exe
    /// opened again, from the Start menu say); or closes it, if open.
    fn open_menu(&mut self, centered: bool) {
        if self.menu.is_some() {
            self.close_menu();
            return;
        }
        // The screen in use: the pointer's. Its island draws it, if it has
        // one; else any island.
        let mut at = POINT::default();
        let active = unsafe {
            let _ = GetCursorPos(&mut at);
            MonitorFromPoint(at, MONITOR_DEFAULTTONEAREST)
        };
        let drawn = |s: &Surface| s.renderer.is_some();
        let Some((owner, renderer)) = self
            .surfaces
            .iter()
            .filter(|s| drawn(s))
            .find(|s| s.mon == active)
            .or_else(|| self.surfaces.iter().find(|s| drawn(s)))
            .and_then(|s| s.renderer.as_ref().map(|r| (s.hwnd, r)))
        else {
            return;
        };
        let startup_on = startup::is_on();
        let kinds: Vec<MenuKind> = self
            .menu_rows(startup_on)
            .iter()
            .map(|r| r.1.kind)
            .collect();
        unsafe {
            let mon = active;
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
            if centered {
                // In the middle of the screen in use, where it is seen.
                at.x = w.left + (w.right - w.left - size.0 as i32) / 2;
                at.y = w.top + (w.bottom - w.top - size.1 as i32) / 2;
            }
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
                owner,
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

    /// The offer, under the island: to install `exe` (`version`).
    fn show_offer(&mut self, offer: Offer, version: String, exe: PathBuf) {
        self.offer = Some(offer);
        self.offer_from = Some((version, exe));
        self.open_menu(true);
        // No island to show it under: the usual menu stays as it is.
        if self.menu.is_none() {
            self.offer = None;
        }
    }

    fn close_menu(&mut self) {
        // The offer is made once: the next menu is the usual one.
        self.offer = None;
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
        let owner = self.menu.as_ref().map(|m| m.owner);
        let r = self
            .surfaces
            .iter_mut()
            .find(|s| Some(s.hwnd) == owner)
            .and_then(|s| s.renderer.as_mut());
        let m = self.menu.as_mut();
        let (Some(r), Some(m)) = (r, m) else {
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

    /// Ends the app: from the menu, or when an island is asked to close
    /// (`taskkill` without /F, the Restart Manager on an update).
    fn quit(&mut self) {
        self.close_menu();
        self.keys = None;
        // The icon goes first: the taskbar keeps a dead one otherwise.
        self.tray = None;
        // The anchor's own WM_DESTROY can't run now (the app is borrowed),
        // so the islands go here.
        let surfaces = std::mem::take(&mut self.surfaces);
        unsafe {
            for sf in &surfaces {
                let _ = DestroyWindow(sf.hwnd);
            }
            let _ = DestroyWindow(self.anchor);
        }
    }

    /// What a menu item does. The switches go to the file, and take effect
    /// at once (also if it can't be written now).
    fn command(&mut self, id: u32) {
        let mut file = self.file.clone();
        let (key, on) = match id {
            MENU_LYRICS => ("lyrics", &mut file.lyrics),
            MENU_OPEN_LYRICS => ("openLyrics", &mut file.open_lyrics),
            // Windows' banners off app by app (as the switch in its
            // settings), or back on: kept as is until chosen again.
            MENU_ONLY_ISLAND => {
                self.banners_quiet = !self.banners_quiet;
                if self.banners_quiet {
                    std::thread::spawn(banners::quiet_all);
                } else {
                    std::thread::spawn(banners::restore);
                }
                return;
            }
            MENU_STARTUP => {
                let _ = startup::set(!startup::is_on());
                return;
            }
            MENU_QUIT => {
                self.quit();
                return;
            }
            // The installer closes this island, then starts the installed
            // one.
            MENU_INSTALL => {
                if let Some((_, exe)) = self.offer_from.take()
                    && install::start_install(&exe).is_ok()
                {
                    self.quit();
                }
                return;
            }
            MENU_PORTABLE => {
                if let Some((version, _)) = self.offer_from.take() {
                    install::decline(&version);
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

/// The menu window's messages go to the app, like the islands'.
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

/// An island window's messages.
extern "system" fn island_wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let handled = APP.with(|cell| match cell.try_borrow_mut() {
        Ok(mut app) => app
            .as_mut()
            .and_then(|a| a.surface_message(hwnd, msg, wp, lp)),
        // A message sent from inside a handler (SetWindowPos, DestroyWindow):
        // the app is busy, so what needs it waits for the queue.
        Err(_) => match msg {
            WM_DPICHANGED | WM_DISPLAYCHANGE => unsafe {
                let _ = PostMessageW(Some(hwnd), WM_APP_LAYOUT, WPARAM(0), LPARAM(0));
                Some(LRESULT(0))
            },
            _ => None,
        },
    });
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, msg, wp, lp) })
}

/// The hidden window's messages: the sources, the tray, the shared timers.
extern "system" fn anchor_wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let handled = APP.with(|cell| match cell.try_borrow_mut() {
        Ok(mut app) => app.as_mut().and_then(|a| a.anchor_message(msg, wp, lp)),
        Err(_) => match msg {
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
        if let Some(m) = &self.monitor {
            s.monitor = m.clone();
        }
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
    // DLLs loaded from here on come from System32 only, never from the
    // exe's folder (see build.rs for the ones it starts with).
    unsafe {
        let _ = SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
    }
    let args: Vec<String> = std::env::args().collect();
    // For uninstalling by hand: Windows' banners back on, without the island
    // open.
    if install::has(&args, "--restore-banners") {
        banners::restore();
        return Ok(());
    }
    if install::is_finish(&args) {
        install::finish(&args);
        return Ok(());
    }
    for (arg, step) in [
        (
            "--install",
            install::install as fn(PCWSTR, u32) -> std::io::Result<()>,
        ),
        ("--uninstall", install::uninstall),
    ] {
        if install::has(&args, arg) {
            let done = step(ANCHOR_CLASS, WM_APP_QUIT);
            // The island it closed comes back, from this exe, and says so:
            // never left with none and no word.
            if done.is_err()
                && arg == "--install"
                && let Ok(me) = std::env::current_exe()
            {
                let _ = std::process::Command::new(me)
                    .arg(install::INSTALL_FAILED)
                    .spawn();
            }
            return done.map_err(|e| Error::new(E_FAIL, e.to_string()));
        }
    }
    unsafe {
        // One island per user session: a second start (the exe opened
        // twice, or from the Start menu while it starts with Windows) opens
        // the running one's menu, so it is seen to be there, and leaves. The
        // handle lives as long as the process.
        let _single = CreateMutexW(None, false, w!("Local\\IslandDesktop"))?;
        // A newer exe than the installed one that is running: that one goes,
        // so this one can offer the update.
        let running = GetLastError() == ERROR_ALREADY_EXISTS;
        if running {
            let update = install::offer() == Some(Offer::Update);
            let anchor = FindWindowExW(Some(HWND_MESSAGE), None, ANCHOR_CLASS, None).ok();
            if let Some(anchor) = anchor {
                // This start was the user's: it may hand the focus on.
                let mut pid = 0;
                GetWindowThreadProcessId(anchor, Some(&mut pid));
                let _ = AllowSetForegroundWindow(pid);
            }
            // A newer exe: the running island offers it, and keeps running
            // until the user chooses.
            let offered = update && anchor.is_some_and(install::send_offer);
            if update && !offered {
                // One from before this message can't: only by taking its
                // place can the update be offered.
                install::close_running(ANCHOR_CLASS, WM_APP_QUIT);
            } else {
                if let (Some(anchor), false) = (anchor, offered) {
                    let _ = PostMessageW(Some(anchor), WM_APP_OPEN, WPARAM(0), LPARAM(0));
                }
                return Ok(());
            }
        }
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // WIC is COM; the UI thread is its apartment.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let instance = GetModuleHandleW(None)?;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(island_wndproc),
            hInstance: instance.into(),
            lpszClassName: ISLAND_CLASS,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err(Error::from_thread());
        }
        // The hidden window the shared things talk to (see `App::anchor`).
        let anchor_wc = WNDCLASSW {
            lpfnWndProc: Some(anchor_wndproc),
            hInstance: instance.into(),
            lpszClassName: ANCHOR_CLASS,
            ..Default::default()
        };
        if RegisterClassW(&anchor_wc) == 0 {
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
        let anchor = CreateWindowExW(
            WS_EX_NOACTIVATE,
            ANCHOR_CLASS,
            w!("Island"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )?;
        let defaults = Settings::default();
        let locale = locale();
        let texts = texts::texts(defaults.language, &locale);
        let app = App {
            anchor,
            surfaces: Vec::new(),
            placement: defaults.placement,
            monitor: defaults.monitor.clone(),
            media: media::Media::start(anchor, WM_APP_MEDIA),
            track: None,
            song: None,
            peek: false,
            file: defaults.clone(),
            settings: defaults.clone(),
            args: Overrides::from_args(),
            locale,
            texts,
            tray: Some(tray::Tray::new(anchor, WM_APP_TRAY, texts.name)),
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
            theme: None,
            theme_watched: false,
            theme_failed: 0,
            // Fitted to the screen by the first layout.
            peek_w: 0.0,
            closed_w: layout::CLOSED_MEDIA_WIDTH,
            page: defaults.default_page,
            timer: focus::Timer::new(&defaults.focus),
            start: Instant::now(),
            notice: None,
            // Started by the settings (on by default).
            volume: None,
            last_volume: None,
            keys: None,
            on_bluetooth: false,
            privacy: None,
            users: Default::default(),
            bluetooth: None,
            notifications: None,
            pulse: None,
            banners_quiet: banners::active(),
            offer: None,
            offer_from: None,
        };
        APP.with_borrow_mut(|a| *a = Some(app));
        // A file with a typo at start: the defaults until it is fixed.
        let file = settings_file::load().unwrap_or_default();
        APP.with_borrow_mut(|a| a.as_mut().map(|a| a.apply(file)));
        settings_file::watch(anchor, WM_APP_SETTINGS);
        if install::has(&args, install::INSTALLED) {
            let _ = PostMessageW(Some(anchor), WM_APP_INSTALLED, WPARAM(0), LPARAM(0));
        } else if install::has(&args, install::INSTALL_FAILED) {
            let _ = PostMessageW(Some(anchor), WM_APP_INSTALL_FAILED, WPARAM(0), LPARAM(0));
        } else {
            let _ = PostMessageW(Some(anchor), WM_APP_OFFER, WPARAM(0), LPARAM(0));
        }
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
