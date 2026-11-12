//! Synced lyrics from lrclib.net, fetched in process (the plugin's
//! `lyrics-fetch.sh` and `LyricsService.qml`). Off unless asked for.
//!
//! A worker thread sleeps on a channel. For each song it asks the disk cache
//! first, then lrclib.net, at most three requests one after another, never
//! retried. The answer goes to the window as a message. The parsing and the
//! checks are island-core's; everything from the network is untrusted.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use island_core::lyrics::{self, Found, Kind, MAX_RECORD};
use serde_json::Value;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

/// The answer to a search can be longer than one record.
const MAX_SEARCH: usize = 1 << 20;
/// The disk cache, as the script keeps it.
const KEEP: usize = 400;
const FOUND_DAYS: u64 = 90;

pub struct Query {
    /// [`lyrics::cache_key`] of the song.
    pub key: String,
    pub artist: String,
    pub title: String,
    pub album: String,
    /// Whole seconds, 10 to 3600.
    pub duration: u32,
}

pub struct Lyrics {
    tx: Sender<Query>,
    latest: Arc<Mutex<Option<(String, Found)>>>,
}

impl Lyrics {
    /// Starts the worker; it posts `msg` to `hwnd` with each answer.
    pub fn start(hwnd: HWND, msg: u32) -> Self {
        let (tx, rx) = mpsc::channel();
        let latest = Arc::new(Mutex::new(None));
        let worker_latest = latest.clone();
        let hwnd = hwnd.0 as isize;
        let _ = std::thread::Builder::new()
            .name("lyrics".into())
            .spawn(move || run(rx, worker_latest, hwnd, msg));
        Self { tx, latest }
    }

    pub fn ask(&self, q: Query) {
        let _ = self.tx.send(q);
    }

    /// The last answer: the song's key and what was found.
    pub fn latest(&self) -> Option<(String, Found)> {
        self.latest.lock().ok()?.clone()
    }
}

fn run(rx: Receiver<Query>, latest: Arc<Mutex<Option<(String, Found)>>>, hwnd: isize, msg: u32) {
    let cache = cache_dir();
    // What was answered this run: found ones, and misses until they may be
    // asked again (a day; ten minutes after a network error).
    let mut memo: HashMap<String, (Found, Option<Instant>)> = HashMap::new();
    while let Ok(mut q) = rx.recv() {
        // Songs skipped quickly: only the last one matters.
        while let Ok(newer) = rx.try_recv() {
            q = newer;
        }
        let known = memo
            .get(&q.key)
            .filter(|(_, until)| until.is_none_or(|t| Instant::now() < t))
            .map(|(f, _)| f.clone());
        let found = match known {
            Some(f) => f,
            None => {
                let (f, retry) = lookup(&q, cache.as_ref());
                if memo.len() >= 64 {
                    memo.clear();
                }
                let until = match retry {
                    Some(d) => Some(Instant::now() + d),
                    None if f.kind == Kind::Nothing => {
                        Some(Instant::now() + Duration::from_millis(lyrics::NOTFOUND_RETRY_MS))
                    }
                    None => None,
                };
                memo.insert(q.key.clone(), (f.clone(), until));
                f
            }
        };
        if let Ok(mut l) = latest.lock() {
            *l = Some((q.key, found));
        }
        unsafe {
            let _ = PostMessageW(Some(HWND(hwnd as *mut _)), msg, WPARAM(0), LPARAM(0));
        }
    }
}

/// `%LOCALAPPDATA%\island\lyrics`, if it can be made.
fn cache_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
        .join("island")
        .join("lyrics");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn age(path: &PathBuf) -> Option<Duration> {
    let m = fs::metadata(path).ok()?.modified().ok()?;
    SystemTime::now().duration_since(m).ok()
}

/// What the song has, and when to ask again after a network error.
fn lookup(q: &Query, cache: Option<&PathBuf>) -> (Found, Option<Duration>) {
    let d = q.duration as f64;
    let name = lyrics::file_name(&q.key);
    let (json, none) = match cache {
        Some(c) => (
            c.join(format!("{name}.json")),
            c.join(format!("{name}.none")),
        ),
        None => (PathBuf::new(), PathBuf::new()),
    };
    if cache.is_some() {
        if age(&json).is_some_and(|a| a < Duration::from_secs(FOUND_DAYS * 86_400))
            && let Ok(text) = fs::read_to_string(&json)
        {
            // Used again: it moves to the back of the line to be pruned.
            if let Ok(f) = fs::File::options().append(true).open(&json) {
                let _ = f.set_modified(SystemTime::now());
            }
            return (lyrics::validate_text(&text, d), None);
        }
        if age(&none).is_some_and(|a| a < Duration::from_millis(lyrics::NOTFOUND_RETRY_MS)) {
            return (Found::nothing(), None);
        }
    }
    let store = |path: &PathBuf, body: &[u8]| {
        if cache.is_some() {
            let part = path.with_extension("part");
            if fs::write(&part, body).is_ok() {
                let _ = fs::rename(&part, path);
            }
            prune(cache);
        }
    };
    let error = (
        Found::nothing(),
        Some(Duration::from_millis(lyrics::ERROR_RETRY_MS)),
    );
    let paths = lyrics::requests(&q.artist, &q.title, &q.album, q.duration);
    let last = paths.len() - 1;
    for (i, path) in paths.iter().enumerate() {
        let search = i == last;
        let cap = if search { MAX_SEARCH } else { MAX_RECORD };
        let Ok((code, body)) = crate::http::get("lrclib.net", path, cap) else {
            return error;
        };
        match code {
            200 if !search && first_char(&body) == Some(b'{') => {
                store(&json, &body);
                let text = String::from_utf8_lossy(&body);
                return (lyrics::validate_text(&text, d), None);
            }
            200 if search && first_char(&body) == Some(b'[') => {
                let list: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                return match lyrics::pick_search(&list, d) {
                    Some(rec) => {
                        let text = rec.to_string();
                        if text.len() <= MAX_RECORD {
                            store(&json, text.as_bytes());
                        }
                        (lyrics::validate(rec, d), None)
                    }
                    None => {
                        store(&none, b"");
                        (Found::nothing(), None)
                    }
                };
            }
            404 if search => {
                store(&none, b"");
                return (Found::nothing(), None);
            }
            404 => continue,
            _ => return error,
        }
    }
    error
}

fn first_char(body: &[u8]) -> Option<u8> {
    body.iter()
        .take(200)
        .copied()
        .find(|b| !b.is_ascii_whitespace())
}

/// The cache keeps the last [`KEEP`] songs used.
fn prune(cache: Option<&PathBuf>) {
    let Some(dir) = cache else { return };
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    if files.len() <= KEEP {
        return;
    }
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    for (_, p) in files.into_iter().skip(KEEP) {
        let _ = fs::remove_file(p);
    }
}
