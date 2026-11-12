//! Visual QA, built only with the `qa` feature: `island.exe --qa casos.txt
//! --qa-out carpeta` runs a script against the real islands, with made-up
//! songs and glances and nothing of the user's touched (no settings file,
//! no tray, no sources, no banners, no keys). Every frame is traced and
//! checked; `grab` and `film` save the islands' strips as PNG and check that
//! nothing is drawn outside the body. The report is `report.txt`; the exit
//! code is the number of failures (0: all good).
//!
//! One command per line (`#` starts a comment; quotes group words):
//!
//! - `set <key> <json>`: a setting, as in settings.json (`set placement
//!   "floating"`, `set islandScale 1.4`).
//! - `scale <n>`: every screen at that scale (DIPs to px).
//! - `slow <n>`: the springs run `n` times slower.
//! - `track "<title>" "<artist>"`, `notrack`: a song playing, or none.
//! - `notice "<title>" "<detail>"`, `level <0-1>`, `toast "<app>" "<title>"
//!   "<body>"`: a glance.
//! - `open [i]`, `close [i]`: one island (or all) opens or closes.
//! - `page home|focus`.
//! - `wait <ms>`; `wait-rest [ms]`: until every island is still (checks it
//!   stopped where it was going); `idle <ms>`: no frame nor timer at all.
//! - `margin <dips>`: how far outside the body a drawing may go (the ring's
//!   glow), 6 by default.
//! - `background`: the screen behind the islands, for what follows.
//! - `grab <name>`: each island's window as `<name>-<i>.png`.
//! - `film <name> [max]`: every frame of what moves next, until still.
//! - `quit`.

use super::*;
use std::cell::RefCell;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA,
    IWICImagingFactory, WICBitmapEncoderNoCache,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow};
use windows::core::GUID;

/// The anchor's timer that runs the script.
pub const TIMER: usize = 40;
const STEP_MS: u32 = 15;
/// A pixel this far from the background (summed over B, G, R) is drawn.
const DRAWN: i32 = 36;

static ACTIVE: AtomicBool = AtomicBool::new(false);

thread_local! {
    static QA: RefCell<Option<Qa>> = const { RefCell::new(None) };
}

enum Wait {
    None,
    Until(Instant),
    Rest(Instant),
    Idle(Instant),
}

/// A strip of the screen, in px.
struct Grab {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    bgra: Vec<u8>,
}

struct Qa {
    cmds: Vec<(usize, Vec<String>)>,
    next: usize,
    out: PathBuf,
    wait: Wait,
    t0: Instant,
    trace: std::io::BufWriter<std::fs::File>,
    fails: Vec<String>,
    notes: Vec<String>,
    slow: f32,
    scale: Option<f32>,
    json: serde_json::Map<String, Value>,
    /// Frames and timers on the islands during `idle`.
    busy: u32,
    busy_what: Vec<String>,
    centers: Vec<Option<f32>>,
    background: Vec<Option<Grab>>,
    film: Option<(String, u32, u32)>,
    margin: f32,
    /// Frames where the body's center moved by half a px or more.
    jitter: u32,
    frames: u64,
}

/// Whether the app runs a script.
pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Reads `--qa <script>` (and `--qa-out <dir>`); `Ok(false)` without it.
pub fn from_args(args: &[String]) -> std::io::Result<bool> {
    let Some(script) = value(args, "--qa") else {
        return Ok(false);
    };
    let out = value(args, "--qa-out")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("island-qa"));
    std::fs::create_dir_all(&out)?;
    let text = std::fs::read_to_string(&script)?;
    let cmds = text
        .lines()
        .enumerate()
        .map(|(n, l)| (n + 1, words(l)))
        .filter(|(_, w)| !w.is_empty())
        .collect();
    let mut trace = std::io::BufWriter::new(std::fs::File::create(out.join("trace.csv"))?);
    writeln!(
        trace,
        "t_ms,island,scale,win_x,win_y,win_w,win_h,w,w_target,w_vel,h,h_target,h_vel,body_x,body_w,body_h,a_closed,a_peek,a_open,openness,open,peek_view,shrinking"
    )?;
    let mut json = serde_json::Map::new();
    // Nothing that watches the system: the script makes up what shows.
    for key in [
        "volumeGlance",
        "hideWindowsFlyout",
        "privacyGlance",
        "bluetoothGlance",
        "notificationGlance",
        "updateCheck",
    ] {
        json.insert(key.into(), Value::Bool(false));
    }
    QA.with_borrow_mut(|q| {
        *q = Some(Qa {
            cmds,
            next: 0,
            out,
            wait: Wait::None,
            t0: Instant::now(),
            trace,
            fails: Vec::new(),
            notes: Vec::new(),
            slow: 1.0,
            scale: None,
            json,
            busy: 0,
            busy_what: Vec::new(),
            centers: Vec::new(),
            background: Vec::new(),
            film: None,
            margin: 6.0,
            jitter: 0,
            frames: 0,
        })
    });
    ACTIVE.store(true, Ordering::Relaxed);
    Ok(true)
}

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter().skip_while(|a| *a != name).nth(1).cloned()
}

/// A line's words; quotes keep spaces in one.
fn words(line: &str) -> Vec<String> {
    let line = line.split_once('#').map_or(line, |(a, _)| a);
    let (mut out, mut cur, mut quoted, mut any) = (Vec::new(), String::new(), false, false);
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                any = false;
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The settings the script starts with.
pub fn settings() -> Settings {
    QA.with_borrow(|q| q.as_ref().map(|q| json_settings(&q.json)))
        .unwrap_or_default()
}

fn json_settings(json: &serde_json::Map<String, Value>) -> Settings {
    Settings::parse(&Value::Object(json.clone()).to_string()).unwrap_or_default()
}

/// Starts the script.
pub fn begin(anchor: HWND) {
    unsafe { SetTimer(Some(anchor), TIMER, STEP_MS, None) };
}

/// The springs' time, slowed down.
pub fn slow() -> f32 {
    QA.with_borrow(|q| q.as_ref().map_or(1.0, |q| q.slow))
}

/// The scale every screen has, if the script set one.
pub fn scale() -> Option<f32> {
    QA.with_borrow(|q| q.as_ref().and_then(|q| q.scale))
}

/// The number of failures, for the exit code.
pub fn code() -> i32 {
    QA.with_borrow(|q| q.as_ref().map_or(0, |q| q.fails.len() as i32))
}

/// An island's message: its timers count while idle. (A frame message the
/// pacer had on its way when it stopped draws nothing: only drawn frames
/// count, in [`frame`].)
pub fn surface_msg(msg: u32, wp: usize) {
    if msg == WM_TIMER {
        busy(format!("timer {wp}"));
    }
}

/// Something ran on an island; while idle, a failure.
fn busy(what: String) {
    QA.with_borrow_mut(|q| {
        if let Some(q) = q
            && matches!(q.wait, Wait::Idle(_))
        {
            q.busy += 1;
            if !q.busy_what.contains(&what) {
                q.busy_what.push(what);
            }
        }
    });
}

/// One island drew a frame: traced, checked, filmed.
pub fn frame(app: &App, i: usize) {
    busy("frame".into());
    let sf = &app.surfaces[i];
    let k = app.settings.island_scale;
    let peek_h = app.peek_unscaled_at(i).1 * k;
    let a = alphas(sf, k, peek_h);
    let o = openness(sf, k, peek_h);
    let s = sf.scale;
    let placement = app.placement;
    let body = shape::body_rect(
        placement,
        sf.window.width,
        sf.width.value * s,
        sf.height.value * s,
        EAR * s,
    );
    let film = QA.with_borrow_mut(|q| {
        let q = q.as_mut()?;
        q.frames += 1;
        let t = q.t0.elapsed().as_secs_f32() * 1000.0;
        let w = sf.window;
        let _ = writeln!(
            q.trace,
            "{t:.1},{i},{s},{},{},{},{},{:.2},{:.2},{:.3},{:.2},{:.2},{:.3},{:.2},{:.2},{:.2},{:.3},{:.3},{:.3},{o:.3},{},{},{}",
            w.x,
            w.y,
            w.width,
            w.height,
            sf.width.value,
            sf.width.target,
            sf.width.velocity,
            sf.height.value,
            sf.height.target,
            sf.height.velocity,
            body.x,
            body.width,
            body.height,
            a[0],
            a[1],
            a[2],
            sf.open as u8,
            sf.peek_view as u8,
            sf.shrinking as u8,
        );
        // 1: the body fits its window, every frame.
        if body.x < -0.5 || body.x + body.width > w.width + 0.5 || body.height > w.height + 0.5 {
            q.fail(format!(
                "1 cuerpo fuera de la ventana: isla {i}, t {t:.0} ms, cuerpo x {:.1} {:.1}x{:.1} px, ventana {}x{} px",
                body.x, body.width, body.height, w.width, w.height
            ));
        }
        // 6: closing never dips under where it stops.
        if sf.shrinking && sf.height.value < sf.height.target - 0.5 {
            q.fail(format!(
                "6 encoge por debajo del destino: isla {i}, t {t:.0} ms, alto {:.1} < {:.1}",
                sf.height.value, sf.height.target
            ));
        }
        // 6: a centered body's center stays put.
        if !matches!(placement, Placement::TopLeft | Placement::TopRight) {
            // On the screen: a window moved by a setting isn't a jump.
            let center = w.x + body.x + body.width / 2.0;
            if q.centers.len() <= i {
                q.centers.resize(i + 1, None);
            }
            if let Some(before) = q.centers[i].replace(center) {
                let moved = (center - before).abs();
                if moved > 1.0 {
                    q.fail(format!(
                        "6 el centro salta {moved:.1} px: isla {i}, t {t:.0} ms"
                    ));
                } else if moved >= 0.5 {
                    q.jitter += 1;
                }
            }
        }
        let (name, n, max) = q.film.as_mut()?;
        *n += 1;
        if *n > *max {
            return None;
        }
        Some(format!("{name}-{i}-{:03}", *n))
    });
    if let Some(name) = film {
        unsafe {
            let _ = DwmFlush();
        }
        shoot(app, i, &name);
    }
}

impl Qa {
    fn fail(&mut self, what: String) {
        // The same failure frame after frame is one.
        let kind: String = what.chars().take(24).collect();
        if self.fails.iter().filter(|f| f.starts_with(&kind)).count() < 5 {
            self.fails.push(what);
        }
    }
}

/// Runs the script on: the next command, once the last one's wait is over.
pub fn step(app: &mut App) {
    loop {
        let ready = QA.with_borrow_mut(|q| {
            let q = q.as_mut()?;
            let still = app.surfaces.iter().all(|s| s.last_frame.is_none());
            match q.wait {
                Wait::None => {}
                Wait::Until(at) if Instant::now() < at => return Some(None),
                Wait::Rest(at) if !still => {
                    if Instant::now() > at {
                        q.fail("4 no llega al reposo a tiempo".into());
                        q.wait = Wait::None;
                    }
                    return Some(None);
                }
                Wait::Rest(_) => {
                    for (i, sf) in app.surfaces.iter().enumerate() {
                        let off = (sf.width.value - sf.width.target).abs()
                            + (sf.height.value - sf.height.target).abs();
                        if off > 0.01 || sf.swap.is_some() {
                            q.fail(format!(
                                "4 reposo a medias: isla {i}, {:.2}x{:.2} hacia {:.2}x{:.2}",
                                sf.width.value, sf.height.value, sf.width.target, sf.height.target
                            ));
                        }
                    }
                    q.film = None;
                }
                Wait::Idle(at) if Instant::now() < at => return Some(None),
                Wait::Idle(_) => {
                    if q.busy > 0 {
                        let what = q.busy_what.join(", ");
                        q.fail(format!("5 en reposo corrieron {} ({what})", q.busy));
                    }
                }
                Wait::Until(_) => {}
            }
            q.wait = Wait::None;
            let Some(cmd) = q.cmds.get(q.next).cloned() else {
                return Some(Some(None));
            };
            q.next += 1;
            Some(Some(Some(cmd)))
        });
        match ready {
            // Not running, or waiting.
            None | Some(None) => return,
            Some(Some(None)) => return finish(app),
            Some(Some(Some((line, cmd)))) => {
                if let Err(e) = run(app, &cmd) {
                    QA.with_borrow_mut(|q| {
                        if let Some(q) = q {
                            q.fail(format!("línea {line}: {e}"));
                        }
                    });
                }
            }
        }
    }
}

fn arg_f(cmd: &[String], i: usize) -> std::result::Result<f32, String> {
    cmd.get(i)
        .ok_or_else(|| format!("«{}» pide un número", cmd[0]))?
        .parse()
        .map_err(|_| format!("«{}»: número no válido", cmd[0]))
}

fn text(cmd: &[String], i: usize) -> String {
    cmd.get(i).cloned().unwrap_or_default()
}

fn set_wait(w: Wait) {
    QA.with_borrow_mut(|q| {
        if let Some(q) = q {
            q.busy = 0;
            q.busy_what.clear();
            q.wait = w;
        }
    });
}

fn in_ms(ms: f32) -> Instant {
    Instant::now() + std::time::Duration::from_millis(ms.max(0.0) as u64)
}

fn run(app: &mut App, cmd: &[String]) -> std::result::Result<(), String> {
    let each = |app: &App, cmd: &[String]| -> Vec<usize> {
        match cmd.get(1).and_then(|s| s.parse::<usize>().ok()) {
            Some(i) if i < app.surfaces.len() => vec![i],
            _ => (0..app.surfaces.len()).collect(),
        }
    };
    match cmd[0].as_str() {
        "set" => {
            let key = cmd.get(1).ok_or("«set» pide una clave")?.clone();
            let raw = text(cmd, 2);
            let v = serde_json::from_str(&raw).unwrap_or(Value::String(raw));
            let s = QA.with_borrow_mut(|q| {
                let q = q.as_mut()?;
                q.json.insert(key, v);
                Some(json_settings(&q.json))
            });
            if let Some(s) = s {
                app.apply(s);
            }
        }
        "scale" => {
            let n = arg_f(cmd, 1)?;
            QA.with_borrow_mut(|q| q.as_mut().map(|q| q.scale = Some(n)));
            app.layout();
        }
        "slow" => {
            let n = arg_f(cmd, 1)?.max(0.01);
            QA.with_borrow_mut(|q| q.as_mut().map(|q| q.slow = n));
        }
        "margin" => {
            let n = arg_f(cmd, 1)?;
            QA.with_borrow_mut(|q| q.as_mut().map(|q| q.margin = n));
        }
        "track" => {
            app.track = Some(media::Track {
                title: text(cmd, 1),
                artist: text(cmd, 2),
                album: String::new(),
                playing: true,
                position_ms: 30_000,
                measured: Instant::now(),
                duration_ms: 200_000,
                art: None,
                art_before: None,
                can_prev: true,
                can_toggle: true,
                can_next: true,
            });
            app.track_changed();
        }
        "notrack" => {
            app.track = None;
            app.track_changed();
        }
        "notice" => app.notify(
            Notice {
                icon: island_core::icons::Icon::Timer,
                tint: render::WHITE,
                title: text(cmd, 1).into(),
                detail: text(cmd, 2),
                ..Default::default()
            },
            PEEK_MS,
        ),
        "level" => {
            let level = arg_f(cmd, 1)?.clamp(0.0, 1.0);
            app.notify(
                Notice {
                    icon: glance::volume_icon(level, false),
                    tint: render::WHITE,
                    title: app.texts.volume.into(),
                    detail: format!("{} %", glance::percent(level)),
                    level: Some(level),
                    ..Default::default()
                },
                glance::LEVEL_MS,
            );
        }
        "toast" => {
            app.pulse = Some(Instant::now());
            app.notify(
                Notice {
                    icon: island_core::notifications::ICON,
                    tint: render::WHITE,
                    title: text(cmd, 1).into(),
                    detail: text(cmd, 2),
                    toast: true,
                    body: text(cmd, 3),
                    ..Default::default()
                },
                TOAST_MS,
            );
        }
        "open" | "close" => {
            for i in each(app, cmd) {
                app.set_open(i, cmd[0] == "open");
            }
        }
        "page" => {
            app.page = Page::parse(&text(cmd, 1)).ok_or("«page»: home o focus")?;
            app.redraw_all();
        }
        "wait" => set_wait(Wait::Until(in_ms(arg_f(cmd, 1)?))),
        "wait-rest" => set_wait(Wait::Rest(in_ms(arg_f(cmd, 1).unwrap_or(10_000.0)))),
        "idle" => set_wait(Wait::Idle(in_ms(arg_f(cmd, 1)?))),
        "background" => background(app),
        "grab" => {
            let name = cmd.get(1).ok_or("«grab» pide un nombre")?.clone();
            unsafe {
                let _ = DwmFlush();
            }
            for i in 0..app.surfaces.len() {
                shoot(app, i, &format!("{name}-{i}"));
            }
        }
        "film" => {
            let name = cmd.get(1).ok_or("«film» pide un nombre")?.clone();
            let max = arg_f(cmd, 2).unwrap_or(60.0) as u32;
            QA.with_borrow_mut(|q| q.as_mut().map(|q| q.film = Some((name, 0, max))));
        }
        "quit" => {
            QA.with_borrow_mut(|q| q.as_mut().map(|q| q.next = q.cmds.len()));
        }
        other => return Err(format!("orden desconocida «{other}»")),
    }
    Ok(())
}

/// Whether the screen shows the desktop: with the screen saver on (or the
/// session locked) a strip would hold that instead, so none is taken and
/// only the frame checks run.
fn screen_free() -> bool {
    let shows = unsafe { SHQueryUserNotificationState() }.is_ok_and(|s| s.0 != 1);
    if !shows {
        QA.with_borrow_mut(|q| {
            if let Some(q) = q {
                let note = "sin capturas: el protector de pantalla o el bloqueo tapa el escritorio";
                if !q.notes.iter().any(|n| n == note) {
                    q.notes.push(note.into());
                }
            }
        });
    }
    shows
}

/// The screen behind each island, with the islands away for a moment.
fn background(app: &App) {
    if !screen_free() {
        return;
    }
    for sf in &app.surfaces {
        unsafe {
            let _ = ShowWindow(sf.hwnd, SW_HIDE);
        }
    }
    unsafe {
        let _ = DwmFlush();
        let _ = DwmFlush();
    }
    std::thread::sleep(std::time::Duration::from_millis(120));
    let grabs: Vec<Option<Grab>> = app.surfaces.iter().map(|sf| grab(sf.window)).collect();
    for sf in &app.surfaces {
        unsafe {
            let _ = ShowWindow(sf.hwnd, SW_SHOWNOACTIVATE);
        }
    }
    QA.with_borrow_mut(|q| q.as_mut().map(|q| q.background = grabs));
}

/// Saves island `i`'s window as `<name>.png` and checks, against the
/// background, that what is drawn stays in its body (plus the margin).
fn shoot(app: &App, i: usize, name: &str) {
    if !screen_free() {
        return;
    }
    let sf = &app.surfaces[i];
    let Some(g) = grab(sf.window) else { return };
    let s = sf.scale;
    let body = shape::body_rect(
        app.placement,
        sf.window.width,
        sf.width.value * s,
        sf.height.value * s,
        EAR * s,
    );
    QA.with_borrow_mut(|q| {
        let Some(q) = q.as_mut() else { return };
        let path = q.out.join(format!("{name}.png"));
        if let Err(e) = save_png(&g, &path) {
            q.notes.push(format!("{name}: no se pudo guardar ({e})"));
        }
        let Some(Some(bg)) = q.background.get(i) else {
            return;
        };
        if bg.w != g.w || bg.h != g.h || bg.x != g.x || bg.y != g.y {
            q.notes.push(format!(
                "{name}: el fondo es de otra ventana; repetir «background»"
            ));
            return;
        }
        let m = q.margin * s;
        let (l, r, b) = (body.x - m, body.x + body.width + m, body.height + m);
        let (mut outside, mut far) = (0u32, 0.0f32);
        for y in 0..g.h {
            for x in 0..g.w {
                let p = ((y * g.w + x) * 4) as usize;
                let d: i32 = (0..3)
                    .map(|c| (g.bgra[p + c] as i32 - bg.bgra[p + c] as i32).abs())
                    .sum();
                if d < DRAWN {
                    continue;
                }
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let dist = (l - fx).max(fx - r).max(fy - b);
                if dist > 0.0 {
                    outside += 1;
                    far = far.max(dist);
                }
            }
        }
        if outside > 0 {
            q.fail(format!(
                "2/3 dibujo fuera del cuerpo en {name}: {outside} px, hasta {:.0} px fuera",
                far
            ));
        }
    });
}

/// A strip of the screen as it is now, in px.
fn grab(r: Rect) -> Option<Grab> {
    let (x, y, w, h) = (r.x as i32, r.y as i32, r.width as i32, r.height as i32);
    if w <= 0 || h <= 0 {
        return None;
    }
    unsafe {
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let out = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
            .ok()
            .and_then(|bmp| {
                let old = SelectObject(dc, bmp.into());
                let ok = BitBlt(dc, 0, 0, w, h, Some(screen), x, y, SRCCOPY | CAPTUREBLT).is_ok();
                let px = ok.then(|| {
                    let mut v = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize)
                        .to_vec();
                    for a in v.iter_mut().skip(3).step_by(4) {
                        *a = 255;
                    }
                    v
                });
                SelectObject(dc, old);
                let _ = DeleteObject(bmp.into());
                px
            });
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
        out.map(|bgra| Grab { x, y, w, h, bgra })
    }
}

fn save_png(g: &Grab, path: &std::path::Path) -> windows::core::Result<()> {
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = wic.CreateStream()?;
        stream.InitializeFromFilename(
            &HSTRING::from(path.as_os_str()),
            windows::Win32::Foundation::GENERIC_WRITE.0,
        )?;
        let enc = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let mut frame = None;
        enc.CreateNewFrame(&mut frame, std::ptr::null_mut())?;
        let frame = frame.ok_or_else(|| windows::core::Error::from(E_FAIL))?;
        frame.Initialize(None)?;
        frame.SetSize(g.w as u32, g.h as u32)?;
        let mut format: GUID = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut format)?;
        frame.WritePixels(g.h as u32, (g.w * 4) as u32, &g.bgra)?;
        frame.Commit()?;
        enc.Commit()
    }
}

/// The script is over: the report, and the app ends.
fn finish(app: &mut App) {
    unsafe {
        let _ = KillTimer(Some(app.anchor), TIMER);
    }
    QA.with_borrow_mut(|q| {
        let Some(q) = q.as_mut() else { return };
        let _ = q.trace.flush();
        let mut r = String::new();
        r += &format!("frames: {}\n", q.frames);
        r += &format!("centro ±0,5 px: {} frames\n", q.jitter);
        for n in &q.notes {
            r += &format!("nota: {n}\n");
        }
        if q.fails.is_empty() {
            r += "OK: sin fallos\n";
        }
        for f in &q.fails {
            r += &format!("FALLO {f}\n");
        }
        let _ = std::fs::write(q.out.join("report.txt"), r);
    });
    app.quit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_keep_words_together() {
        assert_eq!(
            words(r#"track "Una canción" "" # comentario"#),
            ["track", "Una canción", ""]
        );
        assert!(words("   # nada").is_empty());
        assert_eq!(words("set islandScale 1.4"), ["set", "islandScale", "1.4"]);
    }
}
