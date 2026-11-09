.pragma library

// Synced lyrics from lrclib.net, as pure functions: parsing the LRC text,
// finding the line for a playback position, cleaning names for the fallback
// search, and reading what scripts/lyrics-fetch.sh prints. No imports, so it
// loads in node for the tests. Everything that comes from the network is
// untrusted: sizes are capped and text is cleaned before it reaches the UI.

var MAX_TEXT = 64 * 1024        // characters of syncedLyrics accepted
var MAX_LINES = 600
var MAX_LINE = 200              // characters per line once cleaned
var DURATION_SLACK = 3          // seconds, when choosing from /api/search
var NOTFOUND_RETRY = 24 * 3600 * 1000
var ERROR_RETRY = 10 * 60 * 1000

function str(v) { return typeof v === "string" ? v : "" }

// Same idea as AgentModel.cleanText, kept local so the two don't depend on
// each other.
function cleanLine(raw, max) {
  var limit = max || MAX_LINE
  var s = str(raw)
    .replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?/g, "")
    .replace(/\x1b\[[0-?]*[ -\/]*[@-~]/g, "")
    .replace(/\x1b[@-_]/g, "")
    .replace(/[\x00-\x1f\x7f-\x9f]/g, " ")
    .replace(/[\u200B-\u200F\u202A-\u202E\u2066-\u2069\uFEFF]/g, "")
    .replace(/\s+/g, " ")
    .trim()
  if (s.length > limit) {
    s = s.slice(0, limit)
    // Not in the middle of a surrogate pair.
    if (/[\ud800-\udbff]$/.test(s)) s = s.slice(0, -1)
    s = s.trim()
  }
  return s
}

// "mm:ss", "mm:ss.x", "mm:ss.xx", "mm:ss.xxx" or "h:mm:ss.xx" -> ms, or -1.
function parseTime(tag) {
  var m = /^(?:(\d{1,2}):)?(\d{1,3}):(\d{1,2})(?:[.:](\d{1,3}))?$/.exec(str(tag).trim())
  if (!m) return -1
  var h = m[1] === undefined ? 0 : parseInt(m[1], 10)
  var min = parseInt(m[2], 10)
  var sec = parseInt(m[3], 10)
  if (sec > 59 || (m[1] !== undefined && min > 59)) return -1
  var frac = m[4] === undefined ? 0 : Math.round(parseFloat("0." + m[4]) * 1000)
  return ((h * 60 + min) * 60 + sec) * 1000 + frac
}

var META = /^(ar|ti|al|au|by|length|re|ve|tool|la|id|#)$/i

// -> { lines: [{ t: ms, text }], offset: ms, count }
function parseLrc(text) {
  var out = { lines: [], offset: 0, count: 0 }
  if (typeof text !== "string" || text === "") return out
  if (text.length > MAX_TEXT) text = text.slice(0, MAX_TEXT)
  var rows = text.replace(/^\uFEFF/, "").split(/\r\n|\r|\n/)
  var lines = []
  var offset = 0
  for (var i = 0; i < rows.length && lines.length < MAX_LINES; i++) {
    var row = rows[i]
    var stamps = []
    var rest = row
    var m
    // Leading [..] tags: time stamps, offset or metadata.
    while ((m = /^\s*\[([^\]]*)\]/.exec(rest)) !== null) {
      rest = rest.slice(m[0].length)
      var tag = m[1]
      var t = parseTime(tag)
      if (t >= 0) { stamps.push(t); continue }
      var kv = /^\s*([A-Za-z#]+)\s*:\s*(.*)$/.exec(tag)
      if (kv && kv[1].toLowerCase() === "offset") {
        var n = parseInt(kv[2], 10)
        if (isFinite(n)) offset = Math.max(-600000, Math.min(600000, n))
      } else if (!(kv && META.test(kv[1]))) {
        // Not a tag we know: it is part of the text; stop reading tags.
        rest = m[0] + rest
        break
      }
    }
    if (stamps.length === 0) continue
    // Enhanced LRC word stamps are dropped from the text.
    var txt = cleanLine(rest.replace(/<\d{1,3}:\d{1,2}(?:[.:]\d{1,3})?>/g, ""))
    for (var s = 0; s < stamps.length && lines.length < MAX_LINES; s++)
      lines.push({ t: stamps[s], text: txt, i: lines.length })
  }
  // A positive offset shifts the lyrics earlier (LRC convention).
  lines.sort(function(a, b) { return a.t - b.t || a.i - b.i })
  out.offset = offset
  out.lines = lines.map(function(l) { return { t: Math.max(0, l.t - offset), text: l.text } })
  out.count = out.lines.length
  return out
}

// Index of the last line starting at or before posMs; -1 before the first.
function lineAt(lines, posMs) {
  var lo = 0, hi = lines.length - 1, found = -1
  while (lo <= hi) {
    var mid = (lo + hi) >> 1
    if (lines[mid].t <= posMs) { found = mid; lo = mid + 1 } else hi = mid - 1
  }
  return found
}

// What to show at posMs, and when it changes next (-1: never).
function window(lines, posMs) {
  var i = lineAt(lines, posMs)
  var next = ""
  for (var j = i + 1; j < lines.length; j++) {
    if (lines[j].text !== "") { next = lines[j].text; break }
  }
  return {
    index: i,
    current: i >= 0 ? lines[i].text : "",
    next: next,
    nextAt: i + 1 < lines.length ? lines[i + 1].t : -1
  }
}

// For the fallback search only: drops what streaming services append and
// lrclib doesn't have. "(Live)", "(Acoustic)", "(Radio Edit)" stay: they are
// different recordings.
var TITLE_JUNK = [
  /\s+-\s+(?:\d{4}\s+)?(?:digital\s+)?remaster(?:ed)?(?:\s+(?:version|\d{4}))?\s*$/i,
  /\s+-\s+(?:bonus\s+track|single\s+version)\s*$/i,
  /\s*[(\[]\s*(?:\d{4}\s+)?(?:digital\s+)?remaster(?:ed)?(?:\s+(?:version|\d{4}))?\s*[)\]]/ig,
  /\s*[(\[]\s*(?:feat\.?|ft\.?|featuring|with)\s+[^)\]]*[)\]]/ig,
  /\s*[(\[]\s*(?:official\s+)?(?:music\s+video|video|audio|lyric\s+video|lyrics|visuali[sz]er)\s*[)\]]/ig,
  /\s*[(\[]\s*(?:hd|hq|4k)\s*[)\]]/ig,
  /\s*[(\[]\s*(?:bonus\s+track|deluxe[^)\]]*|explicit)\s*[)\]]/ig,
  /\s+(?:feat\.?|ft\.?|featuring)\s+.+$/i
]

function normalizeTitle(raw) {
  var s = cleanLine(raw)
  for (var i = 0; i < TITLE_JUNK.length; i++) s = s.replace(TITLE_JUNK[i], "")
  s = s.replace(/\s+/g, " ").trim()
  return s === "" ? cleanLine(raw) : s
}

// The first artist of a list.
function normalizeArtist(raw) {
  var s = cleanLine(raw)
  var first = s.split(/\s*(?:,|;|&|\s+x\s+|\s+(?:feat\.?|ft\.?|featuring|with)\s+)\s*/i)[0].trim()
  return first === "" ? s : first
}

// What lyrics-fetch.sh hashes (sha256) to name the cache file.
function cacheKey(artist, title, album, durationSec) {
  function norm(v) { return cleanLine(v, 400).toLowerCase() }
  return [norm(artist), norm(title), norm(album), String(Math.round(Number(durationSec) || 0))].join("\n")
}

// A lrclib record (object or JSON text) -> what to do with it.
// { kind: "synced" | "plain" | "instrumental" | "none", lines, offset, id }
function validate(json, durationSec) {
  var none = { kind: "none", lines: [], offset: 0, id: 0 }
  var rec = json
  if (typeof rec === "string") {
    if (rec.length > 262144) return none
    try { rec = JSON.parse(rec) } catch (e) { return none }
  }
  if (rec === null || typeof rec !== "object" || Array.isArray(rec)) return none
  var id = typeof rec.id === "number" && isFinite(rec.id) ? rec.id : 0
  var d = Number(rec.duration)
  if (isFinite(d) && d > 0 && durationSec > 0 && Math.abs(d - durationSec) > DURATION_SLACK) return none
  if (rec.instrumental === true) return { kind: "instrumental", lines: [], offset: 0, id: id }
  if (typeof rec.syncedLyrics === "string" && rec.syncedLyrics !== "" && rec.syncedLyrics.length <= MAX_TEXT) {
    var p = parseLrc(rec.syncedLyrics)
    if (p.count > 0) return { kind: "synced", lines: p.lines, offset: p.offset, id: id }
  }
  if (typeof rec.plainLyrics === "string" && rec.plainLyrics.trim() !== "")
    return { kind: "plain", lines: [], offset: 0, id: id }
  return none
}

// The record of /api/search with synced lyrics and the closest duration.
function pickSearch(list, durationSec) {
  if (!Array.isArray(list)) return null
  var best = null, bestGap = 1e9
  for (var i = 0; i < list.length; i++) {
    var r = list[i]
    if (r === null || typeof r !== "object") continue
    if (typeof r.syncedLyrics !== "string" || r.syncedLyrics === "" || r.syncedLyrics.length > MAX_TEXT) continue
    var gap = Math.abs((Number(r.duration) || 0) - durationSec)
    if (gap <= DURATION_SLACK && gap < bestGap) { best = r; bestGap = gap }
  }
  return best
}

// What lyrics-fetch.sh prints: a status line, then the JSON (if any).
// status: hit | get | search | notfound | error <code> | nocache <status>
// -> { status, result, retryMs }
function parseFetch(text, durationSec) {
  var s = str(text)
  var nl = s.indexOf("\n")
  var head = (nl < 0 ? s : s.slice(0, nl)).trim()
  var body = nl < 0 ? "" : s.slice(nl + 1)
  var m = /^nocache\s+(.*)$/.exec(head)
  if (m) head = m[1].trim()
  var word = head.split(/\s+/)[0]
  // A search answer is cached as it came: a hit can hold an array too.
  var isList = /^\s*\[/.test(body.slice(0, 200))
  if ((word === "hit" || word === "get") && !isList) {
    return { status: word, result: validate(body, durationSec), retryMs: 0 }
  }
  if (word === "search" || word === "hit") {
    var list = null
    try { list = JSON.parse(body) } catch (e) { list = null }
    var rec = pickSearch(list, durationSec)
    if (rec) return { status: "search", result: validate(rec, durationSec), retryMs: 0 }
    return { status: "notfound", result: null, retryMs: NOTFOUND_RETRY }
  }
  if (word === "notfound") return { status: "notfound", result: null, retryMs: NOTFOUND_RETRY }
  return { status: "error", result: null, retryMs: ERROR_RETRY }
}
