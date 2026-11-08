.pragma library

// The island's own agenda: plain events kept in
// ~/.local/state/omarchy/island/events.json, plus lines from an optional
// external command. Everything here is pure, so it can be tested outside
// Quickshell.
//
// Event: { id, date: "YYYY-MM-DD", time: "HH:MM" | "", title,
//          repeat: "none" | "weekly" | "monthly" | "yearly",
//          remind: minutes before | null (null = the island's reminderLead) }

var REPEATS = ["none", "weekly", "monthly", "yearly"]
var DATE_RE = /^(\d{4})-(\d{2})-(\d{2})$/
var TIME_RE = /^([01]?\d|2[0-3]):([0-5]\d)$/

function pad2(n) { return (n < 10 ? "0" : "") + n }

function isoDate(d) {
  return d.getFullYear() + "-" + pad2(d.getMonth() + 1) + "-" + pad2(d.getDate())
}

function parseDate(iso) {
  var m = DATE_RE.exec(String(iso || ""))
  if (!m) return null
  var d = new Date(parseInt(m[1], 10), parseInt(m[2], 10) - 1, parseInt(m[3], 10))
  return isoDate(d) === iso ? d : null
}

// "9:5" or "0905" style input → "09:05"; "" stays "" (all day); null when
// it can't be a time.
function normalizeTime(text) {
  var t = String(text || "").trim()
  if (t === "") return ""
  var compact = /^(\d{1,2})(\d{2})$/.exec(t)
  if (compact) t = compact[1] + ":" + compact[2]
  var m = TIME_RE.exec(t)
  return m ? pad2(parseInt(m[1], 10)) + ":" + m[2] : null
}

// A stored event made safe to use: anything malformed is dropped (null).
function clean(ev) {
  if (!ev || typeof ev !== "object") return null
  var title = String(ev.title || "").trim()
  if (title === "" || !parseDate(ev.date)) return null
  var time = normalizeTime(ev.time)
  var remind = parseInt(ev.remind, 10)
  return {
    id: String(ev.id || ""),
    date: String(ev.date),
    time: time === null ? "" : time,
    title: title,
    repeat: REPEATS.indexOf(ev.repeat) !== -1 ? ev.repeat : "none",
    remind: isNaN(remind) || remind < 0 ? null : remind,
    source: ev.source || "local"
  }
}

function cleanAll(list) {
  var out = []
  var src = Array.isArray(list) ? list : []
  for (var i = 0; i < src.length; i++) {
    var ev = clean(src[i])
    if (ev) out.push(ev)
  }
  return out
}

function newId(now) {
  return "e" + Number(now || Date.now()).toString(36) + Math.floor(Math.random() * 1296).toString(36)
}

// Whether `ev` happens on day `d` (a Date at local midnight).
function occursOn(ev, d) {
  var start = parseDate(ev.date)
  if (!start || d < start) return isoDate(d) === ev.date
  if (isoDate(d) === ev.date) return true
  switch (ev.repeat) {
    case "weekly": return d.getDay() === start.getDay()
    case "monthly": return d.getDate() === start.getDate()
    case "yearly": return d.getDate() === start.getDate() && d.getMonth() === start.getMonth()
  }
  return false
}

function byTime(a, b) {
  if (a.time === b.time) return a.title < b.title ? -1 : (a.title > b.title ? 1 : 0)
  if (a.time === "") return -1
  if (b.time === "") return 1
  return a.time < b.time ? -1 : 1
}

// Events on a day ("YYYY-MM-DD"), all-day first, then by time. Each is a
// copy carrying `on`, the day it is shown for.
function eventsOn(events, iso) {
  var d = parseDate(iso)
  if (!d) return []
  var out = []
  for (var i = 0; i < events.length; i++) {
    if (occursOn(events[i], d)) {
      var e = Object.assign({}, events[i])
      e.on = iso
      out.push(e)
    }
  }
  return out.sort(byTime)
}

// The next `days` days from `from` (a Date), skipping empty ones:
// [{ date: "YYYY-MM-DD", events: [...] }].
function upcoming(events, from, days) {
  var out = []
  for (var i = 0; i < days; i++) {
    var d = new Date(from.getFullYear(), from.getMonth(), from.getDate() + i)
    var list = eventsOn(events, isoDate(d))
    if (list.length > 0) out.push({ date: isoDate(d), events: list })
  }
  return out
}

// { "YYYY-MM-DD": count } for the 42 days a month grid shows.
function dotsForGrid(events, year, month, firstDow) {
  var first = new Date(year, month, 1)
  var qtFirst = first.getDay() === 0 ? 7 : first.getDay()
  var lead = (qtFirst - firstDow + 7) % 7
  var out = {}
  if (events.length === 0) return out
  for (var i = 0; i < 42; i++) {
    var d = new Date(year, month, 1 - lead + i)
    var n = 0
    for (var j = 0; j < events.length; j++) if (occursOn(events[j], d)) n++
    if (n > 0) out[isoDate(d)] = n
  }
  return out
}

// Lines from an external command, "YYYY-MM-DD<TAB>HH:MM<TAB>title" (time
// may be empty for all day). Anything else is ignored.
function parseExternal(text) {
  var out = []
  var lines = String(text || "").split("\n")
  for (var i = 0; i < lines.length && out.length < 500; i++) {
    var f = lines[i].split("\t")
    if (f.length < 3) continue
    var ev = clean({ id: "x" + i, date: f[0].trim(), time: f[1].trim(), title: f.slice(2).join(" "), source: "external" })
    if (ev) out.push(ev)
  }
  return out
}

// Minutes from `now` to the event's time on day `on`; null for all day.
function minutesUntil(ev, on, now) {
  if (!ev.time) return null
  var d = parseDate(on || ev.date)
  if (!d) return null
  var p = ev.time.split(":")
  d.setHours(parseInt(p[0], 10), parseInt(p[1], 10), 0, 0)
  return Math.round((d.getTime() - now.getTime()) / 60000)
}

// Events of today whose reminder is due by `now` (lead minutes
// before their time; lead from the event, else `defaultLead`). Each comes
// with a key so a reminder fires once.
function dueReminders(events, now, defaultLead) {
  var out = []
  var today = isoDate(now)
  var list = eventsOn(events, today)
  for (var i = 0; i < list.length; i++) {
    var ev = list[i]
    var mins = minutesUntil(ev, today, now)
    if (mins === null) continue
    var lead = ev.remind === null ? defaultLead : ev.remind
    // Inside the window rather than on the exact minute, so a reminder
    // isn't lost to a restart or a suspended laptop.
    if (mins >= 0 && mins <= lead)
      out.push({ key: today + "|" + ev.id + "|" + ev.time, event: ev, minutes: mins })
  }
  return out
}

// omarchy-reminder show --json → [{ title, at }] for the agenda.
function parseOmarchyReminders(json) {
  var out = []
  try {
    var data = typeof json === "string" ? JSON.parse(json) : json
    var list = data && Array.isArray(data.reminders) ? data.reminders : []
    for (var i = 0; i < list.length; i++) {
      var r = list[i]
      out.push({ title: String(r.message || r.label || ""), at: String(r.atTime || r.at || ""),
        remaining: String(r.remaining || "") })
    }
  } catch (e) { }
  return out
}
