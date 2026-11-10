//! Synced lyrics from lrclib.net, as pure functions: parsing the LRC text,
//! finding the line for a playback position, cleaning names for the fallback
//! search and checking the records lrclib.net returns.
//!
//! A port of `LyricsModel.js` from the Omarchy plugin, with the same limits.
//! Lengths that JavaScript counts with `.length` are counted here in UTF-16
//! units too, so both sides cut text at the same place. `parseFetch` is not
//! ported: it reads what `scripts/lyrics-fetch.sh` prints, and the desktop
//! apps fetch in process.
//!
//! Everything that comes from the network is untrusted: sizes are capped and
//! text is cleaned before it reaches the UI.

use std::sync::LazyLock;

use regex_lite::Regex;
use serde_json::Value;

/// UTF-16 units of `syncedLyrics` accepted.
pub const MAX_TEXT: usize = 64 * 1024;
pub const MAX_LINES: usize = 600;
/// UTF-16 units per line once cleaned.
pub const MAX_LINE: usize = 200;
/// Seconds, when matching a record's duration with the track's.
pub const DURATION_SLACK: f64 = 3.0;
/// Bytes of a record's JSON text accepted by [`validate_text`].
pub const MAX_RECORD: usize = 262_144;
/// How long a song lrclib.net doesn't have is not asked for again.
pub const NOTFOUND_RETRY_MS: u64 = 24 * 3600 * 1000;
/// How long to wait after the network or the service failed.
pub const ERROR_RETRY_MS: u64 = 10 * 60 * 1000;

const MAX_OFFSET: i64 = 600_000;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid regex")
}

static OSC: LazyLock<Regex> = LazyLock::new(|| re(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?"));
static CSI: LazyLock<Regex> = LazyLock::new(|| re(r"\x1b\[[0-?]*[ -/]*[@-~]"));
static ESC: LazyLock<Regex> = LazyLock::new(|| re(r"\x1b[@-_]"));
static CONTROL: LazyLock<Regex> = LazyLock::new(|| re(r"[\x00-\x1f\x7f-\x{9f}]"));
static INVISIBLE: LazyLock<Regex> =
    LazyLock::new(|| re(r"[\x{200B}-\x{200F}\x{202A}-\x{202E}\x{2066}-\x{2069}\x{FEFF}]"));

static TIME: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:(\d{1,2}):)?(\d{1,3}):(\d{1,2})(?:[.:](\d{1,3}))?$"));
static ROWS: LazyLock<Regex> = LazyLock::new(|| re(r"\r\n|\r|\n"));
static TAG: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*\[([^\]]*)\]"));
static KV: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*([A-Za-z#]+)\s*:\s*(.*)$"));
static META: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(ar|ti|al|au|by|length|re|ve|tool|la|id|#)$"));
static WORD_STAMP: LazyLock<Regex> = LazyLock::new(|| re(r"<\d{1,3}:\d{1,2}(?:[.:]\d{1,3})?>"));

/// Number of UTF-16 units, what JavaScript's `.length` gives.
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// The longest prefix of `s` with at most `max` UTF-16 units, never cutting
/// a character in two.
fn truncate_utf16(s: &str, max: usize) -> &str {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        units += c.len_utf16();
        if units > max {
            return &s[..i];
        }
    }
    s
}

/// Drops terminal escapes, control and invisible characters, collapses
/// spacing and cuts to `max` UTF-16 units.
pub fn clean_line(raw: &str, max: usize) -> String {
    let s = OSC.replace_all(raw, "");
    let s = CSI.replace_all(&s, "");
    let s = ESC.replace_all(&s, "");
    let s = CONTROL.replace_all(&s, " ");
    let s = INVISIBLE.replace_all(&s, "");
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_utf16(&s, max).trim().to_string()
}

/// `mm:ss`, `mm:ss.x`, `mm:ss.xx`, `mm:ss.xxx` or `h:mm:ss.xx` to ms.
pub fn parse_time(tag: &str) -> Option<u64> {
    let m = TIME.captures(tag.trim())?;
    let num = |i: usize| m.get(i).map(|g| g.as_str().parse::<u64>().unwrap_or(0));
    let h = num(1);
    let min = num(2).unwrap_or(0);
    let sec = num(3).unwrap_or(0);
    if sec > 59 || (h.is_some() && min > 59) {
        return None;
    }
    // "5" -> 500, "25" -> 250, "125" -> 125.
    let frac = m.get(4).map_or(0, |g| {
        let d = g.as_str();
        d.parse::<u64>().unwrap_or(0) * 10u64.pow(3 - d.len() as u32)
    });
    Some(((h.unwrap_or(0) * 60 + min) * 60 + sec) * 1000 + frac)
}

/// The leading integer of `s`, as JavaScript's `parseInt` reads it.
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim_start();
    let (neg, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let mut n: i64 = 0;
    let mut any = false;
    for b in digits.bytes().take_while(u8::is_ascii_digit) {
        any = true;
        n = n.saturating_mul(10).saturating_add(i64::from(b - b'0'));
    }
    any.then_some(if neg { -n } else { n })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Start, in ms from the beginning of the track.
    pub t: u64,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lrc {
    /// Sorted by start time.
    pub lines: Vec<Line>,
    /// The `[offset:]` tag, in ms, already applied to `lines`.
    pub offset: i64,
}

pub fn parse_lrc(text: &str) -> Lrc {
    let text = truncate_utf16(text, MAX_TEXT);
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut lines: Vec<(u64, String)> = Vec::new();
    let mut offset = 0;
    for row in ROWS.split(text) {
        if lines.len() >= MAX_LINES {
            break;
        }
        let mut stamps = Vec::new();
        let mut rest = row;
        // Leading [..] tags: time stamps, offset or metadata.
        while let Some(c) = TAG.captures(rest) {
            let after = &rest[c.get(0).map_or(0, |g| g.end())..];
            let tag = c.get(1).map_or("", |g| g.as_str());
            if let Some(t) = parse_time(tag) {
                stamps.push(t);
                rest = after;
                continue;
            }
            if let Some(kv) = KV.captures(tag) {
                let key = kv.get(1).map_or("", |g| g.as_str());
                if key.eq_ignore_ascii_case("offset") {
                    if let Some(n) = parse_int(kv.get(2).map_or("", |g| g.as_str())) {
                        offset = n.clamp(-MAX_OFFSET, MAX_OFFSET);
                    }
                    rest = after;
                    continue;
                }
                if META.is_match(key) {
                    rest = after;
                    continue;
                }
            }
            // Not a tag we know: it is part of the text.
            break;
        }
        if stamps.is_empty() {
            continue;
        }
        // Enhanced LRC word stamps are dropped from the text.
        let txt = clean_line(&WORD_STAMP.replace_all(rest, ""), MAX_LINE);
        for t in stamps {
            if lines.len() >= MAX_LINES {
                break;
            }
            lines.push((t, txt.clone()));
        }
    }
    // Stable: equal stamps keep their order.
    lines.sort_by_key(|l| l.0);
    // A positive offset shifts the lyrics earlier (LRC convention).
    let lines = lines
        .into_iter()
        .map(|(t, text)| Line {
            t: (t as i64 - offset).max(0) as u64,
            text,
        })
        .collect();
    Lrc { lines, offset }
}

/// The last line starting at or before `pos_ms`; `None` before the first.
pub fn line_at(lines: &[Line], pos_ms: u64) -> Option<usize> {
    lines.partition_point(|l| l.t <= pos_ms).checked_sub(1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineWindow<'a> {
    pub index: Option<usize>,
    pub current: &'a str,
    /// The next line with text.
    pub next: &'a str,
    /// When the shown line changes next; `None`: never.
    pub next_at: Option<u64>,
}

/// What to show at `pos_ms`, and when it changes next.
pub fn window(lines: &[Line], pos_ms: u64) -> LineWindow<'_> {
    let index = line_at(lines, pos_ms);
    let from = index.map_or(0, |i| i + 1);
    let later = lines.get(from..).unwrap_or_default();
    LineWindow {
        index,
        current: index.map_or("", |i| lines[i].text.as_str()),
        next: later
            .iter()
            .find(|l| !l.text.is_empty())
            .map_or("", |l| l.text.as_str()),
        next_at: later.first().map(|l| l.t),
    }
}

// For the fallback search only: drops what streaming services append and
// lrclib.net doesn't have. "(Live)", "(Acoustic)", "(Radio Edit)" stay: they
// are different recordings. `true`: every match, `false`: the first.
static TITLE_JUNK: LazyLock<Vec<(Regex, bool)>> = LazyLock::new(|| {
    [
        (r"\s+-\s+(?:\d{4}\s+)?(?:digital\s+)?remaster(?:ed)?(?:\s+(?:version|\d{4}))?\s*$", false),
        (r"\s+-\s+(?:bonus\s+track|single\s+version)\s*$", false),
        (r"\s*[(\[]\s*(?:\d{4}\s+)?(?:digital\s+)?remaster(?:ed)?(?:\s+(?:version|\d{4}))?\s*[)\]]", true),
        (r"\s*[(\[]\s*(?:feat\.?|ft\.?|featuring|with)\s+[^)\]]*[)\]]", true),
        (r"\s*[(\[]\s*(?:official\s+)?(?:music\s+video|video|audio|lyric\s+video|lyrics|visuali[sz]er)\s*[)\]]", true),
        (r"\s*[(\[]\s*(?:hd|hq|4k)\s*[)\]]", true),
        (r"\s*[(\[]\s*(?:bonus\s+track|deluxe[^)\]]*|explicit)\s*[)\]]", true),
        (r"\s+(?:feat\.?|ft\.?|featuring)\s+.+$", false),
    ]
    .into_iter()
    .map(|(p, all)| (re(&format!("(?i){p}")), all))
    .collect()
});

static ARTIST_SEP: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\s*(?:,|;|&|\s+x\s+|\s+(?:feat\.?|ft\.?|featuring|with)\s+)\s*"));

pub fn normalize_title(raw: &str) -> String {
    let mut s = clean_line(raw, MAX_LINE);
    for (junk, all) in TITLE_JUNK.iter() {
        s = if *all {
            junk.replace_all(&s, "").into_owned()
        } else {
            junk.replace(&s, "").into_owned()
        };
    }
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.is_empty() {
        clean_line(raw, MAX_LINE)
    } else {
        s
    }
}

/// The first artist of a list.
pub fn normalize_artist(raw: &str) -> String {
    let s = clean_line(raw, MAX_LINE);
    let first = ARTIST_SEP.split(&s).next().unwrap_or("").trim();
    if first.is_empty() {
        s.clone()
    } else {
        first.to_string()
    }
}

/// What names a song in the cache: artist, title, album and duration.
pub fn cache_key(artist: &str, title: &str, album: &str, duration_sec: f64) -> String {
    let norm = |v: &str| clean_line(v, 400).to_lowercase();
    // Math.round: halves go up.
    let secs = if duration_sec.is_finite() {
        (duration_sec + 0.5).floor() as i64
    } else {
        0
    };
    format!("{}\n{}\n{}\n{secs}", norm(artist), norm(title), norm(album))
}

/// A cache file's name for a [`cache_key`]: 16 hex digits (FNV-1a), so no
/// song text ever reaches the file system.
pub fn file_name(key: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// `/api/<endpoint>?name=value&â€¦`, each value percent-encoded (UTF-8), as
/// `curl --data-urlencode` does in `lyrics-fetch.sh`.
pub fn api_path(endpoint: &str, fields: &[(&str, &str)]) -> String {
    let mut out = format!("/api/{endpoint}");
    for (i, (k, v)) in fields.iter().enumerate() {
        out.push(if i == 0 { '?' } else { '&' });
        out.push_str(k);
        out.push('=');
        for b in v.bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
                out.push(b as char);
            } else {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// The requests for a song, in order, at most three as the script does: by
/// name (with the album when there is one), by the cleaned names when they
/// differ, and a search. `duration` is whole seconds.
pub fn requests(artist: &str, title: &str, album: &str, duration: u32) -> Vec<String> {
    let d = duration.to_string();
    let mut first = vec![
        ("track_name", title),
        ("artist_name", artist),
        ("duration", d.as_str()),
    ];
    if !album.is_empty() {
        first.push(("album_name", album));
    }
    let mut out = vec![api_path("get", &first)];
    let (ca, ct) = (normalize_artist(artist), normalize_title(title));
    if ct != title || ca != artist {
        out.push(api_path(
            "get",
            &[("track_name", &ct), ("artist_name", &ca), ("duration", &d)],
        ));
    }
    out.push(api_path(
        "search",
        &[("track_name", &ct), ("artist_name", &ca)],
    ));
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Synced,
    Plain,
    Instrumental,
    Nothing,
}

/// What a lrclib.net record gives the island.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub kind: Kind,
    pub lines: Vec<Line>,
    pub offset: i64,
    pub id: u64,
}

impl Found {
    pub fn nothing() -> Self {
        Self::of(Kind::Nothing, 0)
    }

    fn of(kind: Kind, id: u64) -> Self {
        Self {
            kind,
            lines: Vec::new(),
            offset: 0,
            id,
        }
    }
}

fn synced_text(rec: &serde_json::Map<String, Value>) -> Option<&str> {
    rec.get("syncedLyrics")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && utf16_len(s) <= MAX_TEXT)
}

/// A record as JSON text; anything broken or too big gives nothing.
pub fn validate_text(json: &str, duration_sec: f64) -> Found {
    if json.len() > MAX_RECORD {
        return Found::nothing();
    }
    match serde_json::from_str::<Value>(json) {
        Ok(rec) => validate(&rec, duration_sec),
        Err(_) => Found::nothing(),
    }
}

pub fn validate(rec: &Value, duration_sec: f64) -> Found {
    let Some(rec) = rec.as_object() else {
        return Found::nothing();
    };
    let id = rec.get("id").and_then(Value::as_u64).unwrap_or(0);
    if let Some(d) = rec.get("duration").and_then(Value::as_f64)
        && d > 0.0
        && duration_sec > 0.0
        && (d - duration_sec).abs() > DURATION_SLACK
    {
        return Found::nothing();
    }
    if rec.get("instrumental") == Some(&Value::Bool(true)) {
        return Found::of(Kind::Instrumental, id);
    }
    if let Some(text) = synced_text(rec) {
        let lrc = parse_lrc(text);
        if !lrc.lines.is_empty() {
            return Found {
                kind: Kind::Synced,
                lines: lrc.lines,
                offset: lrc.offset,
                id,
            };
        }
    }
    if rec
        .get("plainLyrics")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
    {
        return Found::of(Kind::Plain, id);
    }
    Found::nothing()
}

/// The record of a `/api/search` answer with synced lyrics and the closest
/// duration, within [`DURATION_SLACK`].
pub fn pick_search(list: &Value, duration_sec: f64) -> Option<&Value> {
    let mut best = None;
    let mut best_gap = f64::INFINITY;
    for r in list.as_array()? {
        let Some(rec) = r.as_object() else { continue };
        if synced_text(rec).is_none() {
            continue;
        }
        let d = rec.get("duration").and_then(Value::as_f64).unwrap_or(0.0);
        let gap = (d - duration_sec).abs();
        if gap <= DURATION_SLACK && gap < best_gap {
            best = Some(r);
            best_gap = gap;
        }
    }
    best
}
