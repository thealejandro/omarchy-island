.pragma library

// Calendars in iCalendar (RFC 5545) form: .ics files (vdirsyncer, Thunderbird
// exports…) and the private iCal links that Google Calendar, Outlook, iCloud
// and others give out. parse() turns that text into the agenda's own events
// (AgendaModel's shape, source "ics"), one per day an event happens on inside
// a window around now, so repeats are already worked out.
//
// What it understands: VEVENT with SUMMARY, DTSTART/DTEND (dates, UTC, local
// and TZID times), STATUS:CANCELLED, RRULE (DAILY/WEEKLY/MONTHLY/YEARLY with
// INTERVAL, COUNT, UNTIL, BYDAY, BYMONTHDAY, BYMONTH), RDATE, EXDATE and
// RECURRENCE-ID. Anything else is left out, never an error.
//
// Time zones: the loader appends "X-ISLAND-TZ:<zone>;<epoch>;<±hhmm,…>" lines
// with a zone's offset day by day (from the system's tzdata), and failing
// that a calendar's own VTIMEZONE is used (Outlook's "Romance Standard
// Time" and the like). A time in a zone it can't place is read as local.

var MAX_EVENTS = 2000
var MAX_ITER = 20000
var MAX_SPAN_DAYS = 31
var DAY_MS = 86400000
var WEEKDAYS = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"]

function pad2(n) { return (n < 10 ? "0" : "") + n }
function isoOf(d) { return d.getFullYear() + "-" + pad2(d.getMonth() + 1) + "-" + pad2(d.getDate()) }

// Long lines are folded onto the next one, which starts with a space or tab.
function unfold(text) {
  return String(text || "").replace(/\r\n?/g, "\n").replace(/\n[ \t]/g, "")
}

// "DTSTART;TZID=Europe/Madrid:20261110T090000" →
// { name: "DTSTART", params: { TZID: "Europe/Madrid" }, value: "20261110T090000" }
function parseLine(line) {
  var colon = -1, quoted = false
  for (var i = 0; i < line.length; i++) {
    var c = line.charAt(i)
    if (c === '"') quoted = !quoted
    else if (c === ":" && !quoted) { colon = i; break }
  }
  if (colon < 1) return null
  var head = line.slice(0, colon).split(";")
  var params = {}
  for (var k = 1; k < head.length; k++) {
    var eq = head[k].indexOf("=")
    if (eq > 0) params[head[k].slice(0, eq).toUpperCase()] = head[k].slice(eq + 1).replace(/^"|"$/g, "")
  }
  return { name: head[0].toUpperCase(), params: params, value: line.slice(colon + 1) }
}

function unescapeText(v) {
  return String(v).replace(/\\([nN,;\\])/g, function(_, c) { return c === "n" || c === "N" ? " " : c })
    .replace(/\s+/g, " ").trim()
}

// A DATE or DATE-TIME value: { y, m, d, h, mi, allDay, utc, tzid } in the
// event's own wall clock; null when it isn't one.
function parseStamp(value, params) {
  var m = /^(\d{4})(\d{2})(\d{2})(?:T(\d{2})(\d{2})(\d{2})?(Z)?)?$/.exec(String(value || "").trim())
  if (!m) return null
  var st = { y: +m[1], m: +m[2], d: +m[3], h: 0, mi: 0, allDay: m[4] === undefined, utc: m[7] === "Z",
    tzid: params && params.TZID ? params.TZID : "" }
  if (params && params.VALUE === "DATE") st.allDay = true
  if (!st.allDay) { st.h = +m[4]; st.mi = +m[5] }
  if (st.m < 1 || st.m > 12 || st.d < 1 || st.d > 31 || st.h > 23 || st.mi > 59) return null
  return st
}

function stampsOf(prop) {
  var out = []
  if (!prop) return out
  var parts = prop.value.split(",")
  for (var i = 0; i < parts.length; i++) {
    var st = parseStamp(parts[i], prop.params)
    if (st) out.push(st)
  }
  return out
}

// ---------------------------------------------------------------- zones

// "X-ISLAND-TZ:Europe/Madrid;1731153600;+0100,+0100,…" lines → { zone: {start, offsets} }
function parseProbes(text) {
  var map = {}
  var re = /^X-ISLAND-TZ:([^;\n]+);(\d+);([+\-\d,]+)$/gm
  var m
  while ((m = re.exec(text)) !== null) {
    var offs = m[3].split(",").map(function(o) {
      var p = /^([+-])(\d{2})(\d{2})$/.exec(o)
      return p ? (p[1] === "-" ? -1 : 1) * (parseInt(p[2], 10) * 60 + parseInt(p[3], 10)) : 0
    })
    if (offs.length) map[m[1]] = { start: parseInt(m[2], 10) * 1000, offsets: offs }
  }
  return map
}

function offsetMinutes(text) {
  var p = /^([+-])(\d{2})(\d{2})/.exec(String(text || ""))
  return p ? (p[1] === "-" ? -1 : 1) * (parseInt(p[2], 10) * 60 + parseInt(p[3], 10)) : null
}

// The day of the nth (negative: from the end) weekday wd in a month.
function nthWeekday(y, m, wd, n) {
  if (n > 0) {
    var first = new Date(Date.UTC(y, m - 1, 1)).getUTCDay()
    var day = 1 + ((wd - first + 7) % 7) + (n - 1) * 7
    return day <= daysIn(y, m) ? day : 0
  }
  var lastDay = daysIn(y, m)
  var last = new Date(Date.UTC(y, m - 1, lastDay)).getUTCDay()
  var back = lastDay - ((last - wd + 7) % 7) + (n + 1) * 7
  return back >= 1 ? back : 0
}

function daysIn(y, m) { return new Date(Date.UTC(y, m, 0)).getUTCDate() }

// A VTIMEZONE's offset (minutes east of UTC) at a wall-clock moment, from
// its STANDARD/DAYLIGHT parts and their yearly rules.
function vtzOffset(parts, wallMs) {
  var y = new Date(wallMs).getUTCFullYear()
  var best = null, bestAt = -Infinity, first = null
  for (var i = 0; i < parts.length; i++) {
    var p = parts[i]
    if (!p.start || p.to === null) continue
    if (!first || Date.UTC(p.start.y, p.start.m - 1, p.start.d) < Date.UTC(first.start.y, first.start.m - 1, first.start.d)) first = p
    var times = []
    var rule = p.rrule
    if (rule && rule.FREQ === "YEARLY" && rule.BYMONTH) {
      var by = /^(-?\d)?(SU|MO|TU|WE|TH|FR|SA)$/.exec(rule.BYDAY || "")
      for (var yy = y - 1; yy <= y; yy++) {
        var day = by ? nthWeekday(yy, +rule.BYMONTH, WEEKDAYS.indexOf(by[2]), by[1] ? +by[1] : 1) : p.start.d
        if (day) times.push(Date.UTC(yy, +rule.BYMONTH - 1, day, p.start.h, p.start.mi))
      }
    } else {
      times.push(Date.UTC(p.start.y, p.start.m - 1, p.start.d, p.start.h, p.start.mi))
    }
    for (var t = 0; t < times.length; t++) {
      if (times[t] <= wallMs && times[t] > bestAt) { bestAt = times[t]; best = p }
    }
  }
  if (best) return best.to
  return first ? (first.from !== null ? first.from : first.to) : null
}

// Wall-clock components in a zone → milliseconds since the epoch.
function instantOf(st, h, mi, zones) {
  var wall = Date.UTC(st.y, st.m - 1, st.d, h, mi)
  if (st.utc) return wall
  var tz = String(st.tzid || "").replace(/^"|"$/g, "")
  if (tz !== "") {
    var pr = zones.probes[tz]
    if (pr) {
      var idx = Math.floor((wall - pr.start) / DAY_MS)
      idx = Math.max(0, Math.min(pr.offsets.length - 1, idx))
      return wall - pr.offsets[idx] * 60000
    }
    var vt = zones.vtimezones[tz]
    if (vt) {
      var off = vtzOffset(vt, wall)
      if (off !== null) return wall - off * 60000
    }
  }
  // Floating (or a zone we can't place): local time.
  return new Date(st.y, st.m - 1, st.d, h, mi).getTime()
}

// ---------------------------------------------------------------- rules

function parseRule(value) {
  var rule = {}
  var parts = String(value || "").split(";")
  for (var i = 0; i < parts.length; i++) {
    var eq = parts[i].indexOf("=")
    if (eq > 0) rule[parts[i].slice(0, eq).toUpperCase()] = parts[i].slice(eq + 1).toUpperCase()
  }
  return rule
}

// Rules we can follow exactly; anything else keeps only the first time.
function supported(rule) {
  if (["DAILY", "WEEKLY", "MONTHLY", "YEARLY"].indexOf(rule.FREQ) === -1) return false
  for (var k in rule) {
    if (["FREQ", "INTERVAL", "COUNT", "UNTIL", "BYDAY", "BYMONTHDAY", "BYMONTH", "WKST"].indexOf(k) === -1) return false
  }
  return true
}

function byDays(rule) {
  var out = []
  if (!rule.BYDAY) return out
  var list = rule.BYDAY.split(",")
  for (var i = 0; i < list.length; i++) {
    var m = /^([+-]?\d{1,2})?(SU|MO|TU|WE|TH|FR|SA)$/.exec(list[i])
    if (m) out.push({ n: m[1] ? parseInt(m[1], 10) : 0, wd: WEEKDAYS.indexOf(m[2]) })
  }
  return out
}

function intList(text) {
  var out = []
  var parts = String(text || "").split(",")
  for (var i = 0; i < parts.length; i++) {
    var n = parseInt(parts[i], 10)
    if (!isNaN(n) && n !== 0) out.push(n)
  }
  return out
}

// The days of a month a monthly (or yearly) rule picks, in order.
function daysInMonthFor(rule, y, m, startDay) {
  var len = daysIn(y, m)
  var days = []
  var bd = byDays(rule)
  var md = intList(rule.BYMONTHDAY)
  if (md.length) {
    for (var i = 0; i < md.length; i++) {
      var d = md[i] > 0 ? md[i] : len + md[i] + 1
      if (d >= 1 && d <= len) days.push(d)
    }
  } else if (bd.length) {
    for (var k = 0; k < bd.length; k++) {
      if (bd[k].n !== 0) {
        var nd = nthWeekday(y, m, bd[k].wd, bd[k].n)
        if (nd) days.push(nd)
      } else {
        for (var dd = 1; dd <= len; dd++) if (new Date(Date.UTC(y, m - 1, dd)).getUTCDay() === bd[k].wd) days.push(dd)
      }
    }
  } else if (startDay <= len) {
    days.push(startDay)
  }
  if (md.length && bd.length) {
    // Both: a day has to match the two.
    var wds = bd.map(function(b) { return b.wd })
    days = days.filter(function(d) { return wds.indexOf(new Date(Date.UTC(y, m - 1, d)).getUTCDay()) !== -1 })
  }
  days.sort(function(a, b) { return a - b })
  return days.filter(function(d, i) { return i === 0 || days[i - 1] !== d })
}

// The wall-clock dates ({y, m, d}) a rule gives from st, in order, until
// `limitWall` (a UTC-based wall time), `until` or COUNT.
function expandDates(st, rule, fromWall, limitWall, untilFn) {
  var out = []
  var interval = Math.max(1, parseInt(rule.INTERVAL, 10) || 1)
  var count = parseInt(rule.COUNT, 10)
  if (isNaN(count) || count < 1) count = Infinity
  var startWall = Date.UTC(st.y, st.m - 1, st.d)
  var seen = 0
  function push(y, m, d) {
    var w = Date.UTC(y, m - 1, d)
    if (w < startWall) return true
    if (w > limitWall || untilFn({ y: y, m: m, d: d })) return false
    seen++
    out.push({ y: y, m: m, d: d })
    return seen < count
  }
  var months = intList(rule.BYMONTH).filter(function(n) { return n >= 1 && n <= 12 })
  // Without COUNT, a daily or weekly rule started years ago can skip
  // straight to the window.
  var k0 = 0
  if (count === Infinity && (rule.FREQ === "DAILY" || rule.FREQ === "WEEKLY")) {
    var period = interval * (rule.FREQ === "DAILY" ? 1 : 7) * DAY_MS
    k0 = Math.max(0, Math.floor((fromWall - startWall) / period) - 1)
  }
  for (var k = k0; k < k0 + MAX_ITER; k++) {
    var go = true
    if (rule.FREQ === "DAILY") {
      var dd = new Date(startWall + k * interval * DAY_MS)
      if (months.length && months.indexOf(dd.getUTCMonth() + 1) === -1) continue
      go = push(dd.getUTCFullYear(), dd.getUTCMonth() + 1, dd.getUTCDate())
      if (dd.getTime() > limitWall) go = false
    } else if (rule.FREQ === "WEEKLY") {
      // Weeks start on Monday (the default WKST).
      var monday = startWall - ((new Date(startWall).getUTCDay() + 6) % 7) * DAY_MS + k * interval * 7 * DAY_MS
      if (monday > limitWall) break
      var bd = byDays(rule)
      var wds = bd.length ? bd.map(function(b) { return b.wd }) : [new Date(startWall).getUTCDay()]
      wds.sort(function(a, b) { return ((a + 6) % 7) - ((b + 6) % 7) })
      for (var i = 0; i < wds.length && go; i++) {
        var day = new Date(monday + ((wds[i] + 6) % 7) * DAY_MS)
        go = push(day.getUTCFullYear(), day.getUTCMonth() + 1, day.getUTCDate())
      }
    } else if (rule.FREQ === "MONTHLY") {
      var mIndex = (st.m - 1) + k * interval
      var y = st.y + Math.floor(mIndex / 12), m = (mIndex % 12) + 1
      if (Date.UTC(y, m - 1, 1) > limitWall) break
      if (months.length && months.indexOf(m) === -1) continue
      var days = daysInMonthFor(rule, y, m, st.d)
      for (var j = 0; j < days.length && go; j++) go = push(y, m, days[j])
    } else if (rule.FREQ === "YEARLY") {
      var yr = st.y + k * interval
      if (Date.UTC(yr, 0, 1) > limitWall) break
      var ms = months.length ? months : [st.m]
      for (var q = 0; q < ms.length && go; q++) {
        var r = rule
        // Without BYMONTHDAY/BYDAY a yearly rule keeps the start day; with
        // BYDAY and no BYMONTH it would mean every such weekday of the year,
        // which nobody uses: we read it within the start month.
        var dl = daysInMonthFor(r, yr, ms[q], st.d)
        for (var z = 0; z < dl.length && go; z++) go = push(yr, ms[q], dl[z])
      }
    }
    if (!go) break
  }
  return out
}

// ---------------------------------------------------------------- parse

function hash(s) {
  var h = 5381
  for (var i = 0; i < s.length; i++) h = ((h * 33) ^ s.charCodeAt(i)) >>> 0
  return h.toString(36)
}

// Calendar text (one or more VCALENDARs, plus the loader's zone lines) →
// agenda events between `from` and `to` (Dates; default: a year either side
// of now), at most `limit` (the ones closest to now when there are more).
function parse(text, opts) {
  opts = opts || {}
  var now = opts.now ? new Date(opts.now) : new Date()
  var from = opts.from ? new Date(opts.from).getTime() : now.getTime() - 366 * DAY_MS
  var to = opts.to ? new Date(opts.to).getTime() : now.getTime() + 366 * DAY_MS
  var limit = opts.limit || MAX_EVENTS
  var src = unfold(text)
  var zones = { probes: parseProbes(src), vtimezones: {} }
  var vevents = []
  try {
    var lines = src.split("\n")
    var stack = []
    var ev = null, tz = null, tzPart = null
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i]
      if (line === "") continue
      var p = parseLine(line)
      if (!p) continue
      if (p.name === "BEGIN") {
        var comp = p.value.trim().toUpperCase()
        stack.push(comp)
        if (comp === "VEVENT" && stack.length >= 2) ev = { exdates: [], rdates: [] }
        else if (comp === "VTIMEZONE") tz = { id: "", parts: [] }
        else if (tz && (comp === "STANDARD" || comp === "DAYLIGHT")) tzPart = { start: null, from: null, to: null, rrule: null }
        continue
      }
      if (p.name === "END") {
        var done = stack.pop()
        if (done === "VEVENT" && ev) { vevents.push(ev); ev = null }
        else if (done === "VTIMEZONE" && tz) { if (tz.id && !zones.vtimezones[tz.id]) zones.vtimezones[tz.id] = tz.parts; tz = null }
        else if ((done === "STANDARD" || done === "DAYLIGHT") && tz && tzPart) { tz.parts.push(tzPart); tzPart = null }
        continue
      }
      var top = stack[stack.length - 1]
      if (ev && top === "VEVENT") {
        if (p.name === "UID") ev.uid = p.value.trim()
        else if (p.name === "SUMMARY") ev.summary = unescapeText(p.value)
        else if (p.name === "DTSTART") ev.start = parseStamp(p.value, p.params)
        else if (p.name === "DTEND") ev.end = parseStamp(p.value, p.params)
        else if (p.name === "DURATION") ev.duration = p.value.trim()
        else if (p.name === "STATUS") ev.status = p.value.trim().toUpperCase()
        else if (p.name === "RRULE") ev.rrule = parseRule(p.value)
        else if (p.name === "EXDATE") ev.exdates = ev.exdates.concat(stampsOf(p))
        else if (p.name === "RDATE" && p.params.VALUE !== "PERIOD") ev.rdates = ev.rdates.concat(stampsOf(p))
        else if (p.name === "RECURRENCE-ID") ev.recurrenceId = parseStamp(p.value, p.params)
      } else if (tz && top === "VTIMEZONE" && p.name === "TZID") {
        tz.id = p.value.trim()
      } else if (tzPart && (top === "STANDARD" || top === "DAYLIGHT")) {
        if (p.name === "DTSTART") tzPart.start = parseStamp(p.value, {})
        else if (p.name === "TZOFFSETFROM") tzPart.from = offsetMinutes(p.value)
        else if (p.name === "TZOFFSETTO") tzPart.to = offsetMinutes(p.value)
        else if (p.name === "RRULE") tzPart.rrule = parseRule(p.value)
      }
    }
  } catch (e) { }

  // Moved or cancelled single occurrences: the master skips them.
  var overridden = {}
  for (var o = 0; o < vevents.length; o++) {
    var ov = vevents[o]
    if (ov.recurrenceId && ov.uid) {
      var rid = ov.recurrenceId
      var ridKey = rid.allDay ? "d" + Date.UTC(rid.y, rid.m - 1, rid.d) : "t" + instantOf(rid, rid.h, rid.mi, zones)
      overridden[ov.uid + "|" + ridKey] = true
    }
  }

  var out = []
  var guard = 0
  for (var v = 0; v < vevents.length && guard < limit * 4; v++) {
    var e = vevents[v]
    try {
      if (!e.start || !e.summary || e.status === "CANCELLED") continue
      var occ = occurrences(e, zones, from, to)
      var uid = e.uid || (e.summary + "|" + e.start.y + e.start.m + e.start.d)
      for (var n = 0; n < occ.length; n++) {
        var oc = occ[n]
        if (!e.recurrenceId && overridden[uid + "|" + oc.key]) continue
        var days = oc.allDay ? oc.span : 1
        for (var s = 0; s < days; s++) {
          var d = oc.allDay ? new Date(oc.y, oc.m - 1, oc.d + s) : new Date(oc.at)
          var at = d.getTime()
          if (at < from - DAY_MS || at > to) continue
          var iso = isoOf(d)
          var time = oc.allDay ? "" : pad2(d.getHours()) + ":" + pad2(d.getMinutes())
          out.push({ id: "i" + hash(uid) + "-" + iso.replace(/-/g, "") + time.replace(":", ""),
            date: iso, time: time, title: e.summary, repeat: "none", remind: null, source: "ics", _at: at })
          guard++
        }
      }
    } catch (err) { }
  }

  // Same calendar twice (two sources) or a moved event left in place.
  var seenIds = {}
  out = out.filter(function(x) { if (seenIds[x.id]) return false; seenIds[x.id] = true; return true })
  if (out.length > limit) {
    var t0 = now.getTime()
    out.sort(function(a, b) { return Math.abs(a._at - t0) - Math.abs(b._at - t0) })
    out = out.slice(0, limit)
  }
  out.sort(function(a, b) { return a._at - b._at })
  for (var c = 0; c < out.length; c++) delete out[c]._at
  return out
}

// Every time an event happens: { allDay, y, m, d, span, at, key }.
function occurrences(e, zones, from, to) {
  var st = e.start
  var span = 1
  if (st.allDay && e.end && e.end.allDay) {
    span = Math.round((Date.UTC(e.end.y, e.end.m - 1, e.end.d) - Date.UTC(st.y, st.m - 1, st.d)) / DAY_MS)
    span = Math.max(1, Math.min(MAX_SPAN_DAYS, span))
  }
  function make(day) {
    if (st.allDay) {
      return { allDay: true, y: day.y, m: day.m, d: day.d, span: span, at: new Date(day.y, day.m - 1, day.d).getTime(),
        key: "d" + Date.UTC(day.y, day.m - 1, day.d) }
    }
    var at = instantOf({ y: day.y, m: day.m, d: day.d, utc: st.utc, tzid: st.tzid }, st.h, st.mi, zones)
    return { allDay: false, at: at, key: "t" + at }
  }
  var list = []
  // Generous on the wall-clock side; the exact window is applied later.
  var limitWall = to + 2 * DAY_MS
  if (e.rrule && !e.recurrenceId && supported(e.rrule)) {
    var until = null
    if (e.rrule.UNTIL) {
      var u = parseStamp(e.rrule.UNTIL, {})
      if (u) until = u.allDay ? { allDay: true, w: Date.UTC(u.y, u.m - 1, u.d) } : { allDay: false, t: instantOf(u, u.h, u.mi, zones) }
    }
    var untilFn = function(day) {
      if (!until) return false
      if (until.allDay || st.allDay) return Date.UTC(day.y, day.m - 1, day.d) > (until.allDay ? until.w : Date.UTC(new Date(until.t).getUTCFullYear(), new Date(until.t).getUTCMonth(), new Date(until.t).getUTCDate()))
      return make(day).at > until.t
    }
    var dates = expandDates(st, e.rrule, from - (MAX_SPAN_DAYS + 2) * DAY_MS, limitWall, untilFn)
    for (var i = 0; i < dates.length; i++) list.push(make(dates[i]))
  } else {
    list.push(make(st))
  }
  for (var r = 0; r < e.rdates.length; r++) list.push(make(e.rdates[r]))

  var ex = {}
  for (var x = 0; x < e.exdates.length; x++) {
    var xd = e.exdates[x]
    ex[xd.allDay ? "d" + Date.UTC(xd.y, xd.m - 1, xd.d) : "t" + instantOf(xd, xd.h, xd.mi, zones)] = true
    // An all-day event excluded by a date-time, or the other way round.
    ex["d" + Date.UTC(xd.y, xd.m - 1, xd.d)] = ex["d" + Date.UTC(xd.y, xd.m - 1, xd.d)] || st.allDay
  }
  var lowest = from - (MAX_SPAN_DAYS + 1) * DAY_MS
  return list.filter(function(o) { return !ex[o.key] && o.at >= lowest && o.at <= to })
}
