//! What is playing, from Windows' media controls (GSMTC): the same session
//! the volume flyout shows, for any app that publishes one.
//!
//! A worker thread owns the WinRT objects. It sleeps on a channel; the
//! session's events and the UI's commands wake it, it reads the session once
//! and posts a message to the window. Nothing polls.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use island_core::media::position;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::SystemInformation::GetSystemTimePreciseAsFileTime;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::{Interface, Result, RuntimeType};

/// Artwork bigger than this is skipped (covers are a few hundred KB).
const MAX_ART: u64 = 4 << 20;
/// Looks for a late cover this many times, this far apart.
const ART_RETRIES: u32 = 3;
const ART_RETRY: std::time::Duration = std::time::Duration::from_millis(1500);

#[derive(Debug, Clone)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    /// The position at `measured`.
    pub position_ms: u64,
    pub measured: Instant,
    /// 0 when the app doesn't say (a live stream).
    pub duration_ms: u64,
    /// The encoded image, shared until the song changes.
    pub art: Option<Arc<Vec<u8>>>,
    /// The song before's image: apps publish a new song before its cover,
    /// so for a moment `art` can still be this one.
    pub art_before: Option<Arc<Vec<u8>>>,
    pub can_prev: bool,
    pub can_toggle: bool,
    pub can_next: bool,
}

impl Track {
    pub fn position_now(&self) -> u64 {
        let elapsed = self.measured.elapsed().as_millis() as u64;
        position(self.position_ms, elapsed, self.duration_ms, self.playing)
    }
}

pub enum Command {
    Refresh,
    PlayPause,
    Next,
    Prev,
}

pub struct Media {
    tx: Sender<Command>,
    latest: Arc<Mutex<Option<Track>>>,
}

impl Media {
    /// Starts the worker; it posts `msg` to `hwnd` whenever the track changes.
    pub fn start(hwnd: HWND, msg: u32) -> Self {
        let (tx, rx) = mpsc::channel();
        let latest = Arc::new(Mutex::new(None));
        let (worker_tx, worker_latest) = (tx.clone(), latest.clone());
        // HWND isn't Send; the handle itself is just a number.
        let hwnd = hwnd.0 as isize;
        let _ = std::thread::Builder::new()
            .name("media".into())
            .spawn(move || run(rx, worker_tx, worker_latest, hwnd, msg));
        Self { tx, latest }
    }

    pub fn send(&self, c: Command) {
        let _ = self.tx.send(c);
    }

    pub fn latest(&self) -> Option<Track> {
        self.latest.lock().ok()?.clone()
    }
}

fn notify<T: RuntimeType + 'static, A: RuntimeType + 'static>(
    tx: &Sender<Command>,
) -> TypedEventHandler<T, A> {
    let tx = tx.clone();
    TypedEventHandler::new(move |_, _| {
        let _ = tx.send(Command::Refresh);
        Ok(())
    })
}

/// The session whose events wake us, with their tokens.
struct Watched {
    session: Session,
    tokens: [i64; 3],
}

impl Watched {
    fn new(session: &Session, tx: &Sender<Command>) -> Self {
        let s = session;
        Self {
            session: s.clone(),
            tokens: [
                s.MediaPropertiesChanged(&notify(tx)).unwrap_or_default(),
                s.PlaybackInfoChanged(&notify(tx)).unwrap_or_default(),
                s.TimelinePropertiesChanged(&notify(tx)).unwrap_or_default(),
            ],
        }
    }
}

impl Drop for Watched {
    fn drop(&mut self) {
        let [a, b, c] = self.tokens;
        let s = &self.session;
        let _ = s.RemoveMediaPropertiesChanged(a);
        let _ = s.RemovePlaybackInfoChanged(b);
        let _ = s.RemoveTimelinePropertiesChanged(c);
    }
}

fn run(
    rx: Receiver<Command>,
    tx: Sender<Command>,
    latest: Arc<Mutex<Option<Track>>>,
    hwnd: isize,
    msg: u32,
) {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let Ok(manager) = Manager::RequestAsync().and_then(|op| op.join()) else {
        return;
    };
    let _ = manager.CurrentSessionChanged(&notify(&tx));
    let _ = manager.SessionsChanged(&notify(&tx));
    let mut watched: Option<Watched> = None;
    let mut last: Option<Track> = None;
    // The song whose cover is awaited, and how many times it was retried.
    let mut stale: ((String, String), u32) = Default::default();
    let _ = tx.send(Command::Refresh);

    while let Ok(first) = rx.recv() {
        // Events come in bursts: handle what queued up, then read once.
        let cmds: Vec<Command> = std::iter::once(first).chain(rx.try_iter()).collect();
        let session = manager.GetCurrentSession().ok();
        let same = match (&watched, &session) {
            (Some(w), Some(s)) => w.session.as_raw() == s.as_raw(),
            (None, None) => true,
            _ => false,
        };
        if !same {
            watched = session.as_ref().map(|s| Watched::new(s, &tx));
        }
        if let Some(s) = &session {
            for c in &cmds {
                let op = match c {
                    Command::Refresh => continue,
                    Command::PlayPause => s.TryTogglePlayPauseAsync(),
                    Command::Next => s.TrySkipNextAsync(),
                    Command::Prev => s.TrySkipPreviousAsync(),
                };
                let _ = op.and_then(|op| op.join());
            }
        }
        let track = session.as_ref().and_then(|s| read(s, last.as_ref()).ok());
        // Some apps never say when the cover arrives: while it is still the
        // previous song's, look again a few times.
        if let Some(t) = &track {
            let song = (t.title.clone(), t.artist.clone());
            if song != stale.0 {
                stale = (song, 0);
            }
            let old = match (&t.art, &t.art_before) {
                (Some(a), Some(b)) => a.as_slice() == b.as_slice(),
                (None, _) => true,
                _ => false,
            };
            if old && stale.1 < ART_RETRIES {
                stale.1 += 1;
                let tx = tx.clone();
                let _ = std::thread::Builder::new()
                    .name("media-retry".into())
                    .spawn(move || {
                        std::thread::sleep(ART_RETRY);
                        let _ = tx.send(Command::Refresh);
                    });
            }
        }
        if let Ok(mut l) = latest.lock() {
            *l = track.clone();
        }
        last = track;
        unsafe {
            let _ = PostMessageW(Some(HWND(hwnd as *mut _)), msg, WPARAM(0), LPARAM(0));
        }
    }
}

/// 100 ns ticks to ms.
fn ms(ticks: i64) -> u64 {
    (ticks.max(0) / 10_000) as u64
}

fn now_filetime() -> i64 {
    let f = unsafe { GetSystemTimePreciseAsFileTime() };
    ((f.dwHighDateTime as i64) << 32) | f.dwLowDateTime as i64
}

fn read(s: &Session, last: Option<&Track>) -> Result<Track> {
    let props = s.TryGetMediaPropertiesAsync()?.join()?;
    let info = s.GetPlaybackInfo()?;
    let controls = info.Controls()?;
    let timeline = s.GetTimelineProperties()?;
    let playing = info.PlaybackStatus()? == Status::Playing;
    let title = props.Title()?.to_string_lossy();
    let artist = props.Artist()?.to_string_lossy();
    let album = props.AlbumTitle()?.to_string_lossy();

    let start = timeline.StartTime()?.Duration;
    let duration_ms = ms(timeline.EndTime()?.Duration - start);
    let mut at = ms(timeline.Position()?.Duration - start);
    // The position is from when the app last reported it.
    if playing {
        let updated = timeline.LastUpdatedTime()?.UniversalTime;
        let now = now_filetime();
        if updated > 0 && now > updated {
            at += ms(now - updated);
        }
    }

    // The cover is read once per song. Apps often publish the song first and
    // its cover a moment later, so a song without one, or still with the
    // previous song's, tries again on the next event.
    let same_song = last.filter(|l| l.title == title && l.artist == artist && l.album == album);
    let art_before = match same_song {
        Some(l) => l.art_before.clone(),
        None => last.and_then(|l| l.art.clone()),
    };
    let settled = same_song.and_then(|l| l.art.clone()).filter(|a| {
        art_before
            .as_ref()
            .is_none_or(|b| a.as_slice() != b.as_slice())
    });
    let art = match settled {
        Some(a) => Some(a),
        None => artwork(&props).ok().flatten(),
    };

    Ok(Track {
        title,
        artist,
        album,
        playing,
        position_ms: position(at, 0, duration_ms, playing),
        measured: Instant::now(),
        duration_ms,
        art,
        art_before,
        can_prev: controls.IsPreviousEnabled()?,
        can_toggle: controls.IsPlayPauseToggleEnabled()?
            || controls.IsPlayEnabled()?
            || controls.IsPauseEnabled()?,
        can_next: controls.IsNextEnabled()?,
    })
}

fn artwork(props: &Properties) -> Result<Option<Arc<Vec<u8>>>> {
    let Ok(thumb) = props.Thumbnail() else {
        return Ok(None);
    };
    let stream = thumb.OpenReadAsync()?.join()?;
    let size = stream.Size()?;
    if size == 0 || size > MAX_ART {
        return Ok(None);
    }
    let reader = DataReader::CreateDataReader(&stream)?;
    let n = reader.LoadAsync(size as u32)?.join()?;
    let mut buf = vec![0; n as usize];
    reader.ReadBytes(&mut buf)?;
    Ok(Some(Arc::new(buf)))
}
